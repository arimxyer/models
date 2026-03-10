use std::{collections::HashMap, io, time::Duration};

use anyhow::{bail, Result};
use clap::{CommandFactory, Parser, ValueEnum};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table as ComfyTable};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{
        Block, Borders, Cell as TuiCell, HighlightSpacing, Paragraph, Row as TuiRow,
        Table as TuiTable, TableState,
    },
    Frame, Terminal, TerminalOptions, Viewport,
};
use serde::Serialize;

use crate::benchmark_fetch::{BenchmarkFetchResult, BenchmarkFetcher};
use crate::benchmarks::{BenchmarkEntry, BenchmarkStore, ReasoningFilter, ReasoningStatus};

#[derive(Parser, Debug)]
#[command(name = "benchmarks")]
#[command(about = "Query benchmark data from the command line")]
pub struct BenchmarksCli {
    #[command(subcommand)]
    pub command: Option<BenchmarksCommand>,
}

#[derive(clap::Subcommand, Debug)]
pub enum BenchmarksCommand {
    /// List benchmark entries with filtering and sorting
    List {
        /// Filter by model name, display name, slug, or creator
        #[arg(long)]
        search: Option<String>,
        /// Filter by creator slug or display name
        #[arg(long)]
        creator: Option<String>,
        /// Sort column
        #[arg(long, value_enum, default_value_t = BenchmarkSort::Intelligence)]
        sort: BenchmarkSort,
        /// Force ascending sort
        #[arg(long, conflicts_with = "desc")]
        asc: bool,
        /// Force descending sort
        #[arg(long, conflicts_with = "asc")]
        desc: bool,
        /// Only show open-weight models
        #[arg(long, conflicts_with = "closed")]
        open: bool,
        /// Only show closed-weight models
        #[arg(long, conflicts_with = "open")]
        closed: bool,
        /// Only show reasoning/adaptive reasoning models
        #[arg(long, conflicts_with = "non_reasoning")]
        reasoning: bool,
        /// Only show explicitly non-reasoning models
        #[arg(long, conflicts_with = "reasoning")]
        non_reasoning: bool,
        /// Limit rows in human-readable or JSON output
        #[arg(long)]
        limit: Option<usize>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show a single benchmark entry in detail
    Show {
        /// Benchmark model slug, exact display name, or unique partial match
        model: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum BenchmarkSort {
    Intelligence,
    Coding,
    Math,
    Gpqa,
    #[value(name = "mmlu-pro")]
    MmluPro,
    Hle,
    #[value(name = "livecodebench")]
    LiveCodeBench,
    Scicode,
    Ifbench,
    Lcr,
    #[value(name = "terminalbench")]
    TerminalBench,
    Tau2,
    #[value(name = "speed")]
    Speed,
    Ttft,
    Ttfat,
    #[value(name = "price-input")]
    PriceInput,
    #[value(name = "price-output")]
    PriceOutput,
    #[value(name = "price-blended")]
    PriceBlended,
    Name,
    #[value(name = "release-date")]
    ReleaseDate,
}

impl BenchmarkSort {
    fn label(self) -> &'static str {
        match self {
            Self::Intelligence => "Intelligence",
            Self::Coding => "Coding",
            Self::Math => "Math",
            Self::Gpqa => "GPQA",
            Self::MmluPro => "MMLU-Pro",
            Self::Hle => "HLE",
            Self::LiveCodeBench => "LiveCodeBench",
            Self::Scicode => "SciCode",
            Self::Ifbench => "IFBench",
            Self::Lcr => "LCR",
            Self::TerminalBench => "TerminalBench",
            Self::Tau2 => "Tau2",
            Self::Speed => "Tok/s",
            Self::Ttft => "TTFT",
            Self::Ttfat => "TTFAT",
            Self::PriceInput => "Input $/M",
            Self::PriceOutput => "Output $/M",
            Self::PriceBlended => "Blended $/M",
            Self::Name => "Name",
            Self::ReleaseDate => "Release",
        }
    }

    fn default_descending(self) -> bool {
        !matches!(
            self,
            Self::Name
                | Self::Ttft
                | Self::Ttfat
                | Self::PriceInput
                | Self::PriceOutput
                | Self::PriceBlended
        )
    }

    fn extract(self, entry: &BenchmarkEntry) -> Option<f64> {
        match self {
            Self::Intelligence => entry.intelligence_index,
            Self::Coding => entry.coding_index,
            Self::Math => entry.math_index,
            Self::Gpqa => entry.gpqa,
            Self::MmluPro => entry.mmlu_pro,
            Self::Hle => entry.hle,
            Self::LiveCodeBench => entry.livecodebench,
            Self::Scicode => entry.scicode,
            Self::Ifbench => entry.ifbench,
            Self::Lcr => entry.lcr,
            Self::TerminalBench => entry.terminalbench_hard,
            Self::Tau2 => entry.tau2,
            Self::Speed => entry.output_tps,
            Self::Ttft => entry.ttft,
            Self::Ttfat => entry.ttfat,
            Self::PriceInput => entry.price_input,
            Self::PriceOutput => entry.price_output,
            Self::PriceBlended => entry.price_blended,
            Self::Name => Some(0.0),
            Self::ReleaseDate => entry
                .release_date
                .as_deref()
                .and_then(parse_date_to_numeric),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceFilter {
    All,
    Open,
    Closed,
}

#[derive(Debug, Clone)]
struct ListOptions {
    search: Option<String>,
    creator: Option<String>,
    sort: BenchmarkSort,
    descending: bool,
    source_filter: SourceFilter,
    reasoning_filter: ReasoningFilter,
    limit: Option<usize>,
}

#[derive(Serialize)]
struct BenchmarkListItem<'a> {
    slug: &'a str,
    name: &'a str,
    display_name: &'a str,
    creator: &'a str,
    creator_name: &'a str,
    release_date: Option<&'a str>,
    sort: &'static str,
    sort_value: Option<f64>,
    open_weights: Option<bool>,
    reasoning: &'static str,
}

#[derive(Serialize)]
struct BenchmarkDetail<'a> {
    slug: &'a str,
    name: &'a str,
    display_name: &'a str,
    creator: &'a str,
    creator_name: &'a str,
    creator_id: &'a str,
    release_date: Option<&'a str>,
    open_weights: Option<bool>,
    reasoning: &'static str,
    effort_level: Option<&'a str>,
    variant_tag: Option<&'a str>,
    tool_call: Option<bool>,
    context_window: Option<u64>,
    max_output: Option<u64>,
    intelligence_index: Option<f64>,
    coding_index: Option<f64>,
    math_index: Option<f64>,
    mmlu_pro: Option<f64>,
    gpqa: Option<f64>,
    hle: Option<f64>,
    livecodebench: Option<f64>,
    scicode: Option<f64>,
    ifbench: Option<f64>,
    lcr: Option<f64>,
    terminalbench_hard: Option<f64>,
    tau2: Option<f64>,
    math_500: Option<f64>,
    aime: Option<f64>,
    aime_25: Option<f64>,
    output_tps: Option<f64>,
    ttft: Option<f64>,
    ttfat: Option<f64>,
    price_input: Option<f64>,
    price_output: Option<f64>,
    price_blended: Option<f64>,
}

enum ResolveEntry<'a> {
    Single(&'a BenchmarkEntry),
    Ambiguous(Vec<&'a BenchmarkEntry>),
}

const PICKER_VIEWPORT_HEIGHT: u16 = 14;
const PICKER_SORTS: [BenchmarkSort; 9] = [
    BenchmarkSort::Intelligence,
    BenchmarkSort::Coding,
    BenchmarkSort::Math,
    BenchmarkSort::Gpqa,
    BenchmarkSort::Speed,
    BenchmarkSort::Ttft,
    BenchmarkSort::PriceBlended,
    BenchmarkSort::ReleaseDate,
    BenchmarkSort::Name,
];

struct BenchmarkPicker<'a> {
    entries: Vec<&'a BenchmarkEntry>,
    visible_entries: Vec<&'a BenchmarkEntry>,
    open_weights_map: &'a HashMap<String, bool>,
    sort: BenchmarkSort,
    descending: bool,
    title: String,
    query: String,
    filter_mode: bool,
    state: TableState,
}

impl<'a> BenchmarkPicker<'a> {
    fn new(
        entries: Vec<&'a BenchmarkEntry>,
        open_weights_map: &'a HashMap<String, bool>,
        sort: BenchmarkSort,
        descending: bool,
        title: String,
    ) -> Self {
        let mut picker = Self {
            entries,
            visible_entries: Vec::new(),
            open_weights_map,
            sort,
            descending,
            title,
            query: String::new(),
            filter_mode: false,
            state: TableState::default(),
        };
        picker.rebuild_visible_entries(None);
        picker
    }

    fn selected(&self) -> Option<&'a BenchmarkEntry> {
        self.state.selected().map(|idx| self.visible_entries[idx])
    }

    fn next(&mut self) {
        let Some(current) = self.state.selected() else {
            return;
        };
        let last = self.visible_entries.len().saturating_sub(1);
        self.state.select(Some((current + 1).min(last)));
    }

    fn previous(&mut self) {
        let Some(current) = self.state.selected() else {
            return;
        };
        self.state.select(Some(current.saturating_sub(1)));
    }

    fn first(&mut self) {
        if !self.visible_entries.is_empty() {
            self.state.select(Some(0));
        }
    }

    fn last(&mut self) {
        if !self.visible_entries.is_empty() {
            self.state.select(Some(self.visible_entries.len() - 1));
        }
    }

    fn page_down(&mut self) {
        let Some(current) = self.state.selected() else {
            return;
        };
        let last = self.visible_entries.len().saturating_sub(1);
        self.state.select(Some((current + 10).min(last)));
    }

    fn page_up(&mut self) {
        let Some(current) = self.state.selected() else {
            return;
        };
        self.state.select(Some(current.saturating_sub(10)));
    }

    fn cycle_sort(&mut self) {
        let current_idx = PICKER_SORTS
            .iter()
            .position(|&sort| sort == self.sort)
            .unwrap_or(0);
        self.sort = PICKER_SORTS[(current_idx + 1) % PICKER_SORTS.len()];
        self.descending = self.sort.default_descending();
        self.rebuild_visible_entries(self.selected().map(|entry| entry.slug.as_str()));
    }

    fn toggle_descending(&mut self) {
        self.descending = !self.descending;
        self.rebuild_visible_entries(self.selected().map(|entry| entry.slug.as_str()));
    }

    fn start_filter(&mut self) {
        self.filter_mode = true;
    }

    fn finish_filter(&mut self) {
        self.filter_mode = false;
    }

    fn clear_filter(&mut self) {
        self.query.clear();
        self.filter_mode = false;
        self.rebuild_visible_entries(None);
    }

    fn push_filter_char(&mut self, ch: char) {
        self.query.push(ch);
        self.rebuild_visible_entries(self.selected().map(|entry| entry.slug.as_str()));
    }

    fn pop_filter_char(&mut self) {
        self.query.pop();
        self.rebuild_visible_entries(self.selected().map(|entry| entry.slug.as_str()));
    }

    fn rebuild_visible_entries(&mut self, preserve_slug: Option<&str>) {
        self.visible_entries =
            filter_picker_entries(&self.entries, &self.query, self.sort, self.descending);
        let next_selected = preserve_slug
            .and_then(|slug| {
                self.visible_entries
                    .iter()
                    .position(|entry| entry.slug == slug)
            })
            .or_else(|| (!self.visible_entries.is_empty()).then_some(0));
        self.state.select(next_selected);
    }

    fn draw(&mut self, frame: &mut Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(7),
                Constraint::Length(4),
                Constraint::Length(1),
            ])
            .split(frame.area());
        let header_style = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        let rows = self.visible_entries.iter().map(|entry| {
            TuiRow::new(vec![
                TuiCell::from(truncate_picker_text(&entry.display_name, 32)),
                TuiCell::from(truncate_picker_text(creator_label(entry), 16)),
                TuiCell::from(truncate_picker_text(
                    &format_picker_sort_value(self.sort, entry),
                    12,
                )),
                TuiCell::from(truncate_picker_text(reasoning_label(entry), 13)),
                TuiCell::from(format_open_weights(
                    self.open_weights_map.get(&entry.slug).copied(),
                )),
                TuiCell::from(
                    entry
                        .release_date
                        .clone()
                        .unwrap_or_else(|| "\u{2014}".to_string()),
                ),
            ])
        });

        let table = TuiTable::new(
            rows,
            [
                Constraint::Percentage(34),
                Constraint::Percentage(17),
                Constraint::Percentage(13),
                Constraint::Percentage(14),
                Constraint::Percentage(9),
                Constraint::Percentage(13),
            ],
        )
        .header(
            TuiRow::new(vec![
                "Name",
                "Creator",
                picker_sort_label(self.sort),
                "Reasoning",
                "Source",
                "Release",
            ])
            .style(header_style)
            .bottom_margin(1),
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
                .title(self.title_text()),
        );

        frame.render_stateful_widget(table, chunks[0], &mut self.state);

        let preview = Paragraph::new(self.preview_lines()).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Preview "),
        );
        frame.render_widget(preview, chunks[1]);

