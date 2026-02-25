use std::collections::HashSet;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{Context, Result};
use axum::extract::{Query, State};
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
    NewGitHubInstallation, NewGitHubRepository, NewGitHubUserLink, NewGitHubWebhookDelivery,
    NewProjectGitHubLink,
};

use super::api_types::{
    GitHubInstallationItem, GitHubProjectSourceRequest, GitHubRepositoryItem, GitHubStartResponse,
    GitHubStatusResponse,
};
use super::auth::current_user_id;
use super::projects::redeploy_project_by_id;
use super::OrchestratorState;

const OAUTH_STATE_TTL_SECONDS: u64 = 15 * 60;

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

    pub(super) fn is_configured(&self) -> bool {
        self.client_id.is_some()
            && self.client_secret.is_some()
            && self.app_id.is_some()
            && self.app_slug.is_some()
            && self.private_key_path.is_some()
            && self.webhook_secret.is_some()
            && self.public_base_url.is_some()
            && self.cipher.is_some()
    }

    fn callback_url(&self) -> Option<String> {
        self.public_base_url
            .as_deref()
            .map(|base| format!("{base}/api/integrations/github/callback"))
    }

    fn webhook_url(&self) -> Option<String> {
        self.public_base_url
            .as_deref()
            .map(|base| format!("{base}/api/integrations/github/webhook"))
    }

    fn app_install_url(&self) -> Option<String> {
        self.app_slug
            .as_deref()
            .map(|slug| format!("https://github.com/apps/{slug}/installations/new"))
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

    fn oauth_state_secret(&self) -> Option<&str> {
        self.client_secret
            .as_deref()
            .or(self.webhook_secret.as_deref())
    }

    fn build_oauth_state(&self, user_id: &str) -> Result<String, (StatusCode, String)> {
        let issued_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("system clock error: {error}"),
                )
            })?
            .as_secs();
        let nonce = Uuid::new_v4().to_string();
        let payload = format!("{user_id}:{nonce}:{issued_at}");
        let secret = self.oauth_state_secret().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "GitHub OAuth secret is not configured".to_string(),
        ))?;

        let mut mac = <hmac::Hmac<sha2::Sha256> as Mac>::new_from_slice(secret.as_bytes())
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("unable to initialize oauth signer: {error}"),
                )
            })?;
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        Ok(format!("{payload}.{signature}"))
    }

    fn verify_oauth_state(&self, state: &str) -> Result<String, (StatusCode, String)> {
        let (payload, provided_signature) = state
            .rsplit_once('.')
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid OAuth state".to_string()))?;

        let secret = self.oauth_state_secret().ok_or((
            StatusCode::INTERNAL_SERVER_ERROR,
            "GitHub OAuth secret is not configured".to_string(),
        ))?;

        let mut mac = <hmac::Hmac<sha2::Sha256> as Mac>::new_from_slice(secret.as_bytes())
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("unable to initialize oauth verifier: {error}"),
                )
            })?;
        mac.update(payload.as_bytes());
        let expected_signature = hex::encode(mac.finalize().into_bytes());
        if !subtle_compare(expected_signature.as_bytes(), provided_signature.as_bytes()) {
            return Err((StatusCode::UNAUTHORIZED, "Invalid OAuth state".to_string()));
        }

        let mut parts = payload.split(':');
        let user_id = parts
            .next()
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid OAuth state".to_string()))?;
        let _nonce = parts
            .next()
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid OAuth state".to_string()))?;
        let issued_at_raw = parts
            .next()
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid OAuth state".to_string()))?;
        if parts.next().is_some() {
            return Err((StatusCode::UNAUTHORIZED, "Invalid OAuth state".to_string()));
        }

        let issued_at = issued_at_raw
            .parse::<u64>()
            .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid OAuth state".to_string()))?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("system clock error: {error}"),
                )
            })?
            .as_secs();
        if now.saturating_sub(issued_at) > OAUTH_STATE_TTL_SECONDS {
            return Err((StatusCode::UNAUTHORIZED, "OAuth state expired".to_string()));
        }

        Ok(user_id.to_string())
    }
}

pub(super) async fn github_status(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<GitHubStatusResponse>, StatusCode> {
    let user_id = current_user_id(&session).await?;
    let link = state
        .db
        .get_github_user_link_by_local_user(&user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(GitHubStatusResponse {
        enabled: state.github.enabled,
        configured: state.github.is_configured(),
        connected: link.is_some(),
        github_login: link.map(|item| item.github_login),
        app_install_url: state.github.app_install_url(),
    }))
}

pub(super) async fn github_start(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<GitHubStartResponse>, (StatusCode, String)> {
    let user_id = current_user_id(&session).await.map_err(|_| {
        (
            StatusCode::UNAUTHORIZED,
            "Authentication required".to_string(),
        )
    })?;

    if !state.github.enabled || !state.github.is_configured() {
        return Err((
            StatusCode::FAILED_DEPENDENCY,
            "GitHub integration is not configured".to_string(),
        ));
    }

    let state_token = state.github.build_oauth_state(&user_id)?;

    let callback = state.github.callback_url().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Missing callback URL".to_string(),
    ))?;

    let redirect_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={}",
        state.github.client_id.clone().unwrap_or_default(),
        urlencoding::encode(&callback),
        state_token
    );

    Ok(Json(GitHubStartResponse { redirect_url }))
}

#[derive(Debug, Deserialize)]
pub(super) struct GitHubCallbackQuery {
    code: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct OAuthTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct GitHubUserResponse {
    id: i64,
    login: String,
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

#[allow(clippy::too_many_lines)]
pub(super) async fn github_callback(
    State(state): State<OrchestratorState>,
    _session: Session,
    Query(query): Query<GitHubCallbackQuery>,
) -> Result<Redirect, (StatusCode, String)> {
    let user_id = state.github.verify_oauth_state(&query.state)?;

    let callback_url = state.github.callback_url().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "Missing callback URL".to_string(),
    ))?;

