use anyhow::Result;
use clap::{Parser, Subcommand};

use mnf::cli::{SearchArgs, run_cli_search};
use mnf::tui::{TuiArgs, run_tui};

#[derive(Debug, Parser)]
#[command(name = "mnf")]
#[command(about = "Find likely-available Minecraft usernames from the terminal")]
struct App {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Cli(SearchArgs),
    Tui(TuiArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    match App::parse().command {
        Commands::Cli(args) => run_cli_search(args).await,
        Commands::Tui(args) => run_tui(args).await,
    }
}
