use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::middleware;
use axum::routing::{get, post};
use axum::{Json, Router};
use hmac::Mac;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_sessions::{Session, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use uuid::Uuid;

use crate::cluster::protocol::{GenerateTokenResponse, JoinClusterRequest, JoinClusterResponse};
use crate::cluster::signature::verify_cluster_signature;
use crate::cluster::token_store::TokenStore;
use crate::db::{
    DbClient, NewProject, NewServer, NewUser, ProjectDetailsRecord, ProjectListRecord,
    ServerRecord,
};
use crate::deployment::build::{BuildSettings, BuildSystem};
use crate::deployment::git::Git;
use crate::deployment::inactivity_monitor::{InactivityMonitor, MonitoredProject};
use crate::deployment::nginx::NginxGenerator;
use crate::deployment::systemd::SystemdGenerator;
use crate::system::PrivilegeWrapper;

const DEFAULT_DB_PATH: &str = "/opt/nanoscale/data/nanoscale.db";
const DEFAULT_BIND_ADDRESS: &str = "0.0.0.0:4000";
const DEFAULT_LOCAL_SERVER_ID: &str = "orchestrator-local";
const DEFAULT_LOCAL_SERVER_NAME: &str = "orchestrator";
const DEFAULT_LOCAL_SERVER_IP: &str = "127.0.0.1";
const SESSION_USER_ID_KEY: &str = "user_id";

#[derive(Debug, Clone)]
pub struct OrchestratorState {
    pub db: DbClient,
    pub token_store: Arc<TokenStore>,
    pub monitored_projects: Arc<RwLock<Vec<MonitoredProject>>>,
    pub local_server_id: String,
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
    install_command: String,
    run_command: String,
    output_directory: String,
    port: Option<u16>,
    env_vars: Vec<ProjectEnvVar>,
}

#[derive(Debug, Serialize)]
struct CreateProjectResponse {
    id: String,
}

#[derive(Debug, Serialize)]
struct ProjectListItem {
    id: String,
    name: String,
    repo_url: String,
    branch: String,
    run_command: String,
    port: i64,
    status: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ProjectDetailsResponse {
    id: String,
    server_id: String,
    server_name: Option<String>,
    name: String,
    repo_url: String,
    branch: String,
    install_command: String,
    build_command: String,
    run_command: String,
    status: String,
    port: i64,
    created_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct WorkerCreateProjectRequest {
    project_id: String,
    name: String,
    repo_url: String,
    branch: String,
    build_command: String,
    install_command: String,
    run_command: String,
    output_directory: String,
    port: u16,
    env_vars: Vec<ProjectEnvVar>,
}

#[derive(Debug, Serialize)]
struct InternalProjectResponse {
    status: &'static str,
    message: String,
}

pub async fn run() -> Result<()> {
    let database_path =
        std::env::var("NANOSCALE_DB_PATH").unwrap_or_else(|_| DEFAULT_DB_PATH.to_string());
    let bind_address = std::env::var("NANOSCALE_ORCHESTRATOR_BIND")
        .unwrap_or_else(|_| DEFAULT_BIND_ADDRESS.to_string());
    let db_client = DbClient::initialize(&database_path).await?;

    let local_server_id = std::env::var("NANOSCALE_ORCHESTRATOR_SERVER_ID")
        .unwrap_or_else(|_| DEFAULT_LOCAL_SERVER_ID.to_string());
    let local_server_name = std::env::var("NANOSCALE_ORCHESTRATOR_SERVER_NAME")
        .unwrap_or_else(|_| DEFAULT_LOCAL_SERVER_NAME.to_string());
    let orchestrator_worker_ip = std::env::var("NANOSCALE_ORCHESTRATOR_WORKER_IP")
        .unwrap_or_else(|_| DEFAULT_LOCAL_SERVER_IP.to_string());
    let local_server_secret = generate_secret_key();

    db_client
        .upsert_server(&NewServer {
            id: local_server_id.clone(),
            name: local_server_name,
            ip_address: orchestrator_worker_ip,
            status: "online".to_string(),
            secret_key: local_server_secret,
        })
        .await?;

    let state = OrchestratorState {
        db: db_client,
        token_store: Arc::new(TokenStore::new()),
        monitored_projects: Arc::new(RwLock::new(Vec::new())),
        local_server_id,
    };

    let monitor = InactivityMonitor::new(state.monitored_projects.clone());
    monitor.spawn();

    let session_store = SqliteStore::new(state.db.pool());
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let internal_router = Router::new()
        .route("/projects", post(internal_projects))
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
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/{id}", get(get_project))
        .route("/api/projects/:id", get(get_project))
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

#[allow(clippy::too_many_lines)]
async fn internal_projects(
    State(state): State<OrchestratorState>,
    Json(payload): Json<WorkerCreateProjectRequest>,
) -> (StatusCode, Json<InternalProjectResponse>) {
    let project_id = payload.project_id;
    let repo_url = payload.repo_url;
    let branch = payload.branch;
    let build_command = payload.build_command;
    let install_command = payload.install_command;
    let run_command = payload.run_command;
    let output_directory = payload.output_directory;
    let port = payload.port;
    let _env_var_pairs = payload
        .env_vars
        .into_iter()
        .map(|env_var| (env_var.key, env_var.value))
        .collect::<Vec<(String, String)>>();

    let repo_dir = PathBuf::from(format!("/opt/nanoscale/tmp/{project_id}/source"));
    let parent_dir = repo_dir
        .parent()
        .map_or_else(|| PathBuf::from("/opt/nanoscale/tmp"), PathBuf::from);

    let repo_url_for_clone = repo_url.clone();
    let branch_for_checkout = branch.clone();
    let repo_dir_for_clone = repo_dir.clone();
    let project_id_for_build = project_id.clone();
    let build_command_for_run = build_command.clone();
    let install_command_for_run = install_command.clone();
    let run_command_for_systemd = run_command.clone();
    let output_directory_for_run = output_directory.clone();

    let clone_result = tokio::task::spawn_blocking(move || {
        Git::validate_repo_url(&repo_url_for_clone).context("repo URL validation failed")?;
        Git::validate_branch(&branch_for_checkout).context("branch validation failed")?;

        std::fs::create_dir_all(&parent_dir).context("failed to create repo parent directory")?;

        if repo_dir_for_clone.exists() {
            std::fs::remove_dir_all(&repo_dir_for_clone)
                .context("failed to clean existing repo directory")?;
        }

        Git::clone(&repo_url_for_clone, &repo_dir_for_clone).context("git clone step failed")?;
        Git::checkout(&repo_dir_for_clone, &branch_for_checkout)
            .context("git checkout step failed")?;

        let privilege_wrapper = PrivilegeWrapper::new();
        let build_settings = BuildSettings {
            build_command: build_command_for_run,
            output_directory: output_directory_for_run,
            install_command: install_command_for_run,
        };

        let build_output = BuildSystem::execute(
            &project_id_for_build,
            &repo_dir_for_clone,
            &build_settings,
            &privilege_wrapper,
        )
        .context("build pipeline failed")?;

        SystemdGenerator::generate_and_install(
            &project_id_for_build,
            &build_output.source_dir,
            &build_output.runtime,
            &run_command_for_systemd,
            port,
            &privilege_wrapper,
        )
        .context("systemd generation failed")?;
        NginxGenerator::generate_and_install(&project_id_for_build, port, &privilege_wrapper)
            .context("nginx generation failed")?;

        Result::<(), anyhow::Error>::Ok(())
    })
    .await;

    let git_message = match clone_result {
        Ok(Ok(())) => "Source cloned and branch checked out.",
        Ok(Err(error)) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(InternalProjectResponse {
                    status: "error",
                    message: format!("Deployment pipeline failed: {error:#}"),
                }),
            );
        }
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(InternalProjectResponse {
                    status: "error",
                    message: format!("Git task failed: {error:#}"),
                }),
            );
        }
    };

    {
        let mut monitored_projects = state.monitored_projects.write().await;
        monitored_projects
            .retain(|project| project.service_name != format!("nanoscale-{project_id}.service"));
        monitored_projects.push(MonitoredProject {
            service_name: format!("nanoscale-{project_id}.service"),
            port,
            scale_to_zero: true,
        });
    }

    (
        StatusCode::ACCEPTED,
        Json(InternalProjectResponse {
            status: "accepted",
            message: format!(
                "{git_message} Build pipeline, systemd generation, and nginx configuration completed."
            ),
        }),
    )
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

fn map_project_list_record(project: ProjectListRecord) -> ProjectListItem {
    ProjectListItem {
        id: project.id,
        name: project.name,
        repo_url: project.repo_url,
        branch: project.branch,
        run_command: project.start_command,
        port: project.port,
        status: "deployed".to_string(),
        created_at: project.created_at,
    }
}

fn map_project_details_record(project: ProjectDetailsRecord) -> ProjectDetailsResponse {
    ProjectDetailsResponse {
        id: project.id,
        server_id: project.server_id,
        server_name: project.server_name,
        name: project.name,
        repo_url: project.repo_url,
        branch: project.branch,
        install_command: project.install_command,
        build_command: project.build_command,
        run_command: project.start_command,
        status: "deployed".to_string(),
        port: project.port,
        created_at: project.created_at,
    }
}

async fn list_projects(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<Vec<ProjectListItem>>, StatusCode> {
    require_authenticated(&session).await?;

    let projects = state
        .db
        .list_projects()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        projects
            .into_iter()
            .map(map_project_list_record)
            .collect(),
    ))
}

