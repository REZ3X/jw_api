use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    error::{AppError, Result},
    models::VoteResponse,
};

pub struct VoteService;

impl VoteService {
    pub async fn vote_post(
        pool: &MySqlPool,
        post_id: &str,
        user_id: &str,
        vote_type: &str,
    ) -> Result<VoteResponse> {
        Self::validate_vote_type(vote_type)?;

        let existing: Option<(String, String)> = sqlx::query_as(
            "SELECT id, vote_type FROM post_votes WHERE post_id = ? AND user_id = ?"
        )
        .bind(post_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        match existing {
            Some((id, existing_type)) if existing_type == vote_type => {
                // Same vote — remove it (toggle off)
                sqlx::query("DELETE FROM post_votes WHERE id = ?")
                    .bind(&id)
                    .execute(pool)
                    .await
                    .map_err(AppError::DatabaseError)?;

                Self::recalculate_post_votes(pool, post_id).await?;
                let counts = Self::get_post_vote_counts(pool, post_id).await?;
                Ok(VoteResponse { voted: false, vote_type: None, upvote_count: counts.0, downvote_count: counts.1 })
            }
            Some((id, _)) => {
                // Different vote — switch it
                sqlx::query("UPDATE post_votes SET vote_type = ?, created_at = NOW() WHERE id = ?")
                    .bind(vote_type)
                    .bind(&id)
                    .execute(pool)
                    .await
                    .map_err(AppError::DatabaseError)?;

                Self::recalculate_post_votes(pool, post_id).await?;
                let counts = Self::get_post_vote_counts(pool, post_id).await?;
                Ok(VoteResponse { voted: true, vote_type: Some(vote_type.to_string()), upvote_count: counts.0, downvote_count: counts.1 })
            }
            None => {
                // New vote
                let id = Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO post_votes (id, post_id, user_id, vote_type) VALUES (?, ?, ?, ?)")
                    .bind(&id)
                    .bind(post_id)
                    .bind(user_id)
                    .bind(vote_type)
                    .execute(pool)
                    .await
                    .map_err(AppError::DatabaseError)?;

                Self::recalculate_post_votes(pool, post_id).await?;
                let counts = Self::get_post_vote_counts(pool, post_id).await?;
                Ok(VoteResponse { voted: true, vote_type: Some(vote_type.to_string()), upvote_count: counts.0, downvote_count: counts.1 })
            }
        }
    }

    pub async fn vote_comment(
        pool: &MySqlPool,
        comment_id: &str,
        user_id: &str,
        vote_type: &str,
    ) -> Result<VoteResponse> {
        Self::validate_vote_type(vote_type)?;

        let existing: Option<(String, String)> = sqlx::query_as(
            "SELECT id, vote_type FROM comment_votes WHERE comment_id = ? AND user_id = ?"
        )
        .bind(comment_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        match existing {
            Some((id, existing_type)) if existing_type == vote_type => {
                sqlx::query("DELETE FROM comment_votes WHERE id = ?").bind(&id).execute(pool).await.map_err(AppError::DatabaseError)?;
                Self::recalculate_comment_votes(pool, comment_id).await?;
                let counts = Self::get_comment_vote_counts(pool, comment_id).await?;
                Ok(VoteResponse { voted: false, vote_type: None, upvote_count: counts.0, downvote_count: counts.1 })
            }
            Some((id, _)) => {
                sqlx::query("UPDATE comment_votes SET vote_type = ? WHERE id = ?").bind(vote_type).bind(&id).execute(pool).await.map_err(AppError::DatabaseError)?;
                Self::recalculate_comment_votes(pool, comment_id).await?;
                let counts = Self::get_comment_vote_counts(pool, comment_id).await?;
                Ok(VoteResponse { voted: true, vote_type: Some(vote_type.to_string()), upvote_count: counts.0, downvote_count: counts.1 })
            }
            None => {
                let id = Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO comment_votes (id, comment_id, user_id, vote_type) VALUES (?, ?, ?, ?)")
                    .bind(&id).bind(comment_id).bind(user_id).bind(vote_type)
                    .execute(pool).await.map_err(AppError::DatabaseError)?;
                Self::recalculate_comment_votes(pool, comment_id).await?;
                let counts = Self::get_comment_vote_counts(pool, comment_id).await?;
                Ok(VoteResponse { voted: true, vote_type: Some(vote_type.to_string()), upvote_count: counts.0, downvote_count: counts.1 })
            }
        }
    }

    pub async fn vote_sub_comment(
        pool: &MySqlPool,
        sub_comment_id: &str,
        user_id: &str,
        vote_type: &str,
    ) -> Result<VoteResponse> {
        Self::validate_vote_type(vote_type)?;

        let existing: Option<(String, String)> = sqlx::query_as(
            "SELECT id, vote_type FROM sub_comment_votes WHERE sub_comment_id = ? AND user_id = ?"
        )
        .bind(sub_comment_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        match existing {
            Some((id, existing_type)) if existing_type == vote_type => {
                sqlx::query("DELETE FROM sub_comment_votes WHERE id = ?").bind(&id).execute(pool).await.map_err(AppError::DatabaseError)?;
                Self::recalculate_sub_comment_votes(pool, sub_comment_id).await?;
                let counts = Self::get_sub_comment_vote_counts(pool, sub_comment_id).await?;
                Ok(VoteResponse { voted: false, vote_type: None, upvote_count: counts.0, downvote_count: counts.1 })
            }
            Some((id, _)) => {
                sqlx::query("UPDATE sub_comment_votes SET vote_type = ? WHERE id = ?").bind(vote_type).bind(&id).execute(pool).await.map_err(AppError::DatabaseError)?;
                Self::recalculate_sub_comment_votes(pool, sub_comment_id).await?;
                let counts = Self::get_sub_comment_vote_counts(pool, sub_comment_id).await?;
                Ok(VoteResponse { voted: true, vote_type: Some(vote_type.to_string()), upvote_count: counts.0, downvote_count: counts.1 })
            }
            None => {
                let id = Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO sub_comment_votes (id, sub_comment_id, user_id, vote_type) VALUES (?, ?, ?, ?)")
                    .bind(&id).bind(sub_comment_id).bind(user_id).bind(vote_type)
                    .execute(pool).await.map_err(AppError::DatabaseError)?;
                Self::recalculate_sub_comment_votes(pool, sub_comment_id).await?;
                let counts = Self::get_sub_comment_vote_counts(pool, sub_comment_id).await?;
                Ok(VoteResponse { voted: true, vote_type: Some(vote_type.to_string()), upvote_count: counts.0, downvote_count: counts.1 })
            }
        }
    }

    // ── Helpers ──────────────────────────────────────────────

    fn validate_vote_type(vote_type: &str) -> Result<()> {
        if vote_type != "up" && vote_type != "down" {
            return Err(AppError::ValidationError("vote_type must be 'up' or 'down'".into()));
        }
        Ok(())
    }

    async fn recalculate_post_votes(pool: &MySqlPool, post_id: &str) -> Result<()> {
        sqlx::query("UPDATE posts SET upvote_count = (SELECT COUNT(*) FROM post_votes WHERE post_id = ? AND vote_type = 'up'), downvote_count = (SELECT COUNT(*) FROM post_votes WHERE post_id = ? AND vote_type = 'down') WHERE id = ?")
            .bind(post_id).bind(post_id).bind(post_id)
            .execute(pool).await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    async fn recalculate_comment_votes(pool: &MySqlPool, comment_id: &str) -> Result<()> {
        sqlx::query("UPDATE comments SET upvote_count = (SELECT COUNT(*) FROM comment_votes WHERE comment_id = ? AND vote_type = 'up'), downvote_count = (SELECT COUNT(*) FROM comment_votes WHERE comment_id = ? AND vote_type = 'down') WHERE id = ?")
            .bind(comment_id).bind(comment_id).bind(comment_id)
            .execute(pool).await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    async fn recalculate_sub_comment_votes(pool: &MySqlPool, sub_comment_id: &str) -> Result<()> {
        sqlx::query("UPDATE sub_comments SET upvote_count = (SELECT COUNT(*) FROM sub_comment_votes WHERE sub_comment_id = ? AND vote_type = 'up'), downvote_count = (SELECT COUNT(*) FROM sub_comment_votes WHERE sub_comment_id = ? AND vote_type = 'down') WHERE id = ?")
            .bind(sub_comment_id).bind(sub_comment_id).bind(sub_comment_id)
            .execute(pool).await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    async fn get_post_vote_counts(pool: &MySqlPool, post_id: &str) -> Result<(i32, i32)> {
        let row: (i32, i32) = sqlx::query_as("SELECT upvote_count, downvote_count FROM posts WHERE id = ?")
            .bind(post_id).fetch_one(pool).await.map_err(AppError::DatabaseError)?;
        Ok(row)
    }

    async fn get_comment_vote_counts(pool: &MySqlPool, comment_id: &str) -> Result<(i32, i32)> {
        let row: (i32, i32) = sqlx::query_as("SELECT upvote_count, downvote_count FROM comments WHERE id = ?")
            .bind(comment_id).fetch_one(pool).await.map_err(AppError::DatabaseError)?;
        Ok(row)
    }

    async fn get_sub_comment_vote_counts(pool: &MySqlPool, sub_comment_id: &str) -> Result<(i32, i32)> {
        let row: (i32, i32) = sqlx::query_as("SELECT upvote_count, downvote_count FROM sub_comments WHERE id = ?")
            .bind(sub_comment_id).fetch_one(pool).await.map_err(AppError::DatabaseError)?;
        Ok(row)
    }
}
