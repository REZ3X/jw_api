use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PostRow {
    pub id: String,
    pub user_id: String,
    pub caption: String,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub is_private: bool,
    pub department: String,
    pub status: String,
    pub upvote_count: i32,
    pub downvote_count: i32,
    pub comment_count: i32,
    pub is_edited: bool,
    pub editable_until: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}


#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct PostMediaRow {
    pub id: String,
    pub post_id: String,
    pub media_url: String,
    pub media_type: String,
    pub display_order: i8,
    pub created_at: NaiveDateTime,
}


#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct PostTagRow {
    pub id: String,
    pub post_id: String,
    pub tag: String,
}


#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct PostStatusHistoryRow {
    pub id: String,
    pub post_id: String,
    pub user_id: String,
    pub old_status: String,
    pub new_status: String,
    pub note: Option<String>,
    pub created_at: NaiveDateTime,
}


#[derive(Debug, Serialize)]
pub struct PostResponse {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub user_name: String,
    pub user_avatar: Option<String>,
    pub user_role: String,
    pub caption: String,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub is_private: bool,
    pub department: String,
    pub status: String,
    pub upvote_count: i32,
    pub downvote_count: i32,
    pub comment_count: i32,
    pub is_edited: bool,
    pub media: Vec<MediaResponse>,
    pub tags: Vec<String>,
    pub my_vote: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct MediaResponse {
    pub id: String,
    pub media_url: String,
    pub media_type: String,
    pub display_order: i8,
}

#[derive(Debug, Serialize)]
pub struct PostListResponse {
    pub posts: Vec<PostResponse>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}


#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub caption: String,
    pub location: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub is_private: Option<bool>,
    pub department: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub caption: Option<String>,
    pub location: Option<String>,
    pub is_private: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PostFilterParams {
    pub department: Option<String>,
    pub status: Option<String>,
    pub tag: Option<String>,
    pub search: Option<String>,
    pub sort: Option<String>,  // recent, most_upvoted, most_discussed
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostStatusRequest {
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DepartmentRespondRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ClassifyDepartmentRequest {
    pub caption: String,
}
