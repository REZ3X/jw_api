use axum::http::StatusCode;
use jw_api::db;

mod common;

#[tokio::test]
#[ignore]
async fn test_health_endpoint() {
    let (server, state) = common::setup_test_app().await;
    
    db::run_migrations(&state.db).await.unwrap();

    let response = server.get("/health").await;
    
    response.assert_status_ok();
    
    let body = response.json::<serde_json::Value>();
    assert_eq!(body["success"], true);
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "jw-api");
    assert_eq!(body["checks"]["database"], "connected");
}

#[tokio::test]
#[ignore]
async fn test_unauthorized_routes_fail() {
    let (server, _state) = common::setup_test_app().await;

    let response = server.delete("/api/users/me/avatar").await;
    
    assert_eq!(response.status_code(), StatusCode::UNAUTHORIZED);
    
    let body = response.json::<serde_json::Value>();
    assert_eq!(body["success"], false);
    assert!(body["error"].as_str().unwrap().contains("Missing Authorization header"));
}
