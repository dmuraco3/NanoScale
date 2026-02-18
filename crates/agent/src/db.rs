use std::path::Path;

use anyhow::{bail, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Sqlite};

mod projects;
mod servers;
mod types;
mod users;

pub use types::{
    NewProject, NewServer, NewUser, ProjectDetailsRecord, ProjectListRecord, ServerConnectionInfo,
    ServerRecord, UserRecord,
};

const BASE_PROJECT_PORT: i64 = 3100;

#[derive(Debug, Clone)]
pub struct DbClient {
    pool: Pool<Sqlite>,
}

impl DbClient {
    pub const fn min_project_port() -> i64 {
        BASE_PROJECT_PORT
    }

    pub async fn connect(database_url: &str) -> Result<Self> {
        let connect_options = SqliteConnectOptions::new()
            .filename(database_url)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true)
            .synchronous(SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connect_options)
            .await?;

        Ok(Self { pool })
    }

    pub async fn initialize(database_path: &str) -> Result<Self> {
        if let Some(parent_dir) = Path::new(database_path).parent() {
            std::fs::create_dir_all(parent_dir)?;
        }

        let db = Self::connect(database_path).await?;
        db.run_migrations().await?;
        db.ensure_wal_mode().await?;

        Ok(db)
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub async fn ensure_wal_mode(&self) -> Result<()> {
        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode=WAL;")
            .fetch_one(&self.pool)
            .await?;

        if journal_mode.to_uppercase() != "WAL" {
            bail!("SQLite WAL mode is not enabled");
        }

        Ok(())
    }

    pub fn pool(&self) -> Pool<Sqlite> {
        self.pool.clone()
    }
}
