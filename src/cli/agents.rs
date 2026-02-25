use anyhow::Result;
use clap::{CommandFactory, Parser};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Parser, Debug)]
#[command(name = "agents")]
#[command(about = "Track AI coding agent releases and changelogs")]
#[command(version)]
pub struct AgentsCli {
    #[command(subcommand)]
    pub command: Option<AgentsCommand>,
}

#[derive(clap::Subcommand, Debug)]
pub enum AgentsCommand {
    /// Show status table for all tracked agents
    Status,
    /// Show releases from the last 24 hours
    Latest,
    /// List available agent sources
    ListSources,
    /// View changelog for a specific agent tool
    #[command(external_subcommand)]
    Tool(Vec<String>),
}

/// Parse tool-specific flags from the external subcommand args
pub struct ToolArgs {
    pub tool: String,
    pub list: bool,
    pub pick: bool,
    pub version: Option<String>,
    pub web: bool,
}

impl ToolArgs {
    pub fn parse_from(args: Vec<String>) -> Result<Self> {
        if args.is_empty() {
            anyhow::bail!("No tool specified");
        }
        let tool = args[0].clone();
        let mut list = false;
        let mut pick = false;
        let mut version = None;
        let mut web = false;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--list" | "-l" => list = true,
                "--pick" | "-p" => pick = true,
                "--web" | "-w" => web = true,
                "--version" => {
                    i += 1;
                    version = Some(
                        args.get(i)
                            .cloned()
                            .ok_or_else(|| anyhow::anyhow!("--version requires a value"))?,
                    );
                }
                other => anyhow::bail!("Unknown flag: {}", other),
            }
            i += 1;
        }

        // Mutual exclusivity
        let mode_count = [list, pick, version.is_some()]
            .iter()
            .filter(|&&v| v)
            .count();
        if mode_count > 1 {
            anyhow::bail!("--list, --pick, and --version are mutually exclusive");
        }

        Ok(Self {
            tool,
            list,
            pick,
            version,
            web,
        })
    }
}

pub fn run() -> Result<()> {
    let cli = AgentsCli::parse();
    dispatch(cli.command)
}

pub fn run_with_command(command: Option<AgentsCommand>) -> Result<()> {
    dispatch(command)
}

fn dispatch(command: Option<AgentsCommand>) -> Result<()> {
    match command {
        Some(AgentsCommand::Status) => run_status(),
        Some(AgentsCommand::Latest) => run_latest(),
        Some(AgentsCommand::ListSources) => run_list_sources(),
        Some(AgentsCommand::Tool(args)) => {
            let tool_args = ToolArgs::parse_from(args)?;
            run_tool(tool_args)
        }
        None => {
            AgentsCli::command().print_help()?;
            println!();
            Ok(())
        }
    }
}

/// Fetch GitHub data for an agent using ETag-based conditional fetching.
/// Always checks GitHub for updates (like the TUI does), using the cached
/// ETag to avoid re-downloading unchanged data. Falls back to cached data
/// on network errors.
fn get_github_data(
    agent_name: &str,
    repo: &str,
    disk_cache: &mut crate::agents::cache::GitHubCache,
    runtime: &tokio::runtime::Runtime,
) -> Option<crate::agents::data::GitHubData> {
    eprint!("Fetching data for {}...", agent_name);
    let cache_arc = Arc::new(RwLock::new(disk_cache.clone()));
    let client = crate::agents::github::AsyncGitHubClient::with_disk_cache(None, cache_arc.clone());

    let result = runtime.block_on(client.fetch_conditional(repo));

    match result {
        crate::agents::github::ConditionalFetchResult::Fresh(data, _etag) => {
            eprintln!(" done.");
            // fetch_conditional already updates the disk cache internally,
            // so sync back from the Arc to our local copy
            let guard = runtime.block_on(cache_arc.read());
            *disk_cache = guard.clone();
            Some(data)
        }
        crate::agents::github::ConditionalFetchResult::NotModified => {
            eprintln!(" up to date.");
            // Cache key is repo (consistent with TUI and github.rs)
            disk_cache.get(repo).map(|c| c.data.to_github_data())
        }
        crate::agents::github::ConditionalFetchResult::Error(_) => {
            eprintln!(" failed.");
            // Fall back to cached data on network errors
            disk_cache.get(repo).map(|c| c.data.to_github_data())
        }
    }
}

