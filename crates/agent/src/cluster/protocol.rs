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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_cluster_request_serde_roundtrip() {
        let value = JoinClusterRequest {
            token: "t".to_string(),
            ip: "127.0.0.1".to_string(),
            secret_key: "s".to_string(),
            name: "worker".to_string(),
        };

        let json = serde_json::to_string(&value).expect("serialize");
        let decoded = serde_json::from_str::<JoinClusterRequest>(&json).expect("deserialize");
        assert_eq!(decoded.token, "t");
        assert_eq!(decoded.ip, "127.0.0.1");
        assert_eq!(decoded.name, "worker");
    }

    #[test]
    fn generate_token_response_serializes() {
        let value = GenerateTokenResponse {
            token: "abc".to_string(),
            expires_in_seconds: 60,
        };
        let json = serde_json::to_string(&value).expect("serialize");
        assert!(json.contains("\"token\""));
    }
}