        let controls = Paragraph::new(self.status_line());
        frame.render_widget(controls, chunks[2]);
    }

    fn title_text(&self) -> String {
        let results = if self.query.is_empty() {
            format!("{} results", self.visible_entries.len())
        } else {
            format!(
                "{} / {} results",
                self.visible_entries.len(),
                self.entries.len()
            )
        };
        if self.query.is_empty() {
            format!(
                "{} ({}) | {} {}",
                self.title,
                results,
                picker_sort_label(self.sort),
                if self.descending { "desc" } else { "asc" }
            )
        } else {
            format!(
                "{} ({}) | {} {} | / {}",
                self.title,
                results,
                picker_sort_label(self.sort),
                if self.descending { "desc" } else { "asc" },
                self.query
            )
        }
    }

    fn preview_lines(&self) -> Vec<Line<'static>> {
        let Some(entry) = self.selected() else {
            return vec![
                Line::from("No matches"),
                Line::from(""),
                Line::from("Adjust the filter or clear it with Esc while filtering."),
            ];
        };
        vec![
            Line::from(format!("slug: {}", truncate_picker_text(&entry.slug, 72))),
            Line::from(format!(
                "coding: {}   math: {}   gpqa: {}",
                format_metric(entry.coding_index),
                format_metric(entry.math_index),
                format_metric(entry.gpqa),
            )),
            Line::from(format!(
                "ttft: {}   tok/s: {}   blended $/M: {}",
                format_metric(entry.ttft),
                format_metric(entry.output_tps),
                format_metric(entry.price_blended),
            )),
        ]
    }

    fn status_line(&self) -> Line<'static> {
        if self.filter_mode {
            Line::from(format!(
                "Filter: {}_  Enter apply  Esc clear  Backspace delete",
                self.query
            ))
        } else {
            Line::from("Enter inspect   / filter   s sort   S reverse   q quit   ↑↓/j/k move")
        }
    }
}

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
                viewport: Viewport::Inline(PICKER_VIEWPORT_HEIGHT),
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

