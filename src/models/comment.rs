use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CommentRow {
    pub id: String,
    pub post_id: String,
    pub user_id: String,
    pub content: String,
    pub is_edited: bool,
    pub is_pinned: bool,
    pub is_official: bool,
    pub official_image_url: Option<String>,
    pub upvote_count: i32,
    pub downvote_count: i32,
    pub reply_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}


#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubCommentRow {
    pub id: String,
    pub comment_id: String,
    pub user_id: String,
    pub reply_to_user_id: Option<String>,
    pub content: String,
    pub is_edited: bool,
    pub is_official: bool,
    pub upvote_count: i32,
    pub downvote_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}


#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub id: String,
    pub post_id: String,
    pub user_id: String,
    pub username: String,
    pub user_name: String,
    pub user_avatar: Option<String>,
    pub user_role: String,
    pub content: String,
    pub is_edited: bool,
    pub is_pinned: bool,
    pub is_official: bool,
    pub official_image_url: Option<String>,
    pub upvote_count: i32,
    pub downvote_count: i32,
    pub reply_count: i32,
    pub my_vote: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct SubCommentResponse {
    pub id: String,
    pub comment_id: String,
    pub user_id: String,
    pub username: String,
    pub user_name: String,
    pub user_avatar: Option<String>,
    pub user_role: String,
    pub reply_to_user_id: Option<String>,
    pub reply_to_username: Option<String>,
    pub content: String,
    pub is_edited: bool,
    pub is_official: bool,
    pub upvote_count: i32,
    pub downvote_count: i32,
    pub my_vote: Option<String>,
    pub created_at: String,
}


#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCommentRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSubCommentRequest {
    pub content: String,
    pub reply_to_user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommentFilterParams {
    pub sort: Option<String>,  // recent, most_upvote, most_downvote, popular
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SubCommentFilterParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
