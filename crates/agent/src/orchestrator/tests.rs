use super::*;

use axum::extract::State;
use axum::http::header;
use axum::http::Request;
use axum::http::StatusCode;
use axum::Json;
use axum::Router;
use axum::{body::to_bytes, middleware, routing::post};
use hmac::Mac;
use tower::ServiceExt;
use tower_sessions::SessionManagerLayer;
use tower_sessions_sqlx_store::SqliteStore;

use crate::cluster::protocol::JoinClusterRequest;
use crate::db::DbClient;

async fn temp_db() -> DbClient {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let db_path = tempdir.path().join("nanoscale.db");
    std::mem::forget(tempdir);
    DbClient::initialize(&db_path.to_string_lossy())
        .await
        .expect("db init")
}

fn new_state(db: DbClient) -> OrchestratorState {
    OrchestratorState {
        db,
        token_store: Arc::new(TokenStore::new()),
        monitored_projects: Arc::new(RwLock::new(Vec::new())),
        local_server_id: "orchestrator-test".to_string(),
        base_domain: None,
        tls_email: None,
        stats_cache: Arc::new(RwLock::new(stats_cache::StatsCache::default())),
    }
}

async fn test_app(state: OrchestratorState) -> Router {
    let session_store = SqliteStore::new(state.db.pool());
    session_store.migrate().await.expect("session migrate");
    let session_layer = SessionManagerLayer::new(session_store).with_secure(false);

    let internal_router = Router::new()
        .route("/verify-signature", post(cluster::verify_signature_guarded))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            crate::cluster::signature::verify_cluster_signature,
        ));

    Router::new()
        .route("/api/auth/setup", post(auth::auth_setup))
        .route("/api/auth/login", post(auth::auth_login))
        .route("/api/auth/status", post(auth::auth_status))
        .route("/api/auth/session", post(auth::auth_session))
        .route(
            "/api/cluster/generate-token",
            post(cluster::generate_cluster_token),
        )
        .route("/api/cluster/join", post(cluster::join_cluster))
        .nest("/internal", internal_router)
        .layer(session_layer)
        .with_state(state)
}

fn cookie_from_set_cookie(set_cookie: &header::HeaderValue) -> String {
    let raw = set_cookie.to_str().expect("set-cookie utf8");
    raw.split(';').next().expect("cookie pair").to_string()
}

#[test]
fn normalize_base_domain_value_accepts_bare_domains() {
    assert_eq!(
        normalize_base_domain_value(" Example.COM. ").expect("normalized"),
        "example.com"
    );
    assert!(normalize_base_domain_value(" ").is_err());
    assert!(normalize_base_domain_value("example.com/evil").is_err());
    assert!(normalize_base_domain_value("example.com:443").is_err());
    assert!(normalize_base_domain_value("example..com").is_err());
}

#[test]
fn generate_secret_key_is_nonempty_and_expected_length() {
    let secret = generate_secret_key();
    assert_eq!(secret.len(), 64);
    assert!(secret.chars().all(|ch| ch.is_ascii_alphanumeric()));
}

#[tokio::test]
async fn join_cluster_rejects_invalid_token() {
    let db = temp_db().await;
    let state = new_state(db);

    let payload = JoinClusterRequest {
        token: "not-valid".to_string(),
        ip: "127.0.0.1".to_string(),
        secret_key: "secret".to_string(),
        name: "worker".to_string(),
    };

    let result = cluster::join_cluster(State(state), Json(payload)).await;
    assert!(matches!(result, Err(StatusCode::UNAUTHORIZED)));
}

#[tokio::test]
async fn join_cluster_inserts_server_and_returns_id() {
    let db = temp_db().await;
    let state = new_state(db);

    let token = state.token_store.generate_token().await;
    let payload = JoinClusterRequest {
        token,
        ip: "10.0.0.2".to_string(),
        secret_key: "server-secret".to_string(),
        name: "worker-1".to_string(),
    };

    let response = cluster::join_cluster(State(state.clone()), Json(payload))
        .await
        .expect("join should succeed")
        .0;

    assert!(!response.server_id.trim().is_empty());
    let stored = state
        .db
        .get_server_secret(&response.server_id)
        .await
        .expect("db lookup");
    assert_eq!(stored.as_deref(), Some("server-secret"));
}

