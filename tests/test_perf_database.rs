use std::time::Instant;
use jw_api::db;

mod common;

#[tokio::test]
#[ignore]
async fn perf_connection_pool_latency() {
    let (_, state) = common::setup_test_app().await;
    let mut durations = Vec::new();

    for _ in 0..20 {
        let start = Instant::now();
        let conn = state.db.acquire().await.unwrap();
        drop(conn);
        durations.push(start.elapsed());
    }
    durations.sort();

    let p95 = durations[(durations.len() as f64 * 0.95) as usize];
    println!(" pool_acquire_p50: {}ms", durations[durations.len() / 2].as_millis());
    println!(" pool_acquire_p95: {}ms", p95.as_millis());
    common::assert_under("pool_acquire_p95", p95, 50);
}

#[tokio::test]
#[ignore]
async fn migration_idempotent() {
    let (_, state) = common::setup_test_app().await;

    let first = db::run_migrations(&state.db).await;
    assert!(first.is_ok(), "First migration run failed: {:?}", first.err());

    let second = db::run_migrations(&state.db).await;
    assert!(second.is_ok(), "Second migration run should succeed: {:?}", second.err());
}

#[tokio::test]
#[ignore]
async fn perf_concurrent_queries() {
    let (_, state) = common::setup_test_app().await;
    db::run_migrations(&state.db).await.unwrap();

    let start = Instant::now();
    let mut handles = Vec::new();
    for _ in 0..20 {
        let pool = state.db.clone();
        handles.push(tokio::spawn(async move {
            let row: (i64,) = sqlx::query_as("SELECT 1")
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_eq!(row.0, 1);
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    let elapsed = start.elapsed();
    common::assert_under("concurrent_20_queries", elapsed, 2000);
}

#[tokio::test]
#[ignore]
async fn write_read_consistency() {
    let (_, state) = common::setup_test_app().await;
    db::run_migrations(&state.db).await.unwrap();

    let (user_id, _salt) = common::create_test_user(&state.db, "db_consistency").await;

    let row: Option<(String,)> = sqlx::query_as("SELECT name FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await
        .unwrap();

    assert!(row.is_some(), "Inserted row should be immediately readable");
    assert_eq!(row.unwrap().0, "Test User");

    common::cleanup_test_user(&state.db, &user_id).await;
}

#[tokio::test]
#[ignore]
async fn perf_insert_delete_cycle() {
    let (_, state) = common::setup_test_app().await;
    db::run_migrations(&state.db).await.unwrap();

    let start = Instant::now();
    for i in 0..10 {
        let suffix = format!("cycle_{}", i);
        let (uid, _) = common::create_test_user(&state.db, &suffix).await;
        common::cleanup_test_user(&state.db, &uid).await;
    }
    let elapsed = start.elapsed();
    common::assert_under("10_user_insert_delete_cycles", elapsed, 5000);
}
