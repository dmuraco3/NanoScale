use anyhow::Result;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use crate::cluster::protocol::{JoinClusterRequest, JoinClusterResponse};
use crate::system::PrivilegeWrapper;

const DEFAULT_ORCHESTRATOR_URL: &str = "http://127.0.0.1:4000";
const DEFAULT_WORKER_IP: &str = "127.0.0.1";
const DEFAULT_WORKER_NAME: &str = "worker-node";

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
    Ok(())
}

fn generate_secret_key() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}
