use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::extract::Path as AxumPath;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tower_sessions::Session;

use crate::db::ServerRecord;

use super::api_types::{
    ProjectStatsBreakdownResponse, ServerListItem, ServerStatsResponse, ServerTotalsStatsResponse,
};
use super::auth::require_authenticated;
use super::worker_client::call_worker_stats;
use super::OrchestratorState;

use crate::system::collect_host_stats;

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

    let mut items = Vec::with_capacity(servers.len());
    for server in servers {
        let ram_usage_percent = if server.status.to_lowercase() != "online" {
            0
        } else if server.id == state.local_server_id {
            let snapshot = tokio::task::spawn_blocking(|| collect_host_stats(&[]))
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            percent_u8(
                snapshot.totals.used_memory_bytes,
                snapshot.totals.total_memory_bytes,
            )
        } else {
            let connection = state
                .db
                .get_server_connection_info(&server.id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            match connection {
                Some(connection) => {
                    match call_worker_stats(
                        &connection.id,
                        &connection.ip_address,
                        &connection.secret_key,
                        Vec::new(),
                    )
                    .await
                    {
                        Ok(worker_stats) => percent_u8(
                            worker_stats.totals.used_memory_bytes,
                            worker_stats.totals.total_memory_bytes,
                        ),
                        Err(_) => 0,
                    }
                }
                None => 0,
            }
        };

        items.push(map_server_record(server, ram_usage_percent));
    }

    Ok(Json(items))
}

