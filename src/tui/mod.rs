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
pub mod event;
pub mod ui;

use crate::agents::{load_agents, AgentEntry, AsyncGitHubClient, GitHubData};
use crate::api::fetch_providers;
use crate::config::Config;

/// Spawn background GitHub fetches for all agent entries.
/// Returns a receiver that will receive (agent_id, github_data) tuples as they complete.
fn spawn_github_fetches(
    entries: &[AgentEntry],
    client: AsyncGitHubClient,
) -> mpsc::Receiver<(String, GitHubData)> {
    let (tx, rx) = mpsc::channel(100);

    for entry in entries {
        let tx = tx.clone();
        let client = client.clone();
        let id = entry.id.clone();
        let repo = entry.agent.repo.clone();

        tokio::spawn(async move {
            if let Ok(data) = client.fetch(&repo).await {
                let _ = tx.send((id, data)).await;
            }
        });
    }

    rx
}

pub async fn run() -> Result<()> {
    // Load data
    let providers = fetch_providers()?;
    let agents_file = load_agents().ok();
    let config = Config::load().ok();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = app::App::new(providers, agents_file.as_ref(), config);

    // Spawn background GitHub fetches for agents (non-blocking)
    let github_rx = if let Some(ref agents_app) = app.agents_app {
        let client = AsyncGitHubClient::new(None);
        Some(spawn_github_fetches(&agents_app.entries, client))
    } else {
        None
    };

    // Main loop
    let result = run_app(&mut terminal, &mut app, github_rx);

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
    mut github_rx: Option<mpsc::Receiver<(String, GitHubData)>>,
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
            while let Ok((id, data)) = rx.try_recv() {
                app.update(app::Message::GitHubDataReceived(id, data));
            }
        }

        if let Some(msg) = event::handle_events(app)? {
            // Handle clipboard operations and set status with timer
            match &msg {
                app::Message::CopyFull => {
                    if let Some(text) = app.get_copy_full() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                            last_status_time = Some(std::time::Instant::now());
                        }
                    }
                }
                app::Message::CopyModelId => {
                    if let Some(text) = app.get_copy_model_id() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                            last_status_time = Some(std::time::Instant::now());
                        }
                    }
                }
                app::Message::CopyProviderDoc => {
                    if let Some(text) = app.get_provider_doc() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                            last_status_time = Some(std::time::Instant::now());
                        }
                    }
                }
                app::Message::CopyProviderApi => {
                    if let Some(text) = app.get_provider_api() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                            last_status_time = Some(std::time::Instant::now());
                        }
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
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                let _ = clipboard.set_text(&entry.agent.name);
                                app.set_status(format!("Copied: {}", entry.agent.name));
                                last_status_time = Some(std::time::Instant::now());
                            }
                        }
                    }
                }
                app::Message::CopyUpdateCommand => {
                    if let Some(ref agents_app) = app.agents_app {
                        if let Some(entry) = agents_app.current_entry() {
                            if let Some(cmd) = entry.agent.update_command() {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    let _ = clipboard.set_text(&cmd);
                                    app.set_status(format!("Copied: {}", cmd));
                                    last_status_time = Some(std::time::Instant::now());
                                }
                            }
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
