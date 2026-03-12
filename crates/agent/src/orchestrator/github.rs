use std::collections::HashSet;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{Context, Result};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Redirect;
use axum::Json;
use base64::Engine;
use hmac::Mac;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use uuid::Uuid;

use crate::db::{
    NewGitHubAppCredentials, NewGitHubInstallation, NewGitHubRepository, NewGitHubWebhookDelivery,
    NewProjectGitHubLink,
};

use super::api_types::{
    GitHubInstallationItem, GitHubProjectSourceRequest, GitHubRepositoryItem, GitHubStartResponse,
    GitHubStatusResponse,
};
use super::auth::current_user_id;
use super::projects::redeploy_project_by_id;
use super::OrchestratorState;

const GITHUB_PAGE_SIZE: usize = 100;
const GITHUB_APP_CREDENTIALS_ID: &str = "github-app-credentials";

#[derive(Debug, Deserialize)]
pub(super) struct GitHubManifestCallbackQuery {
    code: String,
}

#[derive(Debug, Serialize)]
pub(super) struct GitHubManifestResponse {
    manifest: String,
}

#[derive(Debug, Deserialize)]
struct GitHubManifestConversionResponse {
    id: i64,
    slug: Option<String>,
    client_id: String,
    client_secret: String,
    webhook_secret: String,
    pem: String,
    name: String,
    html_url: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedGitHubAppCredentials {
    app_id: String,
    app_slug: Option<String>,
    webhook_secret: String,
    private_key_pem: String,
    app_name: Option<String>,
    app_html_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct GitHubManifestPayload {
    name: String,
    url: String,
    hook_attributes: GitHubManifestHookAttributes,
    redirect_url: String,
    public: bool,
    default_permissions: serde_json::Value,
    default_events: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct GitHubManifestHookAttributes {
    url: String,
}

#[derive(Clone)]
pub(crate) struct GitHubService {
    pub(super) enabled: bool,
    client_id: Option<String>,
    client_secret: Option<String>,
    app_id: Option<String>,
    app_slug: Option<String>,
    private_key_path: Option<String>,
    webhook_secret: Option<String>,
    public_base_url: Option<String>,
    cipher: Option<Aes256Gcm>,
}

impl fmt::Debug for GitHubService {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GitHubService")
            .field("enabled", &self.enabled)
            .field("client_id", &self.client_id.as_ref().map(|_| "***"))
            .field("client_secret", &self.client_secret.as_ref().map(|_| "***"))
            .field("app_id", &self.app_id)
            .field("app_slug", &self.app_slug)
            .field("private_key_path", &self.private_key_path)
            .field(
                "webhook_secret",
                &self.webhook_secret.as_ref().map(|_| "***"),
            )
            .field("public_base_url", &self.public_base_url)
            .field("cipher", &self.cipher.as_ref().map(|_| "configured"))
            .finish()
    }
}

#[derive(Debug, Clone)]
pub(super) struct ResolvedGitHubSource {
    pub(super) installation_id: i64,
    pub(super) repo_id: i64,
    pub(super) repo_node_id: String,
    pub(super) owner_login: String,
    pub(super) repo_name: String,
    pub(super) full_name: String,
    pub(super) default_branch: String,
    pub(super) selected_branch: String,
    pub(super) clone_url: String,
}

impl GitHubService {
    pub(super) fn from_config(config: &crate::config::NanoScaleConfig) -> Result<Self> {
        let encryption_key = config.github_encryption_key();
        let cipher = if let Some(raw_key) = encryption_key {
            let key_bytes = base64::engine::general_purpose::STANDARD
                .decode(raw_key)
                .context("NANOSCALE_GITHUB_ENCRYPTION_KEY must be base64")?;
            if key_bytes.len() != 32 {
                anyhow::bail!("NANOSCALE_GITHUB_ENCRYPTION_KEY must decode to 32 bytes")
            }
            Some(Aes256Gcm::new_from_slice(&key_bytes).context("invalid encryption key")?)
        } else {
            None
        };

        Ok(Self {
            enabled: config.github_enabled(),
            client_id: config.github_client_id(),
            client_secret: config.github_client_secret(),
            app_id: config.github_app_id(),
            app_slug: config.github_app_slug(),
            private_key_path: config.github_private_key_path(),
            webhook_secret: config.github_webhook_secret(),
            public_base_url: config.public_base_url(),
            cipher,
        })
    }

    pub(super) fn has_static_credentials(&self) -> bool {
        self.client_id.is_some()
            && self.client_secret.is_some()
            && self.app_id.is_some()
            && self.private_key_path.is_some()
            && self.webhook_secret.is_some()
            && self.public_base_url.is_some()
            && self.cipher.is_some()
    }

    fn manifest_callback_url(&self) -> Option<String> {
        self.public_base_url
            .as_deref()
            .map(|base| format!("{base}/api/integrations/github/setup/callback"))
    }

    fn webhook_url(&self) -> Option<String> {
        self.public_base_url
            .as_deref()
            .map(|base| format!("{base}/api/integrations/github/webhook"))
    }

    fn static_app_install_url(&self) -> Option<String> {
        self.app_slug
            .as_deref()
            .map(|slug| format!("https://github.com/apps/{slug}/installations/new"))
    }

    fn app_manifest_json(&self, app_name: &str) -> Option<String> {
        let domain = self.public_base_url.as_deref()?.to_string();
        let manifest = GitHubManifestPayload {
            name: app_name.trim().to_string(),
            url: domain,
            hook_attributes: GitHubManifestHookAttributes {
                url: self.webhook_url()?,
            },
            redirect_url: self.manifest_callback_url()?,
            public: false,
            default_permissions: serde_json::json!({
                "contents": "write",
                "metadata": "read",
                "pull_requests": "write",
                "webhooks": "write",
                "commit_statuses": "write"
            }),
            default_events: vec!["push", "pull_request"],
        };

        serde_json::to_string(&manifest).ok()
    }

    fn encrypt(&self, value: &str) -> Result<String> {
        let cipher = self
            .cipher
            .as_ref()
            .context("GitHub encryption key missing")?;
        let mut nonce_bytes = [0_u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, value.as_bytes())
            .map_err(|_| anyhow::anyhow!("encryption failed"))?;
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        Ok(base64::engine::general_purpose::STANDARD.encode(combined))
    }

    fn decrypt(&self, encrypted_value: &str) -> Result<String> {
        let cipher = self
            .cipher
            .as_ref()
            .context("GitHub encryption key missing")?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(encrypted_value)?;
        if bytes.len() < 13 {
            anyhow::bail!("encrypted value malformed")
        }
        let nonce = Nonce::from_slice(&bytes[..12]);
        let plaintext = cipher
            .decrypt(nonce, &bytes[12..])
            .map_err(|_| anyhow::anyhow!("decryption failed"))?;
        String::from_utf8(plaintext).context("decrypted value is not utf8")
    }
}

async fn resolve_github_app_credentials(
    state: &OrchestratorState,
) -> Result<ResolvedGitHubAppCredentials, (StatusCode, String)> {
    if let Some(record) = state
        .db
        .get_github_app_credentials()
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed loading GitHub app credentials: {error}"),
            )
        })?
    {
        let webhook_secret = state
            .github
            .decrypt(&record.webhook_secret_encrypted)
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed decrypting GitHub webhook secret: {error}"),
                )
            })?;
        let private_key_pem = state
            .github
            .decrypt(&record.private_key_pem_encrypted)
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed decrypting GitHub private key: {error}"),
                )
            })?;

        return Ok(ResolvedGitHubAppCredentials {
            app_id: record.app_id,
            app_slug: record.app_slug,
            webhook_secret,
            private_key_pem,
            app_name: Some(record.app_name),
            app_html_url: record.app_html_url,
        });
    }

    if !state.github.has_static_credentials() {
        return Err((
            StatusCode::FAILED_DEPENDENCY,
            "GitHub App credentials are not configured".to_string(),
        ));
    }

    let private_key_path = state.github.private_key_path.clone().ok_or((
        StatusCode::FAILED_DEPENDENCY,
        "GitHub private key path is not configured".to_string(),
    ))?;
    let private_key_pem = std::fs::read_to_string(&private_key_path).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unable to read GitHub private key: {error}"),
        )
    })?;

    Ok(ResolvedGitHubAppCredentials {
        app_id: state.github.app_id.clone().unwrap_or_default(),
        app_slug: state.github.app_slug.clone(),
        webhook_secret: state.github.webhook_secret.clone().unwrap_or_default(),
        private_key_pem,
        app_name: None,
        app_html_url: None,
    })
}

