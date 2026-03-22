use std::collections::HashMap;

use ratatui::style::Color;
use ratatui::widgets::ListState;

use crate::benchmarks::{BenchmarkEntry, BenchmarkStore, ReasoningFilter};
use crate::formatting::{cmp_opt_f64, parse_date_to_numeric};
use crate::tui::widgets::scroll_offset::ScrollOffset;

/// Page size for page up/down navigation
const PAGE_SIZE: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BenchmarkSortColumn {
    #[default]
    Intelligence,
    Coding,
    Math,
    Gpqa,
    MMLUPro,
    Hle,
    LiveCode,
    SciCode,
    IFBench,
    Lcr,
    Terminal,
    Tau2,
    Speed,
    Ttft,
    Ttfat,
    PriceInput,
    PriceOutput,
    PriceBlended,
    Name,
    ReleaseDate,
}

impl BenchmarkSortColumn {
    pub const ALL: &[Self] = &[
        Self::Intelligence,
        Self::Coding,
        Self::Math,
        Self::Gpqa,
        Self::MMLUPro,
        Self::Hle,
        Self::LiveCode,
        Self::SciCode,
        Self::IFBench,
        Self::Lcr,
        Self::Terminal,
        Self::Tau2,
        Self::Speed,
        Self::Ttft,
        Self::Ttfat,
        Self::PriceInput,
        Self::PriceOutput,
        Self::PriceBlended,
        Self::Name,
        Self::ReleaseDate,
    ];

