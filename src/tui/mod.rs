use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub mod agents_app;
pub mod app;
pub mod event;
pub mod ui;

use crate::agents::{
    load_agents, AgentEntry, AsyncGitHubClient, ConditionalFetchResult, GitHubCache, GitHubData,
};
use crate::config::Config;
use crate::data::ProvidersMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Copy text to clipboard, keeping it alive on Linux.
/// On Linux, the clipboard is selection-based and needs the source app to stay alive.
/// We spawn a thread to hold the clipboard for a few seconds.
fn copy_to_clipboard(text: String) {
    std::thread::spawn(move || {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(&text);
            // Keep clipboard alive for other apps to read on Linux
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    });
}

/// Result of a GitHub fetch operation for an agent.
#[derive(Debug)]
pub enum FetchResult {
    /// Successful fetch: (agent_id, github_data)
    Success(String, GitHubData),
    /// Failed fetch: (agent_id, error_message)
    Failure(String, String),
}

/// Spawn background GitHub fetches for tracked agent entries only.
/// Returns a receiver and the join handles for cleanup.
#[allow(dead_code)]
fn spawn_github_fetches(
    entries: &[AgentEntry],
    client: AsyncGitHubClient,
) -> (mpsc::Receiver<FetchResult>, Vec<JoinHandle<()>>) {
    let (tx, rx) = mpsc::channel(100);
    let tracked_entries: Vec<_> = entries.iter().filter(|e| e.tracked).collect();
    let mut handles = Vec::with_capacity(tracked_entries.len());

    for entry in tracked_entries {
        let tx = tx.clone();
        let client = client.clone();
        let id = entry.id.clone();
        let repo = entry.agent.repo.clone();

        let handle = tokio::spawn(async move {
            let result = match client.fetch(&repo).await {
                Ok(data) => FetchResult::Success(id, data),
                Err(e) => FetchResult::Failure(id, e.to_string()),
            };
            let _ = tx.send(result).await;
        });
        handles.push(handle);
    }

    (rx, handles)
}

/// Spawn background GitHub fetches using conditional requests (ETag-based).
/// Uses cached data when GitHub returns 304 Not Modified.
/// Returns a receiver and the join handles for cleanup.
fn spawn_github_fetches_conditional(
    entries: &[AgentEntry],
    client: AsyncGitHubClient,
    disk_cache: Arc<RwLock<GitHubCache>>,
) -> (mpsc::Receiver<FetchResult>, Vec<JoinHandle<()>>) {
    let (tx, rx) = mpsc::channel(100);
    let tracked_entries: Vec<_> = entries.iter().filter(|e| e.tracked).collect();
    let mut handles = Vec::with_capacity(tracked_entries.len());

    for entry in tracked_entries {
        let tx = tx.clone();
        let client = client.clone();
        let id = entry.id.clone();
        let repo = entry.agent.repo.clone();
        let cache = disk_cache.clone();

        let handle = tokio::spawn(async move {
            let result = match client.fetch_conditional(&repo).await {
                ConditionalFetchResult::Fresh(data, _etag) => FetchResult::Success(id, data),
                ConditionalFetchResult::NotModified => {
                    // Retrieve cached data and send as success
                    let cache_guard = cache.read().await;
                    if let Some(cached) = cache_guard.get(&repo) {
                        FetchResult::Success(id, cached.data.clone().into())
                    } else {
                        // Cache miss despite NotModified - shouldn't happen, treat as error
                        FetchResult::Failure(
                            id,
                            "Cache miss on NotModified response".to_string(),
                        )
                    }
                }
                ConditionalFetchResult::Error(e) => FetchResult::Failure(id, e),
            };
            let _ = tx.send(result).await;
        });
        handles.push(handle);
    }

    (rx, handles)
}

