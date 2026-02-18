use anyhow::Result;
use axum::routing::{delete, post};
use axum::Router;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::cluster::protocol::{JoinClusterRequest, JoinClusterResponse};
use crate::config::NanoScaleConfig;
use crate::deployment::inactivity_monitor::InactivityMonitor;
use crate::system::PrivilegeWrapper;

mod api_types;
mod handlers;

#[cfg(test)]
mod tests;

use api_types::WorkerState;

/// Starts the worker internal API and joins the cluster using `join_token`.
///
/// # Errors
/// Returns an error if configuration loading fails, joining the cluster fails, binding the
/// listener fails, or the HTTP server terminates with an error.
pub async fn run(join_token: &str) -> Result<()> {
    let privilege_wrapper = PrivilegeWrapper::new();

    if std::env::var_os("NANOSCALE_AGENT_SELFTEST_SUDO").is_some() {
        let _ = privilege_wrapper.run("/usr/bin/systemctl", &["status", "nanoscale-agent"]);
    }

    let config = NanoScaleConfig::load()?;
    let orchestrator_url = config.worker_orchestrator_url();
    let worker_ip = config.worker_ip();
    let worker_name = config.worker_name();
    let worker_bind = config.worker_bind();

    let secret_key = generate_secret_key();
    let join_request = JoinClusterRequest {
        token: join_token.to_string(),
        ip: worker_ip,
        secret_key,
        name: worker_name,
    };

    let join_url = format!("{orchestrator_url}/api/cluster/join");
    let join_response = reqwest::Client::new()
        .post(join_url)
        .json(&join_request)
        .send()
        .await?
        .error_for_status()?
        .json::<JoinClusterResponse>()
        .await?;

    println!("Starting worker mode with join token: {join_token}");
    println!(
        "Worker joined cluster with server id: {}",
        join_response.server_id
    );

    let worker_state = WorkerState {
        monitored_projects: Arc::new(RwLock::new(Vec::new())),
    };
    let monitor = InactivityMonitor::new(worker_state.monitored_projects.clone());
    monitor.spawn();

    let app = Router::new()
        .route("/internal/health", post(handlers::internal_health))
        .route("/internal/deploy", post(handlers::internal_deploy))
        .route("/internal/projects", post(handlers::internal_projects))
        .route(
            "/internal/projects/:id",
            delete(handlers::internal_delete_project),
        )
        .with_state(worker_state);

    let listener = tokio::net::TcpListener::bind(&worker_bind).await?;
    println!("Worker internal API listening on: {worker_bind}");

    axum::serve(listener, app).await?;
    Ok(())
}

fn generate_secret_key() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}
