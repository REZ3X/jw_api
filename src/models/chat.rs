use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChatRow {
    pub id: String,
    pub user_id: String,
    pub title: String,
    pub chat_type: String,
    pub is_active: bool,
    pub message_count: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub id: String,
    pub title: String,
    pub chat_type: String,
    pub is_active: bool,
    pub message_count: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<&ChatRow> for ChatResponse {
    fn from(c: &ChatRow) -> Self {
        Self {
            id: c.id.clone(),
            title: c.title.clone(),
            chat_type: c.chat_type.clone(),
            is_active: c.is_active,
            message_count: c.message_count,
            created_at: c.created_at.to_string(),
            updated_at: c.updated_at.to_string(),
        }
    }
}


#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChatMessageRow {
    pub id: String,
    pub chat_id: String,
    pub user_id: String,
    pub role: String,
    pub content_enc: String,
    pub tool_calls_enc: Option<String>,
    pub tool_results_enc: Option<String>,
    pub has_tool_calls: bool,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct ChatMessageResponse {
    pub id: String,
    pub chat_id: String,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<serde_json::Value>,
    pub tool_results: Option<serde_json::Value>,
    pub has_tool_calls: bool,
    pub created_at: String,
}


#[derive(Debug, Deserialize)]
pub struct CreateChatRequest {
    pub chat_type: Option<String>, // "general" | "agentic"
}

#[derive(Debug, Deserialize)]
pub struct UpdateChatRequest {
    pub title: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub message: String,
    pub images: Option<Vec<String>>,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolResult {
    pub tool_name: String,
    pub result: serde_json::Value,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct AgentResponse {
    #[serde(default)]
    pub response: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallRequest>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ToolCallRequest {
    pub tool_name: String,
    pub parameters: serde_json::Value,
}
