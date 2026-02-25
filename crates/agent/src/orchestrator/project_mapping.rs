use crate::db::{ProjectDetailsRecord, ProjectListRecord};

use super::api_types::{ProjectDetailsResponse, ProjectListItem};

pub(super) fn map_project_list_record(project: ProjectListRecord) -> ProjectListItem {
    ProjectListItem {
        id: project.id,
        name: project.name,
        repo_url: project.repo_url,
        branch: project.branch,
        run_command: project.start_command,
        port: project.port,
        domain: project.domain,
        source_provider: project.source_provider,
        source_repo_id: project.source_repo_id,
        status: "deployed".to_string(),
        created_at: project.created_at,
    }
}

pub(super) fn map_project_details_record(project: ProjectDetailsRecord) -> ProjectDetailsResponse {
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
        domain: project.domain,
        source_provider: project.source_provider,
        source_repo_id: project.source_repo_id,
        created_at: project.created_at,
    }
}