pub fn run_with_command(command: Option<BenchmarksCommand>) -> Result<()> {
    match command {
        Some(BenchmarksCommand::List {
            search,
            creator,
            sort,
            asc,
            desc,
            open,
            closed,
            reasoning,
            non_reasoning,
            limit,
            json,
        }) => run_list(
            ListOptions {
                search,
                creator,
                sort,
                descending: if asc {
                    false
                } else if desc {
                    true
                } else {
                    sort.default_descending()
                },
                source_filter: if open {
                    SourceFilter::Open
                } else if closed {
                    SourceFilter::Closed
                } else {
                    SourceFilter::All
                },
                reasoning_filter: if reasoning {
                    ReasoningFilter::Reasoning
                } else if non_reasoning {
                    ReasoningFilter::NonReasoning
                } else {
                    ReasoningFilter::All
                },
                limit,
            },
            json,
        ),
        Some(BenchmarksCommand::Show { model, json }) => run_show(&model, json),
        None => {
            BenchmarksCli::command().print_long_help()?;
            println!();
            Ok(())
        }
    }
}

fn run_list(options: ListOptions, json: bool) -> Result<()> {
    let loaded = load_benchmarks()?;
    let entries = filter_entries(loaded.entries(), &loaded.open_weights_map, &options);

    if json {
        let items: Vec<_> = entries
            .iter()
            .map(|entry| BenchmarkListItem {
                slug: entry.slug.as_str(),
                name: entry.name.as_str(),
                display_name: entry.display_name.as_str(),
                creator: entry.creator.as_str(),
                creator_name: creator_label(entry),
                release_date: entry.release_date.as_deref(),
                sort: options.sort.label(),
                sort_value: options.sort.extract(entry),
                open_weights: loaded.open_weights_map.get(&entry.slug).copied(),
                reasoning: reasoning_label(entry),
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }

    if entries.is_empty() {
        println!("No benchmark entries matched the current filters.");
        return Ok(());
    }

    if super::styles::is_tty() {
        let title = " Benchmark Picker ".to_string();
        if let Some(entry) = pick_benchmark(
            entries,
            &loaded.open_weights_map,
            options.sort,
            options.descending,
            title.as_str(),
        )? {
            print_entry_detail(entry, &loaded.open_weights_map, false)?;
        }
        return Ok(());
    }

    print_list_table(&entries, &loaded.open_weights_map, options.sort);
    Ok(())
}

fn print_list_table(
    entries: &[&BenchmarkEntry],
    open_weights_map: &HashMap<String, bool>,
    sort: BenchmarkSort,
) {
    let mut table = ComfyTable::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec![
        "Slug",
        "Name",
        "Creator",
        sort.label(),
        "Source",
        "Reasoning",
        "Release",
    ]);

    for entry in entries {
        table.add_row(vec![
            entry.slug.clone(),
            entry.display_name.clone(),
            creator_label(entry).to_string(),
            format_sort_value(sort, entry),
            format_open_weights(open_weights_map.get(&entry.slug).copied()),
            reasoning_label(entry).to_string(),
            entry
                .release_date
                .clone()
                .unwrap_or_else(|| "\u{2014}".to_string()),
        ]);
    }

    println!("{table}");
}

fn run_show(model: &str, json: bool) -> Result<()> {
    let loaded = load_benchmarks()?;
    match resolve_entry(loaded.entries(), model)? {
        ResolveEntry::Single(entry) => print_entry_detail(entry, &loaded.open_weights_map, json)?,
        ResolveEntry::Ambiguous(entries) => {
            if json || !super::styles::is_tty() {
                bail!("{}", ambiguous_matches_message(model, &entries));
            }

            let title = format!(" Select Benchmark Match for \"{model}\" ");
            if let Some(entry) = pick_benchmark(
                entries,
                &loaded.open_weights_map,
                BenchmarkSort::Name,
                false,
                &title,
            )? {
                print_entry_detail(entry, &loaded.open_weights_map, false)?;
            }
        }
    }
    Ok(())
}

struct LoadedBenchmarks {
    store: BenchmarkStore,
    open_weights_map: HashMap<String, bool>,
}

impl LoadedBenchmarks {
    fn entries(&self) -> &[BenchmarkEntry] {
        self.store.entries()
    }
}

fn load_benchmarks() -> Result<LoadedBenchmarks> {
    let providers = crate::api::fetch_providers()?;
    let provider_vec: Vec<_> = providers.into_iter().collect();

    let runtime = tokio::runtime::Runtime::new()?;
    let fetcher = BenchmarkFetcher::new();
    let entries = match runtime.block_on(fetcher.fetch()) {
        BenchmarkFetchResult::Fresh(entries) => entries,
        BenchmarkFetchResult::Error => {
            bail!("Failed to fetch benchmark data from the CDN")
        }
    };

    let mut store = BenchmarkStore::from_entries(entries);
    crate::model_traits::apply_model_traits(&provider_vec, store.entries_mut());
    let open_weights_map =
        crate::model_traits::build_open_weights_map(&provider_vec, store.entries());

    Ok(LoadedBenchmarks {
        store,
        open_weights_map,
    })
}

fn filter_entries<'a>(
    entries: &'a [BenchmarkEntry],
    open_weights_map: &HashMap<String, bool>,
    options: &ListOptions,
) -> Vec<&'a BenchmarkEntry> {
    let search = options.search.as_ref().map(|s| s.to_lowercase());
    let creator = options.creator.as_ref().map(|s| s.to_lowercase());

    let mut filtered: Vec<_> = entries
        .iter()
        .filter(|entry| {
            if !matches_source_filter(options.source_filter, entry, open_weights_map) {
                return false;
            }

            if !options.reasoning_filter.matches(entry) {
                return false;
            }

            if let Some(creator_filter) = &creator {
                let creator_name = creator_label(entry).to_lowercase();
                if !entry.creator.to_lowercase().contains(creator_filter)
                    && !creator_name.contains(creator_filter)
                {
                    return false;
                }
            }

            if let Some(search_query) = &search {
                let matches = entry.slug.to_lowercase().contains(search_query)
                    || entry.name.to_lowercase().contains(search_query)
                    || entry.display_name.to_lowercase().contains(search_query)
                    || entry.creator.to_lowercase().contains(search_query)
                    || creator_label(entry).to_lowercase().contains(search_query);
                if !matches {
                    return false;
                }
            }

            true
        })
        .collect();

    if !matches!(options.sort, BenchmarkSort::Name) {
        filtered.retain(|entry| options.sort.extract(entry).is_some());
    }

    filtered.sort_by(|a, b| {
        let order = match options.sort {
            BenchmarkSort::Name => a.display_name.cmp(&b.display_name),
            _ => cmp_opt_f64(options.sort.extract(a), options.sort.extract(b))
                .then_with(|| a.display_name.cmp(&b.display_name)),
        };

        if options.descending {
            order.reverse()
        } else {
            order
        }
    });

    if let Some(limit) = options.limit {
        filtered.truncate(limit);
    }

    filtered
}

