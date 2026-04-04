use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Email not verified")]
    EmailNotVerified,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Internal error: {0}")]
    InternalError(#[from] anyhow::Error),

    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),

    #[error("Unsupported media type: {0}")]
    UnsupportedMediaType(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::ValidationError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg.clone()),
            AppError::EmailNotVerified => (
                StatusCode::FORBIDDEN,
                "Email verification required".to_string(),
            ),
            AppError::DatabaseError(e) => {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
            AppError::EncryptionError(msg) => {
                tracing::error!("Encryption error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Encryption error".to_string())
            }
            AppError::InternalError(e) => {
                tracing::error!("Internal error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
            AppError::PayloadTooLarge(msg) => (StatusCode::PAYLOAD_TOO_LARGE, msg.clone()),
            AppError::UnsupportedMediaType(msg) => (StatusCode::UNSUPPORTED_MEDIA_TYPE, msg.clone()),
        };

        let body = json!({
            "success": false,
            "error": message
        });

        (status, axum::Json(body)).into_response()
    }
}