pub async fn run(providers: ProvidersMap) -> Result<()> {
    use crate::agents::FetchStatus;

    // Load remaining data
    let agents_file = load_agents().ok();
    let config = Config::load().ok();

    // Load disk cache for GitHub data (load before wrapping to avoid blocking in async)
    let disk_cache = GitHubCache::load();

    // Install panic hook to restore terminal on crash
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Restore terminal before printing panic message
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = app::App::new(providers, agents_file.as_ref(), config);

    // Pre-populate agent entries from disk cache for instant display
    if let Some(ref mut agents_app) = app.agents_app {
        for entry in &mut agents_app.entries {
            if entry.tracked {
                // Look up cached data by repo (cache keys are repos)
                if let Some(cached) = disk_cache.get(&entry.agent.repo) {
                    entry.github = cached.data.clone().into();
                    entry.fetch_status = FetchStatus::Loaded;
                }
            }
        }
        // Re-apply sorting after populating cache data (in case sorted by stars/updated)
        agents_app.apply_sort();
    }

    // Now wrap cache in Arc<RwLock> for async sharing
    let disk_cache = Arc::new(RwLock::new(disk_cache));

    // Spawn background GitHub fetches for agents (non-blocking)
    // Uses conditional fetches with ETag to avoid re-downloading unchanged data
    let (github_rx, fetch_handles) = if let Some(ref agents_app) = app.agents_app {
        let client = AsyncGitHubClient::with_disk_cache(None, disk_cache.clone());
        let (rx, handles) =
            spawn_github_fetches_conditional(&agents_app.entries, client, disk_cache.clone());
        (Some(rx), handles)
    } else {
        (None, Vec::new())
    };

    // Main loop
    let result = run_app(&mut terminal, &mut app, github_rx);

    // Abort any remaining fetch tasks to allow clean shutdown
    for handle in fetch_handles {
        handle.abort();
    }

    // Save cache to disk before exiting (best-effort, don't crash on failure)
    // Use try_read() to avoid blocking in async context
    if let Ok(cache_guard) = disk_cache.try_read() {
        // Ignore save errors - cache is not critical and we don't want to crash on exit
        let _ = cache_guard.save();
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut app::App,
    mut github_rx: Option<mpsc::Receiver<FetchResult>>,
) -> Result<()> {
    let mut last_status_time: Option<std::time::Instant> = None;

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Clear status after 2 seconds
        if let Some(time) = last_status_time {
            if time.elapsed() > std::time::Duration::from_secs(2) {
                app.clear_status();
                last_status_time = None;
            }
        }

        // Check for GitHub updates (non-blocking)
        if let Some(ref mut rx) = github_rx {
            while let Ok(result) = rx.try_recv() {
                match result {
                    FetchResult::Success(id, data) => {
                        app.update(app::Message::GitHubDataReceived(id, data));
                    }
                    FetchResult::Failure(id, error) => {
                        app.update(app::Message::GitHubFetchFailed(id, error));
                    }
                }
            }
        }

        if let Some(msg) = event::handle_events(app)? {
            // Handle clipboard operations and set status with timer
            match &msg {
                app::Message::CopyFull => {
                    if let Some(text) = app.get_copy_full() {
                        copy_to_clipboard(text.clone());
                        app.set_status(format!("Copied: {}", text));
                        last_status_time = Some(std::time::Instant::now());
                    }
                }
                app::Message::CopyModelId => {
                    if let Some(text) = app.get_copy_model_id() {
                        copy_to_clipboard(text.clone());
                        app.set_status(format!("Copied: {}", text));
                        last_status_time = Some(std::time::Instant::now());
                    }
                }
                app::Message::CopyProviderDoc => {
                    if let Some(text) = app.get_provider_doc() {
                        copy_to_clipboard(text.clone());
                        app.set_status(format!("Copied: {}", text));
                        last_status_time = Some(std::time::Instant::now());
                    }
                }
                app::Message::CopyProviderApi => {
                    if let Some(text) = app.get_provider_api() {
                        copy_to_clipboard(text.clone());
                        app.set_status(format!("Copied: {}", text));
                        last_status_time = Some(std::time::Instant::now());
                    }
                }
                app::Message::OpenProviderDoc => {
                    if let Some(url) = app.get_provider_doc() {
                        let _ = open::that(&url);
                        app.set_status(format!("Opened: {}", url));
                        last_status_time = Some(std::time::Instant::now());
                    }
                }
                app::Message::OpenAgentDocs => {
                    if let Some(ref agents_app) = app.agents_app {
                        if let Some(entry) = agents_app.current_entry() {
                            if let Some(ref url) = entry.agent.docs {
                                let _ = open::that(url);
                                app.set_status(format!("Opened: {}", url));
                                last_status_time = Some(std::time::Instant::now());
                            } else if let Some(ref url) = entry.agent.homepage {
                                let _ = open::that(url);
                                app.set_status(format!("Opened: {}", url));
                                last_status_time = Some(std::time::Instant::now());
                            }
                        }
                    }
                }
                app::Message::OpenAgentRepo => {
                    if let Some(ref agents_app) = app.agents_app {
                        if let Some(entry) = agents_app.current_entry() {
                            let url = format!("https://github.com/{}", entry.agent.repo);
                            let _ = open::that(&url);
                            app.set_status(format!("Opened: {}", url));
                            last_status_time = Some(std::time::Instant::now());
                        }
                    }
                }
                app::Message::CopyAgentName => {
                    if let Some(ref agents_app) = app.agents_app {
                        if let Some(entry) = agents_app.current_entry() {
                            copy_to_clipboard(entry.agent.name.clone());
                            app.set_status(format!("Copied: {}", entry.agent.name));
                            last_status_time = Some(std::time::Instant::now());
                        }
                    }
                }
                app::Message::PickerSave => {
                    // Picker save sets its own status message via app.update
                    last_status_time = Some(std::time::Instant::now());
                }
                _ => {}
            }

            if !app.update(msg) {
                return Ok(());
            }
        }
    }
}