fn matches_source_filter(
    source_filter: SourceFilter,
    entry: &BenchmarkEntry,
    open_weights_map: &HashMap<String, bool>,
) -> bool {
    match source_filter {
        SourceFilter::All => true,
        SourceFilter::Open => open_weights_map.get(&entry.slug).copied().unwrap_or(false),
        SourceFilter::Closed => open_weights_map
            .get(&entry.slug)
            .map(|&open| !open)
            .unwrap_or(false),
    }
}

fn resolve_entry<'a>(entries: &'a [BenchmarkEntry], query: &str) -> Result<ResolveEntry<'a>> {
    let query_lower = query.to_lowercase();

    if let Some(entry) = entries
        .iter()
        .find(|entry| entry.slug.eq_ignore_ascii_case(query))
    {
        return Ok(ResolveEntry::Single(entry));
    }

    let exact_matches = matching_entries(entries, |entry| {
        entry.name.eq_ignore_ascii_case(query) || entry.display_name.eq_ignore_ascii_case(query)
    });
    match exact_matches.as_slice() {
        [entry] => return Ok(ResolveEntry::Single(entry)),
        [] => {}
        many => return Ok(ResolveEntry::Ambiguous(many.to_vec())),
    }

    let matches = matching_entries(entries, |entry| {
        entry.slug.to_lowercase().contains(&query_lower)
            || entry.name.to_lowercase().contains(&query_lower)
            || entry.display_name.to_lowercase().contains(&query_lower)
    });

    match matches.as_slice() {
        [] => bail!("No benchmark entry matched '{query}'"),
        [entry] => Ok(ResolveEntry::Single(entry)),
        many => Ok(ResolveEntry::Ambiguous(many.to_vec())),
    }
}

