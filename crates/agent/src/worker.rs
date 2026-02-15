use anyhow::Result;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Serialize;
use sysinfo::System;

use crate::cluster::protocol::{JoinClusterRequest, JoinClusterResponse};
use crate::system::PrivilegeWrapper;

const DEFAULT_ORCHESTRATOR_URL: &str = "http://127.0.0.1:4000";
const DEFAULT_WORKER_IP: &str = "127.0.0.1";
const DEFAULT_WORKER_NAME: &str = "worker-node";
const DEFAULT_WORKER_BIND: &str = "0.0.0.0:4000";

#[derive(Debug, Serialize)]
struct HealthResponse {
    cpu_usage_percent: f32,
    used_memory_bytes: u64,
    total_memory_bytes: u64,
}

#[derive(Debug, Serialize)]
struct DeployPlaceholderResponse {
    status: &'static str,
    message: &'static str,
}

pub async fn run(join_token: &str) -> Result<()> {
    let privilege_wrapper = PrivilegeWrapper::new();

    if std::env::var_os("NANOSCALE_AGENT_SELFTEST_SUDO").is_some() {
        let _ = privilege_wrapper.run("/usr/bin/systemctl", &["status", "nanoscale-agent"]);
    }

    let orchestrator_url = std::env::var("NANOSCALE_ORCHESTRATOR_URL")
        .unwrap_or_else(|_| DEFAULT_ORCHESTRATOR_URL.to_string());
    let worker_ip =
        std::env::var("NANOSCALE_WORKER_IP").unwrap_or_else(|_| DEFAULT_WORKER_IP.to_string());
    let worker_name =
        std::env::var("NANOSCALE_WORKER_NAME").unwrap_or_else(|_| DEFAULT_WORKER_NAME.to_string());
    let worker_bind =
        std::env::var("NANOSCALE_WORKER_BIND").unwrap_or_else(|_| DEFAULT_WORKER_BIND.to_string());

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

    let app = Router::new()
        .route("/internal/health", post(internal_health))
        .route("/internal/deploy", post(internal_deploy));

    let listener = tokio::net::TcpListener::bind(&worker_bind).await?;
    println!("Worker internal API listening on: {worker_bind}");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn internal_health() -> Json<HealthResponse> {
    let mut system = System::new_all();
    system.refresh_cpu_usage();
    system.refresh_memory();

    Json(HealthResponse {
        cpu_usage_percent: system.global_cpu_usage(),
        used_memory_bytes: system.used_memory(),
        total_memory_bytes: system.total_memory(),
    })
}

async fn internal_deploy() -> (StatusCode, Json<DeployPlaceholderResponse>) {
    (
        StatusCode::ACCEPTED,
        Json(DeployPlaceholderResponse {
            status: "accepted",
            message: "Deploy endpoint placeholder. Phase 3 will implement deployment pipeline.",
        }),
    )
}

fn generate_secret_key() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}
