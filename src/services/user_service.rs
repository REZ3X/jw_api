use sqlx::MySqlPool;

use crate::{
    error::{AppError, Result},
    models::{UserRow, PublicUserResponse},
};

pub struct UserService;

impl UserService {
    pub async fn get_public_profile(pool: &MySqlPool, username: &str) -> Result<PublicUserResponse> {
        let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("User not found".into()))?;
        Ok(PublicUserResponse::from(&user))
    }

    pub async fn set_custom_avatar(pool: &MySqlPool, user_id: &str, url: &str) -> Result<()> {
        sqlx::query("UPDATE users SET custom_avatar_url = ?, use_custom_avatar = TRUE, updated_at = NOW() WHERE id = ?")
            .bind(url).bind(user_id).execute(pool).await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    pub async fn revert_to_google_avatar(pool: &MySqlPool, user_id: &str) -> Result<()> {
        sqlx::query("UPDATE users SET use_custom_avatar = FALSE, updated_at = NOW() WHERE id = ?")
            .bind(user_id).execute(pool).await.map_err(AppError::DatabaseError)?;
        Ok(())
    }
}
