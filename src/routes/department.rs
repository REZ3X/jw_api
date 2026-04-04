use axum::{extract::{Path, Query, State, Multipart}, routing::{get, post, put}, Json, Router};
use serde_json::{json, Value};
use crate::{
    error::{AppError, Result},
    middleware::{activity_log::log_activity, auth::GovUser},
    models::*,
    services::{DepartmentService, PostService, CommentService, MediaService},
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(dashboard))
        .route("/posts", get(department_posts))
        .route("/posts/:id/status", put(update_status))
        .route("/posts/:id/respond", post(respond_to_post))
        .route("/all-posts", get(all_department_posts))
}

async fn dashboard(State(state): State<AppState>, auth: GovUser) -> Result<Json<Value>> {
    let stats = DepartmentService::get_dashboard_stats(&state.db, &auth.user.role).await?;
    Ok(Json(json!({"success": true, "data": stats})))
}

async fn department_posts(State(state): State<AppState>, auth: GovUser, Query(params): Query<PostFilterParams>) -> Result<Json<Value>> {
    let mut p = params;
    p.department = Some(auth.user.role.clone());
    let result = PostService::list_posts(&state.db, &p, Some(&auth.user.id), true).await?;
    Ok(Json(json!({"success": true, "data": result})))
}

async fn update_status(State(state): State<AppState>, auth: GovUser, Path(id): Path<String>, Json(req): Json<UpdatePostStatusRequest>) -> Result<Json<Value>> {
    DepartmentService::update_post_status(&state.db, &id, &auth.user, &req).await?;
    log_activity(&state.db, &auth.user.id, "update", "department", "post_status", Some(&id), Some(&format!("status: {}", req.status)), None).await;

    // Send email notification to post owner
    let post: crate::models::PostRow = sqlx::query_as("SELECT * FROM posts WHERE id = ?").bind(&id).fetch_one(&state.db).await.map_err(AppError::DatabaseError)?;
    let owner: crate::models::UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(&post.user_id).fetch_one(&state.db).await.map_err(AppError::DatabaseError)?;
    let _ = state.email.send_department_response_notification(&owner.email, &owner.name, &auth.user.role, &post.caption).await;

    Ok(Json(json!({"success": true, "message": "Status updated"})))
}

async fn respond_to_post(State(state): State<AppState>, auth: GovUser, Path(id): Path<String>, mut multipart: Multipart) -> Result<Json<Value>> {
    // Verify department match
    let post: crate::models::PostRow = sqlx::query_as("SELECT * FROM posts WHERE id = ?").bind(&id).fetch_optional(&state.db).await.map_err(AppError::DatabaseError)?
        .ok_or_else(|| AppError::NotFound("Post not found".into()))?;
    if post.department != auth.user.role { return Err(AppError::Forbidden("Department mismatch".into())); }

    let mut content = String::new();
    let mut image_url: Option<String> = None;
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "content" => content = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?,
            "image" => {
                let fname = field.file_name().unwrap_or("response.jpg").to_string();
                let data = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                let max = state.config.media.max_image_size_mb * 1024 * 1024;
                let url = MediaService::save_file(&state.config.media.upload_dir, "comments", &fname, &data, max).await?;
                image_url = Some(url);
            }
            _ => {}
        }
    }
    if content.trim().is_empty() { return Err(AppError::ValidationError("Response content required".into())); }

    let req = CreateCommentRequest { content };
    let comment = CommentService::create_comment(&state.db, &id, &auth.user.id, &req, true, image_url.as_deref()).await?;
    log_activity(&state.db, &auth.user.id, "create", "department", "official_response", Some(&id), None, None).await;

    // Email notification
    let owner: crate::models::UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(&post.user_id).fetch_one(&state.db).await.map_err(AppError::DatabaseError)?;
    let _ = state.email.send_department_response_notification(&owner.email, &owner.name, &auth.user.role, &post.caption).await;

    Ok(Json(json!({"success": true, "data": comment})))
}

async fn all_department_posts(State(state): State<AppState>, auth: GovUser, Query(params): Query<PostFilterParams>) -> Result<Json<Value>> {
    if auth.user.role != "city_major_gov" { return Err(AppError::Forbidden("Only city_major_gov can view all department posts".into())); }
    let result = PostService::list_posts(&state.db, &params, Some(&auth.user.id), true).await?;
    Ok(Json(json!({"success": true, "data": result})))
}
