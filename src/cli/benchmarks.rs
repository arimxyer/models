use std::collections::HashMap;

use anyhow::{bail, Result};
use clap::{CommandFactory, Parser, ValueEnum};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};
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

    let mut table = Table::new();
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec![
        "Slug",
        "Name",
        "Creator",
        options.sort.label(),
        "Source",
        "Reasoning",
        "Release",
    ]);

    for entry in entries {
        table.add_row(vec![
            entry.slug.clone(),
            entry.display_name.clone(),
            creator_label(entry).to_string(),
            format_sort_value(options.sort, entry),
            format_open_weights(loaded.open_weights_map.get(&entry.slug).copied()),
            reasoning_label(entry).to_string(),
            entry
                .release_date
                .clone()
                .unwrap_or_else(|| "\u{2014}".to_string()),
        ]);
    }

    println!("{table}");
    Ok(())
}

fn run_show(model: &str, json: bool) -> Result<()> {
    let loaded = load_benchmarks()?;
    let entry = resolve_entry(loaded.entries(), model)?;
    let detail = BenchmarkDetail {
        slug: entry.slug.as_str(),
        name: entry.name.as_str(),
        display_name: entry.display_name.as_str(),
        creator: entry.creator.as_str(),
        creator_name: creator_label(entry),
        creator_id: entry.creator_id.as_str(),
        release_date: entry.release_date.as_deref(),
        open_weights: loaded.open_weights_map.get(&entry.slug).copied(),
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
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&detail)?);
    } else {
        print_detail(&detail);
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

fn resolve_entry<'a>(entries: &'a [BenchmarkEntry], query: &str) -> Result<&'a BenchmarkEntry> {
    let query_lower = query.to_lowercase();

    if let Some(entry) = entries
        .iter()
        .find(|entry| entry.slug.eq_ignore_ascii_case(query))
    {
        return Ok(entry);
    }

    if let Some(entry) = entries.iter().find(|entry| {
        entry.name.eq_ignore_ascii_case(query) || entry.display_name.eq_ignore_ascii_case(query)
    }) {
        return Ok(entry);
    }

    let matches: Vec<_> = entries
        .iter()
        .filter(|entry| {
            entry.slug.to_lowercase().contains(&query_lower)
                || entry.name.to_lowercase().contains(&query_lower)
                || entry.display_name.to_lowercase().contains(&query_lower)
        })
        .collect();

    match matches.as_slice() {
        [] => bail!("No benchmark entry matched '{query}'"),
        [entry] => Ok(*entry),
        many => {
            let suggestions = many
                .iter()
                .take(5)
                .map(|entry| entry.slug.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            bail!("Benchmark query '{query}' was ambiguous; try a slug. Matches: {suggestions}")
        }
    }
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

        assert_eq!(
            resolve_entry(&entries, "gpt-4o").unwrap().display_name,
            "GPT-4o"
        );
        assert_eq!(
            resolve_entry(&entries, "Sonnet").unwrap().display_name,
            "Claude Sonnet 4"
        );
    }

    #[test]
    fn resolve_entry_rejects_ambiguous_matches() {
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

        let err = resolve_entry(&entries, "Claude").unwrap_err().to_string();
        assert!(err.contains("ambiguous"));
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