fn matching_entries<F>(entries: &[BenchmarkEntry], predicate: F) -> Vec<&BenchmarkEntry>
where
    F: Fn(&BenchmarkEntry) -> bool,
{
    let mut matches: Vec<_> = entries.iter().filter(|entry| predicate(entry)).collect();
    matches.sort_by(|a, b| {
        a.display_name
            .cmp(&b.display_name)
            .then_with(|| a.slug.cmp(&b.slug))
    });
    matches
}

fn filter_picker_entries<'a>(
    entries: &[&'a BenchmarkEntry],
    query: &str,
    sort: BenchmarkSort,
    descending: bool,
) -> Vec<&'a BenchmarkEntry> {
    let query = query.trim().to_lowercase();
    let mut visible: Vec<_> = entries
        .iter()
        .copied()
        .filter(|entry| {
            query.is_empty()
                || entry.slug.to_lowercase().contains(&query)
                || entry.name.to_lowercase().contains(&query)
                || entry.display_name.to_lowercase().contains(&query)
                || entry.creator.to_lowercase().contains(&query)
                || creator_label(entry).to_lowercase().contains(&query)
        })
        .collect();

    if !matches!(sort, BenchmarkSort::Name) {
        visible.retain(|entry| sort.extract(entry).is_some());
    }

    visible.sort_by(|a, b| {
        let order = match sort {
            BenchmarkSort::Name => a.display_name.cmp(&b.display_name),
            _ => cmp_opt_f64(sort.extract(a), sort.extract(b))
                .then_with(|| a.display_name.cmp(&b.display_name)),
        };
        if descending {
            order.reverse()
        } else {
            order
        }
    });

    visible
}

fn ambiguous_matches_message(query: &str, entries: &[&BenchmarkEntry]) -> String {
    let suggestions = entries
        .iter()
        .take(5)
        .map(|entry| entry.slug.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!("Benchmark query '{query}' was ambiguous; try a slug. Matches: {suggestions}")
}

fn creator_label(entry: &BenchmarkEntry) -> &str {
    if entry.creator_name.is_empty() {
        &entry.creator
    } else {
        &entry.creator_name
    }
}

fn reasoning_label(entry: &BenchmarkEntry) -> &'static str {
    match entry.reasoning_status {
        ReasoningStatus::Adaptive => "Adaptive",
        ReasoningStatus::Reasoning => "Reasoning",
        ReasoningStatus::NonReasoning => "Non-reasoning",
        ReasoningStatus::None => "Unknown",
    }
}

fn format_sort_value(sort: BenchmarkSort, entry: &BenchmarkEntry) -> String {
    match sort {
        BenchmarkSort::Name => entry.display_name.clone(),
        BenchmarkSort::ReleaseDate => entry
            .release_date
            .clone()
            .unwrap_or_else(|| "\u{2014}".to_string()),
        _ => format_metric(sort.extract(entry)),
    }
}

fn format_picker_sort_value(sort: BenchmarkSort, entry: &BenchmarkEntry) -> String {
    match sort {
        BenchmarkSort::Name => entry.slug.clone(),
        _ => format_sort_value(sort, entry),
    }
}

fn picker_sort_label(sort: BenchmarkSort) -> &'static str {
    match sort {
        BenchmarkSort::Name => "Slug",
        _ => sort.label(),
    }
}

fn format_metric(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "\u{2014}".to_string())
}

fn format_open_weights(open_weights: Option<bool>) -> String {
    match open_weights {
        Some(true) => "Open".to_string(),
        Some(false) => "Closed".to_string(),
        None => "\u{2014}".to_string(),
    }
}

