mod agents;
mod api;
mod benchmark_fetch;
mod benchmarks;
mod cli;
mod config;
mod data;
mod model_traits;
mod provider_category;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "models")]
#[command(about = "CLI/TUI tool for querying AI model information from models.dev")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List providers or models
    List {
        #[command(subcommand)]
        what: ListCommands,
    },
    /// Show detailed information about a model
    Show {
        /// Model ID (e.g., claude-opus-4-1, gpt-4o)
        model_id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Search models by name or provider
    Search {
        /// Search query
        query: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Track AI coding agent releases and changelogs
    #[command(after_help = "\
\x1b[1;4mTool Commands:\x1b[0m
  agents <tool>                 Show latest changelog for a tool
  agents <tool> --list, -l      List all versions
  agents <tool> --pick, -p      Interactive version picker
  agents <tool> --version <v>   Show changelog for a specific version
  agents <tool> --web, -w       Open releases page in browser

\x1b[1;4mExamples:\x1b[0m
  agents claude                 Latest Claude Code changelog
  agents cursor --list          All Cursor versions
  agents aider --pick           Pick an Aider release interactively")]
    Agents {
        #[command(subcommand)]
        command: Option<cli::agents::AgentsCommand>,
    },
}

#[derive(Subcommand)]
enum ListCommands {
    /// List all providers
    Providers {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List models, optionally filtered by provider
    Models {
        /// Filter by provider ID
        provider: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() -> Result<()> {
    // Check if invoked as "agents" (symlink entry point)
    let binary_name = std::env::args()
        .next()
        .and_then(|s| {
            std::path::Path::new(&s)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
        })
        .unwrap_or_default();

    if binary_name == "agents" {
        return cli::agents::run();
    }

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::List { what }) => match what {
            ListCommands::Providers { json } => cli::list::providers(json)?,
            ListCommands::Models { provider, json } => cli::list::models(provider, json)?,
        },
        Some(Commands::Show { model_id, json }) => cli::show::model(&model_id, json)?,
        Some(Commands::Search { query, json }) => cli::search::search(&query, json)?,
        Some(Commands::Agents { command }) => cli::agents::run_with_command(command)?,
        None => {
            // Fetch providers before entering async runtime to avoid blocking in async context
            let providers = api::fetch_providers()?;

            // Create and run the async runtime only for the TUI
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(tui::run(providers))?;
        }
    }

    Ok(())
}
