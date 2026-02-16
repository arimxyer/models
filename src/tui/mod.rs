use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;

pub mod agents_app;
pub mod app;
pub mod benchmarks_app;
pub mod event;
pub mod ui;

use crate::agents::{
    load_agents, AsyncGitHubClient, ConditionalFetchResult, GitHubCache, GitHubData,
};
use crate::benchmark_cache::BenchmarkCache;
use crate::benchmark_fetch::{BenchmarkFetchResult, BenchmarkFetcher};
use crate::benchmarks::{benchmark_entries_compatible, BenchmarkStore};
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
pub async fn run(providers: ProvidersMap) -> Result<()> {
    use crate::agents::FetchStatus;

    // Load remaining data
    let agents_file = load_agents().ok();
    let config = Config::load().ok();

    // Load benchmark cache and use it if fresh, otherwise fall back to embedded
    let bench_cache = BenchmarkCache::load();
    let benchmark_store = if bench_cache.is_fresh() && bench_cache.has_entries() {
        BenchmarkStore::load_with_cache(&bench_cache)
    } else {
        BenchmarkStore::load()
    };

    // Load disk cache for GitHub data (load before wrapping to avoid blocking in async)
    let disk_cache = GitHubCache::load();

    // Create app BEFORE entering alternate screen
    let mut app = app::App::new(providers, agents_file.as_ref(), config, benchmark_store);

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

    // Create GitHub client and channel for fetch results
    let client = AsyncGitHubClient::with_disk_cache(None, disk_cache.clone());
    let (tx, rx) = mpsc::channel(100);

    // Spawn background GitHub fetches for agents (non-blocking)
    // Uses conditional fetches with ETag to avoid re-downloading unchanged data
    let fetch_handles = if let Some(ref agents_app) = app.agents_app {
        let tracked_entries: Vec<_> = agents_app.entries.iter().filter(|e| e.tracked).collect();
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
                        let cache_guard = cache.read().await;
                        if let Some(cached) = cache_guard.get(&repo) {
                            FetchResult::Success(id, cached.data.clone().into())
                        } else {
                            FetchResult::Failure(id, "Cache miss on NotModified".to_string())
                        }
                    }
                    ConditionalFetchResult::Error(e) => FetchResult::Failure(id, e),
                };
                let _ = tx.send(result).await;
            });
            handles.push(handle);
        }
        handles
    } else {
        Vec::new()
    };

    // Spawn background benchmark fetch if cache is stale
    let (bench_tx, bench_rx) = mpsc::channel(1);
    if !bench_cache.is_fresh() {
        let bench_tx = bench_tx.clone();
        let cached_etag = bench_cache.etag.clone();
        tokio::spawn(async move {
            let fetcher = BenchmarkFetcher::new();
            let result = fetcher.fetch_conditional(cached_etag.as_deref()).await;
            let _ = bench_tx.send(result).await;
        });
    }

    // Main loop - pass client and sender for dynamic fetches
    let result = run_app(
        &mut terminal,
        &mut app,
        rx,
        tx,
        client,
        disk_cache.clone(),
        bench_rx,
    );

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
    mut github_rx: mpsc::Receiver<FetchResult>,
    github_tx: mpsc::Sender<FetchResult>,
    client: AsyncGitHubClient,
    disk_cache: Arc<RwLock<GitHubCache>>,
    mut bench_rx: mpsc::Receiver<BenchmarkFetchResult>,
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

        // Spawn fetches for newly tracked agents
        if !app.pending_fetches.is_empty() {
            let fetches = std::mem::take(&mut app.pending_fetches);
            for (agent_id, repo) in fetches {
                let tx = github_tx.clone();
                let client = client.clone();
                let cache = disk_cache.clone();

                tokio::spawn(async move {
                    let result = match client.fetch_conditional(&repo).await {
                        ConditionalFetchResult::Fresh(data, _etag) => {
                            FetchResult::Success(agent_id, data)
                        }
                        ConditionalFetchResult::NotModified => {
                            let cache_guard = cache.read().await;
                            if let Some(cached) = cache_guard.get(&repo) {
                                FetchResult::Success(agent_id, cached.data.clone().into())
                            } else {
                                FetchResult::Failure(
                                    agent_id,
                                    "Cache miss on NotModified".to_string(),
                                )
                            }
                        }
                        ConditionalFetchResult::Error(e) => FetchResult::Failure(agent_id, e),
                    };
                    let _ = tx.send(result).await;
                });
            }
        }

        // Check for GitHub updates (non-blocking)
        while let Ok(result) = github_rx.try_recv() {
            match result {
                FetchResult::Success(id, data) => {
                    app.update(app::Message::GitHubDataReceived(id, data));
                }
                FetchResult::Failure(id, error) => {
                    app.update(app::Message::GitHubFetchFailed(id, error));
                }
            }
        }

        // Check for benchmark data updates (non-blocking)
        if let Ok(result) = bench_rx.try_recv() {
            match result {
                BenchmarkFetchResult::Fresh(entries, etag) => {
                    // Validate CDN data before replacing currently loaded data.
                    // Reject if CDN schema is stale relative to current app baseline.
                    if benchmark_entries_compatible(&entries, app.benchmark_store.entries()) {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        let cache = BenchmarkCache {
                            version: crate::benchmark_cache::CACHE_VERSION,
                            schema_version: crate::benchmark_cache::DATA_SCHEMA_VERSION,
                            entries: entries.clone(),
                            etag,
                            fetched_at: now,
                        };
                        let _ = cache.save();
                        app.update(app::Message::BenchmarkDataReceived(entries));
                    } else {
                        eprintln!(
                            "Rejected stale benchmark CDN payload: schema compatibility failed"
                        );
                    }
                }
                BenchmarkFetchResult::NotModified => {
                    // Touch cache timestamp only if the cached schema is compatible with
                    // currently loaded data. If stale, keep it stale to force retries.
                    let mut cache = BenchmarkCache::load();
                    if benchmark_entries_compatible(&cache.entries, app.benchmark_store.entries()) {
                        cache.fetched_at = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0);
                        let _ = cache.save();
                    } else {
                        eprintln!(
                            "Skipped cache freshness extension on 304: cached benchmark schema is stale"
                        );
                    }
                }
                BenchmarkFetchResult::Error => {
                    app.update(app::Message::BenchmarkFetchFailed);
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
                        let _ = open::that_in_background(&url);
                        app.set_status(format!("Opened: {}", url));
                        last_status_time = Some(std::time::Instant::now());
                    }
                }
                app::Message::OpenAgentDocs => {
                    if let Some(ref agents_app) = app.agents_app {
                        if let Some(entry) = agents_app.current_entry() {
                            if let Some(ref url) = entry.agent.docs {
                                let _ = open::that_in_background(url);
                                app.set_status(format!("Opened: {}", url));
                                last_status_time = Some(std::time::Instant::now());
                            } else if let Some(ref url) = entry.agent.homepage {
                                let _ = open::that_in_background(url);
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
                            let _ = open::that_in_background(&url);
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
                app::Message::CopyBenchmarkName => {
                    if let Some(entry) = app.benchmarks_app.current_entry(&app.benchmark_store) {
                        copy_to_clipboard(entry.name.clone());
                        app.set_status(format!("Copied: {}", entry.name));
                        last_status_time = Some(std::time::Instant::now());
                    }
                }
                app::Message::OpenBenchmarkUrl => {
                    if let Some(entry) = app.benchmarks_app.current_entry(&app.benchmark_store) {
                        let url = format!("https://artificialanalysis.ai/models/{}", entry.slug);
                        let _ = open::that_in_background(&url);
                        app.set_status(format!("Opened: {}", url));
                        last_status_time = Some(std::time::Instant::now());
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
