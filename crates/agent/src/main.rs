use anyhow::Result;
use clap::{Parser, ValueEnum};

use agent::{orchestrator, worker};

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
        (None, Some(join_token)) => worker::run(&join_token).await?,
        _ => {
            println!("Usage:");
            println!("  agent --role orchestrator");
            println!("  agent --join <token>");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_orchestrator_role() {
        let cli = Cli::try_parse_from(["agent", "--role", "orchestrator"]).expect("parse");
        assert!(matches!(cli.role, Some(Role::Orchestrator)));
        assert!(cli.join.is_none());
    }

    #[test]
    fn cli_parses_join_token() {
        let cli = Cli::try_parse_from(["agent", "--join", "abc"]).expect("parse");
        assert!(cli.role.is_none());
        assert_eq!(cli.join.as_deref(), Some("abc"));
    }
}
