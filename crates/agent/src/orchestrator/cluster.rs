use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tower_sessions::Session;
use uuid::Uuid;

use crate::cluster::protocol::{GenerateTokenResponse, JoinClusterRequest, JoinClusterResponse};
use crate::db::NewServer;

use super::auth::require_authenticated;
use super::OrchestratorState;

pub(super) async fn generate_cluster_token(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<GenerateTokenResponse>, StatusCode> {
    require_authenticated(&session).await?;

    let token = state.token_store.generate_token().await;

    Ok(Json(GenerateTokenResponse {
        token,
        expires_in_seconds: crate::cluster::token_store::TokenStore::token_ttl_seconds(),
    }))
}

pub(super) async fn join_cluster(
    State(state): State<OrchestratorState>,
    Json(payload): Json<JoinClusterRequest>,
) -> Result<Json<JoinClusterResponse>, StatusCode> {
    if !state.token_store.consume_valid_token(&payload.token).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let server_id = Uuid::new_v4().to_string();
    let server = NewServer {
        id: server_id.clone(),
        name: payload.name,
        ip_address: payload.ip,
        status: "online".to_string(),
        secret_key: payload.secret_key,
    };

    state
        .db
        .insert_server(&server)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(JoinClusterResponse { server_id }))
}

pub(super) async fn verify_signature_guarded() -> StatusCode {
    StatusCode::OK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn verify_signature_guarded_returns_ok() {
        assert_eq!(verify_signature_guarded().await, StatusCode::OK);
    }
}
