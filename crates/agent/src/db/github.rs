use anyhow::Result;

use super::{
    DbClient, GitHubInstallationRecord, GitHubRepositoryRecord, GitHubUserLinkRecord,
    NewGitHubInstallation, NewGitHubRepository, NewGitHubUserLink, NewGitHubWebhookDelivery,
    NewProjectGitHubLink, ProjectGitHubLinkRecord,
};

#[allow(clippy::missing_errors_doc)]
impl DbClient {
    pub async fn upsert_github_user_link(&self, link: &NewGitHubUserLink) -> Result<()> {
        sqlx::query(
            "INSERT INTO github_user_links (id, local_user_id, github_user_id, github_login, access_token_encrypted, refresh_token_encrypted, token_expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(local_user_id) DO UPDATE SET github_user_id = excluded.github_user_id, github_login = excluded.github_login, access_token_encrypted = excluded.access_token_encrypted, refresh_token_encrypted = excluded.refresh_token_encrypted, token_expires_at = excluded.token_expires_at, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&link.id)
        .bind(&link.local_user_id)
        .bind(link.github_user_id)
        .bind(&link.github_login)
        .bind(&link.access_token_encrypted)
        .bind(link.refresh_token_encrypted.as_deref())
        .bind(link.token_expires_at.as_deref())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_github_user_link_by_local_user(
        &self,
        local_user_id: &str,
    ) -> Result<Option<GitHubUserLinkRecord>> {
        let row = sqlx::query_as::<_, (String, i64, String, String, Option<String>, Option<String>)>(
            "SELECT local_user_id, github_user_id, github_login, access_token_encrypted, refresh_token_encrypted, token_expires_at FROM github_user_links WHERE local_user_id = ?1",
        )
        .bind(local_user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                local_user_id,
                github_user_id,
                github_login,
                access_token_encrypted,
                refresh_token_encrypted,
                token_expires_at,
            )| GitHubUserLinkRecord {
                local_user_id,
                github_user_id,
                github_login,
                access_token_encrypted,
                refresh_token_encrypted,
                token_expires_at,
            },
        ))
    }