fn print_detail(detail: &BenchmarkDetail<'_>) {
    println!("{}", detail.display_name);
    println!("{}", "=".repeat(detail.display_name.len()));
    println!();
    println!("Slug:         {}", detail.slug);
    println!("Name:         {}", detail.name);
    println!("Creator:      {} ({})", detail.creator_name, detail.creator);
    if !detail.creator_id.is_empty() {
        println!("Creator ID:   {}", detail.creator_id);
    }
    if let Some(release_date) = detail.release_date {
        println!("Released:     {}", release_date);
    }
    println!(
        "Open Weights: {}",
        match detail.open_weights {
            Some(true) => "Yes",
            Some(false) => "No",
            None => "Unknown",
        }
    );
    println!("Reasoning:    {}", detail.reasoning);
    if let Some(effort_level) = detail.effort_level {
        println!("Effort:       {}", effort_level);
    }
    if let Some(variant_tag) = detail.variant_tag {
        println!("Variant:      {}", variant_tag);
    }
    println!();

    println!("Indexes");
    println!("-------");
    println!("Intelligence: {}", format_metric(detail.intelligence_index));
    println!("Coding:       {}", format_metric(detail.coding_index));
    println!("Math:         {}", format_metric(detail.math_index));
    println!();

    println!("Benchmarks");
    println!("----------");
    println!("GPQA:         {}", format_metric(detail.gpqa));
    println!("MMLU-Pro:     {}", format_metric(detail.mmlu_pro));
    println!("HLE:          {}", format_metric(detail.hle));
    println!("LiveCodeBench: {}", format_metric(detail.livecodebench));
    println!("SciCode:      {}", format_metric(detail.scicode));
    println!("IFBench:      {}", format_metric(detail.ifbench));
    println!("LCR:          {}", format_metric(detail.lcr));
    println!(
        "TerminalBench: {}",
        format_metric(detail.terminalbench_hard)
    );
    println!("Tau2:         {}", format_metric(detail.tau2));
    println!("Math-500:     {}", format_metric(detail.math_500));
    println!("AIME:         {}", format_metric(detail.aime));
    println!("AIME 2025:    {}", format_metric(detail.aime_25));
    println!();

    println!("Performance");
    println!("-----------");
    println!("Output tok/s:  {}", format_metric(detail.output_tps));
    println!("TTFT:         {}", format_metric(detail.ttft));
    println!("TTFAT:        {}", format_metric(detail.ttfat));
    println!(
        "Tool Use:     {}",
        match detail.tool_call {
            Some(true) => "Yes",
            Some(false) => "No",
            None => "Unknown",
        }
    );
    if let Some(context_window) = detail.context_window {
        println!("Context:      {} tokens", context_window);
    }
    if let Some(max_output) = detail.max_output {
        println!("Max Output:   {} tokens", max_output);
    }
    println!();

    println!("Pricing");
    println!("-------");
    println!("Input $/M:    {}", format_metric(detail.price_input));
    println!("Output $/M:   {}", format_metric(detail.price_output));
    println!("Blended $/M:  {}", format_metric(detail.price_blended));
}

fn build_detail<'a>(
    entry: &'a BenchmarkEntry,
    open_weights_map: &HashMap<String, bool>,
) -> BenchmarkDetail<'a> {
    BenchmarkDetail {
        slug: entry.slug.as_str(),
        name: entry.name.as_str(),
        display_name: entry.display_name.as_str(),
        creator: entry.creator.as_str(),
        creator_name: creator_label(entry),
        creator_id: entry.creator_id.as_str(),
        release_date: entry.release_date.as_deref(),
        open_weights: open_weights_map.get(&entry.slug).copied(),
        reasoning: reasoning_label(entry),
        effort_level: entry.effort_level.as_deref(),
        variant_tag: entry.variant_tag.as_deref(),
        tool_call: entry.tool_call,
        context_window: entry.context_window,
        max_output: entry.max_output,
        intelligence_index: entry.intelligence_index,
        coding_index: entry.coding_index,
        math_index: entry.math_index,
        mmlu_pro: entry.mmlu_pro,
        gpqa: entry.gpqa,
        hle: entry.hle,
        livecodebench: entry.livecodebench,
        scicode: entry.scicode,
        ifbench: entry.ifbench,
        lcr: entry.lcr,
        terminalbench_hard: entry.terminalbench_hard,
        tau2: entry.tau2,
        math_500: entry.math_500,
        aime: entry.aime,
        aime_25: entry.aime_25,
        output_tps: entry.output_tps,
        ttft: entry.ttft,
        ttfat: entry.ttfat,
        price_input: entry.price_input,
        price_output: entry.price_output,
        price_blended: entry.price_blended,
    }
}

fn print_entry_detail(
    entry: &BenchmarkEntry,
    open_weights_map: &HashMap<String, bool>,
    json: bool,
) -> Result<()> {
    let detail = build_detail(entry, open_weights_map);
    if json {
        println!("{}", serde_json::to_string_pretty(&detail)?);
    } else {
        print_detail(&detail);
    }
    Ok(())
}

