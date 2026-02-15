use anyhow::Result;

use crate::db::DbClient;

const DEFAULT_DB_PATH: &str = "/opt/nanoscale/data/nanoscale.db";

pub async fn run() -> Result<()> {
    let database_path =
        std::env::var("NANOSCALE_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let _db_client = DbClient::initialize(&database_path).await?;

    println!("Starting orchestrator mode: DB + API (skeleton)");
    println!("Database initialized at: {database_path}");
    Ok(())
}
