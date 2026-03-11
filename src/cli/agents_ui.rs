use std::{io, time::Duration};

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell as TuiCell, HighlightSpacing, Paragraph, Row as TuiRow,
        Table as TuiTable, TableState, Wrap,
    },
    Frame, Terminal, TerminalOptions, Viewport,
};

#[derive(Clone)]
pub struct ReleaseBrowserItem {
    pub agent_name: String,
    pub version: String,
    pub released: String,
    pub ago: String,
    pub body: Option<String>,
    pub sort_key: i64,
    pub release: crate::agents::data::Release,
}

#[derive(Clone)]
pub struct AgentSourceItem {
    pub id: String,
    pub name: String,
    pub repo: String,
    pub cli_binary: String,
    pub categories: String,
    pub tracked: bool,
    pub open_source: bool,
    pub supported_providers: String,
    pub platform_support: String,
    pub pricing: String,
    pub homepage: String,
    pub docs: String,
    pub stars: Option<u64>,
    pub latest_version: String,
    pub latest_release_date: String,
    pub release_frequency: String,
}

const VIEWPORT_HEIGHT: u16 = 14;

struct PickerTerminal {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl PickerTerminal {
    fn new() -> Result<Self> {
        crossterm::terminal::enable_raw_mode()?;
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::with_options(
            backend,
            TerminalOptions {
                viewport: Viewport::Inline(VIEWPORT_HEIGHT),
            },
        )?;
        Ok(Self { terminal })
    }
}

impl Drop for PickerTerminal {
    fn drop(&mut self) {
        let _ = self.terminal.clear();
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = self.terminal.show_cursor();
    }
}

struct ReleaseBrowser {
    items: Vec<ReleaseBrowserItem>,
    show_agent: bool,
    title: String,
    state: TableState,
}

impl ReleaseBrowser {
    fn new(items: Vec<ReleaseBrowserItem>, title: String, show_agent: bool) -> Self {
        let mut state = TableState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            show_agent,
            title,
            state,
        }
    }

    fn selected(&self) -> Option<&ReleaseBrowserItem> {
        self.state.selected().map(|idx| &self.items[idx])
    }

    fn next(&mut self) {
        let Some(current) = self.state.selected() else {
            return;
        };
        let last = self.items.len().saturating_sub(1);
        self.state.select(Some((current + 1).min(last)));
    }

    fn previous(&mut self) {
        let Some(current) = self.state.selected() else {
            return;
        };
        self.state.select(Some(current.saturating_sub(1)));
    }

    fn page_down(&mut self) {
        let Some(current) = self.state.selected() else {
            return;
        };
        let last = self.items.len().saturating_sub(1);
        self.state.select(Some((current + 10).min(last)));
    }

    fn page_up(&mut self) {
        let Some(current) = self.state.selected() else {
            return;
        };
        self.state.select(Some(current.saturating_sub(10)));
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(1)])
            .split(frame.area());
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
            .split(outer[0]);

        let widths = if self.show_agent {
            vec![
                Constraint::Percentage(28),
                Constraint::Percentage(22),
                Constraint::Percentage(18),
                Constraint::Percentage(16),
            ]
        } else {
            vec![
                Constraint::Percentage(34),
                Constraint::Percentage(24),
                Constraint::Percentage(20),
            ]
        };

        let rows = self.items.iter().map(|item| {
            if self.show_agent {
                TuiRow::new(vec![
                    TuiCell::from(truncate_text(&item.agent_name, 24)),
                    TuiCell::from(item.version.clone()),
                    TuiCell::from(item.released.clone()),
                    TuiCell::from(item.ago.clone()),
                ])
            } else {
                TuiRow::new(vec![
                    TuiCell::from(item.version.clone()),
                    TuiCell::from(item.released.clone()),
                    TuiCell::from(item.ago.clone()),
                ])
            }
        });

        let headers = if self.show_agent {
            vec!["Tool", "Version", "Released", "Ago"]
        } else {
            vec!["Version", "Released", "Ago"]
        };

        let table = TuiTable::new(rows, widths)
            .header(
                TuiRow::new(headers).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            )
            .column_spacing(1)
            .highlight_symbol(">> ")
            .highlight_spacing(HighlightSpacing::Always)
            .row_highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(format!("{} ({} releases)", self.title, self.items.len())),
            );

        frame.render_stateful_widget(table, main[0], &mut self.state);
        frame.render_widget(
            Paragraph::new(self.preview_lines())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Changelog Preview "),
                )
                .wrap(Wrap { trim: false }),
            main[1],
        );
        frame.render_widget(
            Paragraph::new("Enter print   q quit   ↑↓/j/k move   PgUp/PgDn jump"),
            outer[1],
        );
    }

    fn preview_lines(&self) -> Vec<Line<'static>> {
        let Some(item) = self.selected() else {
            return vec![Line::from("No releases")];
        };
        let mut lines = vec![Line::from(vec![
            Span::styled(
                item.agent_name.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                item.version.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(" ({})", item.released)),
        ])];
        lines.extend(changelog_preview_lines(item.body.as_deref()));
        lines
    }
}

