use sqlx::MySqlPool;
use crate::error::{AppError, Result};

pub struct AnalyticsService;

impl AnalyticsService {
    pub async fn get_trending_tags(pool: &MySqlPool, limit: i64) -> Result<serde_json::Value> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT tag, COUNT(*) as cnt FROM post_tags GROUP BY tag ORDER BY cnt DESC LIMIT ?"
        ).bind(limit).fetch_all(pool).await.map_err(AppError::DatabaseError)?;
        let tags: Vec<serde_json::Value> = rows.iter().map(|(tag, count)| serde_json::json!({"tag": tag, "count": count})).collect();
        Ok(serde_json::json!({"trending_tags": tags}))
    }

    pub async fn get_platform_stats(pool: &MySqlPool) -> Result<serde_json::Value> {
        let total_posts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts").fetch_one(pool).await.map_err(AppError::DatabaseError)?;
        let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users").fetch_one(pool).await.map_err(AppError::DatabaseError)?;
        let resolved: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE status = 'closed'").fetch_one(pool).await.map_err(AppError::DatabaseError)?;
        let pending: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE status = 'pending'").fetch_one(pool).await.map_err(AppError::DatabaseError)?;
        let in_progress: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE status = 'in_progress'").fetch_one(pool).await.map_err(AppError::DatabaseError)?;
        let rate = if total_posts.0 > 0 { (resolved.0 as f64 / total_posts.0 as f64) * 100.0 } else { 0.0 };
        Ok(serde_json::json!({
            "total_posts": total_posts.0, "total_users": total_users.0,
            "resolved": resolved.0, "pending": pending.0, "in_progress": in_progress.0,
            "resolution_rate": format!("{:.1}%", rate),
        }))
    }

    pub async fn get_department_stats(pool: &MySqlPool) -> Result<serde_json::Value> {
        let rows: Vec<(String, i64)> = sqlx::query_as("SELECT department, COUNT(*) FROM posts GROUP BY department")
            .fetch_all(pool).await.map_err(AppError::DatabaseError)?;
        let resolved_rows: Vec<(String, i64)> = sqlx::query_as("SELECT department, COUNT(*) FROM posts WHERE status = 'closed' GROUP BY department")
            .fetch_all(pool).await.map_err(AppError::DatabaseError)?;

        let depts = vec!["city_major_gov","fire_department","health_department","environment_department","police_department"];
        let mut stats = Vec::new();
        for dept in &depts {
            let total = rows.iter().find(|(d, _)| d == dept).map(|(_, c)| *c).unwrap_or(0);
            let resolved = resolved_rows.iter().find(|(d, _)| d == dept).map(|(_, c)| *c).unwrap_or(0);
            let rate = if total > 0 { (resolved as f64 / total as f64) * 100.0 } else { 0.0 };
            stats.push(serde_json::json!({"department": dept, "total": total, "resolved": resolved, "resolution_rate": format!("{:.1}%", rate)}));
        }
        Ok(serde_json::json!({"departments": stats}))
    }

    pub async fn get_trends(pool: &MySqlPool, days: i64) -> Result<serde_json::Value> {
        let rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT DATE(created_at) as d, COUNT(*) FROM posts WHERE created_at >= DATE_SUB(NOW(), INTERVAL ? DAY) GROUP BY d ORDER BY d"
        ).bind(days).fetch_all(pool).await.map_err(AppError::DatabaseError)?;
        let data: Vec<serde_json::Value> = rows.iter().map(|(date, count)| serde_json::json!({"date": date, "count": count})).collect();
        Ok(serde_json::json!({"trends": data, "days": days}))
    }
}