fn app_install_url_from_credentials(credentials: &ResolvedGitHubAppCredentials) -> Option<String> {
    credentials
        .app_html_url
        .as_deref()
        .map(|url| format!("{url}/installations/new"))
        .or_else(|| {
            credentials
                .app_slug
                .as_deref()
                .map(|slug| format!("https://github.com/apps/{slug}/installations/new"))
        })
}

pub(super) async fn github_manifest(
    State(state): State<OrchestratorState>,
    session: Session,
    Path(app_name): Path<String>,
) -> Result<Json<GitHubManifestResponse>, StatusCode> {
    current_user_id(&session).await?;

    let manifest = state
        .github
        .app_manifest_json(&app_name)
        .ok_or(StatusCode::FAILED_DEPENDENCY)?;

    Ok(Json(GitHubManifestResponse { manifest }))
}

pub(super) async fn github_manifest_callback(
    State(state): State<OrchestratorState>,
    Query(query): Query<GitHubManifestCallbackQuery>,
) -> Result<Redirect, (StatusCode, String)> {
    let url = format!(
        "https://api.github.com/app-manifests/{}/conversions",
        query.code
    );
    let response = reqwest::Client::new()
        .post(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "nanoscale-agent")
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("GitHub manifest exchange failed: {error}"),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("GitHub manifest exchange returned {status}: {body}"),
        ));
    }

    let credentials = response
        .json::<GitHubManifestConversionResponse>()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Invalid GitHub manifest response: {error}"),
            )
        })?;

    let client_secret_encrypted =
        state
            .github
            .encrypt(&credentials.client_secret)
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unable to encrypt GitHub client secret: {error}"),
                )
            })?;
    let webhook_secret_encrypted =
        state
            .github
            .encrypt(&credentials.webhook_secret)
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unable to encrypt GitHub webhook secret: {error}"),
                )
            })?;
    let private_key_pem_encrypted = state.github.encrypt(&credentials.pem).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unable to encrypt GitHub private key: {error}"),
        )
    })?;

    state
        .db
        .upsert_github_app_credentials(&NewGitHubAppCredentials {
            id: GITHUB_APP_CREDENTIALS_ID.to_string(),
            app_id: credentials.id.to_string(),
            app_slug: credentials.slug.clone(),
            client_id: credentials.client_id.clone(),
            client_secret_encrypted,
            webhook_secret_encrypted,
            private_key_pem_encrypted,
            app_name: credentials.name.clone(),
            app_html_url: credentials.html_url.clone(),
        })
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed saving GitHub app credentials: {error}"),
            )
        })?;

    let redirect_url = credentials
        .html_url
        .as_deref()
        .map(|url| format!("{url}/installations/new"))
        .or_else(|| {
            credentials
                .slug
                .as_deref()
                .map(|slug| format!("https://github.com/apps/{slug}/installations/new"))
        })
        .unwrap_or_else(|| "/settings".to_string());

    Ok(Redirect::to(&redirect_url))
}