    pub fn picker_label(&self) -> &'static str {
        match self {
            Self::Intelligence => "Intelligence Index",
            Self::Coding => "Coding Index",
            Self::Math => "Math Index",
            Self::Gpqa => "GPQA Diamond",
            Self::MMLUPro => "MMLU-Pro",
            Self::Hle => "HLE",
            Self::LiveCode => "LiveCodeBench",
            Self::SciCode => "SciCode",
            Self::IFBench => "IFBench",
            Self::Lcr => "LCR",
            Self::Terminal => "TerminalBench",
            Self::Tau2 => "Tau2",
            Self::Speed => "Output Speed (tok/s)",
            Self::Ttft => "Time to First Token",
            Self::Ttfat => "Time to First Action Token",
            Self::PriceInput => "Price: Input $/M",
            Self::PriceOutput => "Price: Output $/M",
            Self::PriceBlended => "Price: Blended $/M",
            Self::Name => "Name",
            Self::ReleaseDate => "Release Date",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Intelligence => "Intel",
            Self::Coding => "Code",
            Self::Math => "Math",
            Self::Gpqa => "GPQA",
            Self::MMLUPro => "MMLU",
            Self::Hle => "HLE",
            Self::LiveCode => "LCBench",
            Self::SciCode => "SciCode",
            Self::IFBench => "IFBench",
            Self::Lcr => "LCR",
            Self::Terminal => "Terminal",
            Self::Tau2 => "Tau2",
            Self::Speed => "Tok/s",
            Self::Ttft => "TTFT",
            Self::Ttfat => "TTFAT",
            Self::PriceInput => "In $/M",
            Self::PriceOutput => "Out $/M",
            Self::PriceBlended => "Bld $/M",
            Self::Name => "Name",
            Self::ReleaseDate => "Date",
        }
    }

    /// Whether descending is the default sort direction for this column
    pub fn default_descending(&self) -> bool {
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

    /// Returns the columns to display in the list based on the active sort.
    /// Always includes Name + the 3 composite indexes, plus the sort column's group.
    pub fn visible_columns(&self) -> Vec<BenchmarkSortColumn> {
        use BenchmarkSortColumn::*;

        // Always-shown base columns
        let mut cols = vec![Name, Intelligence, Coding, Math];

        // Add the group that the sort column belongs to
        let group = match self {
            Intelligence | Coding | Math => vec![],
            Gpqa | MMLUPro | Hle => vec![Gpqa, MMLUPro, Hle],
            LiveCode | SciCode | Terminal => vec![LiveCode, SciCode, Terminal],
            IFBench | Lcr | Tau2 => vec![IFBench, Lcr, Tau2],
            Speed | Ttft | Ttfat => vec![Speed, Ttft, Ttfat],
            PriceInput | PriceOutput | PriceBlended => vec![PriceInput, PriceOutput, PriceBlended],
            Name => vec![Speed],
            ReleaseDate => vec![ReleaseDate],
        };

        for col in group {
            if !cols.contains(&col) {
                cols.push(col);
            }
        }

        cols
    }

    /// Extract the relevant field value from a benchmark entry.
    /// Returns `Some` for numeric columns with data, `None` for missing data.
    /// Name always returns `Some` (never filters out entries).
    pub fn extract(&self, entry: &BenchmarkEntry) -> Option<f64> {
        match self {
            Self::Intelligence => entry.intelligence_index,
            Self::Coding => entry.coding_index,
            Self::Math => entry.math_index,
            Self::Gpqa => entry.gpqa,
            Self::MMLUPro => entry.mmlu_pro,
            Self::Hle => entry.hle,
            Self::LiveCode => entry.livecodebench,
            Self::SciCode => entry.scicode,
            Self::IFBench => entry.ifbench,
            Self::Lcr => entry.lcr,
            Self::Terminal => entry.terminalbench_hard,
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
                .as_ref()
                .and_then(|d| parse_date_to_numeric(d)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BenchmarkFocus {
    Creators,
    #[default]
    List,
    Details,
    Compare,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatorListItem {
    All,
    GroupHeader(String), // non-selectable section header
    Creator(String),     // creator slug
}

/// Per-model source filter: uses open_weights_map only (unmatched entries excluded from filtering).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SourceFilter {
    #[default]
    All,
    Open,
    Closed,
}

impl SourceFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Open,
            Self::Open => Self::Closed,
            Self::Closed => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Open => "Open",
            Self::Closed => "Closed",
        }
    }

    /// Check if an entry passes the filter using per-model open_weights_map.
    /// Unmatched entries (not in the map) are excluded when filtering by Open or Closed.
    pub fn matches(
        self,
        entry: &BenchmarkEntry,
        open_weights_map: &std::collections::HashMap<String, bool>,
    ) -> bool {
        match self {
            Self::All => true,
            Self::Open => open_weights_map.get(&entry.slug).copied().unwrap_or(false),
            Self::Closed => open_weights_map
                .get(&entry.slug)
                .map(|&ow| !ow)
                .unwrap_or(false),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorRegion {
    US,
    China,
    Europe,
    MiddleEast,
    SouthKorea,
    Canada,
    Other,
}

impl CreatorRegion {
    pub fn label(self) -> &'static str {
        match self {
            Self::US => "US",
            Self::China => "China",
            Self::Europe => "Europe",
            Self::MiddleEast => "Middle East",
            Self::SouthKorea => "S. Korea",
            Self::Canada => "Canada",
            Self::Other => "Other",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::US => "US",
            Self::China => "CN",
            Self::Europe => "EU",
            Self::MiddleEast => "ME",
            Self::SouthKorea => "KR",
            Self::Canada => "CA",
            Self::Other => "??",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::US => Color::Blue,
            Self::China => Color::Red,
            Self::Europe => Color::Magenta,
            Self::MiddleEast => Color::Yellow,
            Self::SouthKorea => Color::Cyan,
            Self::Canada => Color::Green,
            Self::Other => Color::DarkGray,
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "US" => Some(Self::US),
            "China" => Some(Self::China),
            "Europe" => Some(Self::Europe),
            "Middle East" => Some(Self::MiddleEast),
            "S. Korea" => Some(Self::SouthKorea),
            "Canada" => Some(Self::Canada),
            "Other" => Some(Self::Other),
            _ => None,
        }
    }

    pub fn from_creator(slug: &str) -> Self {
        match slug {
            // United States
            "openai" | "anthropic" | "google" | "meta" | "xai" | "aws" | "nvidia"
            | "perplexity" | "azure" | "ibm" | "databricks" | "servicenow" | "snowflake"
            | "liquidai" | "nous-research" | "ai2" | "prime-intellect" | "deepcogito"
            | "reka-ai" => Self::US,
            // China
            "deepseek" | "alibaba" | "kimi" | "minimax" | "stepfun" | "baidu"
            | "bytedance_seed" | "xiaomi" | "inclusionai" | "kwaikat" | "zai" | "openchat" => {
                Self::China
            }
            // Europe
            "mistral" => Self::Europe,
            // Middle East (UAE, Israel)
            "tii-uae" | "mbzuai" | "ai21-labs" => Self::MiddleEast,
            // South Korea
            "naver" | "korea-telecom" | "lg" | "upstage" | "motif-technologies" => Self::SouthKorea,
            // Canada
            "cohere" => Self::Canada,
            // Other
            _ => Self::Other,
        }
    }
}

/// How creators are grouped in the sidebar (toggle, not filter).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreatorGrouping {
    #[default]
    None,
    ByRegion,
    ByType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorType {
    Startup,
    Giant,
    Research,
}

impl CreatorType {
    pub fn color(self) -> Color {
        match self {
            Self::Startup => Color::Green,
            Self::Giant => Color::Blue,
            Self::Research => Color::Magenta,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Startup => "Startup",
            Self::Giant => "Big Tech",
            Self::Research => "Research",
        }
    }

    pub fn short_label(self) -> &'static str {
        match self {
            Self::Startup => "SU",
            Self::Giant => "BT",
            Self::Research => "RS",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "Startup" => Some(Self::Startup),
            "Big Tech" => Some(Self::Giant),
            "Research" => Some(Self::Research),
            _ => None,
        }
    }

    pub fn from_creator(slug: &str) -> Self {
        match slug {
            // Big tech / large corporations
            "google" | "meta" | "aws" | "nvidia" | "alibaba" | "azure" | "ibm" | "servicenow"
            | "snowflake" | "baidu" | "bytedance_seed" | "xiaomi" | "naver" | "korea-telecom"
            | "lg" | "kwaikat" | "databricks" | "zai" | "inclusionai" => Self::Giant,
            // Research labs / institutes / nonprofits
            "tii-uae" | "mbzuai" | "nous-research" | "ai2" | "openchat" => Self::Research,
            // AI-focused startups (default)
            _ => Self::Startup,
        }
    }
}

/// Pre-computed creator info: display name and model counts.
struct CreatorInfo {
    display_name: String,
    count: usize,
    filtered_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScatterAxis {
    #[default]
    Intelligence,
    Coding,
    Math,
    Speed,
    Price,
}

impl ScatterAxis {
    pub fn next(self) -> Self {
        match self {
            Self::Intelligence => Self::Coding,
            Self::Coding => Self::Math,
            Self::Math => Self::Speed,
            Self::Speed => Self::Price,
            Self::Price => Self::Intelligence,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Intelligence => "Intelligence",
            Self::Coding => "Coding",
            Self::Math => "Math",
            Self::Speed => "Speed (tok/s)",
            Self::Price => "Price ($/M)",
        }
    }

    pub fn extract(self) -> fn(&crate::benchmarks::BenchmarkEntry) -> Option<f64> {
        match self {
            Self::Intelligence => |e| e.intelligence_index,
            Self::Coding => |e| e.coding_index,
            Self::Math => |e| e.math_index,
            Self::Speed => |e| e.output_tps,
            Self::Price => |e| e.price_blended,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RadarPreset {
    #[default]
    Agentic,
    Academic,
    Indexes,
}

impl RadarPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Agentic => "Agentic",
            Self::Academic => "Academic",
            Self::Indexes => "Indexes",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Agentic => Self::Academic,
            Self::Academic => Self::Indexes,
            Self::Indexes => Self::Agentic,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BottomView {
    #[default]
    Detail,
    H2H,
    Scatter,
    Radar,
}

pub struct BenchmarksApp {
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub list_state: ListState,
    pub focus: BenchmarkFocus,
    pub sort_column: BenchmarkSortColumn,
    pub sort_descending: bool,
    pub search_query: String,
    // Creator sidebar
    pub creator_list_items: Vec<CreatorListItem>,
    pub selected_creator: usize,
    pub creator_list_state: ListState,
    pub source_filter: SourceFilter,
    pub reasoning_filter: ReasoningFilter,
    pub creator_grouping: CreatorGrouping,
    creator_info: HashMap<String, CreatorInfo>,
    pub bottom_view: BottomView,
    pub h2h_scroll: ScrollOffset,
    pub show_detail_overlay: bool,
    pub show_creators_in_compare: bool,
    pub scatter_x: ScatterAxis,
    pub scatter_y: ScatterAxis,
    pub radar_preset: RadarPreset,
    pub show_sort_picker: bool,
    pub sort_picker_selected: usize,
    pub loading: bool,
    pub detail_scroll: ScrollOffset,
}

impl BenchmarksApp {
    pub fn new(store: &BenchmarkStore, open_weights_map: &HashMap<String, bool>) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut creator_list_state = ListState::default();
        creator_list_state.select(Some(0));

        let mut app = Self {
            filtered_indices: Vec::new(),
            selected: 0,
            list_state,
            focus: BenchmarkFocus::default(),
            sort_column: BenchmarkSortColumn::default(),
            sort_descending: true,
            search_query: String::new(),
            creator_list_items: Vec::new(),
            selected_creator: 0,
            creator_list_state,
            source_filter: SourceFilter::default(),
            reasoning_filter: ReasoningFilter::default(),
            creator_grouping: CreatorGrouping::default(),
            creator_info: HashMap::new(),
            bottom_view: BottomView::default(),
            h2h_scroll: ScrollOffset::default(),
            show_detail_overlay: false,
            show_creators_in_compare: false,
            scatter_x: ScatterAxis::default(),
            scatter_y: ScatterAxis::Coding,
            radar_preset: RadarPreset::default(),
            show_sort_picker: false,
            sort_picker_selected: 0,
            loading: true,
            detail_scroll: ScrollOffset::default(),
        };

        app.build_creator_list(store, open_weights_map);
        app.update_filtered(store, open_weights_map);
        app
    }

    /// Rebuild all derived state after the underlying store changes.
    /// Re-derives creator list, filtered indices, and resets selection.
    pub fn rebuild(&mut self, store: &BenchmarkStore, open_weights_map: &HashMap<String, bool>) {
        self.build_creator_list(store, open_weights_map);
        self.selected_creator = 0;
        self.creator_list_state.select(Some(0));
        self.selected = 0;
        self.update_filtered(store, open_weights_map);
        self.reset_detail_scroll();
    }

    /// Rebuild creator list and filtered entries after any search/filter change.
    /// Preserves the selected creator if it's still visible.
    pub fn rebuild_after_filter_change(
        &mut self,
        store: &BenchmarkStore,
        open_weights_map: &HashMap<String, bool>,
    ) {
        // Remember which creator was selected
        let prev_creator_slug = match self.creator_list_items.get(self.selected_creator) {
            Some(CreatorListItem::Creator(slug)) => Some(slug.clone()),
            _ => None, // All or GroupHeader
        };

        self.build_creator_list(store, open_weights_map);

        // Try to find the previously selected creator in the new list
        let new_pos = prev_creator_slug.and_then(|prev_slug| {
            self.creator_list_items.iter().position(
                |item| matches!(item, CreatorListItem::Creator(slug) if *slug == prev_slug),
            )
        });

        self.selected_creator = new_pos.unwrap_or(0);
        self.creator_list_state.select(Some(self.selected_creator));
        self.selected = 0;
        self.update_filtered(store, open_weights_map);
        self.reset_detail_scroll();
    }

    fn has_active_filters(&self) -> bool {
        !self.search_query.is_empty()
            || self.source_filter != SourceFilter::All
            || self.reasoning_filter != ReasoningFilter::default()
    }

    fn entry_matches_filters(
        &self,
        entry: &BenchmarkEntry,
        open_weights_map: &HashMap<String, bool>,
    ) -> bool {
        if !self.source_filter.matches(entry, open_weights_map) {
            return false;
        }
        if !self.reasoning_filter.matches(entry) {
            return false;
        }
        if !self.search_query.is_empty() {
            let query_lower = self.search_query.to_lowercase();
            return entry.name.to_lowercase().contains(&query_lower)
                || entry.creator.to_lowercase().contains(&query_lower)
                || entry.slug.to_lowercase().contains(&query_lower);
        }
        true
    }

    fn build_creator_list(
        &mut self,
        store: &BenchmarkStore,
        open_weights_map: &HashMap<String, bool>,
    ) {
        let mut info: HashMap<String, CreatorInfo> = HashMap::new();
        let filtering = self.has_active_filters();

        for entry in store.entries() {
            if entry.creator.is_empty() {
                continue;
            }
            let passes = !filtering || self.entry_matches_filters(entry, open_weights_map);
            info.entry(entry.creator.clone())
                .and_modify(|i| {
                    i.count += 1;
                    if passes {
                        i.filtered_count += 1;
                    }
                })
                .or_insert_with(|| CreatorInfo {
                    display_name: if entry.creator_name.is_empty() {
                        entry.creator.clone()
                    } else {
                        entry.creator_name.clone()
                    },
                    count: 1,
                    filtered_count: if passes { 1 } else { 0 },
                });
        }

        let mut creators: Vec<String> = if filtering {
            info.iter()
                .filter(|(_, i)| i.filtered_count > 0)
                .map(|(k, _)| k.clone())
                .collect()
        } else {
            info.keys().cloned().collect()
        };
        creators.sort_by(|a, b| {
            let name_a = &info[a].display_name;
            let name_b = &info[b].display_name;
            name_a.to_lowercase().cmp(&name_b.to_lowercase())
        });

        self.creator_list_items = Vec::with_capacity(creators.len() + 1);
        self.creator_list_items.push(CreatorListItem::All);

        match self.creator_grouping {
            CreatorGrouping::None => {
                for slug in creators {
                    self.creator_list_items.push(CreatorListItem::Creator(slug));
                }
            }
            CreatorGrouping::ByRegion => {
                let regions = [
                    CreatorRegion::US,
                    CreatorRegion::China,
                    CreatorRegion::Europe,
                    CreatorRegion::MiddleEast,
                    CreatorRegion::SouthKorea,
                    CreatorRegion::Canada,
                    CreatorRegion::Other,
                ];
                for region in &regions {
                    let group: Vec<&String> = creators
                        .iter()
                        .filter(|s| CreatorRegion::from_creator(s) == *region)
                        .collect();
                    if group.is_empty() {
                        continue;
                    }
                    self.creator_list_items
                        .push(CreatorListItem::GroupHeader(region.label().to_string()));
                    for slug in group {
                        self.creator_list_items
                            .push(CreatorListItem::Creator(slug.clone()));
                    }
                }
            }
            CreatorGrouping::ByType => {
                let types = [
                    CreatorType::Startup,
                    CreatorType::Giant,
                    CreatorType::Research,
                ];
                for ct in &types {
                    let group: Vec<&String> = creators
                        .iter()
                        .filter(|s| CreatorType::from_creator(s) == *ct)
                        .collect();
                    if group.is_empty() {
                        continue;
                    }
                    self.creator_list_items
                        .push(CreatorListItem::GroupHeader(ct.label().to_string()));
                    for slug in group {
                        self.creator_list_items
                            .push(CreatorListItem::Creator(slug.clone()));
                    }
                }
            }
        }

        self.creator_info = info;
    }

    /// Get (display_name, count) for a creator slug.
    /// Returns filtered count when search/filters are active, total count otherwise.
    pub fn creator_display<'a>(&'a self, slug: &'a str) -> (&'a str, usize) {
        self.creator_info
            .get(slug)
            .map(|i| {
                let count = if self.has_active_filters() {
                    i.filtered_count
                } else {
                    i.count
                };
                (i.display_name.as_str(), count)
            })
            .unwrap_or((slug, 0))
    }

    /// Total filtered count across all visible creators.
    pub fn filtered_creator_count(&self) -> usize {
        if self.has_active_filters() {
            self.creator_list_items
                .iter()
                .filter_map(|item| match item {
                    CreatorListItem::Creator(slug) => {
                        self.creator_info.get(slug).map(|i| i.filtered_count)
                    }
                    _ => None,
                })
                .sum()
        } else {
            self.creator_info.values().map(|i| i.count).sum()
        }
    }

    /// Get the currently selected creator slug, or None for "All".
    fn selected_creator_slug(&self) -> Option<&str> {
        match self.creator_list_items.get(self.selected_creator) {
            Some(CreatorListItem::Creator(slug)) => Some(slug),
            _ => None,
        }
    }

    /// Get the display name of the currently selected creator, or None for "All".
    pub fn selected_creator_name(&self) -> Option<&str> {
        let slug = self.selected_creator_slug()?;
        Some(self.creator_display(slug).0)
    }

    pub fn update_filtered(
        &mut self,
        store: &BenchmarkStore,
        open_weights_map: &HashMap<String, bool>,
    ) {
        let query_lower = self.search_query.to_lowercase();
        let creator_slug = self.selected_creator_slug().map(|s| s.to_owned());
        let source_filter = self.source_filter;
        let reasoning_filter = self.reasoning_filter.clone();

        self.filtered_indices = store
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                // Per-model source filter (open/closed)
                if !source_filter.matches(entry, open_weights_map) {
                    return false;
                }
                // Reasoning filter
                if !reasoning_filter.matches(entry) {
                    return false;
                }
                // Creator filter
                if let Some(ref slug) = creator_slug {
                    if entry.creator != *slug {
                        return false;
                    }
                }
                // Search filter
                if !query_lower.is_empty() {
                    return entry.name.to_lowercase().contains(&query_lower)
                        || entry.creator.to_lowercase().contains(&query_lower)
                        || entry.slug.to_lowercase().contains(&query_lower);
                }
                true
            })
            .map(|(i, _)| i)
            .collect();

        // Null-filter: hide entries missing data for the active sort column
        if !matches!(self.sort_column, BenchmarkSortColumn::Name) {
            let col = self.sort_column;
            let entries = store.entries();
            self.filtered_indices
                .retain(|&i| col.extract(&entries[i]).is_some());
        }

        self.apply_sort(store);

        if self.selected >= self.filtered_indices.len() {
            self.selected = 0;
        }
        self.list_state.select(Some(self.selected));
        self.reset_detail_scroll();
    }

    pub fn apply_sort(&mut self, store: &BenchmarkStore) {
        let entries = store.entries();
        let col = self.sort_column;
        let desc = self.sort_descending;

        self.filtered_indices.sort_by(|&a, &b| {
            let ea = &entries[a];
            let eb = &entries[b];

            let ord = match col {
                BenchmarkSortColumn::Intelligence => {
                    cmp_opt_f64(ea.intelligence_index, eb.intelligence_index)
                }
                BenchmarkSortColumn::Coding => cmp_opt_f64(ea.coding_index, eb.coding_index),
                BenchmarkSortColumn::Math => cmp_opt_f64(ea.math_index, eb.math_index),
                BenchmarkSortColumn::Gpqa => cmp_opt_f64(ea.gpqa, eb.gpqa),
                BenchmarkSortColumn::MMLUPro => cmp_opt_f64(ea.mmlu_pro, eb.mmlu_pro),
                BenchmarkSortColumn::Hle => cmp_opt_f64(ea.hle, eb.hle),
                BenchmarkSortColumn::LiveCode => cmp_opt_f64(ea.livecodebench, eb.livecodebench),
                BenchmarkSortColumn::SciCode => cmp_opt_f64(ea.scicode, eb.scicode),
                BenchmarkSortColumn::IFBench => cmp_opt_f64(ea.ifbench, eb.ifbench),
                BenchmarkSortColumn::Lcr => cmp_opt_f64(ea.lcr, eb.lcr),
                BenchmarkSortColumn::Terminal => {
                    cmp_opt_f64(ea.terminalbench_hard, eb.terminalbench_hard)
                }
                BenchmarkSortColumn::Tau2 => cmp_opt_f64(ea.tau2, eb.tau2),
                BenchmarkSortColumn::Speed => cmp_opt_f64(ea.output_tps, eb.output_tps),
                BenchmarkSortColumn::Ttft => cmp_opt_f64(ea.ttft, eb.ttft),
                BenchmarkSortColumn::Ttfat => cmp_opt_f64(ea.ttfat, eb.ttfat),
                BenchmarkSortColumn::PriceInput => cmp_opt_f64(ea.price_input, eb.price_input),
                BenchmarkSortColumn::PriceOutput => cmp_opt_f64(ea.price_output, eb.price_output),
                BenchmarkSortColumn::PriceBlended => {
                    cmp_opt_f64(ea.price_blended, eb.price_blended)
                }
                BenchmarkSortColumn::Name => ea.name.cmp(&eb.name),
                BenchmarkSortColumn::ReleaseDate => cmp_opt_f64(
                    ea.release_date
                        .as_ref()
                        .and_then(|d| parse_date_to_numeric(d)),
                    eb.release_date
                        .as_ref()
                        .and_then(|d| parse_date_to_numeric(d)),
                ),
            };

            if desc {
                ord.reverse()
            } else {
                ord
            }
        });
    }

    pub fn toggle_sort_direction(&mut self, store: &BenchmarkStore) {
        self.sort_descending = !self.sort_descending;
        self.apply_sort(store);
    }

    /// Jump directly to a sort column. If already on that column, toggle direction.
    pub fn quick_sort(
        &mut self,
        col: BenchmarkSortColumn,
        store: &BenchmarkStore,
        open_weights_map: &HashMap<String, bool>,
    ) {
        if self.sort_column == col {
            self.sort_descending = !self.sort_descending;
            self.apply_sort(store);
        } else {
            self.sort_column = col;
            self.sort_descending = col.default_descending();
            self.update_filtered(store, open_weights_map);
        }
    }

    pub fn current_entry<'a>(
        &self,
        store: &'a BenchmarkStore,
    ) -> Option<&'a crate::benchmarks::BenchmarkEntry> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&i| store.entries().get(i))
    }

    // --- List navigation ---

    pub fn reset_detail_scroll(&self) {
        self.detail_scroll.jump_top();
    }

    pub fn next(&mut self) {
        if self.selected < self.filtered_indices.len().saturating_sub(1) {
            self.selected += 1;
            self.list_state.select(Some(self.selected));
            self.reset_detail_scroll();
        }
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.list_state.select(Some(self.selected));
            self.reset_detail_scroll();
        }
    }

    pub fn select_first(&mut self) {
        if self.selected > 0 {
            self.selected = 0;
            self.list_state.select(Some(self.selected));
            self.reset_detail_scroll();
        }
    }

    pub fn select_last(&mut self) {
        let last = self.filtered_indices.len().saturating_sub(1);
        if self.selected < last {
            self.selected = last;
            self.list_state.select(Some(self.selected));
            self.reset_detail_scroll();
        }
    }

    pub fn page_down(&mut self) {
        let last_index = self.filtered_indices.len().saturating_sub(1);
        self.selected = (self.selected + PAGE_SIZE).min(last_index);
        self.list_state.select(Some(self.selected));
        self.reset_detail_scroll();
    }

    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(PAGE_SIZE);
        self.list_state.select(Some(self.selected));
        self.reset_detail_scroll();
    }

    pub fn cycle_source_filter(
        &mut self,
        store: &BenchmarkStore,
        open_weights_map: &HashMap<String, bool>,
    ) {
        self.source_filter = self.source_filter.next();
        self.rebuild_after_filter_change(store, open_weights_map);
    }

    pub fn cycle_reasoning_filter(
        &mut self,
        store: &BenchmarkStore,
        open_weights_map: &HashMap<String, bool>,
    ) {
        self.reasoning_filter = self.reasoning_filter.next();
        self.rebuild_after_filter_change(store, open_weights_map);
    }

    pub fn toggle_region_grouping(
        &mut self,
        store: &BenchmarkStore,
        open_weights_map: &HashMap<String, bool>,
    ) {
        self.creator_grouping = if self.creator_grouping == CreatorGrouping::ByRegion {
            CreatorGrouping::None
        } else {
            CreatorGrouping::ByRegion
        };
        self.build_creator_list(store, open_weights_map);
        self.selected_creator = 0;
        self.creator_list_state.select(Some(0));
    }

    pub fn toggle_type_grouping(
        &mut self,
        store: &BenchmarkStore,
        open_weights_map: &HashMap<String, bool>,
    ) {
        self.creator_grouping = if self.creator_grouping == CreatorGrouping::ByType {
            CreatorGrouping::None
        } else {
            CreatorGrouping::ByType
        };
        self.build_creator_list(store, open_weights_map);
        self.selected_creator = 0;
        self.creator_list_state.select(Some(0));
    }

    // --- Creator sidebar navigation ---

    fn is_header(&self, idx: usize) -> bool {
        matches!(
            self.creator_list_items.get(idx),
            Some(CreatorListItem::GroupHeader(_))
        )
    }

    /// Move to the next selectable item, skipping headers.
    fn skip_to_selectable(&mut self, start: usize, forward: bool) {
        let max = self.creator_list_items.len().saturating_sub(1);
        let mut idx = start;
        while self.is_header(idx) {
            if forward {
                if idx >= max {
                    return; // can't go further
                }
                idx += 1;
            } else {
                if idx == 0 {
                    return;
                }
                idx -= 1;
            }
        }
        self.selected_creator = idx;
        self.creator_list_state.select(Some(idx));
    }

    pub fn next_creator(&mut self) {
        let max = self.creator_list_items.len().saturating_sub(1);
        if self.selected_creator < max {
            self.skip_to_selectable(self.selected_creator + 1, true);
        }
    }

    pub fn prev_creator(&mut self) {
        if self.selected_creator > 0 {
            self.skip_to_selectable(self.selected_creator - 1, false);
        }
    }

    pub fn select_first_creator(&mut self) {
        self.skip_to_selectable(0, true);
    }

    pub fn select_last_creator(&mut self) {
        let max = self.creator_list_items.len().saturating_sub(1);
        self.skip_to_selectable(max, false);
    }

    pub fn page_down_creator(&mut self) {
        let max = self.creator_list_items.len().saturating_sub(1);
        let target = (self.selected_creator + PAGE_SIZE).min(max);
        self.skip_to_selectable(target, true);
    }

    pub fn page_up_creator(&mut self) {
        let target = self.selected_creator.saturating_sub(PAGE_SIZE);
        self.skip_to_selectable(target, true);
    }

    pub fn cycle_bottom_view(&mut self) {
        self.bottom_view = match self.bottom_view {
            BottomView::H2H => BottomView::Scatter,
            BottomView::Scatter => BottomView::Radar,
            BottomView::Radar => BottomView::H2H,
            BottomView::Detail => BottomView::H2H,
        };
    }

    pub fn cycle_scatter_x(&mut self) {
        self.scatter_x = self.scatter_x.next();
    }

    pub fn cycle_scatter_y(&mut self) {
        self.scatter_y = self.scatter_y.next();
    }

    pub fn cycle_radar_preset(&mut self) {
        self.radar_preset = self.radar_preset.next();
    }

    /// Auto-transition bottom view based on selection count.
    pub fn update_bottom_view(&mut self, selection_count: usize) {
        if selection_count >= 2 && self.bottom_view == BottomView::Detail {
            self.bottom_view = BottomView::H2H;
            self.h2h_scroll.jump_top();
        } else if selection_count < 2 && self.bottom_view != BottomView::Detail {
            self.bottom_view = BottomView::Detail;
            self.show_detail_overlay = false;
            self.h2h_scroll.jump_top();
        }
    }

    // --- Focus ---

    pub fn focus_right(&mut self, has_compare: bool) {
        self.focus = if has_compare {
            let left = if self.show_creators_in_compare {
                BenchmarkFocus::Creators
            } else {
                BenchmarkFocus::List
            };
            match self.focus {
                BenchmarkFocus::Compare => left,
                _ => BenchmarkFocus::Compare,
            }
        } else {
            match self.focus {
                BenchmarkFocus::Creators => BenchmarkFocus::List,
                BenchmarkFocus::List => BenchmarkFocus::Details,
                BenchmarkFocus::Details => BenchmarkFocus::Creators,
                BenchmarkFocus::Compare => BenchmarkFocus::Creators,
            }
        };
    }

    pub fn focus_left(&mut self, has_compare: bool) {
        self.focus = if has_compare {
            let left = if self.show_creators_in_compare {
                BenchmarkFocus::Creators
            } else {
                BenchmarkFocus::List
            };
            match self.focus {
                BenchmarkFocus::Compare => left,
                _ => BenchmarkFocus::Compare,
            }
        } else {
            match self.focus {
                BenchmarkFocus::Creators => BenchmarkFocus::Details,
                BenchmarkFocus::List => BenchmarkFocus::Creators,
                BenchmarkFocus::Details => BenchmarkFocus::List,
                BenchmarkFocus::Compare => BenchmarkFocus::List,
            }
        };
    }

    // --- H2H Scroll ---

    pub fn scroll_h2h_down(&mut self) {
        self.h2h_scroll.increment(1);
    }

    pub fn scroll_h2h_up(&mut self) {
        self.h2h_scroll.decrement(1);
    }

    pub fn scroll_h2h_top(&mut self) {
        self.h2h_scroll.jump_top();
    }

    pub fn scroll_h2h_page_down(&mut self, page: usize) {
        self.h2h_scroll.increment(page as u16);
    }

    pub fn scroll_h2h_page_up(&mut self, page: usize) {
        self.h2h_scroll.decrement(page as u16);
    }
}
