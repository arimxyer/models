use anyhow::Result;
use clap::{CommandFactory, Parser};

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
#[allow(dead_code)]
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

fn run_status() -> Result<()> {
    println!("agents status: not yet implemented");
    Ok(())
}

fn run_latest() -> Result<()> {
    println!("agents latest: not yet implemented");
    Ok(())
}

fn run_list_sources() -> Result<()> {
    println!("agents list-sources: not yet implemented");
    Ok(())
}

fn run_tool(args: ToolArgs) -> Result<()> {
    println!("agents {}: not yet implemented", args.tool);
    Ok(())
}
