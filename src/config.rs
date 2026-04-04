use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    pub jwt: JwtConfig,
    pub database: DatabaseConfig,
    pub google_oauth: GoogleOAuthConfig,
    pub brevo: BrevoConfig,
    pub encryption: EncryptionConfig,
    pub gemini: GeminiConfig,
    pub media: MediaConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub env: String,
    pub port: u16,
    pub frontend_url: String,
    pub mode: String,
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BrevoConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub from_email: String,
    pub from_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EncryptionConfig {
    pub master_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeminiConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MediaConfig {
    pub max_image_size_mb: u64,
    pub max_video_size_mb: u64,
    pub upload_dir: String,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        Ok(Config {
            app: AppConfig {
                name: env_var("APP_NAME")?,
                env: env_var_default("APP_ENV", "development"),
                port: env_var_default("APP_PORT", "8000").parse()?,
                frontend_url: env_var_default("FRONTEND_URL", "http://localhost:3000"),
                mode: env_var_default("APP_MODE", "internal"),
                api_key: std::env::var("API_KEY").ok(),
            },
            jwt: JwtConfig {
                secret: env_var("JWT_SECRET")?,
                expiration_hours: env_var_default("JWT_EXPIRATION_HOURS", "72").parse()?,
            },
            database: DatabaseConfig {
                url: env_var("DATABASE_URL")?,
                max_connections: env_var_default("DATABASE_MAX_CONNECTIONS", "10").parse()?,
            },
            google_oauth: GoogleOAuthConfig {
                client_id: env_var("GOOGLE_CLIENT_ID")?,
                client_secret: env_var("GOOGLE_CLIENT_SECRET")?,
                redirect_uri: env_var("GOOGLE_REDIRECT_URI")?,
            },
            brevo: BrevoConfig {
                smtp_host: env_var_default("BREVO_SMTP_HOST", "smtp-relay.brevo.com"),
                smtp_port: env_var_default("BREVO_SMTP_PORT", "587").parse()?,
                smtp_user: env_var("BREVO_SMTP_USER")?,
                smtp_pass: env_var("BREVO_SMTP_PASS")?,
                from_email: env_var_default("BREVO_FROM_EMAIL", "noreply@localhost"),
                from_name: env_var_default("BREVO_FROM_NAME", "JW"),
            },
            encryption: EncryptionConfig {
                master_key: env_var("ENCRYPTION_MASTER_KEY")?,
            },
            gemini: GeminiConfig {
                api_key: env_var("GEMINI_API_KEY")?,
                model: env_var_default("GEMINI_MODEL", "gemini-2.0-flash-lite"),
            },
            media: MediaConfig {
                max_image_size_mb: env_var_default("MAX_IMAGE_SIZE_MB", "10").parse()?,
                max_video_size_mb: env_var_default("MAX_VIDEO_SIZE_MB", "50").parse()?,
                upload_dir: env_var_default("UPLOAD_DIR", "./uploads"),
            },
        })
    }
}

fn env_var(key: &str) -> Result<String, anyhow::Error> {
    std::env::var(key).map_err(|_| anyhow::anyhow!("Missing env var: {}", key))
}

fn env_var_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