pub(super) async fn github_status(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<GitHubStatusResponse>, StatusCode> {
    current_user_id(&session).await?;
    let credentials = match resolve_github_app_credentials(&state).await {
        Ok(credentials) => Some(credentials),
        Err((StatusCode::FAILED_DEPENDENCY, _)) => None,
        Err((_status, _message)) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    Ok(Json(GitHubStatusResponse {
        enabled: state.github.enabled,
        configured: credentials.is_some(),
        connected: credentials.is_some(),
        github_login: credentials
            .as_ref()
            .and_then(|item| item.app_slug.clone().or_else(|| item.app_name.clone())),
        app_install_url: credentials
            .as_ref()
            .and_then(app_install_url_from_credentials)
            .or_else(|| state.github.static_app_install_url()),
    }))
}

pub(super) async fn github_start(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<GitHubStartResponse>, (StatusCode, String)> {
    current_user_id(&session).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Authentication required".to_string(),
        )
    })?;

    if !state.github.enabled {
        return Err((
            StatusCode::FAILED_DEPENDENCY,
            "GitHub integration is disabled".to_string(),
        ));
    }

    if state
        .github
        .app_manifest_json("NanoScale-GitHub-App")
        .is_none()
    {
        return Err((
            StatusCode::FAILED_DEPENDENCY,
            "GitHub manifest flow is not available until public base URL and encryption key are configured"
                .to_string(),
        ));
    }

    let redirect_url = "/github/setup".to_string();

    Ok(Json(GitHubStartResponse { redirect_url }))
}

