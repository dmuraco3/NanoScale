use anyhow::Result;
use clap::{Parser, ValueEnum};

mod db;
mod orchestrator;
mod system;
mod worker;

#[derive(Debug, Parser)]
#[command(name = "agent")]
#[command(about = "NanoScale host agent")]
struct Cli {
    #[arg(long, value_enum)]
    role: Option<Role>,

    #[arg(long)]
    join: Option<String>,
}

#[derive(Clone, Debug, ValueEnum)]
enum Role {
    Orchestrator,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match (cli.role, cli.join) {
        (Some(Role::Orchestrator), None) => orchestrator::run().await?,
        (None, Some(join_token)) => worker::run(&join_token),
        _ => {
            println!("Usage:");
            println!("  agent --role orchestrator");
            println!("  agent --join <token>");
        }
    }

    Ok(())
}
