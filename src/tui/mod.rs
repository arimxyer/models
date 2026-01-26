mod app;
mod event;
mod ui;

use std::io::stdout;
use std::time::Instant;

use anyhow::Result;
use arboard::Clipboard;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use crate::api;
use app::{App, Message};

pub fn run() -> Result<()> {
    // Fetch data before setting up terminal
    eprintln!("Fetching model data...");
    let providers = api::fetch_providers()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run
    let mut app = App::new(providers);
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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut clipboard = Clipboard::new().ok();
    let mut status_clear_time: Option<Instant> = None;

    loop {
        // Clear status message after 2 seconds
        if let Some(clear_time) = status_clear_time {
            if clear_time.elapsed().as_secs() >= 2 {
                app.clear_status();
                status_clear_time = None;
            }
        }

        terminal.draw(|f| ui::draw(f, app))?; // app is &mut App

        if let Some(msg) = event::handle_events(app)? {
            match msg {
                Message::CopyFull => {
                    if let Some(text) = app.get_copy_full() {
                        if let Some(ref mut cb) = clipboard {
                            if cb.set_text(&text).is_ok() {
                                app.set_status(format!("Copied: {}", text));
                                status_clear_time = Some(Instant::now());
                            } else {
                                app.set_status("Failed to copy".to_string());
                                status_clear_time = Some(Instant::now());
                            }
                        } else {
                            app.set_status("Clipboard unavailable".to_string());
                            status_clear_time = Some(Instant::now());
                        }
                    }
                }
                Message::CopyModelId => {
                    if let Some(text) = app.get_copy_model_id() {
                        if let Some(ref mut cb) = clipboard {
                            if cb.set_text(&text).is_ok() {
                                app.set_status(format!("Copied: {}", text));
                                status_clear_time = Some(Instant::now());
                            } else {
                                app.set_status("Failed to copy".to_string());
                                status_clear_time = Some(Instant::now());
                            }
                        } else {
                            app.set_status("Clipboard unavailable".to_string());
                            status_clear_time = Some(Instant::now());
                        }
                    }
                }
                Message::CopyProviderDoc => {
                    if let Some(text) = app.get_provider_doc() {
                        if let Some(ref mut cb) = clipboard {
                            if cb.set_text(&text).is_ok() {
                                app.set_status(format!("Copied docs: {}", text));
                                status_clear_time = Some(Instant::now());
                            } else {
                                app.set_status("Failed to copy".to_string());
                                status_clear_time = Some(Instant::now());
                            }
                        } else {
                            app.set_status("Clipboard unavailable".to_string());
                            status_clear_time = Some(Instant::now());
                        }
                    } else {
                        app.set_status("No documentation URL available".to_string());
                        status_clear_time = Some(Instant::now());
                    }
                }
                Message::CopyProviderApi => {
                    if let Some(text) = app.get_provider_api() {
                        if let Some(ref mut cb) = clipboard {
                            if cb.set_text(&text).is_ok() {
                                app.set_status(format!("Copied API: {}", text));
                                status_clear_time = Some(Instant::now());
                            } else {
                                app.set_status("Failed to copy".to_string());
                                status_clear_time = Some(Instant::now());
                            }
                        } else {
                            app.set_status("Clipboard unavailable".to_string());
                            status_clear_time = Some(Instant::now());
                        }
                    } else {
                        app.set_status("No API URL available".to_string());
                        status_clear_time = Some(Instant::now());
                    }
                }
                _ => {
                    if !app.update(msg) {
                        return Ok(());
                    }
                }
            }
        }
    }
}