#[derive(Debug, Deserialize)]
struct InstallationsResponse {
    installations: Vec<InstallationItem>,
}

#[derive(Debug, Deserialize)]
struct InstallationItem {
    id: i64,
    target_id: i64,
    target_type: String,
    account: InstallationAccount,
}

#[derive(Debug, Deserialize)]
struct InstallationAccount {
    login: String,
    #[serde(rename = "type")]
    account_type: String,
}

pub(super) async fn github_disconnect(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<StatusCode, StatusCode> {
    current_user_id(&session).await?;
    state
        .db
        .clear_github_app_credentials()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn sync_installations_for_user(
    state: &OrchestratorState,
    user_id: &str,
) -> Result<(), (StatusCode, String)> {
    let credentials = resolve_github_app_credentials(state).await?;
    let app_token = app_jwt(&credentials)?;
    let installations = fetch_app_installations(&app_token).await?;

    let records = installations
        .into_iter()
        .map(|installation| NewGitHubInstallation {
            id: Uuid::new_v4().to_string(),
            local_user_id: user_id.to_string(),
            installation_id: installation.id,
            account_login: installation.account.login,
            account_type: installation.account.account_type,
            target_type: installation.target_type,
            target_id: installation.target_id,
        })
        .collect::<Vec<_>>();

    state
        .db
        .replace_github_installations_for_user(user_id, &records)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed replacing installations: {error}"),
            )
        })?;

    Ok(())
}

