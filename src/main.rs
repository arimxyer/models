mod api;
mod cli;
mod data;
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
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::List { what }) => match what {
            ListCommands::Providers { json } => cli::list::providers(json)?,
            ListCommands::Models { provider, json } => cli::list::models(provider, json)?,
        },
        Some(Commands::Show { model_id, json }) => cli::show::model(&model_id, json)?,
        Some(Commands::Search { query, json }) => cli::search::search(&query, json)?,
        None => tui::run()?,
    }

    Ok(())
}
