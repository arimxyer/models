use std::collections::HashMap;

use ratatui::style::Color;
use ratatui::widgets::ListState;

use crate::benchmarks::{BenchmarkEntry, BenchmarkStore};

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
    Name,
    ReleaseDate,
}

impl BenchmarkSortColumn {
    pub fn next(self) -> Self {
        match self {
            Self::Intelligence => Self::Coding,
            Self::Coding => Self::Math,
            Self::Math => Self::Gpqa,
            Self::Gpqa => Self::MMLUPro,
            Self::MMLUPro => Self::Hle,
            Self::Hle => Self::LiveCode,
            Self::LiveCode => Self::SciCode,
            Self::SciCode => Self::IFBench,
            Self::IFBench => Self::Lcr,
            Self::Lcr => Self::Terminal,
            Self::Terminal => Self::Tau2,
            Self::Tau2 => Self::Speed,
            Self::Speed => Self::Ttft,
            Self::Ttft => Self::Name,
            Self::Name => Self::ReleaseDate,
            Self::ReleaseDate => Self::Intelligence,
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
            Self::Name => "Name",
            Self::ReleaseDate => "Date",
        }
    }

    /// Whether descending is the default sort direction for this column
    pub fn default_descending(&self) -> bool {
        !matches!(self, Self::Name | Self::Ttft)
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
            Speed | Ttft => vec![Speed, Ttft],
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatorListItem {
    All,
    Creator(String), // creator slug
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorOpenness {
    Open,
    Closed,
    Mixed,
}

impl CreatorOpenness {
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Closed => "Closed",
            Self::Mixed => "Mixed",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Open => Color::Green,
            Self::Closed => Color::Red,
            Self::Mixed => Color::Yellow,
        }
    }

    pub fn from_creator(slug: &str) -> Self {
        match slug {
            // Closed-source (API-only, no public weights)
            "anthropic" | "aws" => Self::Closed,
            // Mixed (both open-weight and proprietary models)
            "openai" | "google" | "mistral" | "xai" | "cohere" | "perplexity" | "stepfun"
            | "reka-ai" => Self::Mixed,
            // Open-weight (default for most other creators)
            _ => Self::Open,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpennessFilter {
    #[default]
    All,
    Open,
    Closed,
    Mixed,
}

impl OpennessFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Open,
            Self::Open => Self::Closed,
            Self::Closed => Self::Mixed,
            Self::Mixed => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Open => "Open",
            Self::Closed => "Closed",
            Self::Mixed => "Mixed",
        }
    }

    pub fn matches(self, openness: CreatorOpenness) -> bool {
        match self {
            Self::All => true,
            Self::Open => openness == CreatorOpenness::Open,
            Self::Closed => openness == CreatorOpenness::Closed,
            Self::Mixed => openness == CreatorOpenness::Mixed,
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
            // Middle East (UAE)
            "tii-uae" | "mbzuai" => Self::MiddleEast,
            // South Korea
            "naver" | "korea-telecom" | "lg" | "upstage" | "motif-technologies" => Self::SouthKorea,
            // Canada
            "cohere" => Self::Canada,
            // Other (Israel: ai21-labs, etc.)
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RegionFilter {
    #[default]
    All,
    US,
    China,
    Europe,
    MiddleEast,
    SouthKorea,
    Canada,
    Other,
}

impl RegionFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::US,
            Self::US => Self::China,
            Self::China => Self::Europe,
            Self::Europe => Self::MiddleEast,
            Self::MiddleEast => Self::SouthKorea,
            Self::SouthKorea => Self::Canada,
            Self::Canada => Self::Other,
            Self::Other => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::US => "US",
            Self::China => "China",
            Self::Europe => "Europe",
            Self::MiddleEast => "Mid East",
            Self::SouthKorea => "S. Korea",
            Self::Canada => "Canada",
            Self::Other => "Other",
        }
    }

    pub fn matches(self, region: CreatorRegion) -> bool {
        match self {
            Self::All => true,
            Self::US => region == CreatorRegion::US,
            Self::China => region == CreatorRegion::China,
            Self::Europe => region == CreatorRegion::Europe,
            Self::MiddleEast => region == CreatorRegion::MiddleEast,
            Self::SouthKorea => region == CreatorRegion::SouthKorea,
            Self::Canada => region == CreatorRegion::Canada,
            Self::Other => region == CreatorRegion::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatorType {
    Startup,
    Giant,
    Research,
}

impl CreatorType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Startup => "Startup",
            Self::Giant => "Big Tech",
            Self::Research => "Research",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TypeFilter {
    #[default]
    All,
    Startup,
    Giant,
    Research,
}

impl TypeFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Startup,
            Self::Startup => Self::Giant,
            Self::Giant => Self::Research,
            Self::Research => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Startup => "Startup",
            Self::Giant => "Big Tech",
            Self::Research => "Research",
        }
    }

    pub fn matches(self, creator_type: CreatorType) -> bool {
        match self {
            Self::All => true,
            Self::Startup => creator_type == CreatorType::Startup,
            Self::Giant => creator_type == CreatorType::Giant,
            Self::Research => creator_type == CreatorType::Research,
        }
    }
}

/// Pre-computed creator info: (display_name, entry_count)
struct CreatorInfo {
    display_name: String,
    count: usize,
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
    pub openness_filter: OpennessFilter,
    pub region_filter: RegionFilter,
    pub type_filter: TypeFilter,
    creator_info: HashMap<String, CreatorInfo>,
}

impl BenchmarksApp {
    pub fn new(store: &BenchmarkStore) -> Self {
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
            openness_filter: OpennessFilter::default(),
            region_filter: RegionFilter::default(),
            type_filter: TypeFilter::default(),
            creator_info: HashMap::new(),
        };

