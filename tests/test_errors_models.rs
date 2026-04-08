use axum::response::IntoResponse;
use axum::http::StatusCode;
use jw_api::error::AppError;

#[tokio::test]
async fn test_app_error_into_response() {
    let not_found_error = AppError::NotFound("User not found".to_string());
    let response = not_found_error.into_response();
    
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let auth_error = AppError::Unauthorized("Invalid token".to_string());
    let response = auth_error.into_response();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    
    let unverified = AppError::EmailNotVerified;
    let response = unverified.into_response();
    
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