#[derive(Debug, Deserialize)]
pub(super) struct RepoQuery {
    installation_id: i64,
    query: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InstallationReposResponse {
    repositories: Vec<GitHubRepoApiItem>,
}

#[derive(Debug, Deserialize)]
struct GitHubRepoApiItem {
    id: i64,
    node_id: String,
    name: String,
    full_name: String,
    private: bool,
    html_url: String,
    clone_url: String,
    default_branch: String,
    archived: bool,
    disabled: bool,
    owner: GitHubRepoOwner,
}

#[derive(Debug, Deserialize)]
struct GitHubRepoOwner {
    login: String,
}

#[derive(Debug, Serialize)]
struct AppJwtClaims {
    iat: u64,
    exp: u64,
    iss: String,
}

#[derive(Debug, Deserialize)]
struct InstallationAccessTokenResponse {
    token: String,
}

#[derive(Debug, Deserialize)]
struct WebhookCreateResponse {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct GitHubWebhookPayload {
    repository: WebhookRepository,
    r#ref: Option<String>,
    after: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WebhookRepository {
    id: i64,
}

#[derive(Debug, Deserialize)]
pub(super) struct SyncReposRequest {
    installation_id: i64,
}

pub(super) async fn github_installations(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<Vec<GitHubInstallationItem>>, StatusCode> {
    let user_id = current_user_id(&session).await?;
    if let Err((status, _message)) = sync_installations_for_user(&state, &user_id).await {
        return Err(status);
    }
    let records = state
        .db
        .list_github_installations_for_user(&user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(
        records
            .into_iter()
            .map(|record| GitHubInstallationItem {
                installation_id: record.installation_id,
                account_login: record.account_login,
                account_type: record.account_type,
            })
            .collect(),
    ))
}

pub(super) async fn github_sync_repos(
    State(state): State<OrchestratorState>,
    session: Session,
    Json(payload): Json<SyncReposRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let user_id = current_user_id(&session).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Authentication required".to_string(),
        )
    })?;
    sync_repositories_for_installation(&state, &user_id, payload.installation_id).await?;
    Ok(StatusCode::ACCEPTED)
}

pub(super) async fn github_repos(
    State(state): State<OrchestratorState>,
    session: Session,
    Query(query): Query<RepoQuery>,
) -> Result<Json<Vec<GitHubRepositoryItem>>, (StatusCode, String)> {
    let user_id = current_user_id(&session).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Authentication required".to_string(),
        )
    })?;

    if state
        .db
        .list_github_repositories(query.installation_id, query.query.as_deref())
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed listing repositories: {error}"),
            )
        })?
        .is_empty()
    {
        sync_repositories_for_installation(&state, &user_id, query.installation_id).await?;
    }

    let repositories = state
        .db
        .list_github_repositories(query.installation_id, query.query.as_deref())
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed listing repositories: {error}"),
            )
        })?;

    Ok(Json(
        repositories
            .into_iter()
            .map(|item| GitHubRepositoryItem {
                installation_id: item.installation_id,
                repo_id: item.repo_id,
                owner_login: item.owner_login,
                name: item.name,
                full_name: item.full_name,
                default_branch: item.default_branch,
                is_private: item.is_private,
                clone_url: item.clone_url,
            })
            .collect(),
    ))
}

pub(super) async fn resolve_github_source(
    state: &OrchestratorState,
    user_id: &str,
    source: &GitHubProjectSourceRequest,
) -> Result<ResolvedGitHubSource, (StatusCode, String)> {
    let allowed = state
        .db
        .list_github_installations_for_user(user_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed listing installations: {error}"),
            )
        })?
        .into_iter()
        .any(|item| item.installation_id == source.installation_id);

    if !allowed {
        return Err((
            StatusCode::FORBIDDEN,
            "Requested installation is not available for current user".to_string(),
        ));
    }

    let repository = state
        .db
        .get_github_repository_by_id(source.repo_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed loading repository: {error}"),
            )
        })?
        .ok_or((
            StatusCode::NOT_FOUND,
            "GitHub repository not found in cache".to_string(),
        ))?;

    if repository.installation_id != source.installation_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "Repository does not belong to selected installation".to_string(),
        ));
    }

    Ok(ResolvedGitHubSource {
        installation_id: repository.installation_id,
        repo_id: repository.repo_id,
        repo_node_id: repository.node_id,
        owner_login: repository.owner_login,
        repo_name: repository.name,
        full_name: repository.full_name,
        default_branch: repository.default_branch,
        selected_branch: source.selected_branch.clone(),
        clone_url: repository.clone_url,
    })
}