        app.build_creator_list(store);
        app.update_filtered(store);
        app
    }

    fn build_creator_list(&mut self, store: &BenchmarkStore) {
        let mut info: HashMap<String, CreatorInfo> = HashMap::new();

        for entry in store.entries() {
            if entry.creator.is_empty() {
                continue;
            }
            info.entry(entry.creator.clone())
                .and_modify(|i| i.count += 1)
                .or_insert_with(|| CreatorInfo {
                    display_name: if entry.creator_name.is_empty() {
                        entry.creator.clone()
                    } else {
                        entry.creator_name.clone()
                    },
                    count: 1,
                });
        }

        let mut creators: Vec<String> = info
            .keys()
            .filter(|slug| {
                self.openness_filter
                    .matches(CreatorOpenness::from_creator(slug))
                    && self
                        .region_filter
                        .matches(CreatorRegion::from_creator(slug))
                    && self.type_filter.matches(CreatorType::from_creator(slug))
            })
            .cloned()
            .collect();
        creators.sort_by(|a, b| {
            let name_a = &info[a].display_name;
            let name_b = &info[b].display_name;
            name_a.to_lowercase().cmp(&name_b.to_lowercase())
        });

        self.creator_list_items = Vec::with_capacity(creators.len() + 1);
        self.creator_list_items.push(CreatorListItem::All);
        for slug in creators {
            self.creator_list_items.push(CreatorListItem::Creator(slug));
        }

        self.creator_info = info;
    }

