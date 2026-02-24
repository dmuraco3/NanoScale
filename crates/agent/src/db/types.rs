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
    pub created_at: String,
    pub server_name: Option<String>,
}
