use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use flate2::read::GzDecoder;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use super::auth::require_authenticated;
use super::OrchestratorState;

const DEFAULT_UPDATE_REPO: &str = "dmuraco3/NanoScale";
const SYSTEM_UPDATER_WRAPPER: &str = "/usr/local/bin/nanoscale-system-updater";
const RELEASE_ARCHIVE_PATH: &str = "/tmp/nanoscale-release.tar.gz";
const STAGING_PATH: &str = "/opt/nanoscale-staging";
const LIVE_PATH: &str = "/opt/nanoscale";
const BACKUP_PATH: &str = "/opt/nanoscale-backup";
const RELEASE_ARCHIVE_ASSET_NAME: &str = "nanoscale-release.tar.gz";
const RELEASE_MANIFEST_ASSET_NAME: &str = "manifest.json";

#[derive(Debug, Deserialize)]
struct ReleaseManifest {
    version: String,
    requires_system_update: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct UpdateStatusResponse {
    current_version: String,
    latest_version: String,
    update_available: bool,
}

pub(super) async fn admin_update(
    State(_state): State<OrchestratorState>,
    session: Session,
) -> Result<StatusCode, StatusCode> {
    require_authenticated(&session).await?;

    tokio::spawn(async {
        if let Err(error) = run_update_task().await {
            eprintln!("Update task failed: {error:#}");
        }
    });

    Ok(StatusCode::ACCEPTED)
}

pub(super) async fn health_check() -> StatusCode {
    StatusCode::OK
}

pub(super) async fn update_status(
    State(_state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<UpdateStatusResponse>, StatusCode> {
    require_authenticated(&session).await?;

    let current_version = current_installed_version();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let release_context = release_context_from_env();
    let manifest_url = latest_download_url(&release_context.repo_slug, RELEASE_MANIFEST_ASSET_NAME);
    let manifest = download_manifest(&client, &manifest_url)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    Ok(Json(UpdateStatusResponse {
        update_available: version_is_newer(&manifest.version, &current_version),
        current_version,
        latest_version: manifest.version,
    }))
}

async fn run_update_task() -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("failed to initialize HTTP client")?;

    let release_context = release_context_from_env();
    let manifest_url = latest_download_url(&release_context.repo_slug, RELEASE_MANIFEST_ASSET_NAME);
    let archive_url = latest_download_url(&release_context.repo_slug, RELEASE_ARCHIVE_ASSET_NAME);

    let manifest = download_manifest(&client, &manifest_url).await?;

    println!(
        "Starting update for version {} (requires_system_update={})",
        manifest.version, manifest.requires_system_update
    );

    if manifest.requires_system_update {
        run_checked_command("sudo", &[SYSTEM_UPDATER_WRAPPER])?;
        return Ok(());
    }

    let archive_bytes = download_archive(&client, &archive_url).await?;

    tokio::task::spawn_blocking(move || perform_code_update_swap(&archive_bytes))
        .await
        .context("code update task join error")??;

    run_checked_command("sudo", &["/bin/systemctl", "restart", "nanoscale.service"])?;

    Ok(())
}

#[derive(Debug)]
struct ReleaseContext {
    repo_slug: String,
}

fn release_context_from_env() -> ReleaseContext {
    let repo_slug = std::env::var("NANOSCALE_UPDATE_REPO")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_UPDATE_REPO.to_string());

    ReleaseContext { repo_slug }
}

fn latest_download_url(repo_slug: &str, asset_name: &str) -> String {
    format!("https://github.com/{repo_slug}/releases/latest/download/{asset_name}")
}

fn current_installed_version() -> String {
    if let Ok(version) = std::env::var("NANOSCALE_VERSION") {
        let trimmed = version.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Ok(contents) = fs::read_to_string("/opt/nanoscale/VERSION") {
        let trimmed = contents.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    "unknown".to_string()
}

fn normalize_version_segments(version: &str) -> Option<Vec<u64>> {
    let normalized = version.trim().trim_start_matches('v');
    if normalized.is_empty() {
        return None;
    }

    let numeric_part = normalized.split(['-', '+']).next().unwrap_or(normalized);

    let mut segments = Vec::new();
    for part in numeric_part.split('.') {
        if part.is_empty() {
            return None;
        }

        let value = part.parse::<u64>().ok()?;
        segments.push(value);
    }

    if segments.is_empty() {
        return None;
    }

    Some(segments)
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    if current == "unknown" {
        return true;
    }

    let Some(latest_segments) = normalize_version_segments(latest) else {
        return latest != current;
    };

    let Some(current_segments) = normalize_version_segments(current) else {
        return latest != current;
    };

    let max_len = std::cmp::max(latest_segments.len(), current_segments.len());
    for index in 0..max_len {
        let left = *latest_segments.get(index).unwrap_or(&0);
        let right = *current_segments.get(index).unwrap_or(&0);

        if left > right {
            return true;
        }

        if left < right {
            return false;
        }
    }

    false
}

fn run_checked_command(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute command: {program} {}", args.join(" ")))?;

    if !status.success() {
        anyhow::bail!(
            "command exited with non-zero status: {program} {}",
            args.join(" ")
        );
    }

    Ok(())
}

async fn download_manifest(
    client: &reqwest::Client,
    manifest_url: &str,
) -> Result<ReleaseManifest> {
    let response = client
        .get(manifest_url)
        .header(USER_AGENT, "nanoscale-updater")
        .send()
        .await
        .context("failed to download update manifest")?
        .error_for_status()
        .with_context(|| format!("manifest download failed for {manifest_url}"))?;

    response
        .json::<ReleaseManifest>()
        .await
        .context("failed to parse update manifest")
}

async fn download_archive(client: &reqwest::Client, archive_url: &str) -> Result<Vec<u8>> {
    let response = client
        .get(archive_url)
        .header(USER_AGENT, "nanoscale-updater")
        .send()
        .await
        .context("failed to download release archive")?
        .error_for_status()
        .with_context(|| format!("release archive download failed for {archive_url}"))?;

    response
        .bytes()
        .await
        .context("failed to read release archive bytes")
        .map(|bytes| bytes.to_vec())
}

fn perform_code_update_swap(archive_bytes: &[u8]) -> Result<()> {
    fs::write(RELEASE_ARCHIVE_PATH, archive_bytes).context("failed to write archive to /tmp")?;

    let staging = Path::new(STAGING_PATH);
    let live = Path::new(LIVE_PATH);
    let backup = Path::new(BACKUP_PATH);

    if staging.exists() {
        fs::remove_dir_all(staging).context("failed to clean old staging directory")?;
    }
    fs::create_dir_all(staging).context("failed to create staging directory")?;

    let archive_file =
        fs::File::open(RELEASE_ARCHIVE_PATH).context("failed to open release archive")?;
    let decoder = GzDecoder::new(archive_file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(staging)
        .context("failed to extract release archive into staging")?;

    if backup.exists() {
        fs::remove_dir_all(backup).context("failed to remove previous backup directory")?;
    }

    if live.exists() {
        fs::rename(live, backup).context("failed to move live directory to backup")?;
    }

    fs::rename(staging, live).context("failed to move staging directory into live path")?;

    Ok(())
}
