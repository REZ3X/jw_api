use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::{
        CommentRow, SubCommentRow, CommentResponse, SubCommentResponse,
        CreateCommentRequest, UpdateCommentRequest, CreateSubCommentRequest,
        CommentFilterParams, SubCommentFilterParams,
    },
};

pub struct CommentService;

impl CommentService {
    pub async fn create_comment(
        pool: &MySqlPool,
        post_id: &str,
        user_id: &str,
        req: &CreateCommentRequest,
        is_official: bool,
        official_image_url: Option<&str>,
    ) -> Result<CommentResponse> {

        let post_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE id = ?")
            .bind(post_id)
            .fetch_one(pool)
            .await
            .map_err(AppError::DatabaseError)?;

        if post_exists.0 == 0 {
            return Err(AppError::NotFound("Post not found".into()));
        }

        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO comments (id, post_id, user_id, content, is_official, official_image_url)
               VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(post_id)
        .bind(user_id)
        .bind(&req.content)
        .bind(is_official)
        .bind(official_image_url)
        .execute(pool)
        .await
        .map_err(AppError::DatabaseError)?;


        sqlx::query("UPDATE posts SET comment_count = comment_count + 1 WHERE id = ?")
            .bind(post_id)
            .execute(pool)
            .await
            .ok();

        Self::get_comment(pool, &id, None).await
    }

    pub async fn list_comments(
        pool: &MySqlPool,
        post_id: &str,
        params: &CommentFilterParams,
        viewer_user_id: Option<&str>,
    ) -> Result<Vec<CommentResponse>> {
        let page = params.page.unwrap_or(1).max(1);
        let per_page = params.per_page.unwrap_or(20).min(50);
        let offset = (page - 1) * per_page;

        let order_by = match params.sort.as_deref() {
            Some("most_upvote") => "c.is_pinned DESC, c.upvote_count DESC",
            Some("most_downvote") => "c.is_pinned DESC, c.downvote_count DESC",
            Some("popular") => "c.is_pinned DESC, (c.upvote_count - c.downvote_count) DESC",
            _ => "c.is_pinned DESC, c.created_at DESC",
        };

        let sql = format!(
            "SELECT c.* FROM comments c WHERE c.post_id = ? ORDER BY {} LIMIT ? OFFSET ?",
            order_by
        );

        let rows: Vec<CommentRow> = sqlx::query_as(&sql)
            .bind(post_id)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(AppError::DatabaseError)?;

        let mut results = Vec::new();
        for row in &rows {
            results.push(Self::build_comment_response(pool, row, viewer_user_id).await?);
        }
        Ok(results)
    }

    pub async fn get_comment(
        pool: &MySqlPool,
        comment_id: &str,
        viewer_user_id: Option<&str>,
    ) -> Result<CommentResponse> {
        let row: CommentRow = sqlx::query_as("SELECT * FROM comments WHERE id = ?")
            .bind(comment_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Comment not found".into()))?;

        Self::build_comment_response(pool, &row, viewer_user_id).await
    }

    pub async fn update_comment(
        pool: &MySqlPool,
        comment_id: &str,
        user_id: &str,
        req: &UpdateCommentRequest,
    ) -> Result<()> {
        let result = sqlx::query(
            "UPDATE comments SET content = ?, is_edited = TRUE, updated_at = NOW() WHERE id = ? AND user_id = ?",
        )
        .bind(&req.content)
        .bind(comment_id)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Comment not found".into()));
        }
        Ok(())
    }