    let token_payload = serde_json::json!({
        "client_id": state.github.client_id.clone().unwrap_or_default(),
        "client_secret": state.github.client_secret.clone().unwrap_or_default(),
        "code": query.code,
        "redirect_uri": callback_url,
        "state": query.state,
    });

    let token_response = reqwest::Client::new()
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&token_payload)
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("GitHub OAuth exchange failed: {error}"),
            )
        })?
        .json::<OAuthTokenResponse>()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Invalid token response: {error}"),
            )
        })?;

    let github_user = reqwest::Client::new()
        .get("https://api.github.com/user")
        .header(
            "Authorization",
            format!("Bearer {}", token_response.access_token),
        )
        .header("User-Agent", "nanoscale-agent")
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("GitHub user lookup failed: {error}"),
            )
        })?
        .json::<GitHubUserResponse>()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("Invalid user response: {error}"),
            )
        })?;

    let encrypted_token = state
        .github
        .encrypt(&token_response.access_token)
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Unable to encrypt token: {error}"),
            )
        })?;

    state
        .db
        .upsert_github_user_link(&NewGitHubUserLink {
            id: Uuid::new_v4().to_string(),
            local_user_id: user_id.clone(),
            github_user_id: github_user.id,
            github_login: github_user.login,
            access_token_encrypted: encrypted_token,
            refresh_token_encrypted: None,
            token_expires_at: None,
        })
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to save user link: {error}"),
            )
        })?;

    sync_installations_for_user(&state, &user_id).await?;

    let installation_count = state
        .db
        .list_github_installations_for_user(&user_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed loading installations after oauth: {error}"),
            )
        })?
        .len();

    if installation_count == 0 {
        if let Some(install_url) = state.github.app_install_url() {
            return Ok(Redirect::to(&install_url));
        }
    }

    Ok(Redirect::to("/projects/new"))
}

pub(super) async fn github_disconnect(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<StatusCode, StatusCode> {
    let user_id = current_user_id(&session).await?;
    state
        .db
        .clear_github_user_link(&user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn sync_installations_for_user(
    state: &OrchestratorState,
    user_id: &str,
) -> Result<(), (StatusCode, String)> {
    let link = state
        .db
        .get_github_user_link_by_local_user(user_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed reading github link: {error}"),
            )
        })?
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "GitHub account not connected".to_string(),
        ))?;

    let token = state
        .github
        .decrypt(&link.access_token_encrypted)
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed decrypting token: {error}"),
            )
        })?;

    let response = reqwest::Client::new()
        .get("https://api.github.com/user/installations")
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "nanoscale-agent")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| {
            (
                StatusCode::BAD_GATEWAY,
                format!("GitHub installation fetch failed: {error}"),
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

    for installation in response.installations {
        state
            .db
            .upsert_github_installation(&NewGitHubInstallation {
                id: Uuid::new_v4().to_string(),
                local_user_id: user_id.to_string(),
                installation_id: installation.id,
                account_login: installation.account.login,
                account_type: installation.account.account_type,
                target_type: installation.target_type,
                target_id: installation.target_id,
            })
            .await
            .map_err(|error| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed saving installation: {error}"),
                )
            })?;
    }

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

    let installation_token =
        installation_access_token(&state.github, source.installation_id).await?;
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
    let token = installation_access_token(&state.github, source.installation_id).await?;
    let encoded_token = urlencoding::encode(&token);
    Ok(source.clone_url.replacen(
        "https://",
        &format!("https://x-access-token:{encoded_token}@"),
        1,
    ))
}

pub(super) async fn deactivate_project_webhook(
    state: &OrchestratorState,
    project_id: &str,
) -> Result<(), (StatusCode, String)> {
    let Some(link) = state
        .db
        .get_project_github_link_by_project_id(project_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed loading project github link: {error}"),
            )
        })?
    else {
        return Ok(());
    };

    if let Some(webhook_id) = link.webhook_id {
        let installation_token =
            installation_access_token(&state.github, link.installation_id).await?;
        let _ = reqwest::Client::new()
            .delete(format!(
                "https://api.github.com/repos/{}/hooks/{webhook_id}",
                link.full_name
            ))
            .header("Authorization", format!("Bearer {installation_token}"))
            .header("User-Agent", "nanoscale-agent")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await;
    }

    state
        .db
        .deactivate_project_github_link(project_id)
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed deactivating project github link: {error}"),
            )
        })?;

    Ok(())
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

    if !verify_webhook_signature(
        headers
            .get("X-Hub-Signature-256")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default(),
        state.github.webhook_secret.as_deref().unwrap_or_default(),
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

    let installation_token = installation_access_token(&state.github, installation_id).await?;
    let repos = reqwest::Client::new()
        .get("https://api.github.com/installation/repositories")
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

    let records = repos
        .repositories
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

async fn installation_access_token(
    service: &GitHubService,
    installation_id: i64,
) -> Result<String, (StatusCode, String)> {
    let app_jwt = app_jwt(service)?;
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

fn app_jwt(service: &GitHubService) -> Result<String, (StatusCode, String)> {
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
        iss: service.app_id.clone().unwrap_or_default(),
    };

    let private_key_path = service.private_key_path.clone().ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "GitHub private key path is not configured".to_string(),
    ))?;
    let private_key_bytes = std::fs::read(private_key_path).map_err(|error| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unable to read GitHub private key: {error}"),
        )
    })?;

    let encoding_key = EncodingKey::from_rsa_pem(&private_key_bytes).map_err(|error| {
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
