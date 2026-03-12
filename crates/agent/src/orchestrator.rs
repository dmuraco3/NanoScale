use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use axum::middleware;
use axum::routing::{delete, get, post};
use axum::Router;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use tokio::sync::{Mutex, RwLock};
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

use crate::cluster::signature::verify_cluster_signature;
use crate::cluster::token_store::TokenStore;
use crate::config::NanoScaleConfig;
use crate::db::{DbClient, NewServer};
use crate::request_logging;

use self::stats_cache::StatsCache;

mod api_types;
mod auth;
mod cluster;
mod github;
mod internal;
mod project_domain;
mod project_mapping;
mod projects;
mod servers;
mod stats_cache;
mod update;
mod worker_client;

#[cfg(test)]
mod tests;

/// Shared state injected into orchestrator handlers.
///
/// This state is cloned into each request and wraps mutable shared resources in
/// `Arc<Mutex<_>>`/`Arc<RwLock<_>>` where needed.
#[derive(Debug, Clone)]
pub struct OrchestratorState {
    /// Database client used by API handlers.
    pub db: DbClient,
    /// Ephemeral join token store for cluster enrollment.
    pub token_store: Arc<TokenStore>,
    /// Server id that identifies this orchestrator in the servers table.
    pub local_server_id: String,
    /// Optional root domain used for project subdomain assignment.
    pub base_domain: Option<String>,
    /// Optional email used by workers for TLS certificate provisioning.
    pub tls_email: Option<String>,
    /// Cached host/project stats refreshed by server stats routes.
    pub stats_cache: Arc<RwLock<StatsCache>>,
    /// GitHub integration service for OAuth, repos, and webhook orchestration.
    pub(super) github: Arc<github::GitHubService>,
    /// Last webhook-trigger timestamp per project id used to debounce redeploys.
    pub(super) redeploy_debounce: Arc<Mutex<HashMap<String, u64>>>,
}

/// Starts orchestrator mode: initializes persistence and serves public/internal APIs.
///
/// # Errors
///
/// This function will return an error if setting up and running the orchestrator server process fails.
#[allow(clippy::too_many_lines)]
pub async fn run() -> Result<()> {
    let config = NanoScaleConfig::load()?;
    let database_path = config.database_path();
    let bind_address = config.orchestrator_bind_address();
    let db_client = DbClient::initialize(&database_path).await?;

    let local_server_id = config.orchestrator_server_id();
    let local_server_name = config.orchestrator_server_name();
    let orchestrator_worker_ip = config.orchestrator_worker_ip();
    let base_domain = config
        .orchestrator_base_domain()
        .as_deref()
        .map(normalize_base_domain_value)
        .transpose()?;
    let tls_email = config.tls_email();
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
        local_server_id,
        base_domain,
        tls_email,
        stats_cache: Arc::new(RwLock::new(StatsCache::default())),
        github: Arc::new(github::GitHubService::from_config(&config)?),
        redeploy_debounce: Arc::new(Mutex::new(HashMap::new())),
    };

    // keep explicit reference to satisfy clippy for imported Duration and document default debounce
    let _default_webhook_redeploy_debounce = Duration::from_secs(15);

    let session_store = SqliteStore::new(state.db.pool());
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    // Internal routes are signed and intended for worker/orchestrator service-to-service calls.
    let internal_router = Router::new()
        .route("/projects", post(internal::internal_projects))
        .route("/projects/:id", delete(internal::internal_delete_project))
        .route("/ports/check", post(internal::internal_port_check))
        .route("/verify-signature", post(cluster::verify_signature_guarded))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            verify_cluster_signature,
        ));

    // Public API used by the dashboard frontend and admin workflows.
    let app = Router::new()
        .route("/api/auth/setup", post(auth::auth_setup))
        .route("/api/auth/login", post(auth::auth_login))
        .route("/api/auth/status", post(auth::auth_status))
        .route("/api/auth/session", post(auth::auth_session))
        .route(
            "/api/auth/change-credentials",
            post(auth::auth_change_credentials),
        )
        .route(
            "/api/integrations/github/status",
            get(github::github_status),
        )
        .route("/api/integrations/github/start", post(github::github_start))
        .route(
            "/api/integrations/github/manifest/:app_name",
            get(github::github_manifest),
        )
        .route(
            "/api/integrations/github/setup/callback",
            get(github::github_manifest_callback),
        )
        .route(
            "/api/integrations/github/disconnect",
            post(github::github_disconnect),
        )
        .route(
            "/api/integrations/github/installations",
            get(github::github_installations),
        )
        .route("/api/integrations/github/repos", get(github::github_repos))
        .route(
            "/api/integrations/github/repos/sync",
            post(github::github_sync_repos),
        )
        .route(
            "/api/integrations/github/webhook",
            post(github::github_webhook),
        )
        .route("/api/admin/update", post(update::admin_update))
        .route("/api/admin/update-status", get(update::update_status))
        .route("/api/health", get(update::health_check))
        .route("/api/servers", get(servers::list_servers))
        .route("/api/servers/:id/stats", get(servers::get_server_stats))
        .route(
            "/api/projects",
            get(projects::list_projects).post(projects::create_project),
        )
        .route(
            "/api/projects/:id",
            get(projects::get_project).delete(projects::delete_project),
        )
        .route(
            "/api/projects/:id/redeploy",
            post(projects::redeploy_project),
        )
        .route(
            "/api/cluster/generate-token",
            post(cluster::generate_cluster_token),
        )
        .route("/api/cluster/join", post(cluster::join_cluster))
        .nest("/internal", internal_router)
        .route_layer(middleware::from_fn(
            request_logging::log_orchestrator_request,
        ))
        .layer(session_layer)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind_address).await?;

    println!("Starting orchestrator mode: DB + API (skeleton)");
    println!("Database initialized at: {database_path}");
    println!("Listening on: {bind_address}");

    axum::serve(listener, app).await?;
    Ok(())
}

fn normalize_base_domain_value(raw_value: &str) -> Result<String, anyhow::Error> {
    let normalized = raw_value.trim().trim_end_matches('.').to_lowercase();
    if normalized.is_empty() {
        anyhow::bail!("Base domain cannot be empty");
    }

    // Restrict to a bare domain (no scheme, port, paths, or traversal-like segments).
    if normalized.contains('/') || normalized.contains(':') || normalized.contains("..") {
        anyhow::bail!("Base domain must be a bare domain like mydomain.com");
    }

    if !normalized
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '.')
    {
        anyhow::bail!("Base domain may only contain letters, digits, dots, and hyphens");
    }

    Ok(normalized)
}

fn generate_secret_key() -> String {
    // Secrets are persisted for orchestrator-to-worker signed internal requests.
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}
