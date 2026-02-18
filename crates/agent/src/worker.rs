use anyhow::{Context, Result};
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::routing::{delete, post};
use axum::{Json, Router};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use sysinfo::System;
use tokio::sync::RwLock;

use crate::cluster::protocol::{JoinClusterRequest, JoinClusterResponse};
use crate::deployment::build::{BuildSettings, BuildSystem};
use crate::deployment::git::Git;
use crate::deployment::inactivity_monitor::{InactivityMonitor, MonitoredProject};
use crate::deployment::nginx::NginxGenerator;
use crate::deployment::systemd::SystemdGenerator;
use crate::deployment::teardown::Teardown;
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
    message: String,
}

#[derive(Debug, Deserialize)]
struct WorkerCreateProjectRequest {
    project_id: String,
    repo_url: String,
    branch: String,
    build_command: String,
    install_command: String,
    run_command: String,
    output_directory: String,
    port: u16,
    env_vars: Vec<WorkerProjectEnvVar>,
}

#[derive(Debug, Deserialize)]
struct WorkerProjectEnvVar {
    key: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct CreateProjectPlaceholderResponse {
    status: &'static str,
    message: String,
}

#[derive(Clone, Debug)]
struct WorkerState {
    monitored_projects: Arc<RwLock<Vec<MonitoredProject>>>,
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

    let worker_state = WorkerState {
        monitored_projects: Arc::new(RwLock::new(Vec::new())),
    };
    let monitor = InactivityMonitor::new(worker_state.monitored_projects.clone());
    monitor.spawn();

    let app = Router::new()
        .route("/internal/health", post(internal_health))
        .route("/internal/deploy", post(internal_deploy))
        .route("/internal/projects", post(internal_projects))
        .route("/internal/projects/:id", delete(internal_delete_project))
        .with_state(worker_state);

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
            message: "Deploy endpoint placeholder. Phase 3 will implement deployment pipeline."
                .to_string(),
        }),
    )
}

async fn internal_delete_project(
    State(state): State<WorkerState>,
    AxumPath(project_id): AxumPath<String>,
) -> (StatusCode, Json<CreateProjectPlaceholderResponse>) {
    let project_id_for_cleanup = project_id.clone();
    let delete_result = tokio::task::spawn_blocking(move || {
        let privilege_wrapper = PrivilegeWrapper::new();
        Teardown::delete_project(&project_id_for_cleanup, &privilege_wrapper)
    })
    .await;

    match delete_result {
        Ok(Ok(())) => {
            let mut monitored_projects = state.monitored_projects.write().await;
            monitored_projects
                .retain(|project| project.service_name != format!("nanoscale-{project_id}.service"));

            (
                StatusCode::NO_CONTENT,
                Json(CreateProjectPlaceholderResponse {
                    status: "accepted",
                    message: "Project resources deleted".to_string(),
                }),
            )
        }
        Ok(Err(error)) => (
            StatusCode::BAD_REQUEST,
            Json(CreateProjectPlaceholderResponse {
                status: "error",
                message: format!("Project cleanup failed: {error:#}"),
            }),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(CreateProjectPlaceholderResponse {
                status: "error",
                message: format!("Project cleanup task failed: {error:#}"),
            }),
        ),
    }
}

#[allow(clippy::too_many_lines)]
async fn internal_projects(
    State(state): State<WorkerState>,
    Json(payload): Json<WorkerCreateProjectRequest>,
) -> (StatusCode, Json<CreateProjectPlaceholderResponse>) {
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
                Json(CreateProjectPlaceholderResponse {
                    status: "error",
                    message: format!("Deployment pipeline failed: {error:#}"),
                }),
            );
        }
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(CreateProjectPlaceholderResponse {
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
        Json(CreateProjectPlaceholderResponse {
            status: "accepted",
            message: format!(
                "{git_message} Build pipeline, systemd generation, and nginx configuration completed."
            ),
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
