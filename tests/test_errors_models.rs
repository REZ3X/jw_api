use axum::response::IntoResponse;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use jw_api::error::AppError;

#[tokio::test]
async fn error_status_codes_all_variants() {
    let cases: Vec<(AppError, StatusCode)> = vec![
        (AppError::BadRequest("bad".into()), StatusCode::BAD_REQUEST),
        (AppError::Unauthorized("unauth".into()), StatusCode::UNAUTHORIZED),
        (AppError::Forbidden("forbidden".into()), StatusCode::FORBIDDEN),
        (AppError::NotFound("missing".into()), StatusCode::NOT_FOUND),
        (AppError::Conflict("conflict".into()), StatusCode::CONFLICT),
        (AppError::ValidationError("invalid".into()), StatusCode::UNPROCESSABLE_ENTITY),
        (AppError::EmailNotVerified, StatusCode::FORBIDDEN),
        (AppError::EncryptionError("enc".into()), StatusCode::INTERNAL_SERVER_ERROR),
        (AppError::InternalError(anyhow::anyhow!("oops")), StatusCode::INTERNAL_SERVER_ERROR),
        (AppError::PayloadTooLarge("big".into()), StatusCode::PAYLOAD_TOO_LARGE),
        (AppError::UnsupportedMediaType("nope".into()), StatusCode::UNSUPPORTED_MEDIA_TYPE),
    ];

    for (error, expected_status) in cases {
        let label = format!("{:?}", error).chars().take(40).collect::<String>();
        let response = error.into_response();
        assert_eq!(
            response.status(), expected_status,
            "Wrong status for {}: expected {}, got {}", label, expected_status, response.status()
        );
    }
}

#[tokio::test]
async fn error_response_body_shape() {
    let errors: Vec<AppError> = vec![
        AppError::NotFound("User not found".into()),
        AppError::Unauthorized("Invalid token".into()),
        AppError::BadRequest("Missing field".into()),
        AppError::Forbidden("Access denied".into()),
        AppError::PayloadTooLarge("Too big".into()),
    ];

    for error in errors {
        let response = error.into_response();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["success"], false, "Error response must have success: false");
        assert!(json["error"].is_string(), "Error response must have error string");
        assert!(!json["error"].as_str().unwrap().is_empty(), "Error message must not be empty");
    }
}

#[test]
fn error_display_messages() {
    assert_eq!(format!("{}", AppError::BadRequest("test".into())), "Bad request: test");
    assert_eq!(format!("{}", AppError::NotFound("x".into())), "Not found: x");
    assert_eq!(format!("{}", AppError::EmailNotVerified), "Email not verified");
    assert_eq!(format!("{}", AppError::PayloadTooLarge("5MB".into())), "Payload too large: 5MB");
}
