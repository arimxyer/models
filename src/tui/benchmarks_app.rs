use ratatui::widgets::ListState;

use crate::benchmarks::BenchmarkStore;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BenchmarkFocus {
    #[default]
    List,
    Details,
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
}

impl BenchmarksApp {
    pub fn new(store: &BenchmarkStore) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        let mut app = Self {
            filtered_indices: Vec::new(),
            selected: 0,
            list_state,
            focus: BenchmarkFocus::default(),
            sort_column: BenchmarkSortColumn::default(),
            sort_descending: true, // Scores default descending
            search_query: String::new(),
            detail_scroll: 0,
        };

        app.update_filtered(store);
        app
    }

    pub fn update_filtered(&mut self, store: &BenchmarkStore) {
        let query_lower = self.search_query.to_lowercase();

        self.filtered_indices = store
            .entries()
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                if query_lower.is_empty() {
                    return true;
                }
                entry.name.to_lowercase().contains(&query_lower)
                    || entry.creator.to_lowercase().contains(&query_lower)
                    || entry.slug.to_lowercase().contains(&query_lower)
            })
            .map(|(i, _)| i)
            .collect();

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
        self.apply_sort(store);
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

    pub fn switch_focus(&mut self) {
        self.focus = match self.focus {
            BenchmarkFocus::List => BenchmarkFocus::Details,
            BenchmarkFocus::Details => BenchmarkFocus::List,
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
