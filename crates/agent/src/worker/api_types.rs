use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::deployment::inactivity_monitor::MonitoredProject;

#[derive(Debug, Serialize)]
pub(super) struct HealthResponse {
    pub(super) cpu_usage_percent: f32,
    pub(super) used_memory_bytes: u64,
    pub(super) total_memory_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct StatsRequest {
    pub(super) project_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct StatsResponse {
    pub(super) totals: StatsTotalsResponse,
    pub(super) projects: Vec<ProjectStatsResponse>,
}

#[derive(Debug, Serialize)]
pub(super) struct StatsTotalsResponse {
    pub(super) cpu_usage_percent: f32,
    pub(super) cpu_cores: usize,
    pub(super) used_memory_bytes: u64,
    pub(super) total_memory_bytes: u64,
    pub(super) used_disk_bytes: u64,
    pub(super) total_disk_bytes: u64,
    pub(super) network_rx_bytes_total: u64,
    pub(super) network_tx_bytes_total: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct ProjectStatsResponse {
    pub(super) project_id: String,
    pub(super) cpu_usage_nsec_total: u64,
    pub(super) memory_current_bytes: u64,
    pub(super) disk_usage_bytes: u64,
    pub(super) network_ingress_bytes_total: u64,
    pub(super) network_egress_bytes_total: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct DeployPlaceholderResponse {
    pub(super) status: &'static str,
    pub(super) message: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct WorkerCreateProjectRequest {
    pub(super) project_id: String,
    pub(super) repo_url: String,
    pub(super) branch: String,
    pub(super) build_command: String,
    pub(super) install_command: String,
    pub(super) run_command: String,
    pub(super) output_directory: String,
    pub(super) port: u16,
    pub(super) domain: Option<String>,
    pub(super) tls_email: Option<String>,
    pub(super) env_vars: Vec<WorkerProjectEnvVar>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WorkerProjectEnvVar {
    pub(super) key: String,
    pub(super) value: String,
}

#[derive(Debug, Serialize)]
pub(super) struct CreateProjectPlaceholderResponse {
    pub(super) status: &'static str,
    pub(super) message: String,
}

#[derive(Clone, Debug)]
pub(super) struct WorkerState {
    pub(super) monitored_projects: Arc<RwLock<Vec<MonitoredProject>>>,
}
