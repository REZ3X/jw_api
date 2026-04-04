use axum::{extract::{Query, State}, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::{error::Result, services::AnalyticsService, state::AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/trending-tags", get(trending_tags))
        .route("/stats", get(platform_stats))
        .route("/department-stats", get(department_stats))
        .route("/trends", get(trends))
}

#[derive(Deserialize)]
struct TrendParams { days: Option<i64> }

async fn trending_tags(State(state): State<AppState>) -> Result<Json<Value>> {
    let data = AnalyticsService::get_trending_tags(&state.db, 20).await?;
    Ok(Json(json!({"success": true, "data": data})))
}

async fn platform_stats(State(state): State<AppState>) -> Result<Json<Value>> {
    let data = AnalyticsService::get_platform_stats(&state.db).await?;
    Ok(Json(json!({"success": true, "data": data})))
}

async fn department_stats(State(state): State<AppState>) -> Result<Json<Value>> {
    let data = AnalyticsService::get_department_stats(&state.db).await?;
    Ok(Json(json!({"success": true, "data": data})))
}

async fn trends(State(state): State<AppState>, Query(p): Query<TrendParams>) -> Result<Json<Value>> {
    let data = AnalyticsService::get_trends(&state.db, p.days.unwrap_or(30)).await?;
    Ok(Json(json!({"success": true, "data": data})))
}
