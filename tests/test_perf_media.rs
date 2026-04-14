use std::time::Instant;
use jw_api::services::MediaService;
use jw_api::error::AppError;


mod common;

const TEST_DIR: &str = "./tests_temp_perf_media";

#[tokio::test]
async fn perf_write_throughput_20_files() {
    let _ = tokio::fs::create_dir_all(TEST_DIR).await;
    let max_size = 10 * 1024 * 1024;
    let file_data = vec![0xCCu8; 512 * 1024]; // 512 KB each

    let start = Instant::now();
    let mut urls = Vec::new();
    for i in 0..20 {
        let name = format!("batch_{}.jpg", i);
        let url = MediaService::save_file(TEST_DIR, "posts", &name, &file_data, max_size)
            .await
            .unwrap();
        urls.push(url);
    }
    let elapsed = start.elapsed();

    common::assert_under("write_20x512kb", elapsed, 3000);

    for url in &urls {
        let _ = MediaService::delete_file(TEST_DIR, url).await;
    }
    let _ = tokio::fs::remove_dir_all(TEST_DIR).await;
}

#[tokio::test]
async fn media_exact_at_limit_accepted() {
    let _ = tokio::fs::create_dir_all(TEST_DIR).await;
    let limit: u64 = 1024;
    let data = vec![0u8; limit as usize];

    let result = MediaService::save_file(TEST_DIR, "posts", "exact.jpg", &data, limit).await;
    assert!(result.is_ok());

    let url = result.unwrap();
    let _ = MediaService::delete_file(TEST_DIR, &url).await;
    let _ = tokio::fs::remove_dir_all(TEST_DIR).await;
}

#[tokio::test]
async fn media_one_byte_over_limit_rejected() {
    let _ = tokio::fs::create_dir_all(TEST_DIR).await;
    let limit: u64 = 1024;
    let data = vec![0u8; (limit + 1) as usize];

    let result = MediaService::save_file(TEST_DIR, "posts", "over.jpg", &data, limit).await;
    assert!(result.is_err());
    match result {
        Err(AppError::PayloadTooLarge(_)) => {}
        _ => panic!("Expected PayloadTooLarge error"),
    }
    let _ = tokio::fs::remove_dir_all(TEST_DIR).await;
}

#[test]
fn media_type_detection_all_supported() {
    let image_exts = ["jpg", "jpeg", "png", "gif", "webp", "bmp"];
    let video_exts = ["mp4", "mov", "avi", "mkv", "webm"];

    for ext in image_exts {
        let result = MediaService::detect_media_type(&format!("file.{}", ext));
        assert_eq!(result.unwrap(), "image", "Failed for .{}", ext);
    }
    for ext in video_exts {
        let result = MediaService::detect_media_type(&format!("file.{}", ext));
        assert_eq!(result.unwrap(), "video", "Failed for .{}", ext);
    }
}

#[test]
fn media_type_unsupported_rejected() {
    let bad_exts = ["exe", "txt", "pdf", "doc", "zip", "rs"];
    for ext in bad_exts {
        let result = MediaService::detect_media_type(&format!("file.{}", ext));
        assert!(result.is_err(), "Should reject .{}", ext);
        match result {
            Err(AppError::UnsupportedMediaType(_)) => {}
            _ => panic!("Expected UnsupportedMediaType for .{}", ext),
        }
    }
}

#[test]
fn media_type_empty_extension_rejected() {
    assert!(MediaService::detect_media_type("noextension").is_err());
}

#[test]
fn media_type_double_extension_uses_last() {
    let result = MediaService::detect_media_type("photo.backup.png");
    assert_eq!(result.unwrap(), "image");
}

#[tokio::test]
async fn perf_concurrent_writes_no_collision() {
    let _ = tokio::fs::create_dir_all(TEST_DIR).await;
    let max_size = 10 * 1024 * 1024;
    let data = vec![0xAAu8; 1024];

    let mut handles = Vec::new();
    for i in 0..10 {
        let d = data.clone();
        handles.push(tokio::spawn(async move {
            MediaService::save_file(TEST_DIR, "posts", &format!("concurrent_{}.jpg", i), &d, max_size).await
        }));
    }

    let mut urls = std::collections::HashSet::new();
    for h in handles {
        let url = h.await.unwrap().unwrap();
        assert!(urls.insert(url.clone()), "Filename collision: {}", url);
    }

    for url in &urls {
        let _ = MediaService::delete_file(TEST_DIR, url).await;
    }
    let _ = tokio::fs::remove_dir_all(TEST_DIR).await;
}

#[tokio::test]
async fn media_delete_removes_from_disk() {
    let _ = tokio::fs::create_dir_all(TEST_DIR).await;
    let data = vec![0u8; 64];

    let url = MediaService::save_file(TEST_DIR, "posts", "todelete.png", &data, 1024).await.unwrap();
    let relative = url.strip_prefix("/uploads/").unwrap();
    let path = std::path::PathBuf::from(TEST_DIR).join(relative);
    assert!(path.exists(), "File should exist before delete");

    MediaService::delete_file(TEST_DIR, &url).await.unwrap();
    assert!(!path.exists(), "File should be gone after delete");

    let _ = tokio::fs::remove_dir_all(TEST_DIR).await;
}
