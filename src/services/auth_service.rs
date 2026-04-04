use chrono::Utc;
use jsonwebtoken::{encode, EncodingKey, Header};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, RedirectUrl,
    TokenResponse, TokenUrl,
};
use rand::Rng;
use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    config::Config,
    crypto::CryptoService,
    error::{AppError, Result},
    models::{Claims, GoogleUserInfo, UserRow},
};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USER_INFO_URL: &str = "https://www.googleapis.com/oauth2/v2/userinfo";

pub struct AuthService;

impl AuthService {
    pub fn google_auth_url(config: &Config) -> Result<String> {
        let client = Self::oauth_client(config)?;
        let (auth_url, _csrf) = client
            .authorize_url(oauth2::CsrfToken::new_random)
            .add_scope(oauth2::Scope::new("email".into()))
            .add_scope(oauth2::Scope::new("profile".into()))
            .url();
        Ok(auth_url.to_string())
    }

    pub async fn exchange_code(
        code: &str,
        config: &Config,
        http: &reqwest::Client,
    ) -> Result<GoogleUserInfo> {
        let client = Self::oauth_client(config)?;

        let token_result = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| AppError::BadRequest(format!("OAuth code exchange failed: {}", e)))?;

        let access_token = token_result.access_token().secret();

        let user_info: GoogleUserInfo = http
            .get(GOOGLE_USER_INFO_URL)
            .header("Authorization", format!("Bearer {}", access_token))
            .send()
            .await
            .map_err(|e| AppError::InternalError(e.into()))?
            .json()
            .await
            .map_err(|e| AppError::InternalError(e.into()))?;

        Ok(user_info)
    }

    pub async fn find_or_create_user(
        pool: &MySqlPool,
        google_user: &GoogleUserInfo,
        _crypto: &CryptoService,
    ) -> Result<(UserRow, bool)> {
        let existing: Option<UserRow> =
            sqlx::query_as("SELECT * FROM users WHERE email = ? OR google_id = ?")
                .bind(&google_user.email)
                .bind(&google_user.id)
                .fetch_optional(pool)
                .await
                .map_err(AppError::DatabaseError)?;

        if let Some(user) = existing {
            sqlx::query(
                "UPDATE users SET google_id = ?, avatar_url = ?, updated_at = NOW() WHERE id = ?",
            )
            .bind(&google_user.id)
            .bind(&google_user.picture)
            .bind(&user.id)
            .execute(pool)
            .await
            .map_err(AppError::DatabaseError)?;

            let updated: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?")
                .bind(&user.id)
                .fetch_one(pool)
                .await
                .map_err(AppError::DatabaseError)?;

            return Ok((updated, false));
        }

        let id = Uuid::new_v4().to_string();
        let salt = CryptoService::generate_user_salt();

        let base_username = google_user
            .email
            .split('@')
            .next()
            .unwrap_or("user")
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .take(30)
            .collect::<String>();

        let username = Self::ensure_unique_username(pool, &base_username).await?;
        let verification_token = Self::generate_verification_token();

        sqlx::query(
            r#"INSERT INTO users (id, google_id, username, name, email, avatar_url, email_verification_status, email_verification_token, role, encryption_salt)
               VALUES (?, ?, ?, ?, ?, ?, 'pending', ?, 'basic', ?)"#,
        )
        .bind(&id)
        .bind(&google_user.id)
        .bind(&username)
        .bind(&google_user.name)
        .bind(&google_user.email)
        .bind(&google_user.picture)
        .bind(&verification_token)
        .bind(&salt)
        .execute(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        let new_user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(&id)
            .fetch_one(pool)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok((new_user, true))
    }

    pub fn generate_jwt(user: &UserRow, config: &Config) -> Result<String> {
        let now = Utc::now().timestamp();
        let exp = now + config.jwt.expiration_hours * 3600;

        let claims = Claims {
            sub: user.id.clone(),
            email: user.email.clone(),
            exp,
            iat: now,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.jwt.secret.as_bytes()),
        )
        .map_err(|e| AppError::InternalError(e.into()))
    }

    pub async fn verify_email(pool: &MySqlPool, token: &str) -> Result<UserRow> {
        let user: Option<UserRow> =
            sqlx::query_as("SELECT * FROM users WHERE email_verification_token = ?")
                .bind(token)
                .fetch_optional(pool)
                .await
                .map_err(AppError::DatabaseError)?;

        let user = user.ok_or_else(|| AppError::BadRequest("Invalid verification token".into()))?;

        if user.email_verification_status == "verified" {
            return Ok(user);
        }

        sqlx::query(
            r#"UPDATE users
               SET email_verification_status = 'verified',
                   email_verified_at = NOW(),
                   email_verification_token = NULL,
                   updated_at = NOW()
               WHERE id = ?"#,
        )
        .bind(&user.id)
        .execute(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        let updated: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(&user.id)
            .fetch_one(pool)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(updated)
    }

    pub fn generate_verification_token() -> String {
        let mut rng = rand::thread_rng();
        (0..48)
            .map(|_| {
                let idx = rng.gen_range(0..62usize);
                match idx {
                    0..=25 => (b'A' + idx as u8) as char,
                    26..=51 => (b'a' + (idx - 26) as u8) as char,
                    _ => (b'0' + (idx - 52) as u8) as char,
                }
            })
            .collect()
    }

    async fn ensure_unique_username(pool: &MySqlPool, base: &str) -> Result<String> {
        let mut candidate = base.to_string();
        let mut suffix = 1u32;
        loop {
            let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = ?")
                .bind(&candidate)
                .fetch_one(pool)
                .await
                .map_err(AppError::DatabaseError)?;
            if count.0 == 0 {
                return Ok(candidate);
            }
            candidate = format!("{}{}", base, suffix);
            suffix += 1;
        }
    }

    fn oauth_client(config: &Config) -> Result<BasicClient> {
        Ok(BasicClient::new(
            ClientId::new(config.google_oauth.client_id.clone()),
            Some(ClientSecret::new(config.google_oauth.client_secret.clone())),
            AuthUrl::new(GOOGLE_AUTH_URL.into()).map_err(|e| AppError::InternalError(e.into()))?,
            Some(
                TokenUrl::new(GOOGLE_TOKEN_URL.into())
                    .map_err(|e| AppError::InternalError(e.into()))?,
            ),
        )
        .set_redirect_uri(
            RedirectUrl::new(config.google_oauth.redirect_uri.clone())
                .map_err(|e| AppError::InternalError(e.into()))?,
        ))
    }
}