enum SourcePickerMode {
    Select,
    Manage,
}

struct SourcePicker {
    items: Vec<AgentSourceItem>,
    title: String,
    mode: SourcePickerMode,
    state: TableState,
}

impl SourcePicker {
    fn new(items: Vec<AgentSourceItem>, title: String, mode: SourcePickerMode) -> Self {
        let mut state = TableState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        Self {
            items,
            title,
            mode,
            state,
        }
    }

    fn selected(&self) -> Option<&AgentSourceItem> {
        self.state.selected().map(|idx| &self.items[idx])
    }

    fn next(&mut self) {
        let Some(current) = self.state.selected() else {
            return;
        };
        let last = self.items.len().saturating_sub(1);
        self.state.select(Some((current + 1).min(last)));
    }

    fn previous(&mut self) {
        let Some(current) = self.state.selected() else {
            return;
        };
        self.state.select(Some(current.saturating_sub(1)));
    }

    fn toggle_current(&mut self) {
        let Some(idx) = self.state.selected() else {
            return;
        };
        if let Some(item) = self.items.get_mut(idx) {
            item.tracked = !item.tracked;
        }
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(10), Constraint::Length(1)])
            .split(frame.area());
        let main = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(outer[0]);

        let rows = self.items.iter().map(|item| {
            TuiRow::new(vec![
                TuiCell::from(if item.tracked { "[x]" } else { "[ ]" }),
                TuiCell::from(item.id.clone()),
                TuiCell::from(truncate_text(&item.name, 22)),
                TuiCell::from(truncate_text(&item.cli_binary, 14)),
            ])
        });

        let table = TuiTable::new(
            rows,
            [
                Constraint::Length(5),
                Constraint::Percentage(24),
                Constraint::Percentage(44),
                Constraint::Percentage(27),
            ],
        )
        .header(
            TuiRow::new(vec!["Track", "ID", "Name", "CLI"]).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .column_spacing(1)
        .highlight_symbol(">> ")
        .highlight_spacing(HighlightSpacing::Always)
        .row_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!("{} ({} agents)", self.title, self.items.len())),
        );

        frame.render_stateful_widget(table, main[0], &mut self.state);
        frame.render_widget(
            Paragraph::new(self.preview_lines())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::DarkGray))
                        .title(" Agent "),
                )
                .wrap(Wrap { trim: false }),
            main[1],
        );
        let status = match self.mode {
            SourcePickerMode::Select => "Enter choose   q quit   ↑↓/j/k move",
            SourcePickerMode::Manage => "Space toggle   Enter save   q cancel   ↑↓/j/k move",
        };
        frame.render_widget(Paragraph::new(status), outer[1]);
    }

    fn preview_lines(&self) -> Vec<Line<'static>> {
        let Some(item) = self.selected() else {
            return vec![Line::from("No agents")];
        };
        let dim = Style::default().fg(Color::DarkGray);
        let label = |s: &str| -> Span<'static> { Span::styled(format!("{s}: "), dim) };
        let has = |s: &str| -> bool { s != "\u{2014}" };

        // Row 1: Name + source tag + stars
        let source_tag = if item.open_source {
            Span::styled("open source", Style::default().fg(Color::Green))
        } else {
            Span::styled("closed source", Style::default().fg(Color::Red))
        };
        let mut header = vec![
            Span::styled(
                item.name.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            source_tag,
        ];
        if let Some(stars) = item.stars {
            let s = if stars >= 1000 {
                format!("{:.1}k", stars as f64 / 1000.0)
            } else {
                stars.to_string()
            };
            header.push(Span::raw("  "));
            header.push(Span::styled(
                format!("\u{2605} {s}"),
                Style::default().fg(Color::Yellow),
            ));
        }

        // Row 2: Repo URL
        let repo_url = format!("https://github.com/{}", item.repo);

        let mut lines = vec![Line::from(header), Line::from(Span::styled(repo_url, dim))];

        // Release info (compact: version + date + cadence on 1-2 lines)
        if has(&item.latest_version) {
            lines.push(Line::from(""));
            let mut release_spans = vec![Span::styled(
                format!("v{}", item.latest_version),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )];
            if has(&item.latest_release_date) {
                release_spans.push(Span::styled(format!("  {}", item.latest_release_date), dim));
            }
            if has(&item.release_frequency) {
                release_spans.push(Span::styled(
                    format!("  \u{2022} {}", item.release_frequency),
                    dim,
                ));
            }
            lines.push(Line::from(release_spans));
        }

        // Agent metadata (combined where possible)
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            label("Categories"),
            Span::raw(item.categories.clone()),
        ]));
        if has(&item.supported_providers) {
            lines.push(Line::from(vec![
                label("Providers"),
                Span::raw(item.supported_providers.clone()),
            ]));
        }
        if has(&item.platform_support) {
            lines.push(Line::from(vec![
                label("Platforms"),
                Span::raw(item.platform_support.clone()),
            ]));
        }
        if has(&item.pricing) {
            lines.push(Line::from(vec![
                label("Pricing"),
                Span::raw(item.pricing.clone()),
            ]));
        }

        // Links (inline if both fit)
        let has_homepage = has(&item.homepage);
        let has_docs = has(&item.docs);
        if has_homepage || has_docs {
            let link_style = Style::default().fg(Color::Cyan);
            if has_homepage && has_docs {
                lines.push(Line::from(vec![
                    Span::styled(item.homepage.clone(), link_style),
                    Span::styled("  \u{2022}  ", dim),
                    Span::styled(item.docs.clone(), link_style),
                ]));
            } else if has_homepage {
                lines.push(Line::from(Span::styled(item.homepage.clone(), link_style)));
            } else {
                lines.push(Line::from(Span::styled(item.docs.clone(), link_style)));
            }
        }

        lines
    }
}

