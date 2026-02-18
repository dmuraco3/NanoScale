use axum::extract::Path as AxumPath;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tower_sessions::Session;
use uuid::Uuid;

use crate::db::{DbClient, NewProject};

use super::api_types::{
    CreateProjectRequest, CreateProjectResponse, ProjectDetailsResponse, ProjectListItem,
};
use super::auth::require_authenticated;
use super::project_domain::assigned_project_domain;
use super::project_mapping::{map_project_details_record, map_project_list_record};
use super::worker_client::{call_worker_create_project, call_worker_delete_project};
use super::OrchestratorState;

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
    let project_domain = assigned_project_domain(&state, &project_id, &payload.name).await?;
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
        None => state
            .db
            .next_available_project_port()
            .await
            .map_err(|error| {
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
        domain: project_domain.clone(),
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

    Ok(Json(CreateProjectResponse {
        id: project_id,
        domain: project_domain,
    }))
}
