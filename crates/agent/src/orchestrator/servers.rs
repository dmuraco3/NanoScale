use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tower_sessions::Session;

use crate::db::ServerRecord;

use super::api_types::ServerListItem;
use super::auth::require_authenticated;
use super::OrchestratorState;

pub(super) async fn list_servers(
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

fn map_server_record(server: ServerRecord) -> ServerListItem {
    ServerListItem {
        id: server.id,
        name: server.name,
        ip_address: server.ip_address,
        status: server.status,
        ram_usage_percent: 0,
    }
}
