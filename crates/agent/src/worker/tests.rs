use super::*;

use axum::http::StatusCode;
use axum::{body::to_bytes, http::header, http::Request, routing::delete, routing::post, Router};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

#[test]
fn generate_secret_key_is_nonempty_and_expected_length() {
    let secret = generate_secret_key();
    assert_eq!(secret.len(), 64);
    assert!(secret.chars().all(|ch| ch.is_ascii_alphanumeric()));
}

#[tokio::test]
async fn internal_deploy_is_placeholder_response() {
    let (status, body) = handlers::internal_deploy().await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(body.0.status, "accepted");
    assert!(body.0.message.contains("placeholder"));
}

#[tokio::test]
async fn internal_health_returns_sane_numbers() {
    let body = handlers::internal_health().await;
    assert!(body.0.cpu_usage_percent >= 0.0);
    assert!(body.0.total_memory_bytes >= body.0.used_memory_bytes);
}

#[test]
fn worker_create_project_request_deserializes() {
    let json = r#"{
  "project_id": "p1",
  "repo_url": "https://example.com/repo.git",
  "branch": "main",
  "build_command": "bun run build",
  "install_command": "bun install",
  "run_command": "bun run start",
  "output_directory": "",
  "port": 3100,
  "domain": null,
  "tls_email": null,
  "env_vars": [{"key": "A", "value": "B"}]
}"#;

    let decoded =
        serde_json::from_str::<api_types::WorkerCreateProjectRequest>(json).expect("deserialize");
    assert_eq!(decoded.project_id, "p1");
    assert_eq!(decoded.env_vars.len(), 1);
    assert_eq!(decoded.env_vars[0].key, "A");
}

#[tokio::test]
async fn worker_router_health_endpoint_returns_json() {
    let state = api_types::WorkerState {
        monitored_projects: Arc::new(RwLock::new(Vec::new())),
    };

    let app = Router::new()
        .route("/internal/health", post(handlers::internal_health))
        .route("/internal/deploy", post(handlers::internal_deploy))
        .route("/internal/projects", post(handlers::internal_projects))
        .route(
            "/internal/projects/:id",
            delete(handlers::internal_delete_project),
        )
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/health")
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    let json = serde_json::from_slice::<serde_json::Value>(&body).expect("json");
    assert!(json.get("cpu_usage_percent").is_some());
    assert!(json.get("total_memory_bytes").is_some());
}

#[tokio::test]
async fn worker_router_deploy_endpoint_returns_placeholder() {
    let state = api_types::WorkerState {
        monitored_projects: Arc::new(RwLock::new(Vec::new())),
    };

    let app = Router::new()
        .route("/internal/health", post(handlers::internal_health))
        .route("/internal/deploy", post(handlers::internal_deploy))
        .with_state(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/deploy")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let body = to_bytes(response.into_body(), 64 * 1024)
        .await
        .expect("body");
    let json = serde_json::from_slice::<serde_json::Value>(&body).expect("json");
    assert_eq!(
        json.get("status").and_then(|v| v.as_str()),
        Some("accepted")
    );
}
