use sqlx::MySqlPool;
use uuid::Uuid;

use crate::{
    crypto::CryptoService,
    error::{AppError, Result},
    models::chat::*,
    services::gemini_service::GeminiService,
};

pub struct ChatService;

impl ChatService {
    pub async fn create_chat(pool: &MySqlPool, user_id: &str, req: &CreateChatRequest) -> Result<ChatResponse> {
        let id = Uuid::new_v4().to_string();
        let chat_type = req.chat_type.as_deref().unwrap_or("general");
        if !["general", "agentic"].contains(&chat_type) {
            return Err(AppError::ValidationError("chat_type must be 'general' or 'agentic'".into()));
        }
        sqlx::query("INSERT INTO chats (id, user_id, title, chat_type) VALUES (?, ?, 'New Chat', ?)")
            .bind(&id).bind(user_id).bind(chat_type).execute(pool).await.map_err(AppError::DatabaseError)?;
        let row: ChatRow = sqlx::query_as("SELECT * FROM chats WHERE id = ?").bind(&id).fetch_one(pool).await.map_err(AppError::DatabaseError)?;
        Ok(ChatResponse::from(&row))
    }

    pub async fn list_chats(pool: &MySqlPool, user_id: &str, limit: i64) -> Result<Vec<ChatResponse>> {
        let rows: Vec<ChatRow> = sqlx::query_as("SELECT * FROM chats WHERE user_id = ? ORDER BY updated_at DESC LIMIT ?")
            .bind(user_id).bind(limit).fetch_all(pool).await.map_err(AppError::DatabaseError)?;
        Ok(rows.iter().map(ChatResponse::from).collect())
    }

    pub async fn get_chat(pool: &MySqlPool, user_id: &str, chat_id: &str) -> Result<ChatResponse> {
        let row: ChatRow = sqlx::query_as("SELECT * FROM chats WHERE id = ? AND user_id = ?")
            .bind(chat_id).bind(user_id).fetch_optional(pool).await.map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Chat not found".into()))?;
        Ok(ChatResponse::from(&row))
    }

    pub async fn update_chat(pool: &MySqlPool, user_id: &str, chat_id: &str, req: &UpdateChatRequest) -> Result<ChatResponse> {
        sqlx::query("UPDATE chats SET title = COALESCE(?, title), is_active = COALESCE(?, is_active), updated_at = NOW() WHERE id = ? AND user_id = ?")
            .bind(req.title.as_deref()).bind(req.is_active).bind(chat_id).bind(user_id)
            .execute(pool).await.map_err(AppError::DatabaseError)?;
        let row: ChatRow = sqlx::query_as("SELECT * FROM chats WHERE id = ?").bind(chat_id).fetch_one(pool).await.map_err(AppError::DatabaseError)?;
        Ok(ChatResponse::from(&row))
    }

    pub async fn delete_chat(pool: &MySqlPool, user_id: &str, chat_id: &str) -> Result<()> {
        let r = sqlx::query("DELETE FROM chats WHERE id = ? AND user_id = ?").bind(chat_id).bind(user_id)
            .execute(pool).await.map_err(AppError::DatabaseError)?;
        if r.rows_affected() == 0 { return Err(AppError::NotFound("Chat not found".into())); }
        Ok(())
    }

    pub async fn get_messages(pool: &MySqlPool, crypto: &CryptoService, user_id: &str, chat_id: &str, salt: &str, limit: i64) -> Result<Vec<ChatMessageResponse>> {
        let _: ChatRow = sqlx::query_as("SELECT * FROM chats WHERE id = ? AND user_id = ?")
            .bind(chat_id).bind(user_id).fetch_optional(pool).await.map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Chat not found".into()))?;
        let rows: Vec<ChatMessageRow> = sqlx::query_as("SELECT * FROM chat_messages WHERE chat_id = ? AND user_id = ? ORDER BY created_at ASC LIMIT ?")
            .bind(chat_id).bind(user_id).bind(limit).fetch_all(pool).await.map_err(AppError::DatabaseError)?;
        rows.iter().map(|r| Self::decrypt_message(crypto, r, salt)).collect()
    }

