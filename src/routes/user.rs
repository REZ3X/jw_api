use axum::{extract::{Path, Query, State, Multipart}, routing::{get, post, delete}, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::{
    error::{AppError, Result},
    middleware::auth::VerifiedUser,
    models::PostFilterParams,
    services::{UserService, MediaService},
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/search", get(search_users))
        .route("/:username", get(get_public_profile))
        .route("/:username/posts", get(get_user_posts))
        .route("/me/avatar", post(upload_avatar))
        .route("/me/avatar", delete(delete_avatar))
}

#[derive(Deserialize)]
struct SearchParams { q: Option<String>, limit: Option<i64> }

async fn search_users(State(state): State<AppState>, Query(p): Query<SearchParams>) -> Result<Json<Value>> {
    let query = p.q.unwrap_or_default();
    if query.trim().is_empty() {
        return Ok(Json(json!({"success": true, "data": [], "count": 0})));
    }
    let users = UserService::search_users(&state.db, &query, p.limit.unwrap_or(20)).await?;
    let count = users.len();
    Ok(Json(json!({"success": true, "data": users, "count": count})))
}

async fn get_public_profile(State(state): State<AppState>, Path(username): Path<String>) -> Result<Json<Value>> {
    let profile = UserService::get_public_profile(&state.db, &username).await?;
    Ok(Json(json!({"success": true, "data": profile})))
}

async fn get_user_posts(State(state): State<AppState>, Path(username): Path<String>) -> Result<Json<Value>> {
    let user: crate::models::UserRow = sqlx::query_as("SELECT * FROM users WHERE username = ?")
        .bind(&username).fetch_optional(&state.db).await.map_err(AppError::DatabaseError)?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;
    let params = PostFilterParams { user_id: Some(user.id.clone()), department: None, status: None, tag: None, search: None, sort: None, page: Some(1), per_page: Some(20) };
    let result = crate::services::PostService::list_posts(&state.db, &params, None, false).await?;
    Ok(Json(json!({"success": true, "data": result})))
}

async fn upload_avatar(State(state): State<AppState>, auth: VerifiedUser, mut multipart: Multipart) -> Result<Json<Value>> {
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(format!("Multipart error: {}", e)))? {
        let filename = field.file_name().unwrap_or("avatar.jpg").to_string();
        let data = field.bytes().await.map_err(|e| AppError::BadRequest(format!("Read error: {}", e)))?;
        let max = state.config.media.max_image_size_mb * 1024 * 1024;
        let url = MediaService::save_file(&state.config.media.upload_dir, "avatars", &filename, &data, max).await?;
        UserService::set_custom_avatar(&state.db, &auth.user.id, &url).await?;
        return Ok(Json(json!({"success": true, "avatar_url": url})));
    }
    Err(AppError::ValidationError("No file provided".into()))
}

async fn delete_avatar(State(state): State<AppState>, auth: VerifiedUser) -> Result<Json<Value>> {
    UserService::revert_to_google_avatar(&state.db, &auth.user.id).await?;
    Ok(Json(json!({"success": true, "message": "Reverted to Google avatar"})))
}
