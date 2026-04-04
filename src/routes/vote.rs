use axum::{extract::{Path, State}, routing::post, Json, Router};
use serde_json::{json, Value};
use crate::{
    error::Result,
    middleware::auth::VerifiedUser,
    models::VoteRequest,
    services::VoteService,
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/post/:id", post(vote_post))
        .route("/comment/:id", post(vote_comment))
        .route("/subcomment/:id", post(vote_sub_comment))
}

async fn vote_post(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>, Json(req): Json<VoteRequest>) -> Result<Json<Value>> {
    let result = VoteService::vote_post(&state.db, &id, &auth.user.id, &req.vote_type).await?;
    Ok(Json(json!({"success": true, "data": result})))
}

async fn vote_comment(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>, Json(req): Json<VoteRequest>) -> Result<Json<Value>> {
    let result = VoteService::vote_comment(&state.db, &id, &auth.user.id, &req.vote_type).await?;
    Ok(Json(json!({"success": true, "data": result})))
}

async fn vote_sub_comment(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>, Json(req): Json<VoteRequest>) -> Result<Json<Value>> {
    let result = VoteService::vote_sub_comment(&state.db, &id, &auth.user.id, &req.vote_type).await?;
    Ok(Json(json!({"success": true, "data": result})))
}
