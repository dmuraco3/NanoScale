use axum::extract::Path as AxumPath;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tower_sessions::Session;
use uuid::Uuid;

use crate::db::{DbClient, NewProject};

use super::api_types::{
    CreateProjectRequest, CreateProjectResponse, ProjectDetailsResponse, ProjectEnvVar,
    ProjectListItem,
};
use super::auth::{current_user_id, require_authenticated};
use super::github::{
    authenticated_clone_url, deactivate_project_webhook, ensure_project_webhook,
    resolve_github_source,
};
use super::project_domain::assigned_project_domain;
use super::project_mapping::{map_project_details_record, map_project_list_record};
use super::worker_client::{
    call_worker_create_project, call_worker_delete_project, call_worker_port_available,
};
use super::OrchestratorState;

pub(super) async fn redeploy_project(
    State(state): State<OrchestratorState>,
    session: Session,
    AxumPath(project_id): AxumPath<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_authenticated(&session)
        .await
        .map_err(|status| (status, "Authentication required".to_string()))?;

    redeploy_project_by_id(&state, &project_id).await?;
    Ok(StatusCode::ACCEPTED)
}

pub(super) async fn redeploy_project_by_id(
    state: &OrchestratorState,
    project_id: &str,
) -> Result<(), (StatusCode, String)> {
    let project = state
        .db
        .get_project_by_id(project_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to load project: {error}"),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Project not found".to_string()))?;

    let connection = state
        .db
        .get_server_connection_info(&project.server_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to load server connection info: {error}"),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Project host server was not found".to_string(),
        ))?;

    let worker_host = if connection.id == state.local_server_id {
        "127.0.0.1"
    } else {
        &connection.ip_address
    };

    let env_vars =
        serde_json::from_str::<Vec<ProjectEnvVar>>(&project.env_vars).map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to deserialize env vars: {error}"),
            )
        })?;

    let project_port: u16 = project.port.try_into().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Project port out of range: {}", project.port),
        )
    })?;

    let payload = CreateProjectRequest {
        server_id: project.server_id.clone(),
        name: project.name.clone(),
        repo_url: project.repo_url.clone(),
        branch: project.branch.clone(),
        build_command: project.build_command.clone(),
        install_command: project.install_command.clone(),
        run_command: project.start_command.clone(),
        output_directory: project.output_directory.clone(),
        port: Some(project_port),
        env_vars,
        github_source: None,
    };

    if let Err(error) = call_worker_delete_project(
        &connection.id,
        worker_host,
        &connection.secret_key,
        project_id,
    )
    .await
    {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Worker cleanup call failed: {error}"),
        ));
    }

    let _ = deactivate_project_webhook(state, project_id).await;

    if let Err(error) = call_worker_create_project(
        &connection.id,
        worker_host,
        &connection.secret_key,
        &payload,
        project_id,
        project.domain.as_deref(),
        project_port,
        state.tls_email.as_deref(),
    )
    .await
    {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Worker deployment call failed: {error}"),
        ));
    }

    Ok(())
}

pub(super) async fn list_projects(
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
        projects.into_iter().map(map_project_list_record).collect(),
    ))
}

pub(super) async fn get_project(
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

pub(super) async fn delete_project(
    State(state): State<OrchestratorState>,
    session: Session,
    AxumPath(project_id): AxumPath<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    require_authenticated(&session)
        .await
        .map_err(|status| (status, "Authentication required".to_string()))?;

    let project = state
        .db
        .get_project_by_id(&project_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to load project: {error}"),
            )
        })?
        .ok_or((StatusCode::NOT_FOUND, "Project not found".to_string()))?;

    let connection = state
        .db
        .get_server_connection_info(&project.server_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to load server connection info: {error}"),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            "Project host server was not found".to_string(),
        ))?;

    let worker_host = if connection.id == state.local_server_id {
        "127.0.0.1"
    } else {
        &connection.ip_address
    };

    if let Err(error) = call_worker_delete_project(
        &connection.id,
        worker_host,
        &connection.secret_key,
        &project_id,
    )
    .await
    {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Worker cleanup call failed: {error}"),
        ));
    }

    state
        .db
        .delete_project_by_id(&project_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete project: {error}"),
            )
        })?;

    {
        let mut monitored_projects = state.monitored_projects.write().await;
        monitored_projects
            .retain(|project| project.service_name != format!("nanoscale-{project_id}.service"));
    }

    Ok(StatusCode::NO_CONTENT)
}

