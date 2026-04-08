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
