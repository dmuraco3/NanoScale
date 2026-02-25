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
    pub output_directory: String,
    pub env_vars: String,
    pub port: i64,
    pub domain: Option<String>,
    pub source_provider: String,
    pub source_repo_id: Option<i64>,
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
    pub port: i64,
    pub domain: Option<String>,
    pub source_provider: String,
    pub source_repo_id: Option<i64>,
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
    pub output_directory: String,
    pub env_vars: String,
    pub port: i64,
    pub domain: Option<String>,
    pub source_provider: String,
    pub source_repo_id: Option<i64>,
    pub created_at: String,
    pub server_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewGitHubUserLink {
    pub id: String,
    pub local_user_id: String,
    pub github_user_id: i64,
    pub github_login: String,
    pub access_token_encrypted: String,
    pub refresh_token_encrypted: Option<String>,
    pub token_expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitHubUserLinkRecord {
    pub local_user_id: String,
    pub github_user_id: i64,
    pub github_login: String,
    pub access_token_encrypted: String,
    pub refresh_token_encrypted: Option<String>,
    pub token_expires_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewGitHubInstallation {
    pub id: String,
    pub local_user_id: String,
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub target_type: String,
    pub target_id: i64,
}

#[derive(Debug, Clone)]
pub struct GitHubInstallationRecord {
    pub installation_id: i64,
    pub account_login: String,
    pub account_type: String,
    pub target_type: String,
    pub target_id: i64,
}

#[derive(Debug, Clone)]
pub struct NewGitHubRepository {
    pub id: String,
    pub installation_id: i64,
    pub repo_id: i64,
    pub node_id: String,
    pub owner_login: String,
    pub name: String,
    pub full_name: String,
    pub default_branch: String,
    pub is_private: bool,
    pub html_url: String,
    pub clone_url: String,
    pub archived: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone)]
pub struct GitHubRepositoryRecord {
    pub installation_id: i64,
    pub repo_id: i64,
    pub node_id: String,
    pub owner_login: String,
    pub name: String,
    pub full_name: String,
    pub default_branch: String,
    pub is_private: bool,
    pub html_url: String,
    pub clone_url: String,
    pub archived: bool,
    pub disabled: bool,
}

#[derive(Debug, Clone)]
pub struct NewProjectGitHubLink {
    pub id: String,
    pub project_id: String,
    pub installation_id: i64,
    pub repo_id: i64,
    pub repo_node_id: String,
    pub owner_login: String,
    pub repo_name: String,
    pub full_name: String,
    pub default_branch: String,
    pub selected_branch: String,
    pub webhook_id: Option<i64>,
    pub webhook_secret_encrypted: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectGitHubLinkRecord {
    pub project_id: String,
    pub installation_id: i64,
    pub repo_id: i64,
    pub repo_node_id: String,
    pub owner_login: String,
    pub repo_name: String,
    pub full_name: String,
    pub default_branch: String,
    pub selected_branch: String,
    pub webhook_id: Option<i64>,
    pub webhook_secret_encrypted: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct NewGitHubWebhookDelivery {
    pub id: String,
    pub delivery_id: String,
    pub event_type: String,
    pub repo_id: Option<i64>,
    pub r#ref: Option<String>,
    pub head_commit: Option<String>,
    pub handled: bool,
    pub status_code: Option<i64>,
    pub error_message: Option<String>,
}