    /// Get (display_name, count) for a creator slug.
    pub fn creator_display<'a>(&'a self, slug: &'a str) -> (&'a str, usize) {
        self.creator_info
            .get(slug)
            .map(|i| (i.display_name.as_str(), i.count))
            .unwrap_or((slug, 0))
    }

    /// Get the openness classification for a creator slug.
    pub fn creator_openness(&self, slug: &str) -> CreatorOpenness {
        CreatorOpenness::from_creator(slug)
    }

    /// Get the region for a creator slug.
    pub fn creator_region(&self, slug: &str) -> CreatorRegion {
        CreatorRegion::from_creator(slug)
    }

    /// Get the currently selected creator slug, or None for "All".
    fn selected_creator_slug(&self) -> Option<&str> {
        match self.creator_list_items.get(self.selected_creator) {
            Some(CreatorListItem::Creator(slug)) => Some(slug),
            _ => None,
        }
    }

    pub fn update_filtered(&mut self, store: &BenchmarkStore) {
        let query_lower = self.search_query.to_lowercase();
        let creator_slug = self.selected_creator_slug().map(|s| s.to_owned());
        let openness_filter = self.openness_filter;

        self.filtered_indices = store
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                // Creator attribute filters (apply even when "All" creators selected)
                if !openness_filter.matches(CreatorOpenness::from_creator(&entry.creator)) {
                    return false;
                }
                if !self
                    .region_filter
                    .matches(CreatorRegion::from_creator(&entry.creator))
                {
                    return false;
                }
                if !self
                    .type_filter
                    .matches(CreatorType::from_creator(&entry.creator))
                {
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

    pub fn cycle_sort(&mut self, store: &BenchmarkStore) {
        self.sort_column = self.sort_column.next();
        self.sort_descending = self.sort_column.default_descending();
        self.update_filtered(store);
    }

    pub fn toggle_sort_direction(&mut self, store: &BenchmarkStore) {
        self.sort_descending = !self.sort_descending;
        self.apply_sort(store);
    }

    /// Jump directly to a sort column. If already on that column, toggle direction.
    pub fn quick_sort(&mut self, col: BenchmarkSortColumn, store: &BenchmarkStore) {
        if self.sort_column == col {
            self.sort_descending = !self.sort_descending;
            self.apply_sort(store);
        } else {
            self.sort_column = col;
            self.sort_descending = col.default_descending();
            self.update_filtered(store);
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

    pub fn next(&mut self) {
        if self.selected < self.filtered_indices.len().saturating_sub(1) {
            self.selected += 1;
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.list_state.select(Some(self.selected));
        }
    }

    pub fn page_down(&mut self) {
        let last_index = self.filtered_indices.len().saturating_sub(1);
        self.selected = (self.selected + PAGE_SIZE).min(last_index);
        self.list_state.select(Some(self.selected));
    }

    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(PAGE_SIZE);
        self.list_state.select(Some(self.selected));
    }

    pub fn cycle_openness_filter(&mut self, store: &BenchmarkStore) {
        self.openness_filter = self.openness_filter.next();
        self.build_creator_list(store);
        self.selected_creator = 0;
        self.creator_list_state.select(Some(0));
        self.update_filtered(store);
    }

    pub fn cycle_region_filter(&mut self, store: &BenchmarkStore) {
        self.region_filter = self.region_filter.next();
        self.build_creator_list(store);
        self.selected_creator = 0;
        self.creator_list_state.select(Some(0));
        self.update_filtered(store);
    }

    pub fn cycle_type_filter(&mut self, store: &BenchmarkStore) {
        self.type_filter = self.type_filter.next();
        self.build_creator_list(store);
        self.selected_creator = 0;
        self.creator_list_state.select(Some(0));
        self.update_filtered(store);
    }

    // --- Creator sidebar navigation ---

    pub fn next_creator(&mut self) {
        let max = self.creator_list_items.len().saturating_sub(1);
        if self.selected_creator < max {
            self.selected_creator += 1;
            self.creator_list_state.select(Some(self.selected_creator));
        }
    }

    pub fn prev_creator(&mut self) {
        if self.selected_creator > 0 {
            self.selected_creator -= 1;
            self.creator_list_state.select(Some(self.selected_creator));
        }
    }

    pub fn page_down_creator(&mut self) {
        let max = self.creator_list_items.len().saturating_sub(1);
        self.selected_creator = (self.selected_creator + PAGE_SIZE).min(max);
        self.creator_list_state.select(Some(self.selected_creator));
    }

    pub fn page_up_creator(&mut self) {
        self.selected_creator = self.selected_creator.saturating_sub(PAGE_SIZE);
        self.creator_list_state.select(Some(self.selected_creator));
    }

    // --- Focus ---

    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            BenchmarkFocus::Creators => BenchmarkFocus::List,
            BenchmarkFocus::List => BenchmarkFocus::Creators,
        };
    }
}

/// Parse "YYYY-MM-DD" to a numeric value for sorting (e.g., 20240115.0)
fn parse_date_to_numeric(date: &str) -> Option<f64> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() == 3 {
        let year = parts[0].parse::<u32>().ok()?;
        let month = parts[1].parse::<u32>().ok()?;
        let day = parts[2].parse::<u32>().ok()?;
        Some((year * 10000 + month * 100 + day) as f64)
    } else {
        None
    }
}

/// Compare two Option<f64> values, putting None last
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
