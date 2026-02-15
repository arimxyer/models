use std::collections::HashMap;

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
            Self::Name => Self::Intelligence,
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
        }
    }

    /// Whether descending is the default sort direction for this column
    pub fn default_descending(&self) -> bool {
        !matches!(self, Self::Name | Self::Ttft)
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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BenchmarkFocus {
    Creators,
    #[default]
    List,
    Details,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreatorListItem {
    All,
    Creator(String), // creator slug
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
    pub detail_scroll: u16,
    // Creator sidebar
    pub creator_list_items: Vec<CreatorListItem>,
    pub selected_creator: usize,
    pub creator_list_state: ListState,
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
            detail_scroll: 0,
            creator_list_items: Vec::new(),
            selected_creator: 0,
            creator_list_state,
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

        let mut creators: Vec<String> = info.keys().cloned().collect();
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

        self.filtered_indices = store
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
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
            self.detail_scroll = 0;
        }
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.list_state.select(Some(self.selected));
            self.detail_scroll = 0;
        }
    }

    pub fn page_down(&mut self) {
        let last_index = self.filtered_indices.len().saturating_sub(1);
        self.selected = (self.selected + PAGE_SIZE).min(last_index);
        self.list_state.select(Some(self.selected));
        self.detail_scroll = 0;
    }

    pub fn page_up(&mut self) {
        self.selected = self.selected.saturating_sub(PAGE_SIZE);
        self.list_state.select(Some(self.selected));
        self.detail_scroll = 0;
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
            BenchmarkFocus::List => BenchmarkFocus::Details,
            BenchmarkFocus::Details => BenchmarkFocus::Creators,
        };
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
