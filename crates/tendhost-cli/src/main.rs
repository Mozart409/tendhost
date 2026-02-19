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
    // TODO: Implement subcommands for hosts, fleet, etc.
}

#[tokio::main]
async fn main() -> Result<()> {
    let _cli = Cli::parse();
    
    // TODO: Implement CLI commands
    println!("tendhost CLI");
    
    Ok(())
}
