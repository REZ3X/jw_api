use axum::{extract::{Path, Query, State}, routing::{get, put}, Json, Router};
use serde_json::{json, Value};
use crate::{
    error::{AppError, Result},
    middleware::{activity_log::log_activity, auth::DevUser},
    models::*,
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/:id/role", put(update_user_role))
        .route("/analytics/overview", get(overview))
}

#[derive(serde::Deserialize)]
struct UserListParams { page: Option<i64>, per_page: Option<i64>, role: Option<String>, search: Option<String> }

async fn list_users(State(state): State<AppState>, _auth: DevUser, Query(p): Query<UserListParams>) -> Result<Json<Value>> {
    let page = p.page.unwrap_or(1).max(1);
    let per_page = p.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;
    let rows: Vec<UserRow> = if let Some(ref role) = p.role {
        sqlx::query_as("SELECT * FROM users WHERE role = ? ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(role).bind(per_page).bind(offset).fetch_all(&state.db).await.map_err(AppError::DatabaseError)?
    } else if let Some(ref search) = p.search {
        sqlx::query_as("SELECT * FROM users WHERE name LIKE ? OR username LIKE ? OR email LIKE ? ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(format!("%{}%", search)).bind(format!("%{}%", search)).bind(format!("%{}%", search))
            .bind(per_page).bind(offset).fetch_all(&state.db).await.map_err(AppError::DatabaseError)?
    } else {
        sqlx::query_as("SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(per_page).bind(offset).fetch_all(&state.db).await.map_err(AppError::DatabaseError)?
    };
    let users: Vec<UserResponse> = rows.iter().map(UserResponse::from).collect();
    Ok(Json(json!({"success": true, "data": users, "count": users.len()})))
}

async fn update_user_role(State(state): State<AppState>, auth: DevUser, Path(id): Path<String>, Json(req): Json<UpdateRoleRequest>) -> Result<Json<Value>> {
    let valid_roles = ["basic","city_major_gov","fire_department","health_department","environment_department","police_department","dev"];
    if !valid_roles.contains(&req.role.as_str()) { return Err(AppError::ValidationError(format!("Invalid role: {}", req.role))); }
    sqlx::query("UPDATE users SET role = ?, updated_at = NOW() WHERE id = ?")
        .bind(&req.role).bind(&id).execute(&state.db).await.map_err(AppError::DatabaseError)?;
    log_activity(&state.db, &auth.user.id, "update", "dev_admin", "user_role", Some(&id), Some(&format!("new_role: {}", req.role)), None).await;
    let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(&id).fetch_one(&state.db).await.map_err(AppError::DatabaseError)?;
    Ok(Json(json!({"success": true, "data": UserResponse::from(&user)})))
}

async fn overview(State(state): State<AppState>, _auth: DevUser) -> Result<Json<Value>> {
    let stats = crate::services::AnalyticsService::get_platform_stats(&state.db).await?;
    let dept = crate::services::AnalyticsService::get_department_stats(&state.db).await?;
    let tags = crate::services::AnalyticsService::get_trending_tags(&state.db, 10).await?;
    Ok(Json(json!({"success": true, "data": {"platform": stats, "departments": dept, "trending": tags}})))
}
