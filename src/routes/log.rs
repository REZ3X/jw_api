use axum::{extract::{Query, State}, routing::get, Json, Router};
use serde_json::{json, Value};
use crate::{
    error::{AppError, Result},
    middleware::auth::DevUser,
    models::{AuthLogRow, ActivityLogRow, LogFilterParams},
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth", get(auth_logs))
        .route("/activity", get(activity_logs))
}

async fn auth_logs(State(state): State<AppState>, _auth: DevUser, Query(p): Query<LogFilterParams>) -> Result<Json<Value>> {
    let page = p.page.unwrap_or(1).max(1);
    let per_page = p.per_page.unwrap_or(50).min(100);
    let offset = (page - 1) * per_page;
    let rows: Vec<AuthLogRow> = if let Some(ref uid) = p.user_id {
        sqlx::query_as("SELECT * FROM user_auth_logs WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(uid).bind(per_page).bind(offset).fetch_all(&state.db).await.map_err(AppError::DatabaseError)?
    } else {
        sqlx::query_as("SELECT * FROM user_auth_logs ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(per_page).bind(offset).fetch_all(&state.db).await.map_err(AppError::DatabaseError)?
    };
    Ok(Json(json!({"success": true, "data": rows, "count": rows.len()})))
}

async fn activity_logs(State(state): State<AppState>, _auth: DevUser, Query(p): Query<LogFilterParams>) -> Result<Json<Value>> {
    let page = p.page.unwrap_or(1).max(1);
    let per_page = p.per_page.unwrap_or(50).min(100);
    let offset = (page - 1) * per_page;
    let rows: Vec<ActivityLogRow> = if let Some(ref uid) = p.user_id {
        sqlx::query_as("SELECT * FROM user_activity_logs WHERE user_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(uid).bind(per_page).bind(offset).fetch_all(&state.db).await.map_err(AppError::DatabaseError)?
    } else {
        sqlx::query_as("SELECT * FROM user_activity_logs ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(per_page).bind(offset).fetch_all(&state.db).await.map_err(AppError::DatabaseError)?
    };
    Ok(Json(json!({"success": true, "data": rows, "count": rows.len()})))
}