#[tokio::test]
async fn auth_setup_sets_cookie_and_enables_session_endpoint() {
    let db = temp_db().await;
    let state = new_state(db);
    let app = test_app(state).await;

    let setup_body = serde_json::json!({
        "username": "admin",
        "password": "password123"
    })
    .to_string();

    let setup_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/setup")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(setup_body))
                .expect("request"),
        )
        .await
        .expect("setup response");

    assert_eq!(setup_response.status(), StatusCode::CREATED);
    let set_cookie = setup_response
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie");
    let cookie = cookie_from_set_cookie(set_cookie);

    let session_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/session")
                .header(header::COOKIE, cookie)
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("session response");

    assert_eq!(session_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn generate_cluster_token_requires_auth_and_returns_json_when_authed() {
    let db = temp_db().await;
    let state = new_state(db);
    let app = test_app(state).await;

    let unauthorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/cluster/generate-token")
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let setup_body = serde_json::json!({
        "username": "admin",
        "password": "password123"
    })
    .to_string();
    let setup_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/setup")
                .header(header::CONTENT_TYPE, "application/json")
                .body(axum::body::Body::from(setup_body))
                .expect("request"),
        )
        .await
        .expect("setup");
    let cookie = cookie_from_set_cookie(
        setup_response
            .headers()
            .get(header::SET_COOKIE)
            .expect("set-cookie"),
    );

    let authorized = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/cluster/generate-token")
                .header(header::COOKIE, cookie)
                .body(axum::body::Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(authorized.status(), StatusCode::OK);

    let body = to_bytes(authorized.into_body(), 64 * 1024)
        .await
        .expect("read body");
    let json = serde_json::from_slice::<serde_json::Value>(&body).expect("json");
    assert!(json.get("token").and_then(|v| v.as_str()).is_some());
    assert!(json
        .get("expires_in_seconds")
        .and_then(serde_json::Value::as_u64)
        .is_some());
}

#[tokio::test]
async fn internal_verify_signature_middleware_accepts_valid_signature() {
    let db = temp_db().await;
    db.insert_server(&crate::db::NewServer {
        id: "srv-1".to_string(),
        name: "server".to_string(),
        ip_address: "127.0.0.1".to_string(),
        status: "online".to_string(),
        secret_key: "super-secret".to_string(),
    })
    .await
    .expect("insert server");

    let state = new_state(db);
    let app = test_app(state).await;

    let body_bytes = b"hello".to_vec();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_secs()
        .to_string();

    let signature = {
        let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(b"super-secret").expect("hmac");
        mac.update(&body_bytes);
        mac.update(timestamp.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    };

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/verify-signature")
                .header("X-Cluster-Signature", signature)
                .header("X-Cluster-Timestamp", &timestamp)
                .header("X-Server-Id", "srv-1")
                .body(axum::body::Body::from(body_bytes))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn internal_verify_signature_middleware_rejects_invalid_signature() {
    let db = temp_db().await;
    db.insert_server(&crate::db::NewServer {
        id: "srv-1".to_string(),
        name: "server".to_string(),
        ip_address: "127.0.0.1".to_string(),
        status: "online".to_string(),
        secret_key: "super-secret".to_string(),
    })
    .await
    .expect("insert server");

    let state = new_state(db);
    let app = test_app(state).await;

    let body_bytes = b"hello".to_vec();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_secs()
        .to_string();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/internal/verify-signature")
                .header("X-Cluster-Signature", "deadbeef")
                .header("X-Cluster-Timestamp", &timestamp)
                .header("X-Server-Id", "srv-1")
                .body(axum::body::Body::from(body_bytes))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[test]
fn project_mapping_preserves_fields_and_sets_deployed_status() {
    let list_record = crate::db::ProjectListRecord {
        id: "p1".to_string(),
        name: "Project".to_string(),
        repo_url: "https://example.com/repo.git".to_string(),
        branch: "main".to_string(),
        start_command: "bun run start".to_string(),
        port: 3100,
        domain: Some("p1.example.com".to_string()),
        created_at: "now".to_string(),
    };

    let item = project_mapping::map_project_list_record(list_record);
    assert_eq!(item.id, "p1");
    assert_eq!(item.status, "deployed");

    let details_record = crate::db::ProjectDetailsRecord {
        id: "p1".to_string(),
        server_id: "srv".to_string(),
        name: "Project".to_string(),
        repo_url: "https://example.com/repo.git".to_string(),
        branch: "main".to_string(),
        install_command: "bun install".to_string(),
        build_command: "bun run build".to_string(),
        start_command: "bun run start".to_string(),
        port: 3100,
        domain: None,
        created_at: "now".to_string(),
        server_name: Some("server".to_string()),
    };

    let details = project_mapping::map_project_details_record(details_record);
    assert_eq!(details.id, "p1");
    assert_eq!(details.status, "deployed");
    assert_eq!(details.server_name.as_deref(), Some("server"));
}
