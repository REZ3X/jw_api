use axum::{extract::{Path, Query, State, Multipart}, routing::{get, post, put, delete}, Json, Router};
use serde_json::{json, Value};
use crate::{
    error::{AppError, Result},
    middleware::{activity_log::log_activity, auth::{AuthUser, VerifiedUser}},
    models::*,
    services::{PostService, MediaService},
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_post))
        .route("/", get(list_posts))
        .route("/me", get(my_posts))
        .route("/:id", get(get_post))
        .route("/:id", put(update_post))
        .route("/:id", delete(delete_post))
        .route("/:id/classify", post(classify_department))
}

async fn create_post(State(state): State<AppState>, auth: VerifiedUser, mut multipart: Multipart) -> Result<Json<Value>> {
    if is_gov_role(&auth.user.role) { return Err(AppError::Forbidden("Government accounts cannot create posts".into())); }

    let mut caption = String::new();
    let mut location: Option<String> = None;
    let mut lat: Option<f64> = None;
    let mut lng: Option<f64> = None;
    let mut is_private = false;
    let mut department: Option<String> = None;
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "caption" => caption = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?,
            "location" => location = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?),
            "latitude" => lat = field.text().await.ok().and_then(|s| s.parse().ok()),
            "longitude" => lng = field.text().await.ok().and_then(|s| s.parse().ok()),
            "is_private" => is_private = field.text().await.unwrap_or_default() == "true",
            "department" => department = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?),
            "media" => {
                let fname = field.file_name().unwrap_or("file.jpg").to_string();
                let data = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                files.push((fname, data.to_vec()));
            }
            _ => {}
        }
    }

    if caption.trim().is_empty() { return Err(AppError::ValidationError("Caption is required".into())); }
    if files.is_empty() { return Err(AppError::ValidationError("At least 1 media file required".into())); }
    if files.len() > 4 { return Err(AppError::ValidationError("Maximum 4 media files allowed".into())); }

    let dept = match department {
        Some(d) => d,
        None => state.gemini.classify_department(&caption).await.map_err(|e| AppError::InternalError(e.into()))?,
    };

    let req = CreatePostRequest { caption: caption.clone(), location, latitude: lat, longitude: lng, is_private: Some(is_private), department: Some(dept.clone()) };
    let post_id = PostService::create_post(&state.db, &auth.user.id, &req, &dept).await?;

    for (i, (fname, data)) in files.iter().enumerate() {
        let media_type = MediaService::detect_media_type(fname)?;
        let max = if media_type == "video" { state.config.media.max_video_size_mb * 1024 * 1024 } else { state.config.media.max_image_size_mb * 1024 * 1024 };
        let url = MediaService::save_file(&state.config.media.upload_dir, "posts", fname, data, max).await?;
        PostService::add_media(&state.db, &post_id, &url, &media_type, i as i8).await?;
    }

    log_activity(&state.db, &auth.user.id, "create", "post", "post", Some(&post_id), None, None).await;
    let post = PostService::get_post(&state.db, &post_id, Some(&auth.user.id)).await?;
    Ok(Json(json!({"success": true, "data": post})))
}

async fn list_posts(State(state): State<AppState>, auth: Option<AuthUser>, Query(params): Query<PostFilterParams>) -> Result<Json<Value>> {
    let viewer = auth.as_ref().map(|a| a.user.id.as_str());
    let result = PostService::list_posts(&state.db, &params, viewer, false).await?;
    Ok(Json(json!({"success": true, "data": result})))
}

async fn my_posts(State(state): State<AppState>, auth: VerifiedUser, Query(params): Query<PostFilterParams>) -> Result<Json<Value>> {
    let mut p = params;
    p.user_id = Some(auth.user.id.clone());
    let result = PostService::list_posts(&state.db, &p, Some(&auth.user.id), true).await?;
    Ok(Json(json!({"success": true, "data": result})))
}

async fn get_post(State(state): State<AppState>, auth: Option<AuthUser>, Path(id): Path<String>) -> Result<Json<Value>> {
    let post = PostService::get_post(&state.db, &id, auth.as_ref().map(|a| a.user.id.as_str())).await?;
    if post.is_private {
        let viewer_id = auth.as_ref().map(|a| a.user.id.as_str()).unwrap_or("");
        let is_owner = viewer_id == post.user_id;
        let is_matched_dept = auth.as_ref().map(|a| a.user.role.as_str() == post.department).unwrap_or(false);
        let is_dev = auth.as_ref().map(|a| a.user.role == "dev").unwrap_or(false);
        if !is_owner && !is_matched_dept && !is_dev { return Err(AppError::NotFound("Post not found".into())); }
    }
    Ok(Json(json!({"success": true, "data": post})))
}

async fn update_post(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>, Json(req): Json<UpdatePostRequest>) -> Result<Json<Value>> {
    PostService::update_post(&state.db, &id, &auth.user.id, &req).await?;
    log_activity(&state.db, &auth.user.id, "update", "post", "post", Some(&id), None, None).await;
    let post = PostService::get_post(&state.db, &id, Some(&auth.user.id)).await?;
    Ok(Json(json!({"success": true, "data": post})))
}

async fn delete_post(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>) -> Result<Json<Value>> {
    PostService::delete_post(&state.db, &id, &auth.user.id).await?;
    log_activity(&state.db, &auth.user.id, "delete", "post", "post", Some(&id), None, None).await;
    Ok(Json(json!({"success": true, "message": "Post deleted"})))
}

async fn classify_department(State(state): State<AppState>, _auth: VerifiedUser, Json(req): Json<ClassifyDepartmentRequest>) -> Result<Json<Value>> {
    let dept = state.gemini.classify_department(&req.caption).await.map_err(|e| AppError::InternalError(e.into()))?;
    Ok(Json(json!({"success": true, "data": {"department": dept}})))
}
