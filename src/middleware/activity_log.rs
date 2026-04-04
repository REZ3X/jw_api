use sqlx::MySqlPool;
use uuid::Uuid;

/// Log an authentication event (login, logout, register, etc.)
pub async fn log_auth_event(
    pool: &MySqlPool,
    user_id: &str,
    action: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
    success: bool,
    failure_reason: Option<&str>,
) {
    let id = Uuid::new_v4().to_string();
    let result = sqlx::query(
        r#"INSERT INTO user_auth_logs (id, user_id, action, ip_address, user_agent, success, failure_reason)
           VALUES (?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(user_id)
    .bind(action)
    .bind(ip_address)
    .bind(user_agent)
    .bind(success)
    .bind(failure_reason)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!("Failed to log auth event: {}", e);
    }
}

/// Log a user activity event.
pub async fn log_activity(
    pool: &MySqlPool,
    user_id: &str,
    action: &str,
    feature: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    details: Option<&str>,
    ip_address: Option<&str>,
) {
    let id = Uuid::new_v4().to_string();
    let result = sqlx::query(
        r#"INSERT INTO user_activity_logs (id, user_id, action, feature, entity_type, entity_id, details, ip_address)
           VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(&id)
    .bind(user_id)
    .bind(action)
    .bind(feature)
    .bind(entity_type)
    .bind(entity_id)
    .bind(details)
    .bind(ip_address)
    .execute(pool)
    .await;

    if let Err(e) = result {
        tracing::warn!("Failed to log activity: {}", e);
    }
}
