use sqlx::MySqlPool;

use crate::{
    config::Config,
    crypto::CryptoService,
    services::{EmailService, GeminiService},
};

#[derive(Clone)]
pub struct AppState {
    pub db: MySqlPool,
    pub config: Config,
    pub crypto: CryptoService,
    pub gemini: GeminiService,
    pub email: EmailService,
    pub http_client: reqwest::Client,
}
