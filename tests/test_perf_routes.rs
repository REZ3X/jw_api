use axum::http::StatusCode;
use jw_api::db;
use std::time::Instant;

mod common;

#[tokio::test]
#[ignore]
async fn perf_health_latency_p95() {
    let (server, state) = common::setup_test_app().await;
    db::run_migrations(&state.db).await.unwrap();

    let mut durations = Vec::new();
    for _ in 0..50 {
        let start = Instant::now();
        let res = server.get("/health").await;
        durations.push(start.elapsed());
        res.assert_status_ok();
    }
    durations.sort();

    let p95 = durations[(durations.len() as f64 * 0.95) as usize];
    println!(
        " health_p50: {}ms",
        durations[durations.len() / 2].as_millis()
    );
    println!(" health_p95: {}ms", p95.as_millis());
    common::assert_under("health_p95", p95, 50);
}

#[tokio::test]
#[ignore]
async fn perf_health_sequential_throughput() {
    let (server, state) = common::setup_test_app().await;
    db::run_migrations(&state.db).await.unwrap();

    let start = Instant::now();
    for _ in 0..50 {
        let res = server.get("/health").await;
        res.assert_status_ok();
    }
    let elapsed = start.elapsed();
    common::assert_under("health_50_sequential", elapsed, 5000);
}

#[tokio::test]
#[ignore]
async fn auth_guard_rejects_all_protected_routes() {
    let (server, _state) = common::setup_test_app().await;

    let protected_routes = vec![
        ("DELETE", "/api/users/me/avatar"),
        ("POST", "/api/posts"),
        ("PUT", "/api/posts/fake-id"),
        ("DELETE", "/api/posts/fake-id"),
        ("POST", "/api/chats"),
        ("GET", "/api/chats"),
        ("GET", "/api/auth/me"),
        ("POST", "/api/auth/resend-verification"),
        ("POST", "/api/comments/post/fake-id"),
        ("GET", "/api/departments/dashboard"),
    ];

    for (method, path) in &protected_routes {
        let response = match *method {
            "GET" => server.get(path).await,
            "POST" => server.post(path).json(&serde_json::json!({})).await,
            "PUT" => server.put(path).json(&serde_json::json!({})).await,
            "DELETE" => server.delete(path).await,
            _ => unreachable!(),
        };

        assert_eq!(
            response.status_code(),
            StatusCode::UNAUTHORIZED,
            "{} {} should require auth, got {}",
            method,
            path,
            response.status_code()
        );
    }
    println!(
        " [PASS] auth_guard: all {} protected routes correctly reject unauthenticated requests ✓",
        protected_routes.len()
    );
}

#[tokio::test]
#[ignore]
async fn nonexistent_route_returns_proper_error() {
    let (server, _state) = common::setup_test_app().await;

    let response = server.get("/api/this-does-not-exist").await;
    // Axum returns 404 for unmatched routes
    assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[ignore]
async fn health_response_body_structure() {
    let (server, state) = common::setup_test_app().await;
    db::run_migrations(&state.db).await.unwrap();

    let response = server.get("/health").await;
    response.assert_status_ok();

    let body = response.json::<serde_json::Value>();
    assert_eq!(body["success"], true);
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "jw-api");
    assert!(body["version"].is_string());
    assert_eq!(body["checks"]["database"], "connected");
}
