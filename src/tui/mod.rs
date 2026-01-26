use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub mod agents_app;
pub mod app;
pub mod event;
pub mod ui;

use crate::agents::load_agents;
use crate::api::fetch_providers;
use crate::config::Config;

pub fn run() -> Result<()> {
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

    // Refresh GitHub data for agents (blocking at startup, but fast with gh cli)
    if let Some(ref mut agents_app) = app.agents_app {
        agents_app.refresh_github_data(&app.github_client);
    }

    // Main loop
    let result = run_app(&mut terminal, &mut app);

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
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if let Some(msg) = event::handle_events(app)? {
            // Handle clipboard operations
            match &msg {
                app::Message::CopyFull => {
                    if let Some(text) = app.get_copy_full() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                        }
                    }
                }
                app::Message::CopyModelId => {
                    if let Some(text) = app.get_copy_model_id() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                        }
                    }
                }
                app::Message::CopyProviderDoc => {
                    if let Some(text) = app.get_provider_doc() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                        }
                    }
                }
                app::Message::CopyProviderApi => {
                    if let Some(text) = app.get_provider_api() {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                            app.set_status(format!("Copied: {}", text));
                        }
                    }
                }
                app::Message::OpenProviderDoc => {
                    if let Some(url) = app.get_provider_doc() {
                        let _ = open::that(&url);
                        app.set_status(format!("Opened: {}", url));
                    }
                }
                app::Message::OpenAgentDocs => {
                    if let Some(ref agents_app) = app.agents_app {
                        if let Some(entry) = agents_app.current_entry() {
                            if let Some(ref url) = entry.agent.docs {
                                let _ = open::that(url);
                                app.set_status(format!("Opened: {}", url));
                            } else if let Some(ref url) = entry.agent.homepage {
                                let _ = open::that(url);
                                app.set_status(format!("Opened: {}", url));
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
                        }
                    }
                }
                app::Message::CopyAgentName => {
                    if let Some(ref agents_app) = app.agents_app {
                        if let Some(entry) = agents_app.current_entry() {
                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                let _ = clipboard.set_text(&entry.agent.name);
                                app.set_status(format!("Copied: {}", entry.agent.name));
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
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            if !app.update(msg) {
                return Ok(());
            }
        }

        // Clear status after a short display
        app.clear_status();
    }
}