pub(super) async fn ensure_project_webhook(
    state: &OrchestratorState,
    project_id: &str,
    source: &ResolvedGitHubSource,
) -> Result<(), (StatusCode, String)> {
    let webhook_secret = format!("{}:{}", Uuid::new_v4(), project_id);
    let encrypted_secret = state.github.encrypt(&webhook_secret).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("failed to encrypt webhook secret: {error}"),
        )
    })?;

    let installation_token = installation_access_token(state, source.installation_id).await?;
    let webhook_url = state.github.webhook_url().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Webhook URL is not configured".to_string(),
    ))?;

    let hook_payload = serde_json::json!({
        "name": "web",
        "active": true,
        "events": ["push"],
        "config": {
            "url": webhook_url,
            "content_type": "json",
            "secret": webhook_secret,
            "insecure_ssl": "0"
        }
    });

    let response = reqwest::Client::new()
        .post(format!(
            "https://api.github.com/repos/{}/hooks",
            source.full_name
        ))
        .header("Authorization", format!("Bearer {installation_token}"))
        .header("User-Agent", "nanoscale-agent")
        .header("Accept", "application/vnd.github+json")
        .json(&hook_payload)
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("webhook create failed: {error}"),
            )
        })?;

    let webhook_id = if response.status().is_success() {
        response
            .json::<WebhookCreateResponse>()
            .await
            .map_err(|error| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("invalid webhook response: {error}"),
                )
            })?
            .id
    } else {
        0
    };

    state
        .db
        .upsert_project_github_link(&NewProjectGitHubLink {
            id: Uuid::new_v4().to_string(),
            project_id: project_id.to_string(),
            installation_id: source.installation_id,
            repo_id: source.repo_id,
            repo_node_id: source.repo_node_id.clone(),
            owner_login: source.owner_login.clone(),
            repo_name: source.repo_name.clone(),
            full_name: source.full_name.clone(),
            default_branch: source.default_branch.clone(),
            selected_branch: source.selected_branch.clone(),
            webhook_id: (webhook_id > 0).then_some(webhook_id),
            webhook_secret_encrypted: encrypted_secret,
            active: true,
        })
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to save project github link: {error}"),
            )
        })?;

    Ok(())
}

pub(super) async fn authenticated_clone_url(
    state: &OrchestratorState,
    source: &ResolvedGitHubSource,
) -> Result<String, (StatusCode, String)> {
    let token = installation_access_token(state, source.installation_id).await?;
    let encoded_token = urlencoding::encode(&token);
    Ok(source.clone_url.replacen(
        "https://",
        &format!("https://x-access-token:{encoded_token}@"),
        1,
    ))
}