    pub async fn delete_comment(pool: &MySqlPool, comment_id: &str, user_id: &str) -> Result<()> {

        let comment: Option<CommentRow> = sqlx::query_as("SELECT * FROM comments WHERE id = ? AND user_id = ?")
            .bind(comment_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?;

        let comment = comment.ok_or_else(|| AppError::NotFound("Comment not found".into()))?;

        sqlx::query("DELETE FROM comments WHERE id = ?")
            .bind(comment_id)
            .execute(pool)
            .await
            .map_err(AppError::DatabaseError)?;


        sqlx::query("UPDATE posts SET comment_count = GREATEST(comment_count - 1, 0) WHERE id = ?")
            .bind(&comment.post_id)
            .execute(pool)
            .await
            .ok();

        Ok(())
    }

    pub async fn toggle_pin(
        pool: &MySqlPool,
        comment_id: &str,
        post_owner_id: &str,
    ) -> Result<bool> {
        let comment: CommentRow = sqlx::query_as("SELECT * FROM comments WHERE id = ?")
            .bind(comment_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Comment not found".into()))?;


        let post_owner: (String,) = sqlx::query_as("SELECT user_id FROM posts WHERE id = ?")
            .bind(&comment.post_id)
            .fetch_one(pool)
            .await
            .map_err(AppError::DatabaseError)?;

        if post_owner.0 != post_owner_id {
            return Err(AppError::Forbidden("Only post owner can pin comments".into()));
        }

        let new_pinned = !comment.is_pinned;
        sqlx::query("UPDATE comments SET is_pinned = ? WHERE id = ?")
            .bind(new_pinned)
            .bind(comment_id)
            .execute(pool)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(new_pinned)
    }

    pub async fn create_sub_comment(
        pool: &MySqlPool,
        comment_id: &str,
        user_id: &str,
        req: &CreateSubCommentRequest,
        is_official: bool,
    ) -> Result<SubCommentResponse> {

        let comment: CommentRow = sqlx::query_as("SELECT * FROM comments WHERE id = ?")
            .bind(comment_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Comment not found".into()))?;

        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO sub_comments (id, comment_id, user_id, reply_to_user_id, content, is_official)
               VALUES (?, ?, ?, ?, ?, ?)"#,
        )
        .bind(&id)
        .bind(comment_id)
        .bind(user_id)
        .bind(&req.reply_to_user_id)
        .bind(&req.content)
        .bind(is_official)
        .execute(pool)
        .await
        .map_err(AppError::DatabaseError)?;


        sqlx::query("UPDATE comments SET reply_count = reply_count + 1 WHERE id = ?")
            .bind(comment_id)
            .execute(pool)
            .await
            .ok();


        sqlx::query("UPDATE posts SET comment_count = comment_count + 1 WHERE id = ?")
            .bind(&comment.post_id)
            .execute(pool)
            .await
            .ok();

        Self::get_sub_comment(pool, &id, None).await
    }

    pub async fn list_sub_comments(
        pool: &MySqlPool,
        comment_id: &str,
        params: &SubCommentFilterParams,
        viewer_user_id: Option<&str>,
    ) -> Result<Vec<SubCommentResponse>> {
        let page = params.page.unwrap_or(1).max(1);
        let per_page = params.per_page.unwrap_or(20).min(50);
        let offset = (page - 1) * per_page;

        let rows: Vec<SubCommentRow> = sqlx::query_as(
            "SELECT * FROM sub_comments WHERE comment_id = ? ORDER BY created_at ASC LIMIT ? OFFSET ?"
        )
        .bind(comment_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        let mut results = Vec::new();
        for row in &rows {
            results.push(Self::build_sub_comment_response(pool, row, viewer_user_id).await?);
        }
        Ok(results)
    }

    async fn get_sub_comment(
        pool: &MySqlPool,
        sub_comment_id: &str,
        viewer_user_id: Option<&str>,
    ) -> Result<SubCommentResponse> {
        let row: SubCommentRow = sqlx::query_as("SELECT * FROM sub_comments WHERE id = ?")
            .bind(sub_comment_id)
            .fetch_optional(pool)
            .await
            .map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Reply not found".into()))?;

        Self::build_sub_comment_response(pool, &row, viewer_user_id).await
    }

    // ── Builders ────────────────────────────────────────────

    async fn build_comment_response(
        pool: &MySqlPool,
        row: &CommentRow,
        viewer_user_id: Option<&str>,
    ) -> Result<CommentResponse> {
        let user: (String, String, Option<String>, Option<String>, bool, String) = sqlx::query_as(
            "SELECT username, name, avatar_url, custom_avatar_url, use_custom_avatar, role FROM users WHERE id = ?"
        )
        .bind(&row.user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        let avatar = if user.4 { user.3.or(user.2.clone()) } else { user.2.clone() };

        let my_vote = if let Some(uid) = viewer_user_id {
            sqlx::query_as::<_, (String,)>("SELECT vote_type FROM comment_votes WHERE comment_id = ? AND user_id = ?")
                .bind(&row.id)
                .bind(uid)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .map(|v| v.0)
        } else {
            None
        };

        Ok(CommentResponse {
            id: row.id.clone(),
            post_id: row.post_id.clone(),
            user_id: row.user_id.clone(),
            username: user.0,
            user_name: user.1,
            user_avatar: avatar,
            user_role: user.5,
            content: row.content.clone(),
            is_edited: row.is_edited,
            is_pinned: row.is_pinned,
            is_official: row.is_official,
            official_image_url: row.official_image_url.clone(),
            upvote_count: row.upvote_count,
            downvote_count: row.downvote_count,
            reply_count: row.reply_count,
            my_vote,
            created_at: row.created_at.to_string(),
        })
    }

    async fn build_sub_comment_response(
        pool: &MySqlPool,
        row: &SubCommentRow,
        viewer_user_id: Option<&str>,
    ) -> Result<SubCommentResponse> {
        let user: (String, String, Option<String>, Option<String>, bool, String) = sqlx::query_as(
            "SELECT username, name, avatar_url, custom_avatar_url, use_custom_avatar, role FROM users WHERE id = ?"
        )
        .bind(&row.user_id)
        .fetch_one(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        let avatar = if user.4 { user.3.or(user.2.clone()) } else { user.2.clone() };

        let reply_to_username = if let Some(ref rtu) = row.reply_to_user_id {
            sqlx::query_as::<_, (String,)>("SELECT username FROM users WHERE id = ?")
                .bind(rtu)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .map(|u| u.0)
        } else {
            None
        };

        let my_vote = if let Some(uid) = viewer_user_id {
            sqlx::query_as::<_, (String,)>("SELECT vote_type FROM sub_comment_votes WHERE sub_comment_id = ? AND user_id = ?")
                .bind(&row.id)
                .bind(uid)
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
                .map(|v| v.0)
        } else {
            None
        };

        Ok(SubCommentResponse {
            id: row.id.clone(),
            comment_id: row.comment_id.clone(),
            user_id: row.user_id.clone(),
            username: user.0,
            user_name: user.1,
            user_avatar: avatar,
            user_role: user.5,
            reply_to_user_id: row.reply_to_user_id.clone(),
            reply_to_username,
            content: row.content.clone(),
            is_edited: row.is_edited,
            is_official: row.is_official,
            upvote_count: row.upvote_count,
            downvote_count: row.downvote_count,
            my_vote,
            created_at: row.created_at.to_string(),
        })
    }
}