#[allow(clippy::too_many_lines)]
pub(super) async fn get_server_stats(
    State(state): State<OrchestratorState>,
    session: Session,
    AxumPath(server_id): AxumPath<String>,
) -> Result<Json<ServerStatsResponse>, StatusCode> {
    require_authenticated(&session).await?;

    println!("servers.rs\tget_server_stats\tserver_id='{server_id}'");

    let project_rows = state
        .db
        .list_projects_for_server_stats(&server_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let project_ids = project_rows
        .iter()
        .map(|(id, _name)| id.clone())
        .collect::<Vec<String>>();

    let project_name_by_id = project_rows
        .into_iter()
        .collect::<std::collections::HashMap<String, String>>();

    let (totals_snapshot, project_snapshots) = if server_id == state.local_server_id {
        let snapshot = tokio::task::spawn_blocking({
            let project_ids = project_ids.clone();
            move || collect_host_stats(&project_ids)
        })
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        (
            snapshot.totals,
            snapshot
                .projects
                .into_iter()
                .map(|(id, counters)| {
                    (
                        id,
                        counters.cpu_usage_nsec_total,
                        counters.memory_current_bytes,
                        counters.disk_usage_bytes,
                        counters.network_ingress_bytes_total,
                        counters.network_egress_bytes_total,
                    )
                })
                .collect::<Vec<_>>(),
        )
    } else {
        let connection = state
            .db
            .get_server_connection_info(&server_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        let worker_response = call_worker_stats(
            &connection.id,
            &connection.ip_address,
            &connection.secret_key,
            project_ids.clone(),
        )
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

        let totals = crate::system::SystemTotalsSnapshot {
            cpu_usage_percent: worker_response.totals.cpu_usage_percent,
            cpu_cores: worker_response.totals.cpu_cores,
            used_memory_bytes: worker_response.totals.used_memory_bytes,
            total_memory_bytes: worker_response.totals.total_memory_bytes,
            used_disk_bytes: worker_response.totals.used_disk_bytes,
            total_disk_bytes: worker_response.totals.total_disk_bytes,
            network_rx_bytes_total: worker_response.totals.network_rx_bytes_total,
            network_tx_bytes_total: worker_response.totals.network_tx_bytes_total,
        };

        let projects = worker_response
            .projects
            .into_iter()
            .map(|project| {
                (
                    project.project_id,
                    project.cpu_usage_nsec_total,
                    project.memory_current_bytes,
                    project.disk_usage_bytes,
                    project.network_ingress_bytes_total,
                    project.network_egress_bytes_total,
                )
            })
            .collect::<Vec<_>>();

        (totals, projects)
    };

    let now_instant = Instant::now();
    let projects_for_cache = project_snapshots
        .iter()
        .map(|(project_id, cpu_nsec, _mem, _disk, net_in, net_out)| {
            (project_id.clone(), *cpu_nsec, *net_in, *net_out)
        })
        .collect::<Vec<(String, u64, u64, u64)>>();

    let computed_rates = {
        let mut cache = state.stats_cache.write().await;
        cache.compute_and_update(
            &server_id,
            now_instant,
            totals_snapshot.cpu_cores,
            totals_snapshot.network_rx_bytes_total,
            totals_snapshot.network_tx_bytes_total,
            &projects_for_cache,
        )
    };

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_millis();
    let sample_unix_ms = u64::try_from(now_ms).unwrap_or(u64::MAX);

    let totals = ServerTotalsStatsResponse {
        cpu_usage_percent: totals_snapshot.cpu_usage_percent,
        cpu_cores: totals_snapshot.cpu_cores,
        used_memory_bytes: totals_snapshot.used_memory_bytes,
        total_memory_bytes: totals_snapshot.total_memory_bytes,
        used_disk_bytes: totals_snapshot.used_disk_bytes,
        total_disk_bytes: totals_snapshot.total_disk_bytes,
        network_rx_bytes_total: totals_snapshot.network_rx_bytes_total,
        network_tx_bytes_total: totals_snapshot.network_tx_bytes_total,
        network_rx_bytes_per_sec: computed_rates.network_rx_bytes_per_sec,
        network_tx_bytes_per_sec: computed_rates.network_tx_bytes_per_sec,
    };

    let mut projects = Vec::new();
    for (project_id, _cpu_nsec, memory_current_bytes, disk_usage_bytes, net_in, net_out) in
        project_snapshots
    {
        let name = project_name_by_id
            .get(&project_id)
            .cloned()
            .unwrap_or_else(|| project_id.clone());
        let rates = computed_rates.projects.get(&project_id);
        projects.push(ProjectStatsBreakdownResponse {
            project_id: project_id.clone(),
            project_name: name,
            cpu_usage_percent: rates.map_or(0.0, |value| value.cpu_usage_percent),
            memory_current_bytes,
            disk_usage_bytes,
            network_ingress_bytes_total: net_in,
            network_egress_bytes_total: net_out,
            network_ingress_bytes_per_sec: rates
                .map_or(0.0, |value| value.network_ingress_bytes_per_sec),
            network_egress_bytes_per_sec: rates
                .map_or(0.0, |value| value.network_egress_bytes_per_sec),
        });
    }
    projects.sort_by(|a, b| a.project_name.cmp(&b.project_name));

    Ok(Json(ServerStatsResponse {
        server_id,
        sample_unix_ms,
        totals,
        projects,
    }))
}

fn map_server_record(server: ServerRecord, ram_usage_percent: u8) -> ServerListItem {
    ServerListItem {
        id: server.id,
        name: server.name,
        ip_address: server.ip_address,
        status: server.status,
        ram_usage_percent,
    }
}

fn percent_u8(used: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }

    let used_u128 = u128::from(used);
    let total_u128 = u128::from(total);
    let scaled = used_u128.saturating_mul(100);
    let rounded = (scaled.saturating_add(total_u128 / 2)).saturating_div(total_u128);
    let capped = rounded.min(100);
    u8::try_from(capped).unwrap_or(100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_server_record_sets_ram_usage_placeholder() {
        let record = ServerRecord {
            id: "srv".to_string(),
            name: "name".to_string(),
            ip_address: "127.0.0.1".to_string(),
            status: "online".to_string(),
        };
        let mapped = map_server_record(record, 42);
        assert_eq!(mapped.ram_usage_percent, 42);
        assert_eq!(mapped.status, "online");
    }
}
