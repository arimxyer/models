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
                    version = args.get(i).cloned();
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

/// Fetch GitHub data for an agent, using disk cache with live fallback.
/// If cached data exists, returns it immediately. Otherwise fetches live
/// from the GitHub API and updates the disk cache.
fn get_github_data(
    agent_id: &str,
    agent_name: &str,
    repo: &str,
    disk_cache: &mut crate::agents::cache::GitHubCache,
) -> Option<crate::agents::data::GitHubData> {
    // Try cache first
    if let Some(cached) = disk_cache.get(agent_id) {
        return Some(cached.data.to_github_data());
    }

    // Live fetch fallback
    eprint!("Fetching data for {}...", agent_name);
    let runtime = tokio::runtime::Runtime::new().ok()?;
    let cache_arc = Arc::new(RwLock::new(disk_cache.clone()));
    let client = crate::agents::github::AsyncGitHubClient::with_disk_cache(None, cache_arc.clone());

    let result = runtime.block_on(client.fetch_conditional(repo));

    match result {
        crate::agents::github::ConditionalFetchResult::Fresh(data, etag) => {
            eprintln!(" done.");
            let serializable = crate::agents::cache::SerializableGitHubData::from(&data);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            disk_cache.insert(
                agent_id.to_string(),
                crate::agents::cache::CachedGitHubData {
                    data: serializable,
                    etag,
                    fetched_at: now,
                },
            );
            Some(data)
        }
        crate::agents::github::ConditionalFetchResult::NotModified => {
            eprintln!(" done.");
            // Shouldn't happen if cache was empty, but handle gracefully
            disk_cache.get(agent_id).map(|c| c.data.to_github_data())
        }
        crate::agents::github::ConditionalFetchResult::Error(_) => {
            eprintln!(" failed.");
            None
        }
    }
}

fn run_status() -> Result<()> {
    let agents_file = crate::agents::loader::load_agents()?;
    let config = crate::config::Config::load()?;
    let mut disk_cache = crate::agents::cache::GitHubCache::load();

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec![
        "Tool",
        "24h",
        "Installed",
        "Latest",
        "Updated",
        "Freq.",
    ]);

    let mut entries: Vec<_> = agents_file
        .agents
        .iter()
        .filter(|(id, _)| config.is_tracked(id))
        .collect();
    entries.sort_by_key(|(_, agent)| agent.name.clone());

    for (id, agent) in &entries {
        let installed = crate::agents::detect::detect_installed(agent);
        let github = get_github_data(id, &agent.name, &agent.repo, &mut disk_cache);

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

        table.add_row(vec![
            agent.name.clone(),
            if is_24h {
                "\u{2713}".to_string()
            } else {
                String::new()
            },
            installed
                .version
                .clone()
                .unwrap_or_else(|| "\u{2014}".to_string()),
            latest_version.to_string(),
            updated,
            freq,
        ]);
    }

    disk_cache.save().ok();
    println!("{table}");
    Ok(())
}

fn run_latest() -> Result<()> {
    let agents_file = crate::agents::loader::load_agents()?;
    let config = crate::config::Config::load()?;
    let mut disk_cache = crate::agents::cache::GitHubCache::load();

    let mut recent: Vec<(String, String, String, chrono::DateTime<chrono::Utc>)> = Vec::new();

    for (id, agent) in &agents_file.agents {
        if !config.is_tracked(id) {
            continue;
        }
        if let Some(github) = get_github_data(id, &agent.name, &agent.repo, &mut disk_cache) {
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
        println!("\n{} {} ({})", name, version, ago);
        println!("{}", "\u{2500}".repeat(40));
        if body.is_empty() {
            println!("(no changelog)");
        } else {
            print_changelog_body(body);
        }
    }

    Ok(())
}

fn run_list_sources() -> Result<()> {
    let agents_file = crate::agents::loader::load_agents()?;
    let config = crate::config::Config::load()?;

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec!["ID", "Name", "Repo", "CLI Binary", "Tracked"]);

    let mut entries: Vec<_> = agents_file.agents.iter().collect();
    entries.sort_by_key(|(id, _)| (*id).clone());

    for (id, agent) in entries {
        let tracked = if config.is_tracked(id) {
            "\u{2713}"
        } else {
            ""
        };
        let cli = agent.cli_binary.as_deref().unwrap_or("\u{2014}");
        table.add_row(vec![id.as_str(), &agent.name, &agent.repo, cli, tracked]);
    }

    println!("{table}");
    Ok(())
}

fn run_tool(args: ToolArgs) -> Result<()> {
    let agents_file = crate::agents::loader::load_agents()?;
    let mut disk_cache = crate::agents::cache::GitHubCache::load();

    let (agent_id, agent) = resolve_tool(&args.tool, &agents_file)?;

    if args.web {
        let url = format!("https://github.com/{}/releases", agent.repo);
        open::that(&url)?;
        println!("Opened {}", url);
        return Ok(());
    }

    let github =
        get_github_data(&agent_id, &agent.name, &agent.repo, &mut disk_cache).unwrap_or_default();

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
            println!("No release found for {} ({})", agent.name, target);
        }
    }

    Ok(())
}

fn resolve_tool<'a>(
    tool: &str,
    agents_file: &'a crate::agents::data::AgentsFile,
) -> Result<(String, &'a crate::agents::data::Agent)> {
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
    for (id, agent) in &agents_file.agents {
        if agent.name.to_lowercase().contains(&lower) {
            return Ok((id.clone(), agent));
        }
    }
    anyhow::bail!(
        "Unknown agent: '{}'. Run 'agents list-sources' to see available agents.",
        tool
    )
}

fn print_release(name: &str, release: &crate::agents::data::Release) {
    let version = &release.version;
    let date = release.date.as_deref().unwrap_or("unknown date");
    println!("{} {} ({})", name, version, date);
    println!("{}", "\u{2500}".repeat(40));
    if let Some(body) = &release.changelog {
        print_changelog_body(body);
    } else {
        println!("(no changelog body)");
    }
}

fn print_changelog_body(body: &str) {
    use std::io::IsTerminal;
    if std::io::stdout().is_terminal() {
        termimad::print_text(body);
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
    println!(
        "{} \u{2014} {} releases\n",
        agent.name,
        github.releases.len()
    );

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec!["Version", "Released", "Ago"]);

    for release in &github.releases {
        let date_str = release.date.as_deref().unwrap_or("\u{2014}");
        let ago = release
            .date
            .as_deref()
            .and_then(crate::agents::helpers::parse_date)
            .map(|d| crate::agents::helpers::format_relative_time(&d))
            .unwrap_or_else(|| "\u{2014}".to_string());
        table.add_row(vec![release.version.as_str(), date_str, &ago]);
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
            let date = r.date.as_deref().unwrap_or("unknown");
            let ago = r
                .date
                .as_deref()
                .and_then(crate::agents::helpers::parse_date)
                .map(|d| crate::agents::helpers::format_relative_time(&d))
                .unwrap_or_default();
            format!("{:<16} {:<12} {}", r.version, date, ago)
        })
        .collect();

    let selection = dialoguer::FuzzySelect::new()
        .with_prompt(format!("Select a {} release", agent.name))
        .items(&items)
        .default(0)
        .interact()?;

    let release = &github.releases[selection];
    println!();
    print_release(&agent.name, release);
    Ok(())
}
