use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(super) struct SetupRequest {
    pub(super) username: String,
    pub(super) password: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct LoginRequest {
    pub(super) username: String,
    pub(super) password: String,
}

#[derive(Debug, Serialize)]
pub(super) struct AuthStatusResponse {
    pub(super) users_count: i64,
    pub(super) authenticated: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct ServerListItem {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) ip_address: String,
    pub(super) status: String,
    pub(super) ram_usage_percent: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ProjectEnvVar {
    pub(super) key: String,
    pub(super) value: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct CreateProjectRequest {
    pub(super) server_id: String,
    pub(super) name: String,
    pub(super) repo_url: String,
    pub(super) branch: String,
    pub(super) build_command: String,
    pub(super) install_command: String,
    pub(super) run_command: String,
    pub(super) output_directory: String,
    pub(super) port: Option<u16>,
    pub(super) env_vars: Vec<ProjectEnvVar>,
}

#[derive(Debug, Serialize)]
pub(super) struct CreateProjectResponse {
    pub(super) id: String,
    pub(super) domain: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ProjectListItem {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) repo_url: String,
    pub(super) branch: String,
    pub(super) run_command: String,
    pub(super) port: i64,
    pub(super) domain: Option<String>,
    pub(super) status: String,
    pub(super) created_at: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ProjectDetailsResponse {
    pub(super) id: String,
    pub(super) server_id: String,
    pub(super) server_name: Option<String>,
    pub(super) name: String,
    pub(super) repo_url: String,
    pub(super) branch: String,
    pub(super) install_command: String,
    pub(super) build_command: String,
    pub(super) run_command: String,
    pub(super) status: String,
    pub(super) port: i64,
    pub(super) domain: Option<String>,
    pub(super) created_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct WorkerCreateProjectRequest {
    pub(super) project_id: String,
    pub(super) name: String,
    pub(super) repo_url: String,
    pub(super) branch: String,
    pub(super) build_command: String,
    pub(super) install_command: String,
    pub(super) run_command: String,
    pub(super) output_directory: String,
    pub(super) port: u16,
    pub(super) domain: Option<String>,
    pub(super) tls_email: Option<String>,
    pub(super) env_vars: Vec<ProjectEnvVar>,
}

#[derive(Debug, Serialize)]
pub(super) struct InternalProjectResponse {
    pub(super) status: &'static str,
    pub(super) message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_env_var_serde_roundtrip() {
        let value = ProjectEnvVar {
            key: "KEY".to_string(),
            value: "VALUE".to_string(),
        };

        let json = serde_json::to_string(&value).expect("serialize");
        let decoded = serde_json::from_str::<ProjectEnvVar>(&json).expect("deserialize");
        assert_eq!(decoded.key, "KEY");
        assert_eq!(decoded.value, "VALUE");
    }
}
