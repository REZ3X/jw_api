use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

use crate::error::{AppError, Result};

pub struct MediaService;

impl MediaService {
    pub async fn ensure_dirs(upload_dir: &str) -> Result<()> {
        let dirs = ["posts", "avatars", "comments"];
        for dir in &dirs {
            let path = PathBuf::from(upload_dir).join(dir);
            fs::create_dir_all(&path).await.map_err(|e| {
                AppError::InternalError(anyhow::anyhow!("Failed to create dir {:?}: {}", path, e))
            })?;
        }
        Ok(())
    }

    pub async fn save_file(
        upload_dir: &str,
        sub_dir: &str,
        filename: &str,
        data: &[u8],
        max_size_bytes: u64,
    ) -> Result<String> {
        if data.len() as u64 > max_size_bytes {
            return Err(AppError::PayloadTooLarge(format!(
                "File exceeds max size of {} MB",
                max_size_bytes / (1024 * 1024)
            )));
        }

        let ext = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin")
            .to_lowercase();

        let unique_name = format!("{}.{}", Uuid::new_v4(), ext);
        let dir_path = PathBuf::from(upload_dir).join(sub_dir);
        fs::create_dir_all(&dir_path).await.map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to create dir: {}", e))
        })?;

        let file_path = dir_path.join(&unique_name);
        fs::write(&file_path, data).await.map_err(|e| {
            AppError::InternalError(anyhow::anyhow!("Failed to write file: {}", e))
        })?;

        Ok(format!("/uploads/{}/{}", sub_dir, unique_name))
    }

    pub async fn delete_file(upload_dir: &str, url_path: &str) -> Result<()> {
        let relative = url_path.strip_prefix("/uploads/").unwrap_or(url_path);
        let file_path = PathBuf::from(upload_dir).join(relative);
        if file_path.exists() {
            fs::remove_file(&file_path).await.map_err(|e| {
                AppError::InternalError(anyhow::anyhow!("Failed to delete file: {}", e))
            })?;
        }
        Ok(())
    }

    pub fn detect_media_type(filename: &str) -> Result<String> {
        let ext = Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" => Ok("image".to_string()),
            "mp4" | "mov" | "avi" | "mkv" | "webm" => Ok("video".to_string()),
            _ => Err(AppError::UnsupportedMediaType(format!(
                "Unsupported file type: .{}",
                ext
            ))),
        }
    }
}
