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
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "models")]
#[command(about = "CLI/TUI for browsing AI models, benchmarks, and coding agents")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List models, optionally filtered by provider
    #[command(after_help = "\
\x1b[1;4mExamples:\x1b[0m
  models list                         Open the interactive model picker
  models list openai                  Picker prefiltered to a provider
  models list --json                  Dump model rows as JSON")]
    List {
        /// Filter by provider ID or exact provider name
        provider: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// List providers
    #[command(after_help = "\
\x1b[1;4mExamples:\x1b[0m
  models providers
  models providers --json")]
    Providers {
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
    #[command(after_help = "\
\x1b[1;4mExamples:\x1b[0m
  models search claude
  models search gpt-4o --json

\x1b[1;4mNote:\x1b[0m
  Search now uses the same matcher and interactive picker flow as `models list`.")]
    Search {
        /// Search query
        query: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
    /// Track AI coding agent releases and changelogs
    #[command(after_help = "\
\x1b[1;4mTool Commands:\x1b[0m
  agents <tool>                 Browse releases for a tool
  agents <tool> --latest        Show latest changelog directly
  agents <tool> --list, -l      List all versions
  agents <tool> --pick, -p      Alias for the interactive release browser
  agents <tool> --version <v>   Show changelog for a specific version
  agents <tool> --web, -w       Open releases page in browser

\x1b[1;4mExamples:\x1b[0m
  agents claude                 Browse Claude Code releases
  agents claude --latest        Latest Claude Code changelog
  agents cursor --list          All Cursor versions
  agents aider --pick           Pick an Aider release interactively")]
    Agents {
        #[command(subcommand)]
        command: Option<cli::agents::AgentsCommand>,
    },
    /// Query benchmark data from the command line
    #[command(after_help = "\
\x1b[1;4mExamples:\x1b[0m
  models benchmarks list                     Open the interactive benchmark picker
  models benchmarks list --sort speed --limit 10
  models benchmarks list --creator openai --reasoning
  models benchmarks list --json
  models benchmarks show gpt-4o
  models benchmarks show \"Claude Sonnet 4\"")]
    Benchmarks {
        #[command(subcommand)]
        command: Option<cli::benchmarks::BenchmarksCommand>,
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
        Some(Commands::List { provider, json }) => cli::list::models(provider, json)?,
        Some(Commands::Providers { json }) => cli::list::providers(json)?,
        Some(Commands::Show { model_id, json }) => cli::show::model(&model_id, json)?,
        Some(Commands::Search { query, json }) => cli::search::search(&query, json)?,
        Some(Commands::Completions { shell }) => {
            clap_complete::generate(shell, &mut Cli::command(), "models", &mut std::io::stdout());
        }
        Some(Commands::Agents { command }) => cli::agents::run_with_command(command)?,
        Some(Commands::Benchmarks { command }) => cli::benchmarks::run_with_command(command)?,
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
