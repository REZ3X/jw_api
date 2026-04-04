use axum::http::{HeaderValue, Method};
use std::net::SocketAddr;
use tokio::signal;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use jw_api::{config::Config, crypto::CryptoService, db, routes, state::AppState};
use jw_api::services::{EmailService, GeminiService};

#[tokio::main]
async fn main() {

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "jw_api=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();


    dotenvy::dotenv().ok();
    let config = Config::from_env().expect("Failed to load configuration");

    tracing::info!("Starting {} API v{}", config.app.name, env!("CARGO_PKG_VERSION"));


    let db = db::create_pool(&config.database)
        .await
        .expect("Failed to create database pool");

    db::run_migrations(&db)
        .await
        .expect("Failed to run migrations");


    let crypto = CryptoService::new(&config.encryption.master_key)
        .expect("Failed to initialize crypto service");

    let gemini = GeminiService::new(
        config.gemini.api_key.clone(),
        config.gemini.model.clone(),
    );

    let email = EmailService::new(
        &config.brevo,
        &config.app.name,
        &config.app.frontend_url,
    );

    let http_client = reqwest::Client::new();


    let state = AppState {
        db: db.clone(),
        config: config.clone(),
        crypto,
        gemini,
        email,
        http_client,
    };


    let cors = build_cors(&config);


    let app = routes::build_router(state)
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http());


    let addr = SocketAddr::from(([0, 0, 0, 0], config.app.port));
    tracing::info!("Server listening on {}", addr);
    tracing::info!("Mode: {}", config.app.mode);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");

    tracing::info!("Server shutdown complete");
}

fn build_cors(config: &Config) -> CorsLayer {
    let origins = if config.app.mode == "internal" {
        vec![
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
            "http://127.0.0.1:3000".parse::<HeaderValue>().unwrap(),
        ]
    } else {
        vec![
            config.app.frontend_url.parse::<HeaderValue>().unwrap(),
        ]
    };

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any)
        .allow_credentials(true)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("Ctrl+C received, shutting down..."); },
        _ = terminate => { tracing::info!("SIGTERM received, shutting down..."); },
    }
}
