use serde_json::{json, Value};
use sqlx::MySqlPool;

use crate::{
    crypto::CryptoService,
    error::{AppError, Result},
    models::chat::*,
    services::{chat_service::ChatService, gemini_service::GeminiService, analytics_service::AnalyticsService},
};

pub struct AgentService;

impl AgentService {
    pub async fn process_message(
        pool: &MySqlPool, crypto: &CryptoService, gemini: &GeminiService,
        user_id: &str, user_name: &str, chat_id: &str, salt: &str, req: &SendMessageRequest,
    ) -> Result<(ChatMessageResponse, ChatMessageResponse)> {
        let chat: crate::models::ChatRow = sqlx::query_as("SELECT * FROM chats WHERE id = ? AND user_id = ? AND chat_type = 'agentic'")
            .bind(chat_id).bind(user_id).fetch_optional(pool).await.map_err(AppError::DatabaseError)?
            .ok_or_else(|| AppError::NotFound("Agentic chat not found".into()))?;

        let user_msg = ChatService::save_message(pool, crypto, chat_id, user_id, "user", &req.message, None, None, salt).await?;

        let history_rows: Vec<ChatMessageRow> = sqlx::query_as("SELECT * FROM chat_messages WHERE chat_id = ? AND user_id = ? ORDER BY created_at DESC LIMIT 16")
            .bind(chat_id).bind(user_id).fetch_all(pool).await.map_err(AppError::DatabaseError)?;

        let mut history: Vec<(String, String)> = history_rows.iter().rev()
            .filter_map(|row| {
                let content = crypto.decrypt(&row.content_enc, salt).ok()?;
                let role = if row.role == "user" { "user" } else { "model" };
                Some((role.to_string(), content))
            }).collect();
        if history.last().map(|(r, _)| r.as_str()) == Some("user") { history.pop(); }

        let system_prompt = Self::build_agent_system_prompt(user_name);
        let ai_response = gemini.generate_chat_response(&system_prompt, &history, &req.message, 1.0).await
            .map_err(|e| AppError::InternalError(e.into()))?;

        let (response_text, tool_calls, tool_results) = Self::parse_and_execute_tools(pool, user_id, &ai_response).await?;

        let final_response = if !tool_results.is_empty() {
            let ctx: String = tool_results.iter().map(|tr| format!("Tool: {}\nResult: {}", tr.tool_name, serde_json::to_string_pretty(&tr.result).unwrap_or_default())).collect::<Vec<_>>().join("\n\n");
            let follow_up = format!("{}\n\nTOOL RESULTS:\n{}\n\nSummarize the results naturally.", response_text, ctx);
            gemini.generate_with_system(&follow_up, Some("You are JW AI, a civic engagement assistant. Summarize tool results naturally."), 0.7, 4096).await.unwrap_or(response_text)
        } else { response_text };

        let tc_json = if !tool_calls.is_empty() { Some(serde_json::to_value(&tool_calls).unwrap_or(Value::Null)) } else { None };
        let tr_json = if !tool_results.is_empty() { Some(serde_json::to_value(&tool_results).unwrap_or(Value::Null)) } else { None };

        let assistant_msg = ChatService::save_message(pool, crypto, chat_id, user_id, "assistant", &final_response, tc_json.as_ref(), tr_json.as_ref(), salt).await?;

        sqlx::query("UPDATE chats SET message_count = ?, updated_at = NOW() WHERE id = ?").bind(chat.message_count + 2).bind(chat_id).execute(pool).await.ok();
        if chat.message_count == 0 {
            if let Ok(title) = gemini.generate_chat_title(&req.message).await {
                sqlx::query("UPDATE chats SET title = ? WHERE id = ?").bind(&title).bind(chat_id).execute(pool).await.ok();
            }
        }
        Ok((user_msg, assistant_msg))
    }

    async fn parse_and_execute_tools(pool: &MySqlPool, user_id: &str, ai_response: &str) -> Result<(String, Vec<ToolCall>, Vec<ToolResult>)> {
        let parsed = serde_json::from_str::<AgentResponse>(ai_response.trim())
            .or_else(|_| { let c = ai_response.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim(); serde_json::from_str::<AgentResponse>(c) });

        if let Ok(agent_resp) = parsed {
            if !agent_resp.tool_calls.is_empty() {
                let mut tool_calls = Vec::new();
                let mut tool_results = Vec::new();
                for tc in &agent_resp.tool_calls {
                    let result = Self::execute_tool(pool, user_id, tc).await;
                    let (success, result_value) = match result {
                        Ok(v) => (true, v),
                        Err(e) => (false, json!({"error": format!("{}", e)})),
                    };
                    tool_calls.push(ToolCall { tool_name: tc.tool_name.clone(), parameters: tc.parameters.clone() });
                    tool_results.push(ToolResult { tool_name: tc.tool_name.clone(), result: result_value, success });
                }
                return Ok((agent_resp.response, tool_calls, tool_results));
            }
            return Ok((agent_resp.response, vec![], vec![]));
        }
        Ok((ai_response.to_string(), vec![], vec![]))
    }

    async fn execute_tool(pool: &MySqlPool, user_id: &str, tc: &ToolCallRequest) -> std::result::Result<Value, AppError> {
        match tc.tool_name.to_uppercase().as_str() {
            "GET_MY_POSTS" => {
                let limit = tc.parameters["limit"].as_i64().unwrap_or(10);
                let rows: Vec<(String, String, String, i32, i32)> = sqlx::query_as(
                    "SELECT id, caption, status, upvote_count, comment_count FROM posts WHERE user_id = ? ORDER BY created_at DESC LIMIT ?"
                ).bind(user_id).bind(limit).fetch_all(pool).await.map_err(AppError::DatabaseError)?;
                let posts: Vec<Value> = rows.iter().map(|r| json!({"id": r.0, "caption": r.1, "status": r.2, "upvotes": r.3, "comments": r.4})).collect();
                Ok(json!({"posts": posts, "count": posts.len()}))
            }
            "GET_TRENDING_TAGS" => { Ok(AnalyticsService::get_trending_tags(pool, 10).await?) }
            "GET_PLATFORM_STATS" => { Ok(AnalyticsService::get_platform_stats(pool).await?) }
            "GET_DEPARTMENT_STATS" => { Ok(AnalyticsService::get_department_stats(pool).await?) }
            _ => Err(AppError::BadRequest(format!("Unknown tool: {}", tc.tool_name))),
        }
    }

    fn build_agent_system_prompt(user_name: &str) -> String {
        format!(r#"You are JW Agentic AI, a civic engagement assistant with data access tools. Helping {name}.

TOOLS:
1. GET_MY_POSTS — Get user's own posts. Params: limit (number)
2. GET_TRENDING_TAGS — Get trending hashtags. Params: (none)
3. GET_PLATFORM_STATS — Get platform statistics. Params: (none)
4. GET_DEPARTMENT_STATS — Get per-department stats. Params: (none)

RESPONSE FORMAT (when using tools):
{{"response": "Brief explanation", "tool_calls": [{{"tool_name": "TOOL_NAME", "parameters": {{}}}}]}}

When not using tools, respond in plain text. Be helpful and informative about civic issues."#, name = user_name)
    }
}