fn run_status() -> Result<()> {
    use super::styles;

    let agents_file = crate::agents::loader::load_agents()?;
    let config = crate::config::Config::load()?;
    let mut disk_cache = crate::agents::cache::GitHubCache::load();

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec![
        styles::header_cell("Tool"),
        styles::header_cell("24h"),
        styles::header_cell("Installed"),
        styles::header_cell("Latest"),
        styles::header_cell("Updated"),
        styles::header_cell("Freq."),
    ]);

    let mut entries: Vec<_> = agents_file
        .agents
        .iter()
        .filter(|(id, _)| config.is_tracked(id))
        .collect();
    entries.sort_by(|(_, a), (_, b)| a.name.cmp(&b.name));

    let runtime = tokio::runtime::Runtime::new()?;
    for (_id, agent) in &entries {
        let installed = crate::agents::detect::detect_installed(agent);
        let github = get_github_data(&agent.name, &agent.repo, &mut disk_cache, &runtime);

        let latest_version = github
            .as_ref()
            .and_then(|g| g.latest_version())
            .unwrap_or("\u{2014}");

        let latest_date = github
            .as_ref()
            .and_then(|g| g.latest_release())
            .and_then(|r| r.date.as_deref())
            .and_then(crate::agents::helpers::parse_date);

        let is_24h = latest_date
            .map(|d| crate::agents::helpers::is_within_24h(&d))
            .unwrap_or(false);
        let updated = latest_date
            .map(|d| crate::agents::helpers::format_relative_time(&d))
            .unwrap_or_else(|| "\u{2014}".to_string());

        let release_dates: Vec<_> = github
            .as_ref()
            .map(|g| {
                g.releases
                    .iter()
                    .filter_map(|r| r.date.as_deref())
                    .filter_map(crate::agents::helpers::parse_date)
                    .collect()
            })
            .unwrap_or_default();

        let freq = crate::agents::helpers::calculate_release_frequency(&release_dates);

        let installed_str = installed
            .version
            .clone()
            .unwrap_or_else(|| "\u{2014}".to_string());

        let installed_cell = if installed_str == "\u{2014}" {
            styles::dim_cell(&installed_str)
        } else if installed_str == latest_version {
            styles::green_cell(&installed_str)
        } else {
            styles::yellow_cell(&installed_str)
        };

        table.add_row(vec![
            styles::bold_cell(&agent.name),
            if is_24h {
                styles::green_cell("\u{2713}")
            } else {
                comfy_table::Cell::new("")
            },
            installed_cell,
            styles::bold_cell(latest_version),
            styles::dim_cell(&updated),
            styles::dim_cell(&freq),
        ]);
    }

    disk_cache.save().ok();
    println!("{table}");
    Ok(())
}