#[allow(clippy::too_many_lines)]
pub(super) async fn github_webhook(
    State(state): State<OrchestratorState>,
    headers: HeaderMap,
    body: String,
) -> (StatusCode, String) {
    let delivery_id = headers
        .get("X-GitHub-Delivery")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let event_type = headers
        .get("X-GitHub-Event")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();

    if delivery_id.is_empty() || event_type.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "Missing GitHub delivery headers".to_string(),
        );
    }

    let webhook_secret = match resolve_github_app_credentials(&state).await {
        Ok(credentials) => credentials.webhook_secret,
        Err((_status, _message)) => {
            return (
                StatusCode::FAILED_DEPENDENCY,
                "GitHub App credentials are not configured".to_string(),
            );
        }
    };

    if !verify_webhook_signature(
        headers
            .get("X-Hub-Signature-256")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default(),
        &webhook_secret,
        body.as_bytes(),
    ) {
        return (
            StatusCode::UNAUTHORIZED,
            "Invalid webhook signature".to_string(),
        );
    }

    let payload = match serde_json::from_str::<GitHubWebhookPayload>(&body) {
        Ok(parsed) => parsed,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid webhook payload: {error}"),
            )
        }
    };

    let ref_name = payload.r#ref.unwrap_or_default();
    let branch = ref_name.strip_prefix("refs/heads/").unwrap_or(&ref_name);

    let inserted = state
        .db
        .mark_github_webhook_delivery(&NewGitHubWebhookDelivery {
            id: Uuid::new_v4().to_string(),
            delivery_id: delivery_id.clone(),
            event_type: event_type.clone(),
            repo_id: Some(payload.repository.id),
            r#ref: Some(ref_name.clone()),
            head_commit: payload.after.clone(),
            handled: false,
            status_code: None,
            error_message: None,
        })
        .await;

    if inserted.as_ref().is_err() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to persist delivery".to_string(),
        );
    }

    if !inserted.ok().unwrap_or(false) {
        return (StatusCode::OK, "Duplicate delivery ignored".to_string());
    }

    let linked_projects = match state
        .db
        .list_active_project_links_for_repo_branch(payload.repository.id, branch)
        .await
    {
        Ok(records) => records,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed loading project links: {error}"),
            );
        }
    };

    let mut deployed_projects = HashSet::new();
    for link in linked_projects {
        if !deployed_projects.insert(link.project_id.clone()) {
            continue;
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        let mut debounce = state.redeploy_debounce.lock().await;
        if let Some(last_trigger_unix) = debounce.get(&link.project_id).copied() {
            if now.saturating_sub(last_trigger_unix) < 15 {
                continue;
            }
        }
        debounce.insert(link.project_id.clone(), now);
        drop(debounce);

        if redeploy_project_by_id(&state, &link.project_id)
            .await
            .is_err()
        {
            let _ = state
                .db
                .complete_github_webhook_delivery(&delivery_id, 502, Some("redeploy failed"))
                .await;
            return (StatusCode::BAD_GATEWAY, "Redeploy failed".to_string());
        }
    }

    let _ = state
        .db
        .complete_github_webhook_delivery(&delivery_id, 202, None)
        .await;

    (StatusCode::ACCEPTED, "Webhook processed".to_string())
}

fn verify_webhook_signature(signature_header: &str, secret: &str, body: &[u8]) -> bool {
    if !signature_header.starts_with("sha256=") || secret.is_empty() {
        return false;
    }

    let provided = signature_header.trim_start_matches("sha256=");
    let mac = <hmac::Hmac<sha2::Sha256> as Mac>::new_from_slice(secret.as_bytes());
    if mac.is_err() {
        return false;
    }
    let mut mac = mac.expect("validated above");
    mac.update(body);
    let expected = hex::encode(mac.finalize().into_bytes());

    subtle_compare(expected.as_bytes(), provided.as_bytes())
}

fn subtle_compare(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut mismatch = 0_u8;
    for (left_value, right_value) in left.iter().zip(right.iter()) {
        mismatch |= left_value ^ right_value;
    }
    mismatch == 0
}

async fn sync_repositories_for_installation(
    state: &OrchestratorState,
    user_id: &str,
    installation_id: i64,
) -> Result<(), (StatusCode, String)> {
    let allowed = state
        .db
        .list_github_installations_for_user(user_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to list installations: {error}"),
            )
        })?
        .into_iter()
        .any(|item| item.installation_id == installation_id);

    if !allowed {
        return Err((
            StatusCode::FORBIDDEN,
            "Installation not found for user".to_string(),
        ));
    }

    let installation_token = installation_access_token(state, installation_id).await?;
    let repositories = fetch_installation_repositories(&installation_token).await?;

    let records = repositories
        .into_iter()
        .map(|repository| NewGitHubRepository {
            id: Uuid::new_v4().to_string(),
            installation_id,
            repo_id: repository.id,
            node_id: repository.node_id,
            owner_login: repository.owner.login,
            name: repository.name,
            full_name: repository.full_name,
            default_branch: repository.default_branch,
            is_private: repository.private,
            html_url: repository.html_url,
            clone_url: repository.clone_url,
            archived: repository.archived,
            disabled: repository.disabled,
        })
        .collect::<Vec<_>>();

    state
        .db
        .replace_github_repositories(installation_id, &records)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed saving repositories: {error}"),
            )
        })?;

    Ok(())
}