async fn get_project(
    State(state): State<OrchestratorState>,
    session: Session,
    AxumPath(project_id): AxumPath<String>,
) -> Result<Json<ProjectDetailsResponse>, StatusCode> {
    require_authenticated(&session).await?;

    let project = state
        .db
        .get_project_by_id(&project_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(map_project_details_record(project)))
}

async fn create_project(
    State(state): State<OrchestratorState>,
    session: Session,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<Json<CreateProjectResponse>, (StatusCode, String)> {
    require_authenticated(&session)
        .await
        .map_err(|status| (status, "Authentication required".to_string()))?;

    if payload.name.trim().is_empty()
        || payload.repo_url.trim().is_empty()
        || payload.install_command.trim().is_empty()
        || payload.build_command.trim().is_empty()
        || payload.run_command.trim().is_empty()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Project name, repository URL, install/build/run commands are required".to_string(),
        ));
    }

    let connection = state
        .db
        .get_server_connection_info(&payload.server_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to load server connection info: {error}"),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Selected server was not found".to_string(),
        ))?;

    let project_id = Uuid::new_v4().to_string();
    let project_port = match payload.port {
        Some(requested_port) => {
            let requested_port = i64::from(requested_port);
            if requested_port < DbClient::min_project_port() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Requested port must be {} or higher",
                        DbClient::min_project_port()
                    ),
                ));
            }

            let in_use = state
                .db
                .is_project_port_in_use(requested_port)
                .await
                .map_err(|error| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unable to validate requested port: {error}"),
                    )
                })?;

            if in_use {
                return Err((
                    StatusCode::CONFLICT,
                    format!("Requested port {requested_port} is already in use"),
                ));
            }

            requested_port
        }
        None => state.db.next_available_project_port().await.map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to allocate project port: {error}"),
            )
        })?,
    };

    let project = NewProject {
        id: project_id.clone(),
        server_id: payload.server_id.clone(),
        name: payload.name.clone(),
        repo_url: payload.repo_url.clone(),
        branch: payload.branch.clone(),
        install_command: payload.install_command.clone(),
        build_command: payload.build_command.clone(),
        start_command: payload.run_command.clone(),
        env_vars: serde_json::to_string(&payload.env_vars).map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize env vars: {error}"),
            )
        })?,
        port: project_port,
    };

    state.db.insert_project(&project).await.map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to persist project record: {error}"),
        )
    })?;

    let worker_host = if connection.id == state.local_server_id {
        "127.0.0.1"
    } else {
        &connection.ip_address
    };

    if let Err(error) = call_worker_create_project(
        &connection.id,
        worker_host,
        &connection.secret_key,
        &payload,
        &project_id,
        u16::try_from(project_port).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Allocated port out of range: {project_port}"),
            )
        })?,
    )
    .await
    {
        let _ = state.db.delete_project_by_id(&project_id).await;
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Worker deployment call failed: {error}"),
        ));
    }

    Ok(Json(CreateProjectResponse { id: project_id }))
}

async fn call_worker_create_project(
    server_id: &str,
    worker_host: &str,
    secret_key: &str,
    payload: &CreateProjectRequest,
    project_id: &str,
    project_port: u16,
) -> Result<(), anyhow::Error> {
    let worker_payload = WorkerCreateProjectRequest {
        project_id: project_id.to_string(),
        name: payload.name.clone(),
        repo_url: payload.repo_url.clone(),
        branch: payload.branch.clone(),
        build_command: payload.build_command.clone(),
        install_command: payload.install_command.clone(),
        run_command: payload.run_command.clone(),
        output_directory: payload.output_directory.clone(),
        port: project_port,
        env_vars: payload.env_vars.clone(),
    };

    let body = serde_json::to_vec(&worker_payload)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string();

    let signature = sign_internal_payload(&body, &timestamp, secret_key)?;
    let url = format!("http://{worker_host}:4000/internal/projects");

    let response = reqwest::Client::new()
        .post(url)
        .header("X-Cluster-Timestamp", timestamp)
        .header("X-Cluster-Signature", signature)
        .header("X-Server-Id", server_id)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("internal projects endpoint returned {status}: {body}");
    }

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

fn generate_secret_key() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}
