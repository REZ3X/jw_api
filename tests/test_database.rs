use jw_api::db;

mod common;

#[tokio::test]
#[ignore]
async fn test_database_connection_and_migrations() {
    let (_, state) = common::setup_test_app().await;
    
    let res = db::run_migrations(&state.db).await;
    assert!(res.is_ok(), "Migrations failed to run. {:?}", res.err());

    let row: (i64,) = sqlx::query_as("SELECT 1")
        .fetch_one(&state.db)
        .await
        .unwrap();

    assert_eq!(row.0, 1);
}
