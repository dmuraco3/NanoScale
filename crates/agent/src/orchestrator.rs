use std::sync::Arc;

use anyhow::Result;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::routing::post;
use axum::{Json, Router};
use uuid::Uuid;

use crate::cluster::protocol::{GenerateTokenResponse, JoinClusterRequest, JoinClusterResponse};
use crate::cluster::signature::verify_cluster_signature;
use crate::cluster::token_store::TokenStore;
use crate::db::{DbClient, NewServer};

const DEFAULT_DB_PATH: &str = "/opt/nanoscale/data/nanoscale.db";
const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:4000";

#[derive(Debug, Clone)]
pub struct OrchestratorState {
    pub db: DbClient,
    pub token_store: Arc<TokenStore>,
}

pub async fn run() -> Result<()> {
    let database_path =
        std::env::var("NANOSCALE_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let bind_address = std::env::var("NANOSCALE_ORCHESTRATOR_BIND")
        .unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_string());
    let db_client = DbClient::initialize(&database_path).await?;

    let state = OrchestratorState {
        db: db_client,
        token_store: Arc::new(TokenStore::new()),
    };

    let internal_router = Router::new()
        .route("/verify-signature", post(verify_signature_guarded))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            verify_cluster_signature,
        ));

    let app = Router::new()
        .route("/api/cluster/generate-token", post(generate_cluster_token))
        .route("/api/cluster/join", post(join_cluster))
        .nest("/internal", internal_router)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_address).await?;

    println!("Starting orchestrator mode: DB + API (skeleton)");
    println!("Database initialized at: {database_path}");
    println!("Listening on: {bind_address}");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn generate_cluster_token(
    State(state): State<OrchestratorState>,
) -> Json<GenerateTokenResponse> {
    let token = state.token_store.generate_token().await;

    Json(GenerateTokenResponse {
        token,
        expires_in_seconds: TokenStore::token_ttl_seconds(),
    })
}

async fn join_cluster(
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

async fn verify_signature_guarded() -> StatusCode {
    StatusCode::OK
}
