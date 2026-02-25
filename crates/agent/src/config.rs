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
    pub tls_email: Option<String>,
    pub orchestrator: OrchestratorConfig,
    pub worker: WorkerConfig,
    pub github: GitHubConfig,
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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct GitHubConfig {
    pub enabled: Option<bool>,
    pub app_id: Option<String>,
    pub app_slug: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub private_key_path: Option<String>,
    pub webhook_secret: Option<String>,
    pub public_base_url: Option<String>,
    pub encryption_key: Option<String>,
}

impl NanoScaleConfig {
    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if the config file is located but JSON contents cannot be parsed
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

    #[must_use]
    pub fn database_path(&self) -> String {
        self.database_path
            .as_deref()
            .unwrap_or(DEFAULT_DB_PATH)
            .trim()
            .to_string()
    }

    #[must_use]
    pub fn orchestrator_bind_address(&self) -> String {
        self.orchestrator
            .bind_address
            .as_deref()
            .unwrap_or(DEFAULT_ORCHESTRATOR_BIND_ADDRESS)
            .trim()
            .to_string()
    }

    #[must_use]
    pub fn orchestrator_server_id(&self) -> String {
        self.orchestrator
            .server_id
            .as_deref()
            .unwrap_or(DEFAULT_ORCHESTRATOR_SERVER_ID)
            .trim()
            .to_string()
    }

    #[must_use]
    pub fn orchestrator_server_name(&self) -> String {
        self.orchestrator
            .server_name
            .as_deref()
            .unwrap_or(DEFAULT_ORCHESTRATOR_SERVER_NAME)
            .trim()
            .to_string()
    }

    #[must_use]
    pub fn orchestrator_worker_ip(&self) -> String {
        self.orchestrator
            .worker_ip
            .as_deref()
            .unwrap_or(DEFAULT_ORCHESTRATOR_WORKER_IP)
            .trim()
            .to_string()
    }

    pub fn orchestrator_base_domain(&self) -> Option<String> {
        self.orchestrator
            .base_domain
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
    }

    pub fn tls_email(&self) -> Option<String> {
        self.tls_email
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .or_else(|| {
                std::env::var("NANOSCALE_TLS_EMAIL")
                    .ok()
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
            })
    }

    #[must_use]
    pub fn worker_orchestrator_url(&self) -> String {
        self.worker
            .orchestrator_url
            .as_deref()
            .unwrap_or(DEFAULT_WORKER_ORCHESTRATOR_URL)
            .trim()
            .to_string()
    }

    #[must_use]
    pub fn worker_ip(&self) -> String {
        self.worker
            .ip
            .as_deref()
            .unwrap_or(DEFAULT_WORKER_IP)
            .trim()
            .to_string()
    }

    #[must_use]
    pub fn worker_name(&self) -> String {
        self.worker
            .name
            .as_deref()
            .unwrap_or(DEFAULT_WORKER_NAME)
            .trim()
            .to_string()
    }

    #[must_use]
    pub fn worker_bind(&self) -> String {
        self.worker
            .bind
            .as_deref()
            .unwrap_or(DEFAULT_WORKER_BIND)
            .trim()
            .to_string()
    }

    #[must_use]
    pub fn github_enabled(&self) -> bool {
        self.github.enabled.unwrap_or(false)
            || std::env::var("NANOSCALE_GITHUB_ENABLED")
                .ok()
                .is_some_and(|value| {
                    value.trim().eq_ignore_ascii_case("true") || value.trim() == "1"
                })
    }

