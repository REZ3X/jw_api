#![allow(dead_code)]

use std::future::Future;
use std::time::{Duration, Instant};
use axum_test::TestServer;
use jw_api::{
    config::Config,
    crypto::CryptoService,
    db, routes,
    services::{EmailService, GeminiService},
    state::AppState,
};

pub async fn setup_test_app() -> (TestServer, AppState) {
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let env_path = std::path::Path::new(&manifest_dir).join(".env");
        let _ = dotenvy::from_path(env_path);
    } else {
        let _ = dotenvy::dotenv();
    }
    let config = Config::from_env().unwrap();

    let db_pool = db::create_pool(&config.database)
        .await
        .expect("Failed to create test database pool (Make sure MariaDB is running!)");

    let crypto = CryptoService::new(&config.encryption.master_key).unwrap();
    let gemini = GeminiService::new(config.gemini.api_key.clone(), config.gemini.model.clone());
    let email = EmailService::new(&config.brevo, &config.app.name, &config.app.frontend_url);

    let state = AppState {
        db: db_pool,
        config: config.clone(),
        crypto,
        gemini,
        email,
        http_client: reqwest::Client::new(),
    };

    let router = routes::build_router(state.clone());
    let test_server = TestServer::new(router).unwrap();

    (test_server, state)
}

// Measures wall-clock duration of an async block and returns (duration, result).
pub async fn timed<T, F: Future<Output = T>>(label: &str, fut: F) -> (Duration, T) {
    let start = Instant::now();
    let result = fut.await;
    let elapsed = start.elapsed();
    println!(" {}: {}ms", label, elapsed.as_millis());
    (elapsed, result)
}

// Panics if the measured duration exceeds the budget.
pub fn assert_under(label: &str, actual: Duration, budget_ms: u64) {
    let budget = Duration::from_millis(budget_ms);
    if actual > budget {
        panic!(
            " [FAIL] {} took {}ms, budget was {}ms",
            label,
            actual.as_millis(),
            budget_ms
        );
    }
    println!(
        " [PASS] {}: {}ms (budget: {}ms) ✓",
        label,
        actual.as_millis(),
        budget_ms
    );
}

// Inserts a test user and returns their id + encryption salt.
pub async fn create_test_user(pool: &sqlx::MySqlPool, suffix: &str) -> (String, String) {
    let id = format!("test-user-{}", suffix);
    let salt = CryptoService::generate_user_salt();
    let username = format!("testuser_{}", suffix);

    sqlx::query(
        r#"INSERT INTO users (id, google_id, username, name, email, role,
           email_verification_status, encryption_salt)
           VALUES (?, ?, ?, 'Test User', ?, 'basic', 'verified', ?)
           ON DUPLICATE KEY UPDATE id = id"#,
    )
    .bind(&id)
    .bind(&format!("google-{}", suffix))
    .bind(&username)
    .bind(&format!("test_{}@test.local", suffix))
    .bind(&salt)
    .execute(pool)
    .await
    .expect("Failed to create test user");

    (id, salt)
}

pub async fn cleanup_test_user(pool: &sqlx::MySqlPool, user_id: &str) {
    sqlx::query("DELETE FROM chat_messages WHERE user_id = ?")
        .bind(user_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM chats WHERE user_id = ?")
        .bind(user_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM sub_comment_votes WHERE user_id = ?")
        .bind(user_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM comment_votes WHERE user_id = ?")
        .bind(user_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM post_votes WHERE user_id = ?")
        .bind(user_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM sub_comments WHERE user_id = ?")
        .bind(user_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM comments WHERE user_id = ?")
        .bind(user_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM post_tags WHERE post_id IN (SELECT id FROM posts WHERE user_id = ?)")
        .bind(user_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM post_media WHERE post_id IN (SELECT id FROM posts WHERE user_id = ?)")
        .bind(user_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM posts WHERE user_id = ?")
        .bind(user_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id).execute(pool).await.ok();
}

// Creates a test post and returns its id.
pub async fn create_test_post(
    pool: &sqlx::MySqlPool,
    user_id: &str,
    caption: &str,
) -> String {
    let post_id = format!("test-post-{}", uuid::Uuid::new_v4());
    let editable_until = chrono::Utc::now().naive_utc() + chrono::Duration::hours(24);

    sqlx::query(
        r#"INSERT INTO posts (id, user_id, caption, location, department, status, editable_until)
           VALUES (?, ?, ?, 'Test Location', 'city_major_gov', 'pending', ?)"#,
    )
    .bind(&post_id)
    .bind(user_id)
    .bind(caption)
    .bind(editable_until)
    .execute(pool)
    .await
    .expect("Failed to create test post");

    post_id
}

pub async fn cleanup_test_post(pool: &sqlx::MySqlPool, post_id: &str) {
    sqlx::query("DELETE FROM post_tags WHERE post_id = ?")
        .bind(post_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM post_media WHERE post_id = ?")
        .bind(post_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM post_votes WHERE post_id = ?")
        .bind(post_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM sub_comment_votes WHERE sub_comment_id IN (SELECT sc.id FROM sub_comments sc JOIN comments c ON c.id = sc.comment_id WHERE c.post_id = ?)")
        .bind(post_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM comment_votes WHERE comment_id IN (SELECT id FROM comments WHERE post_id = ?)")
        .bind(post_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM sub_comments WHERE comment_id IN (SELECT id FROM comments WHERE post_id = ?)")
        .bind(post_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM comments WHERE post_id = ?")
        .bind(post_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM post_status_history WHERE post_id = ?")
        .bind(post_id).execute(pool).await.ok();
    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(post_id).execute(pool).await.ok();
}