    pub async fn send_general_message(
        pool: &MySqlPool, crypto: &CryptoService, gemini: &GeminiService,
        user_id: &str, user_name: &str, chat_id: &str, salt: &str, req: &SendMessageRequest,
    ) -> Result<(ChatMessageResponse, ChatMessageResponse)> {
        let chat: ChatRow = sqlx::query_as("SELECT * FROM chats WHERE id = ? AND user_id = ?")
            .bind(chat_id).bind(user_id).fetch_optional(pool).await.map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Chat not found".into()))?;

        let user_msg = Self::save_message(pool, crypto, chat_id, user_id, "user", &req.message, None, None, salt).await?;

        let history_rows: Vec<ChatMessageRow> = sqlx::query_as("SELECT * FROM chat_messages WHERE chat_id = ? AND user_id = ? ORDER BY created_at DESC LIMIT 20")
            .bind(chat_id).bind(user_id).fetch_all(pool).await.map_err(AppError::DatabaseError)?;

        let mut history: Vec<(String, String)> = history_rows.iter().rev()
            .filter_map(|row| {
                let content = crypto.decrypt(&row.content_enc, salt).ok()?;
                let role = if row.role == "user" { "user" } else { "model" };
                Some((role.to_string(), content))
            }).collect();
        if !history.is_empty() && history.last().map(|(r, _)| r.as_str()) == Some("user") { history.pop(); }

        let system_prompt = Self::build_system_prompt(user_name);
        let ai_response = gemini.generate_chat_response(&system_prompt, &history, &req.message, 0.8).await
            .map_err(|e| AppError::InternalError(e.into()))?;

        let assistant_msg = Self::save_message(pool, crypto, chat_id, user_id, "assistant", &ai_response, None, None, salt).await?;

        sqlx::query("UPDATE chats SET message_count = ?, updated_at = NOW() WHERE id = ?")
            .bind(chat.message_count + 2).bind(chat_id).execute(pool).await.ok();

        if chat.message_count == 0 {
            if let Ok(title) = gemini.generate_chat_title(&req.message).await {
                sqlx::query("UPDATE chats SET title = ? WHERE id = ?").bind(&title).bind(chat_id).execute(pool).await.ok();
            }
        }
        Ok((user_msg, assistant_msg))
    }

    pub async fn save_message(
        pool: &MySqlPool, crypto: &CryptoService, chat_id: &str, user_id: &str,
        role: &str, content: &str, tool_calls: Option<&serde_json::Value>,
        tool_results: Option<&serde_json::Value>, salt: &str,
    ) -> Result<ChatMessageResponse> {
        let id = Uuid::new_v4().to_string();
        let content_enc = crypto.encrypt(content, salt)?;
        let tc_enc = tool_calls.map(|tc| { let j = serde_json::to_string(tc).unwrap_or_default(); crypto.encrypt(&j, salt) }).transpose()?;
        let tr_enc = tool_results.map(|tr| { let j = serde_json::to_string(tr).unwrap_or_default(); crypto.encrypt(&j, salt) }).transpose()?;
        let has_tc = tool_calls.is_some();

        sqlx::query("INSERT INTO chat_messages (id, chat_id, user_id, role, content_enc, tool_calls_enc, tool_results_enc, has_tool_calls) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(&id).bind(chat_id).bind(user_id).bind(role).bind(&content_enc)
            .bind(tc_enc.as_deref()).bind(tr_enc.as_deref()).bind(has_tc)
            .execute(pool).await.map_err(AppError::DatabaseError)?;

        Ok(ChatMessageResponse {
            id, chat_id: chat_id.to_string(), role: role.to_string(), content: content.to_string(),
            tool_calls: tool_calls.cloned(), tool_results: tool_results.cloned(), has_tool_calls: has_tc,
            created_at: chrono::Local::now().naive_local().to_string(),
        })
    }

    fn decrypt_message(crypto: &CryptoService, row: &ChatMessageRow, salt: &str) -> Result<ChatMessageResponse> {
        let content = crypto.decrypt(&row.content_enc, salt)?;
        let tool_calls = row.tool_calls_enc.as_ref().and_then(|enc| { let s = crypto.decrypt(enc, salt).ok()?; serde_json::from_str(&s).ok() });
        let tool_results = row.tool_results_enc.as_ref().and_then(|enc| { let s = crypto.decrypt(enc, salt).ok()?; serde_json::from_str(&s).ok() });
        Ok(ChatMessageResponse {
            id: row.id.clone(), chat_id: row.chat_id.clone(), role: row.role.clone(), content,
            tool_calls, tool_results, has_tool_calls: row.has_tool_calls, created_at: row.created_at.to_string(),
        })
    }

    fn build_system_prompt(user_name: &str) -> String {
        format!(r#"You are JW AI Assistant, a helpful civic engagement companion. You are talking to {name}.

YOUR ROLE:
- Help citizens understand their rights and how to report local issues
- Provide information about government departments and their responsibilities
- Offer guidance on civic participation and community engagement
- Be knowledgeable about common urban issues (infrastructure, environment, public safety)

GUIDELINES:
1. Be helpful, friendly, and informative
2. Use the user's name naturally
3. If asked about specific post status or department data, suggest using the agentic chat mode
4. Guide users on how to effectively report issues (clear photos, location, description)
5. Explain government department jurisdictions when asked
6. Keep responses concise but informative (2-3 paragraphs max)

DEPARTMENTS:
- City Major Gov: roads, infrastructure, general city issues
- Fire Department: fire hazards, fire safety
- Health Department: hospitals, clinics, public health
- Environment Department: pollution, waste, environmental damage
- Police Department: criminal activity, public safety"#, name = user_name)
    }
}
