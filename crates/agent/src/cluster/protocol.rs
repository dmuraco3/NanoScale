use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct GenerateTokenResponse {
    pub token: String,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JoinClusterRequest {
    pub token: String,
    pub ip: String,
    pub secret_key: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct JoinClusterResponse {
    pub server_id: String,
}
