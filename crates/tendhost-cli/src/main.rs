//! tendhost CLI
//!
//! Command-line interface for interacting with tendhost daemon

use clap::{Parser, Subcommand};
use color_eyre::Result;

#[derive(Parser)]
#[command(name = "tendhost")]
#[command(about = "Actor-based homelab orchestration CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all hosts
    #[command(name = "hosts")]
    Hosts,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Hosts => {
            println!("Listing hosts...");
        }
    }

    Ok(())
}