#[allow(clippy::too_many_lines)]
pub(super) async fn create_project(
    State(state): State<OrchestratorState>,
    session: Session,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<Json<CreateProjectResponse>, (StatusCode, String)> {
    let user_id = current_user_id(&session)
        .await
        .map_err(|status| (status, "Authentication required".to_string()))?;

    validate_create_project_required_fields(&payload)?;

    let resolved_github_source = if let Some(source) = payload.github_source.as_ref() {
        Some(resolve_github_source(&state, &user_id, source).await?)
    } else {
        None
    };

    let repo_url = resolved_github_source.as_ref().map_or_else(
        || payload.repo_url.clone(),
        |source| source.clone_url.clone(),
    );
    let branch = resolved_github_source.as_ref().map_or_else(
        || payload.branch.clone(),
        |source| source.selected_branch.clone(),
    );

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
    let project_domain = assigned_project_domain(&state, &project_id, &payload.name).await?;

    let worker_host = if connection.id == state.local_server_id {
        "127.0.0.1"
    } else {
        &connection.ip_address
    };
    let project_port = if let Some(requested_port) = payload.port {
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

        let requested_port_u16: u16 = requested_port.try_into().map_err(|_| {
            (
                StatusCode::BAD_REQUEST,
                "Requested port is out of range".to_string(),
            )
        })?;

        let is_available = call_worker_port_available(
            &connection.id,
            worker_host,
            &connection.secret_key,
            requested_port_u16,
        )
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_REQUEST,
                format!("Unable to validate requested port on worker: {error}"),
            )
        })?;

        if !is_available {
            return Err((
                StatusCode::CONFLICT,
                format!("Requested port {requested_port} is already bound on the target server"),
            ));
        }

        requested_port
    } else {
        let mut candidate = state
            .db
            .next_available_project_port()
            .await
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unable to allocate project port: {error}"),
                )
            })?;

        // The DB can be out of sync with already-deployed systemd sockets/services (e.g.
        // after a redeploy/reset). Probe the worker to find a bindable port.
        for _ in 0..100 {
            let candidate_u16: u16 = candidate.try_into().map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Allocated port is out of range".to_string(),
                )
            })?;

            let in_use = state
                .db
                .is_project_port_in_use(candidate)
                .await
                .map_err(|error| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unable to validate allocated port: {error}"),
                    )
                })?;

            if in_use {
                candidate += 1;
                continue;
            }

            let is_available = call_worker_port_available(
                &connection.id,
                worker_host,
                &connection.secret_key,
                candidate_u16,
            )
            .await
            .map_err(|error| {
                (
                    StatusCode::BAD_REQUEST,
                    format!("Unable to validate allocated port on worker: {error}"),
                )
            })?;

            if is_available {
                break;
            }

            candidate += 1;
        }

        candidate
    };

    let project = NewProject {
        id: project_id.clone(),
        server_id: payload.server_id.clone(),
        name: payload.name.clone(),
        repo_url: repo_url.clone(),
        branch: branch.clone(),
        install_command: payload.install_command.clone(),
        build_command: payload.build_command.clone(),
        start_command: payload.run_command.clone(),
        output_directory: payload.output_directory.clone(),
        env_vars: serde_json::to_string(&payload.env_vars).map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize env vars: {error}"),
            )
        })?,
        port: project_port,
        domain: project_domain.clone(),
        source_provider: if resolved_github_source.is_some() {
            "github".to_string()
        } else {
            "manual".to_string()
        },
        source_repo_id: resolved_github_source.as_ref().map(|source| source.repo_id),
    };

    state.db.insert_project(&project).await.map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to persist project record: {error}"),
        )
    })?;

    let mut worker_payload = payload;
    worker_payload.repo_url = repo_url;
    worker_payload.branch = branch;

    if let Some(source) = resolved_github_source.as_ref() {
        worker_payload.repo_url = authenticated_clone_url(&state, source).await?;
    }

    if let Err(error) = call_worker_create_project(
        &connection.id,
        worker_host,
        &connection.secret_key,
        &worker_payload,
        &project_id,
        project_domain.as_deref(),
        u16::try_from(project_port).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Allocated port out of range: {project_port}"),
            )
        })?,
        state.tls_email.as_deref(),
    )
    .await
    {
        let _ = state.db.delete_project_by_id(&project_id).await;
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Worker deployment call failed: {error}"),
        ));
    }

    if let Some(source) = resolved_github_source.as_ref() {
        ensure_project_webhook(&state, &project_id, source).await?;
    }

    Ok(Json(CreateProjectResponse {
        id: project_id,
        domain: project_domain,
    }))
}

fn validate_create_project_required_fields(
    payload: &CreateProjectRequest,
) -> Result<(), (StatusCode, String)> {
    let repo_missing = payload.repo_url.trim().is_empty() && payload.github_source.is_none();

    if payload.name.trim().is_empty()
        || repo_missing
        || payload.install_command.trim().is_empty()
        || payload.build_command.trim().is_empty()
        || payload.run_command.trim().is_empty()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Project name, repository URL, install/build/run commands are required".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_create_project_required_fields_rejects_blanks() {
        let payload = CreateProjectRequest {
            server_id: "srv".to_string(),
            name: String::new(),
            repo_url: "https://example.com/repo.git".to_string(),
            branch: "main".to_string(),
            build_command: "bun run build".to_string(),
            install_command: "bun install".to_string(),
            run_command: "bun run start".to_string(),
            output_directory: String::new(),
            port: None,
            env_vars: vec![],
            github_source: None,
        };

        assert_eq!(
            validate_create_project_required_fields(&payload)
                .expect_err("should reject blanks")
                .0,
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn validate_create_project_required_fields_accepts_minimal_payload() {
        let payload = CreateProjectRequest {
            server_id: "srv".to_string(),
            name: "My Project".to_string(),
            repo_url: "https://example.com/repo.git".to_string(),
            branch: "main".to_string(),
            build_command: "bun run build".to_string(),
            install_command: "bun install".to_string(),
            run_command: "bun run start".to_string(),
            output_directory: String::new(),
            port: None,
            env_vars: vec![],
            github_source: None,
        };

        validate_create_project_required_fields(&payload).expect("should be valid");
    }
}
