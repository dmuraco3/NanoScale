use std::path::Path;

use anyhow::{bail, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Pool, Sqlite};

#[derive(Debug, Clone)]
pub struct NewServer {
    pub id: String,
    pub name: String,
    pub ip_address: String,
    pub status: String,
    pub secret_key: String,
}

#[derive(Debug, Clone)]
pub struct DbClient {
    pool: Pool<Sqlite>,
}

impl DbClient {
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

    pub async fn insert_server(&self, server: &NewServer) -> Result<()> {
        sqlx::query(
            "INSERT INTO servers (id, name, ip_address, status, secret_key) VALUES (?1, ?2, ?3, ?4, ?5)",
        )
        .bind(&server.id)
        .bind(&server.name)
        .bind(&server.ip_address)
        .bind(&server.status)
        .bind(&server.secret_key)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_server_secret(&self, server_id: &str) -> Result<Option<String>> {
        let secret =
            sqlx::query_scalar::<_, String>("SELECT secret_key FROM servers WHERE id = ?1")
                .bind(server_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(secret)
    }
}
