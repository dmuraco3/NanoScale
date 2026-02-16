use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::State;
use axum::http::StatusCode;
use axum::middleware;
use axum::routing::{get, post};
use axum::{Json, Router};
use hmac::Mac;
use serde::{Deserialize, Serialize};
use tower_sessions::{Session, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use uuid::Uuid;

use crate::cluster::protocol::{GenerateTokenResponse, JoinClusterRequest, JoinClusterResponse};
use crate::cluster::signature::verify_cluster_signature;
use crate::cluster::token_store::TokenStore;
use crate::db::{DbClient, NewProject, NewServer, NewUser, ServerRecord};

const DEFAULT_DB_PATH: &str = "/opt/nanoscale/data/nanoscale.db";
const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:4000";
const SESSION_USER_ID_KEY: &str = "user_id";

#[derive(Debug, Clone)]
pub struct OrchestratorState {
    pub db: DbClient,
    pub token_store: Arc<TokenStore>,
}

#[derive(Debug, Deserialize)]
struct SetupRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct AuthStatusResponse {
    users_count: i64,
    authenticated: bool,
}

#[derive(Debug, Serialize)]
struct ServerListItem {
    id: String,
    name: String,
    ip_address: String,
    status: String,
    ram_usage_percent: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ProjectEnvVar {
    key: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct CreateProjectRequest {
    server_id: String,
    name: String,
    repo_url: String,
    branch: String,
    build_command: String,
    env_vars: Vec<ProjectEnvVar>,
}

#[derive(Debug, Serialize)]
struct CreateProjectResponse {
    id: String,
}

#[derive(Debug, Serialize)]
struct WorkerCreateProjectRequest {
    project_id: String,
    name: String,
    repo_url: String,
    branch: String,
    build_command: String,
    port: u16,
    env_vars: Vec<ProjectEnvVar>,
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

    let session_store = SqliteStore::new(state.db.pool());
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let internal_router = Router::new()
        .route("/verify-signature", post(verify_signature_guarded))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            verify_cluster_signature,
        ));

    let app = Router::new()
        .route("/api/auth/setup", post(auth_setup))
        .route("/api/auth/login", post(auth_login))
        .route("/api/auth/status", post(auth_status))
        .route("/api/auth/session", post(auth_session))
        .route("/api/servers", get(list_servers))
        .route("/api/projects", post(create_project))
        .route("/api/cluster/generate-token", post(generate_cluster_token))
        .route("/api/cluster/join", post(join_cluster))
        .nest("/internal", internal_router)
        .layer(session_layer)
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
    session: Session,
) -> Result<Json<GenerateTokenResponse>, StatusCode> {
    require_authenticated(&session).await?;

    let token = state.token_store.generate_token().await;

    Ok(Json(GenerateTokenResponse {
        token,
        expires_in_seconds: TokenStore::token_ttl_seconds(),
    }))
}

async fn list_servers(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<Vec<ServerListItem>>, StatusCode> {
    require_authenticated(&session).await?;

    let servers = state
        .db
        .list_servers()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(servers.into_iter().map(map_server_record).collect()))
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

async fn auth_setup(
    State(state): State<OrchestratorState>,
    session: Session,
    Json(payload): Json<SetupRequest>,
) -> Result<StatusCode, StatusCode> {
    if state
        .db
        .users_count()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        > 0
    {
        return Err(StatusCode::CONFLICT);
    }

    if payload.username.trim().is_empty() || payload.password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let user_id = Uuid::new_v4().to_string();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    let new_user = NewUser {
        id: user_id.clone(),
        username: payload.username,
        password_hash,
    };

    state
        .db
        .insert_user(&new_user)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    session
        .insert(SESSION_USER_ID_KEY, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

async fn auth_login(
    State(state): State<OrchestratorState>,
    session: Session,
    Json(payload): Json<LoginRequest>,
) -> Result<StatusCode, StatusCode> {
    let user = state
        .db
        .find_user_by_username(&payload.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let parsed_hash =
        PasswordHash::new(&user.password_hash).map_err(|_| StatusCode::UNAUTHORIZED)?;

    Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    session
        .insert(SESSION_USER_ID_KEY, user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

async fn auth_status(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<AuthStatusResponse>, StatusCode> {
    let users_count = state
        .db
        .users_count()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let authenticated = session
        .get::<String>(SESSION_USER_ID_KEY)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    Ok(Json(AuthStatusResponse {
        users_count,
        authenticated,
    }))
}

async fn auth_session(session: Session) -> Result<StatusCode, StatusCode> {
    require_authenticated(&session).await?;
    Ok(StatusCode::OK)
}

async fn require_authenticated(session: &Session) -> Result<(), StatusCode> {
    let authenticated = session
        .get::<String>(SESSION_USER_ID_KEY)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    if authenticated {
        return Ok(());
    }

    Err(StatusCode::UNAUTHORIZED)
}

fn map_server_record(server: ServerRecord) -> ServerListItem {
    ServerListItem {
        id: server.id,
        name: server.name,
        ip_address: server.ip_address,
        status: server.status,
        ram_usage_percent: 0,
    }
}

async fn create_project(
    State(state): State<OrchestratorState>,
    session: Session,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<Json<CreateProjectResponse>, StatusCode> {
    require_authenticated(&session).await?;

    if payload.name.trim().is_empty() || payload.repo_url.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let connection = state
        .db
        .get_server_connection_info(&payload.server_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let project_id = Uuid::new_v4().to_string();
    let project = NewProject {
        id: project_id.clone(),
        server_id: payload.server_id.clone(),
        name: payload.name.clone(),
        repo_url: payload.repo_url.clone(),
        branch: payload.branch.clone(),
        build_command: payload.build_command.clone(),
        env_vars: serde_json::to_string(&payload.env_vars)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        port: 3000,
    };

    state
        .db
        .insert_project(&project)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    call_worker_create_project(
        &connection.id,
        &connection.ip_address,
        &connection.secret_key,
        &payload,
        &project_id,
    )
    .await
    .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok(Json(CreateProjectResponse { id: project_id }))
}

async fn call_worker_create_project(
    server_id: &str,
    worker_host: &str,
    secret_key: &str,
    payload: &CreateProjectRequest,
    project_id: &str,
) -> Result<(), anyhow::Error> {
    let worker_payload = WorkerCreateProjectRequest {
        project_id: project_id.to_string(),
        name: payload.name.clone(),
        repo_url: payload.repo_url.clone(),
        branch: payload.branch.clone(),
        build_command: payload.build_command.clone(),
        port: 3000,
        env_vars: payload.env_vars.clone(),
    };

    let body = serde_json::to_vec(&worker_payload)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string();

    let signature = sign_internal_payload(&body, &timestamp, secret_key)?;
    let url = format!("http://{worker_host}:4000/internal/projects");

    reqwest::Client::new()
        .post(url)
        .header("X-Cluster-Timestamp", timestamp)
        .header("X-Cluster-Signature", signature)
        .header("X-Server-Id", server_id)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

fn sign_internal_payload(
    body: &[u8],
    timestamp: &str,
    secret_key: &str,
) -> Result<String, anyhow::Error> {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(secret_key.as_bytes())?;
    hmac::Mac::update(&mut mac, body);
    hmac::Mac::update(&mut mac, timestamp.as_bytes());
    let signature = hex::encode(hmac::Mac::finalize(mac).into_bytes());
    Ok(signature)
}
