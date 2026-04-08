use jw_api::db;
use serde_json::json;

mod common;

#[tokio::test]
#[ignore]
async fn test_auth_jwt_and_chat_flow() {
    let (server, state) = common::setup_test_app().await;
    db::run_migrations(&state.db).await.unwrap();

    let res = server.post("/api/chats")
        .json(&json!({
            "first_message": "Hello, regarding potholes"
        }))
        .await;
    
    assert_eq!(res.status_code(), 401);
}
