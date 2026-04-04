use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostVoteRow {
    pub id: String,
    pub post_id: String,
    pub user_id: String,
    pub vote_type: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CommentVoteRow {
    pub id: String,
    pub comment_id: String,
    pub user_id: String,
    pub vote_type: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubCommentVoteRow {
    pub id: String,
    pub sub_comment_id: String,
    pub user_id: String,
    pub vote_type: String,
    pub created_at: NaiveDateTime,
}


#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    pub vote_type: String, // "up" or "down"
}

#[derive(Debug, Serialize)]
pub struct VoteResponse {
    pub voted: bool,
    pub vote_type: Option<String>,
    pub upvote_count: i32,
    pub downvote_count: i32,
}