pub fn browse_releases(
    items: Vec<ReleaseBrowserItem>,
    title: &str,
    show_agent: bool,
) -> Result<Option<ReleaseBrowserItem>> {
    let mut browser = ReleaseBrowser::new(items, title.to_string(), show_agent);
    let mut terminal = PickerTerminal::new()?;

    loop {
        terminal.terminal.draw(|frame| browser.draw(frame))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        match event::read()? {
            Event::Resize(_, _) => terminal.terminal.autoresize()?,
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Up | KeyCode::Char('k') => browser.previous(),
                KeyCode::Down | KeyCode::Char('j') => browser.next(),
                KeyCode::PageUp => browser.page_up(),
                KeyCode::PageDown => browser.page_down(),
                KeyCode::Enter => return Ok(browser.selected().cloned()),
                KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                _ => {}
            },
            _ => {}
        }
    }
}

pub fn pick_agent(items: Vec<AgentSourceItem>, title: &str) -> Result<Option<AgentSourceItem>> {
    let mut picker = SourcePicker::new(items, title.to_string(), SourcePickerMode::Select);
    let mut terminal = PickerTerminal::new()?;

    loop {
        terminal.terminal.draw(|frame| picker.draw(frame))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        match event::read()? {
            Event::Resize(_, _) => terminal.terminal.autoresize()?,
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Up | KeyCode::Char('k') => picker.previous(),
                KeyCode::Down | KeyCode::Char('j') => picker.next(),
                KeyCode::Enter => return Ok(picker.selected().cloned()),
                KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                _ => {}
            },
            _ => {}
        }
    }
}

pub fn manage_agent_sources(
    items: Vec<AgentSourceItem>,
    title: &str,
) -> Result<Option<Vec<AgentSourceItem>>> {
    let mut picker = SourcePicker::new(items, title.to_string(), SourcePickerMode::Manage);
    let mut terminal = PickerTerminal::new()?;

    loop {
        terminal.terminal.draw(|frame| picker.draw(frame))?;
        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        match event::read()? {
            Event::Resize(_, _) => terminal.terminal.autoresize()?,
            Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Up | KeyCode::Char('k') => picker.previous(),
                KeyCode::Down | KeyCode::Char('j') => picker.next(),
                KeyCode::Char(' ') => picker.toggle_current(),
                KeyCode::Enter => return Ok(Some(picker.items)),
                KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                _ => {}
            },
            _ => {}
        }
    }
}

fn changelog_preview_lines(body: Option<&str>) -> Vec<Line<'static>> {
    use crate::agents::changelog_parser::{parse_changelog, ChangelogBlock};

    let Some(body) = body.filter(|body| !body.trim().is_empty()) else {
        return vec![Line::from("(no changelog)")];
    };

    let changelog = parse_changelog(body);
    if changelog.blocks.is_empty() {
        return vec![Line::from("(no changelog)")];
    }

    let mut lines: Vec<Line<'static>> = Vec::new();
    let preview_budget = 8;

    for block in &changelog.blocks {
        if lines.len() >= preview_budget {
            break;
        }
        match block {
            ChangelogBlock::Heading(text) => {
                lines.push(Line::from(Span::styled(
                    format!("[{}]", text),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )));
            }
            ChangelogBlock::Bullet(text) => {
                lines.push(Line::from(format!("  - {}", text)));
            }
            ChangelogBlock::Paragraph(text) => {
                lines.push(Line::from(Span::styled(
                    text.clone(),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        }
    }

    lines
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }
    if max_chars <= 3 {
        return value.chars().take(max_chars).collect();
    }
    let visible: String = value.chars().take(max_chars - 3).collect();
    format!("{visible}...")
}