fn run_latest() -> Result<()> {
    use super::styles;

    let agents_file = crate::agents::loader::load_agents()?;
    let config = crate::config::Config::load()?;
    let mut disk_cache = crate::agents::cache::GitHubCache::load();

    let mut recent: Vec<(String, String, String, chrono::DateTime<chrono::Utc>)> = Vec::new();

    let runtime = tokio::runtime::Runtime::new()?;
    for (id, agent) in &agents_file.agents {
        if !config.is_tracked(id) {
            continue;
        }
        if let Some(github) = get_github_data(&agent.name, &agent.repo, &mut disk_cache, &runtime) {
            for release in &github.releases {
                if let Some(date) = release
                    .date
                    .as_deref()
                    .and_then(crate::agents::helpers::parse_date)
                {
                    if crate::agents::helpers::is_within_24h(&date) {
                        recent.push((
                            agent.name.clone(),
                            release.version.clone(),
                            release.changelog.clone().unwrap_or_default(),
                            date,
                        ));
                    }
                }
            }
        }
    }

    disk_cache.save().ok();

    if recent.is_empty() {
        println!("No releases in the last 24 hours.");
        return Ok(());
    }

    recent.sort_by(|a, b| b.3.cmp(&a.3));

    for (name, version, body, date) in &recent {
        let ago = crate::agents::helpers::format_relative_time(date);
        println!(
            "\n{} {} ({})",
            styles::agent_name(name),
            styles::key_value(version),
            styles::dim(&ago)
        );
        println!("{}", styles::separator(40));
        if body.is_empty() {
            println!("(no changelog)");
        } else {
            print_changelog_body(body);
        }
    }

    Ok(())
}

fn run_list_sources() -> Result<()> {
    use super::styles;

    let agents_file = crate::agents::loader::load_agents()?;
    let config = crate::config::Config::load()?;

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec![
        styles::header_cell("ID"),
        styles::header_cell("Name"),
        styles::header_cell("Repo"),
        styles::header_cell("CLI Binary"),
        styles::header_cell("Tracked"),
    ]);

    let mut entries: Vec<_> = agents_file.agents.iter().collect();
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));

    for (id, agent) in entries {
        let tracked = if config.is_tracked(id) {
            styles::green_cell("\u{2713}")
        } else {
            comfy_table::Cell::new("")
        };
        let cli = agent.cli_binary.as_deref().unwrap_or("\u{2014}");
        table.add_row(vec![
            comfy_table::Cell::new(id.as_str()),
            styles::bold_cell(&agent.name),
            styles::dim_cell(&agent.repo),
            comfy_table::Cell::new(cli),
            tracked,
        ]);
    }

    println!("{table}");
    Ok(())
}

fn run_tool(args: ToolArgs) -> Result<()> {
    use super::styles;

    let agents_file = crate::agents::loader::load_agents()?;
    let mut disk_cache = crate::agents::cache::GitHubCache::load();

    let (_agent_id, agent) = resolve_tool(&args.tool, &agents_file)?;

    if args.web {
        let url = format!("https://github.com/{}/releases", agent.repo);
        open::that(&url)?;
        println!("Opened {}", styles::url(&url));
        return Ok(());
    }

    let runtime = tokio::runtime::Runtime::new()?;
    let github =
        get_github_data(&agent.name, &agent.repo, &mut disk_cache, &runtime).unwrap_or_default();

    disk_cache.save().ok();

    if args.list {
        return run_version_list(agent, &github);
    }

    if args.pick {
        return run_pick(agent, &github);
    }

    // Default: show latest changelog (or specific version)
    let release = if let Some(ref ver) = args.version {
        github.releases.iter().find(|r| r.version == *ver)
    } else {
        github.latest_release()
    };

    match release {
        Some(r) => print_release(&agent.name, r),
        None => {
            let target = args.version.as_deref().unwrap_or("latest");
            println!(
                "{} No release found for {} ({})",
                styles::error_prefix(),
                styles::agent_name(&agent.name),
                styles::input_badge(target)
            );
        }
    }

    Ok(())
}

