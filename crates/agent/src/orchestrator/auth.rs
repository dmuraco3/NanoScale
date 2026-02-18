use anyhow::Result;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use tower_sessions::Session;
use uuid::Uuid;

use crate::db::NewUser;

use super::api_types::{AuthStatusResponse, LoginRequest, SetupRequest};
use super::OrchestratorState;

const SESSION_USER_ID_KEY: &str = "user_id";

pub(super) async fn auth_setup(
    State(state): State<OrchestratorState>,
    session: Session,
    Json(payload): Json<SetupRequest>,
) -> Result<StatusCode, StatusCode> {
    if state
        .db
        .users_count()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        > 0
    {
        return Err(StatusCode::CONFLICT);
    }

    if payload.username.trim().is_empty() || payload.password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let user_id = Uuid::new_v4().to_string();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    let new_user = NewUser {
        id: user_id.clone(),
        username: payload.username,
        password_hash,
    };

    state
        .db
        .insert_user(&new_user)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    session
        .insert(SESSION_USER_ID_KEY, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::CREATED)
}

pub(super) async fn auth_login(
    State(state): State<OrchestratorState>,
    session: Session,
    Json(payload): Json<LoginRequest>,
) -> Result<StatusCode, StatusCode> {
    let user = state
        .db
        .find_user_by_username(&payload.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let parsed_hash =
        PasswordHash::new(&user.password_hash).map_err(|_| StatusCode::UNAUTHORIZED)?;

    Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    session
        .insert(SESSION_USER_ID_KEY, user.id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

pub(super) async fn auth_status(
    State(state): State<OrchestratorState>,
    session: Session,
) -> Result<Json<AuthStatusResponse>, StatusCode> {
    let users_count = state
        .db
        .users_count()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let authenticated = session
        .get::<String>(SESSION_USER_ID_KEY)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    Ok(Json(AuthStatusResponse {
        users_count,
        authenticated,
    }))
}

pub(super) async fn auth_session(session: Session) -> Result<StatusCode, StatusCode> {
    require_authenticated(&session).await?;
    Ok(StatusCode::OK)
}

pub(super) async fn require_authenticated(session: &Session) -> Result<(), StatusCode> {
    let authenticated = session
        .get::<String>(SESSION_USER_ID_KEY)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .is_some();

    if authenticated {
        return Ok(());
    }

    Err(StatusCode::UNAUTHORIZED)
}
