use std::path::PathBuf;

use anyhow::{Context, Result};
use axum::extract::Path as AxumPath;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::deployment::build::{BuildSettings, BuildSystem};
use crate::deployment::git::Git;
use crate::deployment::nginx::{NginxGenerator, NginxTlsMode};
use crate::deployment::systemd::SystemdGenerator;
use crate::deployment::teardown::Teardown;
use crate::deployment::tls::TlsProvisioner;
use crate::system::PrivilegeWrapper;

use super::api_types::{InternalProjectResponse, WorkerCreateProjectRequest};
use super::OrchestratorState;

fn repo_paths(project_id: &str) -> (PathBuf, PathBuf) {
    let repo_dir = PathBuf::from(format!("/opt/nanoscale/tmp/{project_id}/source"));
    let parent_dir = repo_dir
        .parent()
        .map_or_else(|| PathBuf::from("/opt/nanoscale/tmp"), PathBuf::from);
    (repo_dir, parent_dir)
}

#[allow(clippy::too_many_lines)]
pub(super) async fn internal_projects(
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
    let domain = payload.domain;
    let tls_email = payload.tls_email;
    let _env_var_pairs = payload
        .env_vars
        .into_iter()
        .map(|env_var| (env_var.key, env_var.value))
        .collect::<Vec<(String, String)>>();

    let (repo_dir, parent_dir) = repo_paths(&project_id);

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
        monitored_projects.push(crate::deployment::inactivity_monitor::MonitoredProject {
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
                "{git_message} Build pipeline, systemd generation, and nginx configuration completed. {tls_message}.",
            ),
        }),
    )
}

pub(super) async fn internal_delete_project(
    State(state): State<OrchestratorState>,
    AxumPath(project_id): AxumPath<String>,
) -> (StatusCode, Json<InternalProjectResponse>) {
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
                StatusCode::OK,
                Json(InternalProjectResponse {
                    status: "accepted",
                    message: "Project resources deleted".to_string(),
                }),
            )
        }
        Ok(Err(error)) => (
            StatusCode::BAD_REQUEST,
            Json(InternalProjectResponse {
                status: "error",
                message: format!("Project cleanup failed: {error:#}"),
            }),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(InternalProjectResponse {
                status: "error",
                message: format!("Project cleanup task failed: {error:#}"),
            }),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_paths_build_expected_paths() {
        let (repo_dir, parent_dir) = repo_paths("p1");
        assert!(repo_dir
            .to_string_lossy()
            .ends_with("/opt/nanoscale/tmp/p1/source"));
        assert!(parent_dir
            .to_string_lossy()
            .ends_with("/opt/nanoscale/tmp/p1"));
    }
}
