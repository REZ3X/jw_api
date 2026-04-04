use axum::{extract::{Path, Query, State}, routing::{get, post, put, delete}, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::{
    error::{AppError, Result},
    middleware::{activity_log::log_activity, auth::VerifiedUser},
    models::chat::*,
    services::{AgentService, ChatService},
    state::AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_chat))
        .route("/", get(list_chats))
        .route("/:id", get(get_chat))
        .route("/:id", put(update_chat))
        .route("/:id", delete(delete_chat))
        .route("/:id/messages", get(get_messages))
        .route("/:id/messages", post(send_message))
}

#[derive(Deserialize)]
struct ListParams { limit: Option<i64> }

async fn create_chat(State(state): State<AppState>, auth: VerifiedUser, Json(req): Json<CreateChatRequest>) -> Result<Json<Value>> {
    let chat = ChatService::create_chat(&state.db, &auth.user.id, &req).await?;
    log_activity(&state.db, &auth.user.id, "create", "chat", "chat", Some(&chat.id), Some(&format!("type: {}", chat.chat_type)), None).await;
    Ok(Json(json!({"success": true, "data": chat})))
}

async fn list_chats(State(state): State<AppState>, auth: VerifiedUser, Query(p): Query<ListParams>) -> Result<Json<Value>> {
    let chats = ChatService::list_chats(&state.db, &auth.user.id, p.limit.unwrap_or(20)).await?;
    Ok(Json(json!({"success": true, "data": chats, "count": chats.len()})))
}

async fn get_chat(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>) -> Result<Json<Value>> {
    let chat = ChatService::get_chat(&state.db, &auth.user.id, &id).await?;
    Ok(Json(json!({"success": true, "data": chat})))
}

async fn update_chat(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>, Json(req): Json<UpdateChatRequest>) -> Result<Json<Value>> {
    let chat = ChatService::update_chat(&state.db, &auth.user.id, &id, &req).await?;
    Ok(Json(json!({"success": true, "data": chat})))
}

async fn delete_chat(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>) -> Result<Json<Value>> {
    ChatService::delete_chat(&state.db, &auth.user.id, &id).await?;
    Ok(Json(json!({"success": true, "message": "Chat deleted"})))
}

async fn get_messages(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>, Query(p): Query<ListParams>) -> Result<Json<Value>> {
    let msgs = ChatService::get_messages(&state.db, &state.crypto, &auth.user.id, &id, &auth.user.encryption_salt, p.limit.unwrap_or(50)).await?;
    Ok(Json(json!({"success": true, "data": msgs, "count": msgs.len()})))
}

async fn send_message(State(state): State<AppState>, auth: VerifiedUser, Path(id): Path<String>, Json(req): Json<SendMessageRequest>) -> Result<Json<Value>> {
    if req.message.trim().is_empty() { return Err(AppError::ValidationError("Message cannot be empty".into())); }
    let chat = ChatService::get_chat(&state.db, &auth.user.id, &id).await?;
    let (user_msg, ai_msg) = match chat.chat_type.as_str() {
        "agentic" => AgentService::process_message(&state.db, &state.crypto, &state.gemini, &auth.user.id, &auth.user.name, &id, &auth.user.encryption_salt, &req).await?,
        _ => ChatService::send_general_message(&state.db, &state.crypto, &state.gemini, &auth.user.id, &auth.user.name, &id, &auth.user.encryption_salt, &req).await?,
    };
    log_activity(&state.db, &auth.user.id, "create", "chat", "chat_message", Some(&id), Some(&format!("type: {}", chat.chat_type)), None).await;
    Ok(Json(json!({"success": true, "data": {"user_message": user_msg, "assistant_message": ai_msg}})))
}
