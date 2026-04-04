use axum::{extract::{Path, Query, State}, routing::{get, post, put, delete}, Json, Router};
use serde_json::{json, Value};
use crate::{
    error::{AppError, Result},
    middleware::auth::VerifiedUser,
    models::*,
    services::CommentService,
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/post/:post_id", post(create_comment))
        .route("/post/:post_id", get(list_comments))
        .route("/:id", put(update_comment))
        .route("/:id", delete(delete_comment))
        .route("/:id/pin", post(toggle_pin))
        .route("/:id/replies", post(create_reply))
        .route("/:id/replies", get(list_replies))
}

async fn create_comment(State(state): State<AppState>, auth: VerifiedUser, Path(post_id): Path<String>, Json(req): Json<CreateCommentRequest>) -> Result<Json<Value>> {
    if req.content.trim().is_empty() { return Err(AppError::ValidationError("Content required".into())); }
    let is_official = is_gov_role(&auth.user.role);
    let comment = CommentService::create_comment(&state.db, &post_id, &auth.user.id, &req, is_official, None).await?;
    Ok(Json(json!({"success": true, "data": comment})))
}

async fn list_comments(State(state): State<AppState>, Path(post_id): Path<String>, Query(params): Query<CommentFilterParams>) -> Result<Json<Value>> {
    let comments = CommentService::list_comments(&state.db, &post_id, &params, None).await?;
    Ok(Json(json!({"success": true, "data": comments, "count": comments.len()})))
}

async fn update_comment(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>, Json(req): Json<UpdateCommentRequest>) -> Result<Json<Value>> {
    CommentService::update_comment(&state.db, &id, &auth.user.id, &req).await?;
    Ok(Json(json!({"success": true, "message": "Comment updated"})))
}

async fn delete_comment(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>) -> Result<Json<Value>> {
    CommentService::delete_comment(&state.db, &id, &auth.user.id).await?;
    Ok(Json(json!({"success": true, "message": "Comment deleted"})))
}

async fn toggle_pin(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>) -> Result<Json<Value>> {
    let pinned = CommentService::toggle_pin(&state.db, &id, &auth.user.id).await?;
    Ok(Json(json!({"success": true, "is_pinned": pinned})))
}

async fn create_reply(State(state): State<AppState>, auth: VerifiedUser, Path(comment_id): Path<String>, Json(req): Json<CreateSubCommentRequest>) -> Result<Json<Value>> {
    if req.content.trim().is_empty() { return Err(AppError::ValidationError("Content required".into())); }
    let is_official = is_gov_role(&auth.user.role);
    let reply = CommentService::create_sub_comment(&state.db, &comment_id, &auth.user.id, &req, is_official).await?;
    Ok(Json(json!({"success": true, "data": reply})))
}

async fn list_replies(State(state): State<AppState>, Path(comment_id): Path<String>, Query(params): Query<SubCommentFilterParams>) -> Result<Json<Value>> {
    let replies = CommentService::list_sub_comments(&state.db, &comment_id, &params, None).await?;
    Ok(Json(json!({"success": true, "data": replies, "count": replies.len()})))
}