async fn fetch_app_installations(
    app_jwt: &str,
) -> Result<Vec<InstallationItem>, (StatusCode, String)> {
    let client = reqwest::Client::new();
    let mut page = 1_usize;
    let mut installations = Vec::new();

    loop {
        let response = client
            .get("https://api.github.com/app/installations")
            .query(&[("per_page", GITHUB_PAGE_SIZE), ("page", page)])
            .header("Authorization", format!("Bearer {app_jwt}"))
            .header("User-Agent", "nanoscale-agent")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|error| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("GitHub app installation fetch failed: {error}"),
                )
            })?
            .json::<InstallationsResponse>()
            .await
            .map_err(|error| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("Invalid installations response: {error}"),
                )
            })?;

        let batch_len = response.installations.len();
        installations.extend(response.installations);

        if batch_len < GITHUB_PAGE_SIZE {
            break;
        }
        page = page.saturating_add(1);
    }

    Ok(installations)
}

async fn fetch_installation_repositories(
    installation_token: &str,
) -> Result<Vec<GitHubRepoApiItem>, (StatusCode, String)> {
    let client = reqwest::Client::new();
    let mut page = 1_usize;
    let mut repositories = Vec::new();

    loop {
        let response = client
            .get("https://api.github.com/installation/repositories")
            .query(&[("per_page", GITHUB_PAGE_SIZE), ("page", page)])
            .header("Authorization", format!("Bearer {installation_token}"))
            .header("User-Agent", "nanoscale-agent")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|error| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("failed loading installation repositories: {error}"),
                )
            })?
            .json::<InstallationReposResponse>()
            .await
            .map_err(|error| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!("invalid repository response: {error}"),
                )
            })?;

        let batch_len = response.repositories.len();
        repositories.extend(response.repositories);

        if batch_len < GITHUB_PAGE_SIZE {
            break;
        }
        page = page.saturating_add(1);
    }

    Ok(repositories)
}

async fn installation_access_token(
    state: &OrchestratorState,
    installation_id: i64,
) -> Result<String, (StatusCode, String)> {
    let credentials = resolve_github_app_credentials(state).await?;
    let app_jwt = app_jwt(&credentials)?;
    let response = reqwest::Client::new()
        .post(format!(
            "https://api.github.com/app/installations/{installation_id}/access_tokens"
        ))
        .header("Authorization", format!("Bearer {app_jwt}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "nanoscale-agent")
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("failed generating installation token: {error}"),
            )
        })?
        .json::<InstallationAccessTokenResponse>()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("invalid installation token response: {error}"),
            )
        })?;

    Ok(response.token)
}

fn app_jwt(credentials: &ResolvedGitHubAppCredentials) -> Result<String, (StatusCode, String)> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("system clock error: {error}"),
            )
        })?
        .as_secs();

    let claims = AppJwtClaims {
        iat: now.saturating_sub(30),
        exp: now.saturating_add(9 * 60),
        iss: credentials.app_id.clone(),
    };

    let encoding_key =
        EncodingKey::from_rsa_pem(credentials.private_key_pem.as_bytes()).map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to parse GitHub private key: {error}"),
            )
        })?;

    jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &encoding_key).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unable to sign app jwt: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subtle_compare_requires_equal_inputs() {
        assert!(subtle_compare(b"abc", b"abc"));
        assert!(!subtle_compare(b"abc", b"abd"));
        assert!(!subtle_compare(b"abc", b"ab"));
    }

    #[test]
    fn verify_webhook_signature_checks_sha256_header() {
        let secret = "super-secret";
        let body = b"hello";
        let mut mac = <hmac::Hmac<sha2::Sha256> as Mac>::new_from_slice(secret.as_bytes())
            .expect("hmac init");
        mac.update(body);
        let signature = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));

        assert!(verify_webhook_signature(&signature, secret, body));
        assert!(!verify_webhook_signature("sha256=deadbeef", secret, body));
        assert!(!verify_webhook_signature(&signature, "", body));
    }
}
