use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use hmac::Mac;
use serde::{Deserialize, Serialize};

use super::api_types::{CreateProjectRequest, WorkerCreateProjectRequest};

#[derive(Debug, Serialize)]
struct WorkerPortAvailabilityRequest {
    port: u16,
}

#[derive(Debug, Deserialize)]
struct WorkerPortAvailabilityResponse {
    available: bool,
}

#[derive(Debug, Serialize)]
struct WorkerStatsRequest {
    project_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WorkerStatsResponse {
    pub(super) totals: WorkerStatsTotals,
    pub(super) projects: Vec<WorkerProjectStats>,
}

#[derive(Debug, Deserialize)]
pub(super) struct WorkerStatsTotals {
    pub(super) cpu_usage_percent: f32,
    pub(super) cpu_cores: usize,
    pub(super) used_memory_bytes: u64,
    pub(super) total_memory_bytes: u64,
    pub(super) used_disk_bytes: u64,
    pub(super) total_disk_bytes: u64,
    pub(super) network_rx_bytes_total: u64,
    pub(super) network_tx_bytes_total: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct WorkerProjectStats {
    pub(super) project_id: String,
    pub(super) cpu_usage_nsec_total: u64,
    pub(super) memory_current_bytes: u64,
    pub(super) disk_usage_bytes: u64,
    pub(super) network_ingress_bytes_total: u64,
    pub(super) network_egress_bytes_total: u64,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn call_worker_create_project(
    server_id: &str,
    worker_host: &str,
    secret_key: &str,
    payload: &CreateProjectRequest,
    project_id: &str,
    domain: Option<&str>,
    project_port: u16,
    tls_email: Option<&str>,
) -> Result<()> {
    let worker_payload = WorkerCreateProjectRequest {
        project_id: project_id.to_string(),
        name: payload.name.clone(),
        repo_url: payload.repo_url.clone(),
        branch: payload.branch.clone(),
        build_command: payload.build_command.clone(),
        install_command: payload.install_command.clone(),
        run_command: payload.run_command.clone(),
        output_directory: payload.output_directory.clone(),
        port: project_port,
        domain: domain.map(ToOwned::to_owned),
        tls_email: tls_email.map(ToOwned::to_owned),
        env_vars: payload.env_vars.clone(),
    };

    let body = serde_json::to_vec(&worker_payload)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string();
    let signature = sign_internal_payload(&body, &timestamp, secret_key)?;
    let url = format!("http://{worker_host}:4000/internal/projects");

    let response = reqwest::Client::new()
        .post(url)
        .header("X-Cluster-Timestamp", timestamp)
        .header("X-Cluster-Signature", signature)
        .header("X-Server-Id", server_id)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("internal projects endpoint returned {status}: {body}");
    }

    Ok(())
}

pub(super) async fn call_worker_delete_project(
    server_id: &str,
    worker_host: &str,
    secret_key: &str,
    project_id: &str,
) -> Result<()> {
    let body: [u8; 0] = [];
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string();
    let signature = sign_internal_payload(&body, &timestamp, secret_key)?;
    let url = format!("http://{worker_host}:4000/internal/projects/{project_id}");

    let response = reqwest::Client::new()
        .delete(url)
        .header("X-Cluster-Timestamp", timestamp)
        .header("X-Cluster-Signature", signature)
        .header("X-Server-Id", server_id)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("internal delete projects endpoint returned {status}: {body}");
    }

    Ok(())
}

pub(super) async fn call_worker_stats(
    server_id: &str,
    worker_host: &str,
    secret_key: &str,
    project_ids: Vec<String>,
) -> Result<WorkerStatsResponse> {
    let payload = WorkerStatsRequest { project_ids };
    let body = serde_json::to_vec(&payload)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string();
    let signature = sign_internal_payload(&body, &timestamp, secret_key)?;
    let url = format!("http://{worker_host}:4000/internal/stats");

    let response = reqwest::Client::new()
        .post(url)
        .header("X-Cluster-Timestamp", timestamp)
        .header("X-Cluster-Signature", signature)
        .header("X-Server-Id", server_id)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("internal stats endpoint returned {status}: {body}");
    }

    Ok(response.json::<WorkerStatsResponse>().await?)
}

pub(super) async fn call_worker_port_available(
    server_id: &str,
    worker_host: &str,
    secret_key: &str,
    port: u16,
) -> Result<bool> {
    let payload = WorkerPortAvailabilityRequest { port };
    let body = serde_json::to_vec(&payload)?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string();
    let signature = sign_internal_payload(&body, &timestamp, secret_key)?;
    let url = format!("http://{worker_host}:4000/internal/ports/check");

    let response = reqwest::Client::new()
        .post(url)
        .header("X-Cluster-Timestamp", timestamp)
        .header("X-Cluster-Signature", signature)
        .header("X-Server-Id", server_id)
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("internal ports/check endpoint returned {status}: {body}");
    }

    let parsed = response.json::<WorkerPortAvailabilityResponse>().await?;
    Ok(parsed.available)
}

fn sign_internal_payload(body: &[u8], timestamp: &str, secret_key: &str) -> Result<String> {
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(secret_key.as_bytes())?;
    hmac::Mac::update(&mut mac, body);
    hmac::Mac::update(&mut mac, timestamp.as_bytes());
    Ok(hex::encode(hmac::Mac::finalize(mac).into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmac::Mac;

    #[test]
    fn sign_internal_payload_matches_reference_hmac() {
        let body = b"{\"hello\":\"world\"}";
        let timestamp = "1700000000";
        let secret = "super-secret";

        let signed = sign_internal_payload(body, timestamp, secret).expect("sign");

        let mut mac =
            hmac::Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes()).expect("hmac init");
        mac.update(body);
        mac.update(timestamp.as_bytes());
        let expected = hex::encode(mac.finalize().into_bytes());

        assert_eq!(signed, expected);
    }
}
