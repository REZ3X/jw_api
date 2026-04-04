use axum::{extract::{Query, State}, routing::{get, post, put}, Json, Router};
use serde_json::{json, Value};
use crate::{
    error::{AppError, Result},
    middleware::{activity_log::{log_activity, log_auth_event}, auth::{AuthUser, VerifiedUser}},
    models::*,
    services::AuthService,
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/google/url", get(google_auth_url))
        .route("/google/callback", get(google_callback))
        .route("/verify-email", get(verify_email))
        .route("/resend-verification", post(resend_verification))
        .route("/me", get(get_me))
        .route("/me", put(update_profile))
}

async fn google_auth_url(State(state): State<AppState>) -> Result<Json<Value>> {
    let url = AuthService::google_auth_url(&state.config)?;
    Ok(Json(json!({"success": true, "url": url})))
}

async fn google_callback(State(state): State<AppState>, Query(req): Query<GoogleCallbackRequest>) -> Result<Json<Value>> {
    let google_user = AuthService::exchange_code(&req.code, &state.config, &state.http_client).await?;
    let (user, is_new) = AuthService::find_or_create_user(&state.db, &google_user, &state.crypto).await?;
    let token = AuthService::generate_jwt(&user, &state.config)?;
    log_auth_event(&state.db, &user.id, if is_new { "register" } else { "login" }, None, None, true, None).await;
    if is_new {
        if let Some(ref vtoken) = user.email_verification_token {
            let _ = state.email.send_verification_email(&user.email, &user.name, vtoken).await;
            log_auth_event(&state.db, &user.id, "verification_sent", None, None, true, None).await;
        }
    }
    Ok(Json(json!({"success": true, "data": AuthResponse { token, user: UserResponse::from(&user), is_new_user: is_new }})))
}

async fn verify_email(State(state): State<AppState>, Query(q): Query<VerifyEmailQuery>) -> Result<Json<Value>> {
    let user = AuthService::verify_email(&state.db, &q.token).await?;
    log_auth_event(&state.db, &user.id, "email_verify", None, None, true, None).await;
    Ok(Json(json!({"success": true, "message": "Email verified", "user": UserResponse::from(&user)})))
}

async fn resend_verification(State(state): State<AppState>, auth: AuthUser) -> Result<Json<Value>> {
    if auth.user.email_verification_status == "verified" { return Err(AppError::Conflict("Already verified".into())); }
    let new_token = AuthService::generate_verification_token();
    sqlx::query("UPDATE users SET email_verification_token = ?, updated_at = NOW() WHERE id = ?")
        .bind(&new_token).bind(&auth.user.id).execute(&state.db).await.map_err(AppError::DatabaseError)?;
    state.email.send_verification_email(&auth.user.email, &auth.user.name, &new_token).await
        .map_err(|e| AppError::InternalError(e.into()))?;
    Ok(Json(json!({"success": true, "message": "Verification email sent"})))
}

async fn get_me(auth: AuthUser) -> Result<Json<Value>> {
    Ok(Json(json!({"success": true, "data": UserResponse::from(&auth.user)})))
}

async fn update_profile(State(state): State<AppState>, auth: VerifiedUser, Json(req): Json<UpdateProfileRequest>) -> Result<Json<Value>> {
    let mut updates = Vec::new();
    let mut binds: Vec<String> = Vec::new();
    if let Some(ref name) = req.name { if name.trim().is_empty() { return Err(AppError::ValidationError("Name cannot be empty".into())); } updates.push("name = ?"); binds.push(name.trim().to_string()); }
    if let Some(ref bio) = req.bio { updates.push("bio = ?"); binds.push(bio.clone()); }
    if let Some(ref birth) = req.birth {
        chrono::NaiveDate::parse_from_str(birth, "%Y-%m-%d").map_err(|_| AppError::ValidationError("Invalid date format".into()))?;
        updates.push("birth = ?"); binds.push(birth.clone());
    }
    if updates.is_empty() { return Err(AppError::ValidationError("At least one field required".into())); }
    updates.push("updated_at = NOW()");
    let sql = format!("UPDATE users SET {} WHERE id = ?", updates.join(", "));
    let mut q = sqlx::query(&sql);
    for val in &binds { q = q.bind(val); }
    q = q.bind(&auth.user.id);
    q.execute(&state.db).await.map_err(AppError::DatabaseError)?;
    let user: UserRow = sqlx::query_as("SELECT * FROM users WHERE id = ?").bind(&auth.user.id).fetch_one(&state.db).await.map_err(AppError::DatabaseError)?;
    log_activity(&state.db, &auth.user.id, "update", "profile", "user", Some(&auth.user.id), None, None).await;
    Ok(Json(json!({"success": true, "data": UserResponse::from(&user)})))
}
