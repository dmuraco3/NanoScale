use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

const DEFAULT_CONFIG_PATH: &str = "/opt/nanoscale/config.json";

const DEFAULT_DB_PATH: &str = "/opt/nanoscale/data/nanoscale.db";
const DEFAULT_ORCHESTRATOR_BIND_ADDRESS: &str = "0.0.0.0:4000";
const DEFAULT_ORCHESTRATOR_SERVER_ID: &str = "orchestrator-local";
const DEFAULT_ORCHESTRATOR_SERVER_NAME: &str = "orchestrator";
const DEFAULT_ORCHESTRATOR_WORKER_IP: &str = "127.0.0.1";

const DEFAULT_WORKER_ORCHESTRATOR_URL: &str = "http://127.0.0.1:4000";
const DEFAULT_WORKER_IP: &str = "127.0.0.1";
const DEFAULT_WORKER_NAME: &str = "worker-node";
const DEFAULT_WORKER_BIND: &str = "0.0.0.0:4000";

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct NanoScaleConfig {
    pub database_path: Option<String>,
    pub orchestrator: OrchestratorConfig,
    pub worker: WorkerConfig,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct OrchestratorConfig {
    pub bind_address: Option<String>,
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub worker_ip: Option<String>,
    pub base_domain: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct WorkerConfig {
    pub orchestrator_url: Option<String>,
    pub ip: Option<String>,
    pub name: Option<String>,
    pub bind: Option<String>,
}

impl NanoScaleConfig {
    pub fn load() -> Result<Self> {
        let config_path = std::env::var("NANOSCALE_CONFIG_PATH")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_CONFIG_PATH.to_string());

        let path = Path::new(&config_path);
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {config_path}"))?;

        let config = serde_json::from_str::<Self>(&raw)
            .with_context(|| format!("Failed to parse config JSON: {config_path}"))?;

        Ok(config)
    }

    pub fn database_path(&self) -> String {
        self.database_path
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_DB_PATH)
            .trim()
            .to_string()
    }

    pub fn orchestrator_bind_address(&self) -> String {
        self.orchestrator
            .bind_address
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_ORCHESTRATOR_BIND_ADDRESS)
            .trim()
            .to_string()
    }

    pub fn orchestrator_server_id(&self) -> String {
        self.orchestrator
            .server_id
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_ORCHESTRATOR_SERVER_ID)
            .trim()
            .to_string()
    }

    pub fn orchestrator_server_name(&self) -> String {
        self.orchestrator
            .server_name
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_ORCHESTRATOR_SERVER_NAME)
            .trim()
            .to_string()
    }

    pub fn orchestrator_worker_ip(&self) -> String {
        self.orchestrator
            .worker_ip
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_ORCHESTRATOR_WORKER_IP)
            .trim()
            .to_string()
    }

    pub fn orchestrator_base_domain(&self) -> Option<String> {
        self.orchestrator
            .base_domain
            .as_ref()
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    }

    pub fn worker_orchestrator_url(&self) -> String {
        self.worker
            .orchestrator_url
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_WORKER_ORCHESTRATOR_URL)
            .trim()
            .to_string()
    }

    pub fn worker_ip(&self) -> String {
        self.worker
            .ip
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_WORKER_IP)
            .trim()
            .to_string()
    }

    pub fn worker_name(&self) -> String {
        self.worker
            .name
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_WORKER_NAME)
            .trim()
            .to_string()
    }

    pub fn worker_bind(&self) -> String {
        self.worker
            .bind
            .as_ref()
            .map(String::as_str)
            .unwrap_or(DEFAULT_WORKER_BIND)
            .trim()
            .to_string()
    }
}
