use std::sync::atomic::{AtomicU64, Ordering};

use axum::body::{to_bytes, Body};
use axum::extract::MatchedPath;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;

static LOG_COUNTER: AtomicU64 = AtomicU64::new(1);

const BODY_READ_LIMIT_BYTES: usize = 1_048_576;
const SHORT_BODY_MAX_CHARS: usize = 160;

pub async fn log_orchestrator_request(request: Request<Body>, next: Next) -> Response {
    log_request(request, next, orchestrator_method_name).await
}

pub async fn log_worker_request(request: Request<Body>, next: Next) -> Response {
    log_request(request, next, worker_method_name).await
}

async fn log_request(
    request: Request<Body>,
    next: Next,
    method_name_for_route: fn(&str, &str) -> &'static str,
) -> Response {
    let method = request.method().as_str().to_string();
    let route = request.extensions().get::<MatchedPath>().map_or_else(
        || request.uri().path().to_string(),
        |matched_path| matched_path.as_str().to_string(),
    );

    let (parts, body) = request.into_parts();
    let (body_bytes, short_body) = match to_bytes(body, BODY_READ_LIMIT_BYTES).await {
        Ok(bytes) => {
            let shortened = shorten_request_body(&String::from_utf8_lossy(&bytes));
            (bytes, shortened)
        }
        Err(_) => (
            axum::body::Bytes::new(),
            "<request-body-unavailable>".to_string(),
        ),
    };

    let method_name = method_name_for_route(method.as_str(), route.as_str());
    let log_number = LOG_COUNTER.fetch_add(1, Ordering::Relaxed);
    println!("{log_number}\t{method_name}\t{route}\t{short_body}");

    let request = Request::from_parts(parts, Body::from(body_bytes));
    next.run(request).await
}

fn shorten_request_body(raw_body: &str) -> String {
    if raw_body.is_empty() {
        return "-".to_string();
    }

    let single_line = raw_body
        .replace(['\r', '\n', '\t'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if single_line.chars().count() <= SHORT_BODY_MAX_CHARS {
        return single_line;
    }

    let mut shortened = single_line
        .chars()
        .take(SHORT_BODY_MAX_CHARS)
        .collect::<String>();
    shortened.push_str("...");
    shortened
}

fn orchestrator_method_name(method: &str, route: &str) -> &'static str {
    match (method, route) {
        ("POST", "/api/auth/setup") => "auth.auth_setup",
        ("POST", "/api/auth/login") => "auth.auth_login",
        ("POST", "/api/auth/status") => "auth.auth_status",
        ("POST", "/api/auth/session") => "auth.auth_session",
        ("GET", "/api/servers") => "servers.list_servers",
        ("GET", "/api/servers/:id/stats") => "servers.get_server_stats",
        ("GET", "/api/projects") => "projects.list_projects",
        ("POST", "/api/projects") => "projects.create_project",
        ("GET", "/api/projects/:id") => "projects.get_project",
        ("DELETE", "/api/projects/:id") => "projects.delete_project",
        ("POST", "/api/projects/:id/redeploy") => "projects.redeploy_project",
        ("POST", "/api/cluster/generate-token") => "cluster.generate_cluster_token",
        ("POST", "/api/cluster/join") => "cluster.join_cluster",
        ("POST", "/internal/projects") => "internal.internal_projects",
        ("DELETE", "/internal/projects/:id") => "internal.internal_delete_project",
        ("POST", "/internal/ports/check") => "internal.internal_port_check",
        ("POST", "/internal/verify-signature") => "cluster.verify_signature_guarded",
        _ => "unknown.unknown_handler",
    }
}

fn worker_method_name(method: &str, route: &str) -> &'static str {
    match (method, route) {
        ("POST", "/internal/health") => "handlers.internal_health",
        ("POST", "/internal/stats") => "handlers.internal_stats",
        ("POST", "/internal/deploy") => "handlers.internal_deploy",
        ("POST", "/internal/ports/check") => "handlers.internal_port_check",
        ("POST", "/internal/projects") => "handlers.internal_projects",
        ("DELETE", "/internal/projects/:id") => "handlers.internal_delete_project",
        _ => "unknown.unknown_handler",
    }
}

#[cfg(test)]
mod tests {
    use super::{shorten_request_body, SHORT_BODY_MAX_CHARS};

    #[test]
    fn shorten_request_body_returns_dash_for_empty_input() {
        assert_eq!(shorten_request_body(""), "-");
    }

    #[test]
    fn shorten_request_body_normalizes_whitespace() {
        let body = "{\n  \"name\":\t\"demo\"\r\n}";
        assert_eq!(shorten_request_body(body), "{ \"name\": \"demo\" }");
    }

    #[test]
    fn shorten_request_body_truncates_and_appends_ellipsis() {
        let input = "a".repeat(SHORT_BODY_MAX_CHARS + 10);
        let shortened = shorten_request_body(&input);

        assert_eq!(shortened.chars().count(), SHORT_BODY_MAX_CHARS + 3);
        assert!(shortened.ends_with("..."));
    }
}
