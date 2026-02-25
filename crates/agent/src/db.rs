use std::path::Path;

use anyhow::{bail, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Sqlite};

mod github;
mod projects;
mod servers;
mod types;
mod users;

#[cfg(test)]
mod tests;

pub use types::{
    GitHubInstallationRecord, GitHubRepositoryRecord, GitHubUserLinkRecord, NewGitHubInstallation,
    NewGitHubRepository, NewGitHubUserLink, NewGitHubWebhookDelivery, NewProject,
    NewProjectGitHubLink, NewServer, NewUser, ProjectDetailsRecord, ProjectGitHubLinkRecord,
    ProjectListRecord, ServerConnectionInfo, ServerRecord, UserRecord,
};

const BASE_PROJECT_PORT: i64 = 3100;

#[derive(Debug, Clone)]
pub struct DbClient {
    pool: Pool<Sqlite>,
}

impl DbClient {
    #[must_use]
    pub const fn min_project_port() -> i64 {
        BASE_PROJECT_PORT
    }

    /// Connects to a `SQLite` database at `database_url`.
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened or the connection pool cannot be created.
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

    /// Ensures the database directory exists, connects, runs migrations, and enables WAL mode.
    ///
    /// # Errors
    /// Returns an error if creating directories, connecting, migrating, or enabling WAL mode fails.
    pub async fn initialize(database_path: &str) -> Result<Self> {
        if let Some(parent_dir) = Path::new(database_path).parent() {
            std::fs::create_dir_all(parent_dir)?;
        }

        let db = Self::connect(database_path).await?;
        db.run_migrations().await?;
        db.ensure_wal_mode().await?;

        Ok(db)
    }

    /// Runs SQL migrations from `crates/agent/migrations`.
    ///
    /// # Errors
    /// Returns an error if applying migrations fails.
    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    /// Ensures `SQLite` is operating in WAL mode.
    ///
    /// # Errors
    /// Returns an error if the PRAGMA query fails or WAL mode could not be enabled.
    pub async fn ensure_wal_mode(&self) -> Result<()> {
        let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode=WAL;")
            .fetch_one(&self.pool)
            .await?;

        if journal_mode.to_uppercase() != "WAL" {
            bail!("SQLite WAL mode is not enabled");
        }

        Ok(())
    }

    #[must_use]
    pub fn pool(&self) -> Pool<Sqlite> {
        self.pool.clone()
    }
}