fn pick_benchmark<'a>(
    entries: Vec<&'a BenchmarkEntry>,
    open_weights_map: &'a HashMap<String, bool>,
    sort: BenchmarkSort,
    descending: bool,
    title: &str,
) -> Result<Option<&'a BenchmarkEntry>> {
    let mut picker = BenchmarkPicker::new(
        entries,
        open_weights_map,
        sort,
        descending,
        title.to_string(),
    );
    let mut terminal = PickerTerminal::new()?;

    loop {
        terminal.terminal.draw(|frame| picker.draw(frame))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        match event::read()? {
            Event::Resize(_, _) => terminal.terminal.autoresize()?,
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if picker.filter_mode {
                    match key.code {
                        KeyCode::Enter => picker.finish_filter(),
                        KeyCode::Esc => picker.clear_filter(),
                        KeyCode::Backspace => picker.pop_filter_char(),
                        KeyCode::Char(ch) => picker.push_filter_char(ch),
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => picker.previous(),
                    KeyCode::Down | KeyCode::Char('j') => picker.next(),
                    KeyCode::PageUp => picker.page_up(),
                    KeyCode::PageDown => picker.page_down(),
                    KeyCode::Home | KeyCode::Char('g') => picker.first(),
                    KeyCode::End | KeyCode::Char('G') => picker.last(),
                    KeyCode::Char('/') => picker.start_filter(),
                    KeyCode::Char('s') => picker.cycle_sort(),
                    KeyCode::Char('S') => picker.toggle_descending(),
                    KeyCode::Enter => return Ok(picker.selected()),
                    KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

fn truncate_picker_text(value: &str, max_chars: usize) -> String {
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

fn parse_date_to_numeric(date: &str) -> Option<f64> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return None;
    }

    let year = parts[0].parse::<u32>().ok()?;
    let month = parts[1].parse::<u32>().ok()?;
    let day = parts[2].parse::<u32>().ok()?;
    Some((year * 10000 + month * 100 + day) as f64)
}

fn cmp_opt_f64(a: Option<f64>, b: Option<f64>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(a_val), Some(b_val)) => a_val
            .partial_cmp(&b_val)
            .unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(
        slug: &str,
        display_name: &str,
        creator: &str,
        creator_name: &str,
        intelligence_index: Option<f64>,
    ) -> BenchmarkEntry {
        BenchmarkEntry {
            id: slug.to_string(),
            name: display_name.to_string(),
            slug: slug.to_string(),
            creator: creator.to_string(),
            creator_id: String::new(),
            creator_name: creator_name.to_string(),
            release_date: Some("2025-01-01".to_string()),
            intelligence_index,
            coding_index: Some(50.0),
            math_index: Some(55.0),
            mmlu_pro: Some(60.0),
            gpqa: Some(61.0),
            hle: Some(62.0),
            livecodebench: Some(63.0),
            scicode: Some(64.0),
            ifbench: Some(65.0),
            lcr: Some(66.0),
            terminalbench_hard: Some(67.0),
            tau2: Some(68.0),
            math_500: Some(69.0),
            aime: Some(70.0),
            aime_25: Some(71.0),
            output_tps: Some(72.0),
            ttft: Some(1.5),
            ttfat: Some(2.5),
            price_input: Some(3.5),
            price_output: Some(4.5),
            price_blended: Some(5.5),
            reasoning_status: ReasoningStatus::None,
            effort_level: None,
            variant_tag: None,
            display_name: display_name.to_string(),
            tool_call: Some(true),
            context_window: Some(200_000),
            max_output: Some(8_000),
        }
    }

    #[test]
    fn filter_entries_applies_sort_filters_and_limit() {
        let mut alpha = make_entry("alpha", "Alpha", "openai", "OpenAI", Some(90.0));
        alpha.reasoning_status = ReasoningStatus::Reasoning;

        let mut beta = make_entry("beta", "Beta", "meta", "Meta", Some(80.0));
        beta.reasoning_status = ReasoningStatus::NonReasoning;

        let gamma = make_entry("gamma", "Gamma", "openai", "OpenAI", None);

        let entries = vec![beta.clone(), gamma, alpha.clone()];
        let open_weights_map =
            HashMap::from([(alpha.slug.clone(), false), (beta.slug.clone(), true)]);

        let filtered = filter_entries(
            &entries,
            &open_weights_map,
            &ListOptions {
                search: Some("a".to_string()),
                creator: Some("openai".to_string()),
                sort: BenchmarkSort::Intelligence,
                descending: true,
                source_filter: SourceFilter::Closed,
                reasoning_filter: ReasoningFilter::Reasoning,
                limit: Some(5),
            },
        );

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].slug, "alpha");
    }

    #[test]
    fn filter_entries_sorts_name_ascending() {
        let entries = vec![
            make_entry("beta", "Beta", "meta", "Meta", Some(80.0)),
            make_entry("alpha", "Alpha", "openai", "OpenAI", Some(90.0)),
        ];

        let filtered = filter_entries(
            &entries,
            &HashMap::new(),
            &ListOptions {
                search: None,
                creator: None,
                sort: BenchmarkSort::Name,
                descending: false,
                source_filter: SourceFilter::All,
                reasoning_filter: ReasoningFilter::All,
                limit: None,
            },
        );

        assert_eq!(filtered[0].display_name, "Alpha");
        assert_eq!(filtered[1].display_name, "Beta");
    }

    #[test]
    fn resolve_entry_prefers_exact_slug_then_unique_partial() {
        let entries = vec![
            make_entry("gpt-4o", "GPT-4o", "openai", "OpenAI", Some(90.0)),
            make_entry(
                "claude-sonnet-4",
                "Claude Sonnet 4",
                "anthropic",
                "Anthropic",
                Some(88.0),
            ),
        ];

        match resolve_entry(&entries, "gpt-4o").unwrap() {
            ResolveEntry::Single(entry) => assert_eq!(entry.display_name, "GPT-4o"),
            ResolveEntry::Ambiguous(_) => panic!("expected exact slug to resolve to a single row"),
        }
        match resolve_entry(&entries, "Sonnet").unwrap() {
            ResolveEntry::Single(entry) => assert_eq!(entry.display_name, "Claude Sonnet 4"),
            ResolveEntry::Ambiguous(_) => {
                panic!("expected unique partial match to resolve to a single row")
            }
        }
    }

    #[test]
    fn resolve_entry_returns_ambiguous_partial_matches() {
        let entries = vec![
            make_entry(
                "claude-sonnet-4",
                "Claude Sonnet 4",
                "anthropic",
                "Anthropic",
                Some(88.0),
            ),
            make_entry(
                "claude-opus-4",
                "Claude Opus 4",
                "anthropic",
                "Anthropic",
                Some(89.0),
            ),
        ];

        match resolve_entry(&entries, "Claude").unwrap() {
            ResolveEntry::Single(_) => panic!("expected ambiguous partial query"),
            ResolveEntry::Ambiguous(matches) => {
                assert_eq!(matches.len(), 2);
                assert_eq!(matches[0].slug, "claude-opus-4");
                assert_eq!(matches[1].slug, "claude-sonnet-4");
            }
        }
    }

    #[test]
    fn resolve_entry_returns_ambiguous_exact_display_matches() {
        let entries = vec![
            make_entry(
                "claude-sonnet-4-6-adaptive",
                "Claude Sonnet 4.6",
                "anthropic",
                "Anthropic",
                Some(88.0),
            ),
            make_entry(
                "claude-sonnet-4-6-non-reasoning",
                "Claude Sonnet 4.6",
                "anthropic",
                "Anthropic",
                Some(82.0),
            ),
        ];

        match resolve_entry(&entries, "Claude Sonnet 4.6").unwrap() {
            ResolveEntry::Single(_) => panic!("expected ambiguous exact display-name query"),
            ResolveEntry::Ambiguous(matches) => {
                assert_eq!(matches.len(), 2);
                assert_eq!(matches[0].slug, "claude-sonnet-4-6-adaptive");
                assert_eq!(matches[1].slug, "claude-sonnet-4-6-non-reasoning");
            }
        }
    }

    #[test]
    fn ambiguous_matches_message_lists_candidate_slugs() {
        let entries = vec![
            make_entry("alpha", "Alpha", "openai", "OpenAI", Some(90.0)),
            make_entry("beta", "Beta", "openai", "OpenAI", Some(80.0)),
        ];
        let matches = vec![&entries[0], &entries[1]];

        let message = ambiguous_matches_message("a", &matches);
        assert!(message.contains("ambiguous"));
        assert!(message.contains("alpha"));
        assert!(message.contains("beta"));
    }

    #[test]
    fn filter_picker_entries_applies_live_query() {
        let entries = vec![
            make_entry(
                "claude-opus",
                "Claude Opus",
                "anthropic",
                "Anthropic",
                Some(90.0),
            ),
            make_entry(
                "gpt-5-3-codex",
                "GPT-5.3 Codex",
                "openai",
                "OpenAI",
                Some(88.0),
            ),
        ];
        let selected = entries.iter().collect::<Vec<_>>();

        let filtered = filter_picker_entries(&selected, "claude", BenchmarkSort::Name, false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].slug, "claude-opus");
    }

    #[test]
    fn filter_picker_entries_resorts_by_requested_metric() {
        let entries = vec![
            make_entry("alpha", "Alpha", "openai", "OpenAI", Some(80.0)),
            make_entry("beta", "Beta", "openai", "OpenAI", Some(90.0)),
        ];
        let selected = entries.iter().collect::<Vec<_>>();

        let filtered = filter_picker_entries(&selected, "", BenchmarkSort::Intelligence, true);
        assert_eq!(filtered[0].slug, "beta");
        assert_eq!(filtered[1].slug, "alpha");
    }

    #[test]
    fn print_detail_includes_key_sections() {
        let detail = BenchmarkDetail {
            slug: "gpt-4o",
            name: "GPT-4o",
            display_name: "GPT-4o",
            creator: "openai",
            creator_name: "OpenAI",
            creator_id: "",
            release_date: Some("2025-01-01"),
            open_weights: Some(false),
            reasoning: "Reasoning",
            effort_level: Some("high"),
            variant_tag: None,
            tool_call: Some(true),
            context_window: Some(200_000),
            max_output: Some(8_000),
            intelligence_index: Some(90.0),
            coding_index: Some(88.0),
            math_index: Some(87.0),
            mmlu_pro: Some(86.0),
            gpqa: Some(85.0),
            hle: Some(84.0),
            livecodebench: Some(83.0),
            scicode: Some(82.0),
            ifbench: Some(81.0),
            lcr: Some(80.0),
            terminalbench_hard: Some(79.0),
            tau2: Some(78.0),
            math_500: Some(77.0),
            aime: Some(76.0),
            aime_25: Some(75.0),
            output_tps: Some(74.0),
            ttft: Some(1.2),
            ttfat: Some(2.3),
            price_input: Some(3.4),
            price_output: Some(4.5),
            price_blended: Some(5.6),
        };

        let mut output = Vec::new();
        {
            use std::io::Write;

            writeln!(&mut output, "{}", detail.display_name).unwrap();
        }
        assert_eq!(String::from_utf8(output).unwrap().trim(), "GPT-4o");
        assert_eq!(format_open_weights(Some(false)), "Closed");
        assert_eq!(format_metric(Some(74.0)), "74.00");
    }
}
