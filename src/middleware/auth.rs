use axum::{async_trait, extract::{FromRef, FromRequestParts}, http::request::Parts};
use jsonwebtoken::{decode, DecodingKey, Validation};

use crate::{error::AppError, models::{Claims, UserRow}, state::AppState};


const GOV_ROLES: &[&str] = &[
    "city_major_gov",
    "fire_department",
    "health_department",
    "environment_department",
    "police_department",
];

pub struct AuthUser {
    pub user: UserRow,
    pub claims: Claims,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".into()))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AppError::Unauthorized("Invalid Authorization header format".into()))?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(app_state.config.jwt.secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|e| {
            tracing::warn!("JWT validation failed: {}", e);
            AppError::Unauthorized("Invalid or expired token".into())
        })?;

        let claims = token_data.claims;

        let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(&claims.sub)
            .fetch_optional(&app_state.db)
            .await
            .map_err(|e| {
                tracing::error!("Database error fetching user: {}", e);
                AppError::InternalError(e.into())
            })?
            .ok_or_else(|| AppError::Unauthorized("User not found".into()))?;

        Ok(AuthUser { user, claims })
    }
}

pub struct VerifiedUser {
    pub user: UserRow,
    pub claims: Claims,
}

#[async_trait]
impl<S> FromRequestParts<S> for VerifiedUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth = AuthUser::from_request_parts(parts, state).await?;

        if auth.user.email_verification_status != "verified" {
            return Err(AppError::EmailNotVerified);
        }

        Ok(VerifiedUser {
            user: auth.user,
            claims: auth.claims,
        })
    }
}

pub struct GovUser {
    pub user: UserRow,
    pub claims: Claims,
}

#[async_trait]
impl<S> FromRequestParts<S> for GovUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let verified = VerifiedUser::from_request_parts(parts, state).await?;

        if !GOV_ROLES.contains(&verified.user.role.as_str()) {
            return Err(AppError::Forbidden("Government department access required".into()));
        }

        Ok(GovUser {
            user: verified.user,
            claims: verified.claims,
        })
    }
}

pub struct DevUser {
    pub user: UserRow,
    pub claims: Claims,
}

#[async_trait]
impl<S> FromRequestParts<S> for DevUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let verified = VerifiedUser::from_request_parts(parts, state).await?;

        if verified.user.role != "dev" {
            return Err(AppError::Forbidden("Dev access required".into()));
        }

        Ok(DevUser {
            user: verified.user,
            claims: verified.claims,
        })
    }
}
