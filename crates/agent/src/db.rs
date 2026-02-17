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
pub struct NewProject {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub repo_url: String,
    pub branch: String,
    pub install_command: String,
    pub build_command: String,
    pub start_command: String,
    pub env_vars: String,
    pub port: i64,
}

#[derive(Debug, Clone)]
pub struct ServerRecord {
    pub id: String,
    pub name: String,
    pub ip_address: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ServerConnectionInfo {
    pub id: String,
    pub ip_address: String,
    pub secret_key: String,
}

#[derive(Debug, Clone)]
pub struct NewUser {
    pub id: String,
    pub username: String,
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: String,
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct ProjectListRecord {
    pub id: String,
    pub name: String,
    pub repo_url: String,
    pub branch: String,
    pub start_command: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ProjectDetailsRecord {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub repo_url: String,
    pub branch: String,
    pub install_command: String,
    pub build_command: String,
    pub start_command: String,
    pub port: i64,
    pub created_at: String,
    pub server_name: Option<String>,
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

    pub async fn upsert_server(&self, server: &NewServer) -> Result<()> {
        sqlx::query(
            "INSERT INTO servers (id, name, ip_address, status, secret_key) VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(id) DO UPDATE SET
              name = excluded.name,
              ip_address = excluded.ip_address,
              status = excluded.status,
              secret_key = excluded.secret_key",
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

    pub async fn list_servers(&self) -> Result<Vec<ServerRecord>> {
        let rows = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, name, ip_address, status FROM servers ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let servers = rows
            .into_iter()
            .map(|(id, name, ip_address, status)| ServerRecord {
                id,
                name,
                ip_address,
                status,
            })
            .collect();

        Ok(servers)
    }

    pub async fn get_server_connection_info(
        &self,
        server_id: &str,
    ) -> Result<Option<ServerConnectionInfo>> {
        let row = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, ip_address, secret_key FROM servers WHERE id = ?1",
        )
        .bind(server_id)
        .fetch_optional(&self.pool)
        .await?;

        let server = row.map(|(id, ip_address, secret_key)| ServerConnectionInfo {
            id,
            ip_address,
            secret_key,
        });

        Ok(server)
    }

    pub async fn users_count(&self) -> Result<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;

        Ok(count)
    }

    pub async fn insert_user(&self, user: &NewUser) -> Result<()> {
        sqlx::query("INSERT INTO users (id, username, password_hash) VALUES (?1, ?2, ?3)")
            .bind(&user.id)
            .bind(&user.username)
            .bind(&user.password_hash)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<UserRecord>> {
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT id, password_hash FROM users WHERE username = ?1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        let user = row.map(|(id, password_hash)| UserRecord { id, password_hash });

        Ok(user)
    }

    pub async fn insert_project(&self, project: &NewProject) -> Result<()> {
        sqlx::query(
            "INSERT INTO projects (id, server_id, name, repo_url, branch, install_command, build_command, start_command, env_vars, port) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .bind(&project.id)
        .bind(&project.server_id)
        .bind(&project.name)
        .bind(&project.repo_url)
        .bind(&project.branch)
        .bind(&project.install_command)
        .bind(&project.build_command)
        .bind(&project.start_command)
        .bind(&project.env_vars)
        .bind(project.port)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_project_by_id(&self, project_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM projects WHERE id = ?1")
            .bind(project_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn list_projects(&self) -> Result<Vec<ProjectListRecord>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, name, repo_url, branch, start_command, created_at FROM projects ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let projects = rows
            .into_iter()
            .map(
                |(id, name, repo_url, branch, start_command, created_at)| ProjectListRecord {
                    id,
                    name,
                    repo_url,
                    branch,
                    start_command,
                    created_at,
                },
            )
            .collect();

        Ok(projects)
    }

    pub async fn get_project_by_id(&self, project_id: &str) -> Result<Option<ProjectDetailsRecord>> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                String,
                i64,
                String,
                Option<String>,
            ),
        >(
            "SELECT p.id, p.server_id, p.name, p.repo_url, p.branch, p.install_command, p.build_command, p.start_command, p.port, p.created_at, s.name FROM projects p LEFT JOIN servers s ON s.id = p.server_id WHERE p.id = ?1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                id,
                server_id,
                name,
                repo_url,
                branch,
                install_command,
                build_command,
                start_command,
                port,
                created_at,
                server_name,
            )| ProjectDetailsRecord {
                id,
                server_id,
                name,
                repo_url,
                branch,
                install_command,
                build_command,
                start_command,
                port,
                created_at,
                server_name,
            },
        ))
    }

    pub fn pool(&self) -> Pool<Sqlite> {
        self.pool.clone()
    }
}
