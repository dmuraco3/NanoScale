use anyhow::Result;

use super::{DbClient, NewProject, ProjectDetailsRecord, ProjectListRecord, BASE_PROJECT_PORT};

impl DbClient {
    /// Inserts a new project record.
    ///
    /// # Errors
    /// Returns an error if the insert fails.
    pub async fn insert_project(&self, project: &NewProject) -> Result<()> {
        sqlx::query(
            "INSERT INTO projects (id, server_id, name, repo_url, branch, install_command, build_command, start_command, output_directory, env_vars, port, domain) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        )
        .bind(&project.id)
        .bind(&project.server_id)
        .bind(&project.name)
        .bind(&project.repo_url)
        .bind(&project.branch)
        .bind(&project.install_command)
        .bind(&project.build_command)
        .bind(&project.start_command)
        .bind(&project.output_directory)
        .bind(&project.env_vars)
        .bind(project.port)
        .bind(project.domain.as_deref())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Deletes a project by id.
    ///
    /// # Errors
    /// Returns an error if the delete fails.
    pub async fn delete_project_by_id(&self, project_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM projects WHERE id = ?1")
            .bind(project_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Lists projects in reverse creation order.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn list_projects(&self) -> Result<Vec<ProjectListRecord>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, i64, Option<String>, String)>(
            "SELECT id, name, repo_url, branch, start_command, port, domain, created_at FROM projects ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(id, name, repo_url, branch, start_command, port, domain, created_at)| {
                    ProjectListRecord {
                        id,
                        name,
                        repo_url,
                        branch,
                        start_command,
                        port,
                        domain,
                        created_at,
                    }
                },
            )
            .collect())
    }

    /// Lists projects for a specific server (id + name).
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn list_projects_for_server_stats(
        &self,
        server_id: &str,
    ) -> Result<Vec<(String, String)>> {
        let rows = sqlx::query_as::<_, (String, String)>(
            "SELECT id, name FROM projects WHERE server_id = ?1 ORDER BY created_at DESC",
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Fetches a project's full details.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn get_project_by_id(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectDetailsRecord>> {
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
                String,
                String,
                i64,
                Option<String>,
                String,
                Option<String>,
            ),
        >(
            "SELECT p.id, p.server_id, p.name, p.repo_url, p.branch, p.install_command, p.build_command, p.start_command, p.output_directory, p.env_vars, p.port, p.domain, p.created_at, s.name FROM projects p LEFT JOIN servers s ON s.id = p.server_id WHERE p.id = ?1",
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
                output_directory,
                env_vars,
                port,
                domain,
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
                output_directory,
                env_vars,
                port,
                domain,
                created_at,
                server_name,
            },
        ))
    }

    /// Returns the next available project port.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn next_available_project_port(&self) -> Result<i64> {
        let max_port = sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(port) FROM projects")
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(BASE_PROJECT_PORT - 1);

        Ok(if max_port < BASE_PROJECT_PORT {
            BASE_PROJECT_PORT
        } else {
            max_port + 1
        })
    }

    /// Checks whether a port is already assigned to a project.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn is_project_port_in_use(&self, port: i64) -> Result<bool> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM projects WHERE port = ?1")
            .bind(port)
            .fetch_one(&self.pool)
            .await?;

        Ok(count > 0)
    }

    /// Checks whether a domain is already assigned to a project.
    ///
    /// # Errors
    /// Returns an error if the query fails.
    pub async fn is_project_domain_in_use(&self, domain: &str) -> Result<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM projects WHERE domain IS NOT NULL AND domain = ?1",
        )
        .bind(domain)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }
}
