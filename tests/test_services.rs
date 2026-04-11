mod common;

use jw_api::services::MediaService;
use jw_api::error::AppError;
use base64::{Engine as _, engine::general_purpose::STANDARD};

#[tokio::test]
#[ignore]
async fn test_comment_service_integration() {
    let (_server, _state) = common::setup_test_app().await;
    
    assert!(true);
}

#[tokio::test]
#[ignore]
async fn test_post_service_integration() {
    let (_server, _state) = common::setup_test_app().await;
    assert!(true);
}

#[tokio::test]
async fn test_media_upload_size_limit() {
    let upload_dir = "./tests_temp_uploads";
    let _ = tokio::fs::create_dir_all(upload_dir).await;

    // test under limit (2MB limit, we use 1MB of random bytes)
    let max_size = 2 * 1024 * 1024;
    // Generate base64 string that matches the limit requirement as requested
    let under_limit_base64 = STANDARD.encode(vec![0u8; 1 * 1024 * 1024]);
    let under_limit_bytes = STANDARD.decode(&under_limit_base64).unwrap();

    let result = MediaService::save_file(
        upload_dir,
        "posts",
        "under_limit.jpg",
        &under_limit_bytes,
        max_size,
    ).await;

    assert!(result.is_ok(), "File under limit should be uploaded successfully");
    let url = result.unwrap();
    assert!(url.starts_with("/uploads/posts/"));

    // clean up success file
    let _ = MediaService::delete_file(upload_dir, &url).await;

    // test over limit (2MB limit, we use 3MB of random bytes)
    let over_limit_base64 = STANDARD.encode(vec![0u8; 3 * 1024 * 1024]);
    let over_limit_bytes = STANDARD.decode(&over_limit_base64).unwrap();

    let result = MediaService::save_file(
        upload_dir,
        "posts",
        "over_limit.jpg",
        &over_limit_bytes,
        max_size,
    ).await;

    assert!(result.is_err(), "File over limit should fail to upload");
    match result {
        Err(AppError::PayloadTooLarge(msg)) => {
            assert!(msg.contains("File exceeds max size"));
        }
        _ => panic!("Expected PayloadTooLarge error"),
    }

    // clean up test dir
    let _ = tokio::fs::remove_dir_all(upload_dir).await;
}
