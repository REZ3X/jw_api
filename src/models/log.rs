use chrono::NaiveDateTime;
use serde::Serialize;


#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct AuthLogRow {
    pub id: String,
    pub user_id: String,
    pub action: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub failure_reason: Option<String>,
    pub created_at: NaiveDateTime,
}


#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct ActivityLogRow {
    pub id: String,
    pub user_id: String,
    pub action: String,
    pub feature: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub details: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: NaiveDateTime,
}


#[derive(Debug, serde::Deserialize)]
pub struct LogFilterParams {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub feature: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}
