use std::time::{SystemTime, UNIX_EPOCH};

use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::orchestrator::OrchestratorState;

const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_TIMESTAMP_AGE_SECONDS: i64 = 30;

type HmacSha256 = Hmac<Sha256>;

pub async fn verify_cluster_signature(
    State(state): State<OrchestratorState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let signature = header_value(&request, "X-Cluster-Signature")?;
    let timestamp = header_value(&request, "X-Cluster-Timestamp")?;
    let server_id = header_value(&request, "X-Server-Id")?;

    validate_timestamp(&timestamp)?;

    let (parts, body) = request.into_parts();
    let body_bytes = to_bytes(body, MAX_BODY_BYTES)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let secret = state
        .db
        .get_server_secret(&server_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let signature_bytes = hex::decode(signature).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    mac.update(&body_bytes);
    mac.update(timestamp.as_bytes());
    mac.verify_slice(&signature_bytes)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let request = Request::from_parts(parts, Body::from(body_bytes));
    Ok(next.run(request).await)
}

fn header_value(request: &Request, header_name: &str) -> Result<String, StatusCode> {
    let value = request
        .headers()
        .get(header_name)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let parsed = value.to_str().map_err(|_| StatusCode::UNAUTHORIZED)?;
    Ok(parsed.to_string())
}

fn validate_timestamp(timestamp: &str) -> Result<(), StatusCode> {
    let timestamp_seconds = timestamp
        .parse::<i64>()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let now_seconds_u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .as_secs();

    let now_seconds = i64::try_from(now_seconds_u64).map_err(|_| StatusCode::UNAUTHORIZED)?;

    if now_seconds - timestamp_seconds > MAX_TIMESTAMP_AGE_SECONDS {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(())
}