    #[must_use]
    pub fn github_app_id(&self) -> Option<String> {
        self.github
            .app_id
            .clone()
            .or_else(|| std::env::var("NANOSCALE_GITHUB_APP_ID").ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    #[must_use]
    pub fn github_app_slug(&self) -> Option<String> {
        self.github
            .app_slug
            .clone()
            .or_else(|| std::env::var("NANOSCALE_GITHUB_APP_SLUG").ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    #[must_use]
    pub fn github_client_id(&self) -> Option<String> {
        self.github
            .client_id
            .clone()
            .or_else(|| std::env::var("NANOSCALE_GITHUB_APP_CLIENT_ID").ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    #[must_use]
    pub fn github_client_secret(&self) -> Option<String> {
        self.github
            .client_secret
            .clone()
            .or_else(|| std::env::var("NANOSCALE_GITHUB_APP_CLIENT_SECRET").ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    #[must_use]
    pub fn github_private_key_path(&self) -> Option<String> {
        self.github
            .private_key_path
            .clone()
            .or_else(|| std::env::var("NANOSCALE_GITHUB_APP_PRIVATE_KEY_PATH").ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    #[must_use]
    pub fn github_webhook_secret(&self) -> Option<String> {
        self.github
            .webhook_secret
            .clone()
            .or_else(|| std::env::var("NANOSCALE_GITHUB_WEBHOOK_SECRET").ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    #[must_use]
    pub fn public_base_url(&self) -> Option<String> {
        self.github
            .public_base_url
            .clone()
            .or_else(|| std::env::var("NANOSCALE_PUBLIC_BASE_URL").ok())
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty())
    }

    #[must_use]
    pub fn github_encryption_key(&self) -> Option<String> {
        self.github
            .encryption_key
            .clone()
            .or_else(|| std::env::var("NANOSCALE_GITHUB_ENCRYPTION_KEY").ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn load_returns_default_when_file_missing() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        std::env::set_var(
            "NANOSCALE_CONFIG_PATH",
            "/path/that/does/not/exist/config.json",
        );

        let config = NanoScaleConfig::load().expect("load should succeed");
        assert_eq!(config.database_path(), DEFAULT_DB_PATH);
        assert_eq!(
            config.orchestrator_bind_address(),
            DEFAULT_ORCHESTRATOR_BIND_ADDRESS
        );
        assert_eq!(
            config.worker_orchestrator_url(),
            DEFAULT_WORKER_ORCHESTRATOR_URL
        );

        std::env::remove_var("NANOSCALE_CONFIG_PATH");
    }

    #[test]
    fn load_parses_and_trims_values() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        let tempdir = tempfile::tempdir().expect("tempdir");
        let config_path = tempdir.path().join("config.json");

        fs::write(
            &config_path,
            r#"{
  "database_path": "  /tmp/test.db  ",
  "tls_email": "  admin@example.com  ",
  "orchestrator": {
    "bind_address": "  127.0.0.1:9999  ",
    "base_domain": "  Example.COM.  "
  },
  "worker": {
    "orchestrator_url": "  http://localhost:1234  ",
    "ip": "  10.0.0.5  ",
    "name": "  worker-a  ",
    "bind": "  0.0.0.0:7777  "
  }
}"#,
        )
        .expect("write config");

        std::env::set_var(
            "NANOSCALE_CONFIG_PATH",
            config_path.to_string_lossy().to_string(),
        );
        std::env::remove_var("NANOSCALE_TLS_EMAIL");

        let config = NanoScaleConfig::load().expect("load should succeed");
        assert_eq!(config.database_path(), "/tmp/test.db");
        assert_eq!(config.tls_email().as_deref(), Some("admin@example.com"));
        assert_eq!(config.orchestrator_bind_address(), "127.0.0.1:9999");
        assert_eq!(
            config.orchestrator_base_domain().as_deref(),
            Some("Example.COM.")
        );
        assert_eq!(config.worker_orchestrator_url(), "http://localhost:1234");
        assert_eq!(config.worker_ip(), "10.0.0.5");
        assert_eq!(config.worker_name(), "worker-a");
        assert_eq!(config.worker_bind(), "0.0.0.0:7777");

        std::env::remove_var("NANOSCALE_CONFIG_PATH");
    }

    #[test]
    fn tls_email_falls_back_to_env_var() {
        let _guard = env_lock().lock().expect("env lock poisoned");
        std::env::set_var("NANOSCALE_TLS_EMAIL", "  ops@example.com  ");

        let config = NanoScaleConfig::default();
        assert_eq!(config.tls_email().as_deref(), Some("ops@example.com"));

        std::env::remove_var("NANOSCALE_TLS_EMAIL");
    }
}