    pub async fn clear_github_user_link(&self, local_user_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM github_user_links WHERE local_user_id = ?1")
            .bind(local_user_id)
            .execute(&self.pool)
            .await?;

        sqlx::query("DELETE FROM github_installations WHERE local_user_id = ?1")
            .bind(local_user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn upsert_github_installation(
        &self,
        installation: &NewGitHubInstallation,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO github_installations (id, local_user_id, installation_id, account_login, account_type, target_type, target_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(installation_id) DO UPDATE SET local_user_id = excluded.local_user_id, account_login = excluded.account_login, account_type = excluded.account_type, target_type = excluded.target_type, target_id = excluded.target_id, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&installation.id)
        .bind(&installation.local_user_id)
        .bind(installation.installation_id)
        .bind(&installation.account_login)
        .bind(&installation.account_type)
        .bind(&installation.target_type)
        .bind(installation.target_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn replace_github_installations_for_user(
        &self,
        local_user_id: &str,
        installations: &[NewGitHubInstallation],
    ) -> Result<()> {
        let mut transaction = self.pool.begin().await?;

        sqlx::query(
            "DELETE FROM github_repositories WHERE installation_id IN (SELECT installation_id FROM github_installations WHERE local_user_id = ?1)",
        )
        .bind(local_user_id)
        .execute(&mut *transaction)
        .await?;

        sqlx::query("DELETE FROM github_installations WHERE local_user_id = ?1")
            .bind(local_user_id)
            .execute(&mut *transaction)
            .await?;

        for installation in installations {
            sqlx::query(
                "INSERT INTO github_installations (id, local_user_id, installation_id, account_login, account_type, target_type, target_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .bind(&installation.id)
            .bind(&installation.local_user_id)
            .bind(installation.installation_id)
            .bind(&installation.account_login)
            .bind(&installation.account_type)
            .bind(&installation.target_type)
            .bind(installation.target_id)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }

    pub async fn list_github_installations_for_user(
        &self,
        local_user_id: &str,
    ) -> Result<Vec<GitHubInstallationRecord>> {
        let rows = sqlx::query_as::<_, (i64, String, String, String, i64)>(
            "SELECT installation_id, account_login, account_type, target_type, target_id FROM github_installations WHERE local_user_id = ?1 ORDER BY account_login ASC",
        )
        .bind(local_user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(installation_id, account_login, account_type, target_type, target_id)| {
                    GitHubInstallationRecord {
                        installation_id,
                        account_login,
                        account_type,
                        target_type,
                        target_id,
                    }
                },
            )
            .collect())
    }

    pub async fn replace_github_repositories(
        &self,
        installation_id: i64,
        repositories: &[NewGitHubRepository],
    ) -> Result<()> {
        sqlx::query("DELETE FROM github_repositories WHERE installation_id = ?1")
            .bind(installation_id)
            .execute(&self.pool)
            .await?;

        for repository in repositories {
            sqlx::query(
                "INSERT INTO github_repositories (id, installation_id, repo_id, node_id, owner_login, name, full_name, default_branch, is_private, html_url, clone_url, archived, disabled) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )
            .bind(&repository.id)
            .bind(repository.installation_id)
            .bind(repository.repo_id)
            .bind(&repository.node_id)
            .bind(&repository.owner_login)
            .bind(&repository.name)
            .bind(&repository.full_name)
            .bind(&repository.default_branch)
            .bind(repository.is_private)
            .bind(&repository.html_url)
            .bind(&repository.clone_url)
            .bind(repository.archived)
            .bind(repository.disabled)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    pub async fn list_github_repositories(
        &self,
        installation_id: i64,
        query: Option<&str>,
    ) -> Result<Vec<GitHubRepositoryRecord>> {
        let rows = if let Some(search_query) = query {
            let like_pattern = format!("%{}%", search_query.trim().to_lowercase());
            sqlx::query_as::<
                _,
                (i64, i64, String, String, String, String, String, bool, String, String, bool, bool),
            >(
                "SELECT installation_id, repo_id, node_id, owner_login, name, full_name, default_branch, is_private, html_url, clone_url, archived, disabled FROM github_repositories WHERE installation_id = ?1 AND LOWER(full_name) LIKE ?2 ORDER BY full_name ASC",
            )
            .bind(installation_id)
            .bind(like_pattern)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<
                _,
                (i64, i64, String, String, String, String, String, bool, String, String, bool, bool),
            >(
                "SELECT installation_id, repo_id, node_id, owner_login, name, full_name, default_branch, is_private, html_url, clone_url, archived, disabled FROM github_repositories WHERE installation_id = ?1 ORDER BY full_name ASC",
            )
            .bind(installation_id)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(rows
            .into_iter()
            .map(
                |(
                    installation_id,
                    repo_id,
                    node_id,
                    owner_login,
                    name,
                    full_name,
                    default_branch,
                    is_private,
                    html_url,
                    clone_url,
                    archived,
                    disabled,
                )| GitHubRepositoryRecord {
                    installation_id,
                    repo_id,
                    node_id,
                    owner_login,
                    name,
                    full_name,
                    default_branch,
                    is_private,
                    html_url,
                    clone_url,
                    archived,
                    disabled,
                },
            )
            .collect())
    }

    pub async fn get_github_repository_by_id(
        &self,
        repo_id: i64,
    ) -> Result<Option<GitHubRepositoryRecord>> {
        let row = sqlx::query_as::<
            _,
            (i64, i64, String, String, String, String, String, bool, String, String, bool, bool),
        >(
            "SELECT installation_id, repo_id, node_id, owner_login, name, full_name, default_branch, is_private, html_url, clone_url, archived, disabled FROM github_repositories WHERE repo_id = ?1",
        )
        .bind(repo_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                installation_id,
                repo_id,
                node_id,
                owner_login,
                name,
                full_name,
                default_branch,
                is_private,
                html_url,
                clone_url,
                archived,
                disabled,
            )| GitHubRepositoryRecord {
                installation_id,
                repo_id,
                node_id,
                owner_login,
                name,
                full_name,
                default_branch,
                is_private,
                html_url,
                clone_url,
                archived,
                disabled,
            },
        ))
    }

    pub async fn upsert_project_github_link(&self, link: &NewProjectGitHubLink) -> Result<()> {
        sqlx::query(
            "INSERT INTO project_github_links (id, project_id, installation_id, repo_id, repo_node_id, owner_login, repo_name, full_name, default_branch, selected_branch, webhook_id, webhook_secret_encrypted, active) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13) ON CONFLICT(project_id) DO UPDATE SET installation_id = excluded.installation_id, repo_id = excluded.repo_id, repo_node_id = excluded.repo_node_id, owner_login = excluded.owner_login, repo_name = excluded.repo_name, full_name = excluded.full_name, default_branch = excluded.default_branch, selected_branch = excluded.selected_branch, webhook_id = excluded.webhook_id, webhook_secret_encrypted = excluded.webhook_secret_encrypted, active = excluded.active, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(&link.id)
        .bind(&link.project_id)
        .bind(link.installation_id)
        .bind(link.repo_id)
        .bind(&link.repo_node_id)
        .bind(&link.owner_login)
        .bind(&link.repo_name)
        .bind(&link.full_name)
        .bind(&link.default_branch)
        .bind(&link.selected_branch)
        .bind(link.webhook_id)
        .bind(&link.webhook_secret_encrypted)
        .bind(link.active)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_project_github_link_by_project_id(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectGitHubLinkRecord>> {
        let row = sqlx::query_as::<
            _,
            (
                String,
                i64,
                i64,
                String,
                String,
                String,
                String,
                String,
                String,
                Option<i64>,
                String,
                bool,
            ),
        >(
            "SELECT project_id, installation_id, repo_id, repo_node_id, owner_login, repo_name, full_name, default_branch, selected_branch, webhook_id, webhook_secret_encrypted, active FROM project_github_links WHERE project_id = ?1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                project_id,
                installation_id,
                repo_id,
                repo_node_id,
                owner_login,
                repo_name,
                full_name,
                default_branch,
                selected_branch,
                webhook_id,
                webhook_secret_encrypted,
                active,
            )| ProjectGitHubLinkRecord {
                project_id,
                installation_id,
                repo_id,
                repo_node_id,
                owner_login,
                repo_name,
                full_name,
                default_branch,
                selected_branch,
                webhook_id,
                webhook_secret_encrypted,
                active,
            },
        ))
    }

    pub async fn list_active_project_links_for_repo_branch(
        &self,
        repo_id: i64,
        selected_branch: &str,
    ) -> Result<Vec<ProjectGitHubLinkRecord>> {
        let rows = sqlx::query_as::<
            _,
            (
                String,
                i64,
                i64,
                String,
                String,
                String,
                String,
                String,
                String,
                Option<i64>,
                String,
                bool,
            ),
        >(
            "SELECT project_id, installation_id, repo_id, repo_node_id, owner_login, repo_name, full_name, default_branch, selected_branch, webhook_id, webhook_secret_encrypted, active FROM project_github_links WHERE repo_id = ?1 AND selected_branch = ?2 AND active = 1",
        )
        .bind(repo_id)
        .bind(selected_branch)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    project_id,
                    installation_id,
                    repo_id,
                    repo_node_id,
                    owner_login,
                    repo_name,
                    full_name,
                    default_branch,
                    selected_branch,
                    webhook_id,
                    webhook_secret_encrypted,
                    active,
                )| ProjectGitHubLinkRecord {
                    project_id,
                    installation_id,
                    repo_id,
                    repo_node_id,
                    owner_login,
                    repo_name,
                    full_name,
                    default_branch,
                    selected_branch,
                    webhook_id,
                    webhook_secret_encrypted,
                    active,
                },
            )
            .collect())
    }

    pub async fn mark_github_webhook_delivery(
        &self,
        delivery: &NewGitHubWebhookDelivery,
    ) -> Result<bool> {
        let result = sqlx::query(
            "INSERT OR IGNORE INTO github_webhook_deliveries (id, delivery_id, event_type, repo_id, ref, head_commit, handled, status_code, error_message) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        )
        .bind(&delivery.id)
        .bind(&delivery.delivery_id)
        .bind(&delivery.event_type)
        .bind(delivery.repo_id)
        .bind(delivery.r#ref.as_deref())
        .bind(delivery.head_commit.as_deref())
        .bind(delivery.handled)
        .bind(delivery.status_code)
        .bind(delivery.error_message.as_deref())
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn complete_github_webhook_delivery(
        &self,
        delivery_id: &str,
        status_code: i64,
        error_message: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE github_webhook_deliveries SET handled = 1, status_code = ?2, error_message = ?3 WHERE delivery_id = ?1",
        )
        .bind(delivery_id)
        .bind(status_code)
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn set_project_github_webhook_id(
        &self,
        project_id: &str,
        webhook_id: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE project_github_links SET webhook_id = ?2, updated_at = CURRENT_TIMESTAMP WHERE project_id = ?1",
        )
        .bind(project_id)
        .bind(webhook_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn deactivate_project_github_link(&self, project_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE project_github_links SET active = 0, updated_at = CURRENT_TIMESTAMP WHERE project_id = ?1",
        )
        .bind(project_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
