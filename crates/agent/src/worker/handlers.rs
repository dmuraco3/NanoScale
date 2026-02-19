use std::path::PathBuf;

use anyhow::{Context, Result};
use axum::extract::Path as AxumPath;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use sysinfo::System;

use crate::deployment::build::{BuildSettings, BuildSystem};
use crate::deployment::git::Git;
use crate::deployment::inactivity_monitor::MonitoredProject;
use crate::deployment::nginx::{NginxGenerator, NginxTlsMode};
use crate::deployment::systemd::SystemdGenerator;
use crate::deployment::teardown::Teardown;
use crate::deployment::tls::TlsProvisioner;
use crate::system::PrivilegeWrapper;

use super::api_types::{
    CreateProjectPlaceholderResponse, DeployPlaceholderResponse, HealthResponse,
    PortAvailabilityRequest, PortAvailabilityResponse, ProjectStatsResponse, StatsRequest,
    StatsResponse, StatsTotalsResponse, WorkerCreateProjectRequest, WorkerState,
};

use crate::system::collect_host_stats;

pub(super) async fn internal_health() -> Json<HealthResponse> {
    let mut system = System::new_all();
    system.refresh_cpu_usage();
    system.refresh_memory();

    Json(HealthResponse {
        cpu_usage_percent: system.global_cpu_usage(),
        used_memory_bytes: system.used_memory(),
        total_memory_bytes: system.total_memory(),
    })
}

pub(super) async fn internal_stats(
    Json(payload): Json<StatsRequest>,
) -> Result<Json<StatsResponse>, StatusCode> {
    let project_ids = payload.project_ids;

    let snapshot = tokio::task::spawn_blocking(move || collect_host_stats(&project_ids))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let totals = StatsTotalsResponse {
        cpu_usage_percent: snapshot.totals.cpu_usage_percent,
        cpu_cores: snapshot.totals.cpu_cores,
        used_memory_bytes: snapshot.totals.used_memory_bytes,
        total_memory_bytes: snapshot.totals.total_memory_bytes,
        used_disk_bytes: snapshot.totals.used_disk_bytes,
        total_disk_bytes: snapshot.totals.total_disk_bytes,
        network_rx_bytes_total: snapshot.totals.network_rx_bytes_total,
        network_tx_bytes_total: snapshot.totals.network_tx_bytes_total,
    };

    let mut projects = Vec::with_capacity(snapshot.projects.len());
    for (project_id, counters) in snapshot.projects {
        projects.push(ProjectStatsResponse {
            project_id,
            cpu_usage_nsec_total: counters.cpu_usage_nsec_total,
            memory_current_bytes: counters.memory_current_bytes,
            disk_usage_bytes: counters.disk_usage_bytes,
            network_ingress_bytes_total: counters.network_ingress_bytes_total,
            network_egress_bytes_total: counters.network_egress_bytes_total,
        });
    }
    projects.sort_by(|a, b| a.project_id.cmp(&b.project_id));

    Ok(Json(StatsResponse { totals, projects }))
}

pub(super) async fn internal_deploy() -> (StatusCode, Json<DeployPlaceholderResponse>) {
    (
        StatusCode::ACCEPTED,
        Json(DeployPlaceholderResponse {
            status: "accepted",
            message: "Deploy endpoint placeholder. Phase 3 will implement deployment pipeline."
                .to_string(),
        }),
    )
}

pub(super) async fn internal_port_check(
    Json(payload): Json<PortAvailabilityRequest>,
) -> (StatusCode, Json<PortAvailabilityResponse>) {
    let bind_result = tokio::net::TcpListener::bind(("127.0.0.1", payload.port)).await;
    let available = bind_result.is_ok();

    (StatusCode::OK, Json(PortAvailabilityResponse { available }))
}

pub(super) async fn internal_delete_project(
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
            monitored_projects.retain(|project| {
                project.service_name != format!("nanoscale-{project_id}.service")
            });

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
pub(super) async fn internal_projects(
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
    let domain = payload.domain;
    let tls_email = payload.tls_email;
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

    let clone_result = tokio::task::spawn_blocking(move || -> Result<String> {
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
        NginxGenerator::generate_and_install(
            &project_id_for_build,
            port,
            domain.as_deref(),
            NginxTlsMode::Disabled,
            &privilege_wrapper,
        )
        .context("nginx generation failed")?;

        let tls_summary = match (domain.as_deref(), tls_email.as_deref()) {
            (Some(domain), Some(email)) => {
                match TlsProvisioner::ensure_certificate(domain, email, &privilege_wrapper) {
                    Ok(()) => {
                        NginxGenerator::generate_and_install(
                            &project_id_for_build,
                            port,
                            Some(domain),
                            NginxTlsMode::Enabled { domain },
                            &privilege_wrapper,
                        )
                        .context("nginx TLS generation failed")?;
                        "TLS enabled".to_string()
                    }
                    Err(error) => {
                        eprintln!("TLS provisioning failed for {domain}: {error:#}");
                        format!("TLS provisioning failed: {error}")
                    }
                }
            }
            (Some(_), None) => "TLS skipped: NANOSCALE_TLS_EMAIL not configured".to_string(),
            _ => "TLS skipped: no domain assigned".to_string(),
        };

        Ok(tls_summary)
    })
    .await;

    let (git_message, tls_message) = match clone_result {
        Ok(Ok(tls_message)) => ("Source cloned and branch checked out.", tls_message),
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
                "{git_message} Build pipeline, systemd generation, and nginx configuration completed. {tls_message}.",
            ),
        }),
    )
}