fn resolve_tool<'a>(
    tool: &str,
    agents_file: &'a crate::agents::data::AgentsFile,
) -> Result<(String, &'a crate::agents::data::Agent)> {
    use super::styles;
    // Exact ID match
    if let Some(agent) = agents_file.agents.get(tool) {
        return Ok((tool.to_string(), agent));
    }
    // Match by cli_binary
    for (id, agent) in &agents_file.agents {
        if agent.cli_binary.as_deref() == Some(tool) {
            return Ok((id.clone(), agent));
        }
    }
    // Fuzzy match on name
    let lower = tool.to_lowercase();
    let matches: Vec<_> = agents_file
        .agents
        .iter()
        .filter(|(_, a)| a.name.to_lowercase().contains(&lower))
        .collect();
    match matches.len() {
        1 => return Ok((matches[0].0.clone(), matches[0].1)),
        n if n > 1 => {
            let names: Vec<_> = matches.iter().map(|(id, _)| styles::code_ref(id)).collect();
            anyhow::bail!(
                "{} Ambiguous tool {}. Matches: {}",
                styles::error_prefix(),
                styles::input_badge(tool),
                names.join(", ")
            );
        }
        _ => {}
    }
    anyhow::bail!(
        "{} Unknown agent {}. Run {} to see available agents.",
        styles::error_prefix(),
        styles::input_badge(tool),
        styles::code_badge("agents list-sources")
    )
}

fn print_release(name: &str, release: &crate::agents::data::Release) {
    use super::styles;

    let version = &release.version;
    let date = release
        .date
        .as_deref()
        .and_then(crate::agents::helpers::parse_date)
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown date".to_string());
    println!(
        "{} {} ({})",
        styles::agent_name(name),
        styles::key_value(version),
        styles::dim(&date)
    );
    println!("{}", styles::separator(40));
    if let Some(body) = &release.changelog {
        print_changelog_body(body);
    } else {
        println!("(no changelog body)");
    }
}

fn print_changelog_body(body: &str) {
    if super::styles::is_tty() {
        let skin = super::styles::changelog_skin();
        let rendered = skin.term_text(body).to_string();
        let styled = super::styles::style_urls(&rendered);
        print!("{}", styled);
    } else {
        // Plain text when piped
        let (sections, ungrouped) = crate::agents::changelog_parser::parse_release_body(body);
        for change in &ungrouped {
            println!("  - {}", change);
        }
        for section in &sections {
            println!("\n[{}]", section.name);
            for change in &section.changes {
                println!("  - {}", change);
            }
        }
    }
}

fn run_version_list(
    agent: &crate::agents::data::Agent,
    github: &crate::agents::data::GitHubData,
) -> Result<()> {
    use super::styles;

    let count = github.releases.len().to_string();
    println!(
        "{} \u{2014} {} releases\n",
        styles::agent_name(&agent.name),
        styles::dim(&count)
    );

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec![
        styles::header_cell("Version"),
        styles::header_cell("Released"),
        styles::header_cell("Ago"),
    ]);

    for release in &github.releases {
        let parsed = release
            .date
            .as_deref()
            .and_then(crate::agents::helpers::parse_date);
        let date_str = parsed
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "\u{2014}".to_string());
        let ago = parsed
            .map(|d| crate::agents::helpers::format_relative_time(&d))
            .unwrap_or_else(|| "\u{2014}".to_string());
        table.add_row(vec![
            styles::bold_cell(&release.version),
            comfy_table::Cell::new(&date_str),
            styles::dim_cell(&ago),
        ]);
    }

    println!("{table}");
    Ok(())
}

fn run_pick(
    agent: &crate::agents::data::Agent,
    github: &crate::agents::data::GitHubData,
) -> Result<()> {
    if github.releases.is_empty() {
        println!("No releases found for {}", agent.name);
        return Ok(());
    }

    let items: Vec<String> = github
        .releases
        .iter()
        .map(|r| {
            let parsed = r
                .date
                .as_deref()
                .and_then(crate::agents::helpers::parse_date);
            let date = parsed
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let ago = parsed
                .map(|d| crate::agents::helpers::format_relative_time(&d))
                .unwrap_or_default();
            format!("{:<16} {:<12} {}", r.version, date, ago)
        })
        .collect();

    let theme = super::styles::picker_theme();
    let selection = dialoguer::FuzzySelect::with_theme(&theme)
        .with_prompt(format!("Select a {} release", agent.name))
        .items(&items)
        .default(0)
        .interact()?;

    let release = &github.releases[selection];
    println!();
    print_release(&agent.name, release);
    Ok(())
}
