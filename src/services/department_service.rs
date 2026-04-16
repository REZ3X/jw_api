use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::{PostRow, PostStatusHistoryRow, UpdatePostStatusRequest, UserRow},
};

pub struct DepartmentService;

impl DepartmentService {
    pub async fn get_dashboard_stats(pool: &MySqlPool, department: &str) -> Result<serde_json::Value> {
        let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE department = ?")
            .bind(department).fetch_one(pool).await.map_err(AppError::DatabaseError)?;

        let pending: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE department = ? AND status = 'pending'")
            .bind(department).fetch_one(pool).await.map_err(AppError::DatabaseError)?;

        let in_progress: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE department = ? AND status = 'in_progress'")
            .bind(department).fetch_one(pool).await.map_err(AppError::DatabaseError)?;

        let closed: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE department = ? AND status = 'closed'")
            .bind(department).fetch_one(pool).await.map_err(AppError::DatabaseError)?;

        let resolution_rate = if total.0 > 0 {
            (closed.0 as f64 / total.0 as f64) * 100.0
        } else {
            0.0
        };

        Ok(serde_json::json!({
            "department": department,
            "total_posts": total.0,
            "pending_posts": pending.0,
            "in_progress_posts": in_progress.0,
            "closed_posts": closed.0,
            "resolution_rate": format!("{:.1}%", resolution_rate),
        }))
    }

    pub async fn update_post_status(
        pool: &MySqlPool,
        post_id: &str,
        gov_user: &UserRow,
        req: &UpdatePostStatusRequest,
    ) -> Result<()> {
        let post: PostRow = sqlx::query_as("SELECT * FROM posts WHERE id = ?")
            .bind(post_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Post not found".into()))?;


        if post.department != gov_user.role {
            return Err(AppError::Forbidden("You can only manage reports matched to your department".into()));
        }


        let valid = matches!(
            (post.status.as_str(), req.status.as_str()),
            ("pending", "in_progress") | ("pending", "closed") | ("in_progress", "closed")
        );

        if !valid {
            return Err(AppError::ValidationError(format!(
                "Invalid status transition: {} → {}",
                post.status, req.status
            )));
        }


        sqlx::query("UPDATE posts SET status = ?, updated_at = NOW() WHERE id = ?")
            .bind(&req.status)
            .bind(post_id)
            .execute(pool)
            .await
            .map_err(AppError::DatabaseError)?;


        let history_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO post_status_history (id, post_id, user_id, old_status, new_status, note) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&history_id)
        .bind(post_id)
        .bind(&gov_user.id)
        .bind(&post.status)
        .bind(&req.status)
        .bind(&req.note)
        .execute(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        Ok(())
    }

    pub async fn get_status_history(pool: &MySqlPool, post_id: &str) -> Result<Vec<PostStatusHistoryRow>> {
        let rows: Vec<PostStatusHistoryRow> = sqlx::query_as(
            "SELECT * FROM post_status_history WHERE post_id = ? ORDER BY created_at DESC"
        )
        .bind(post_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(rows)
    }
}
