use anyhow::Result;
use clap::{Parser, ValueEnum};

use agent::{orchestrator, worker};

/// CLI arguments used to select which agent role to start.
#[derive(Debug, Parser)]
#[command(name = "agent")]
#[command(about = "NanoScale host agent")]
struct Cli {
    #[arg(long, value_enum)]
    role: Option<Role>,

    #[arg(long)]
    join: Option<String>,
}

/// Runtime role for this process.
#[derive(Clone, Debug, ValueEnum)]
enum Role {
    /// Starts the control-plane API server and database-backed orchestrator state.
    Orchestrator,
}

/// Entrypoint that dispatches to orchestrator or worker mode.
///
/// # Errors
/// Returns an error if the selected runtime role fails to start or exits with an error.
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
