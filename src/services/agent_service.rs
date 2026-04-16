use serde_json::{json, Value};
use sqlx::MySqlPool;

use crate::{
    crypto::CryptoService,
    error::{AppError, Result},
    models::chat::*,
    models::is_gov_role,
    services::{chat_service::ChatService, gemini_service::GeminiService, analytics_service::AnalyticsService},
};

pub struct AgentService;

impl AgentService {
    pub async fn process_message(
        pool: &MySqlPool, crypto: &CryptoService, gemini: &GeminiService,
        user_id: &str, user_name: &str, user_role: &str,
        chat_id: &str, salt: &str, req: &SendMessageRequest,
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

        let system_prompt = Self::build_system_prompt(user_name, user_role);
        let ai_response = gemini.generate_chat_response(&system_prompt, &history, &req.message, 1.0).await
            .map_err(|e| AppError::InternalError(e.into()))?;

        let (response_text, tool_calls, tool_results) = Self::parse_and_execute_tools(pool, user_id, user_role, &ai_response).await?;

        let final_response = if !tool_results.is_empty() {
            let ctx: String = tool_results.iter()
                .map(|tr| format!("Tool: {}\nResult: {}", tr.tool_name, serde_json::to_string_pretty(&tr.result).unwrap_or_default()))
                .collect::<Vec<_>>().join("\n\n");
            let follow_up = format!("{}\n\nTOOL RESULTS:\n{}\n\nSummarize the results for the user.", response_text, ctx);
            gemini.generate_with_system(&follow_up, Some("You are JW AI. Summarize tool results concisely."), 0.7, 4096).await
                .unwrap_or(response_text)
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

    async fn parse_and_execute_tools(pool: &MySqlPool, user_id: &str, user_role: &str, ai_response: &str) -> Result<(String, Vec<ToolCall>, Vec<ToolResult>)> {
        let parsed = serde_json::from_str::<AgentResponse>(ai_response.trim())
            .or_else(|_| {
                let c = ai_response.trim()
                    .trim_start_matches("```json").trim_start_matches("```")
                    .trim_end_matches("```").trim();
                serde_json::from_str::<AgentResponse>(c)
            });

        if let Ok(agent_resp) = parsed {
            if !agent_resp.tool_calls.is_empty() {
                let mut tool_calls = Vec::new();
                let mut tool_results = Vec::new();
                for tc in &agent_resp.tool_calls {
                    let result = Self::execute_tool(pool, user_id, user_role, tc).await;
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

    async fn execute_tool(pool: &MySqlPool, user_id: &str, user_role: &str, tc: &ToolCallRequest) -> std::result::Result<Value, AppError> {
        match tc.tool_name.to_uppercase().as_str() {

            // ── Shared tools (all roles) ────────────────────────

            "GET_TRENDING_TAGS" => Ok(AnalyticsService::get_trending_tags(pool, 10).await?),
            "GET_PLATFORM_STATS" => Ok(AnalyticsService::get_platform_stats(pool).await?),
            "GET_DEPARTMENT_STATS" => Ok(AnalyticsService::get_department_stats(pool).await?),

            "SEARCH_POSTS" => {
                let department = tc.parameters["department"].as_str();
                let status = tc.parameters["status"].as_str();
                let tag = tc.parameters["tag"].as_str();
                let search = tc.parameters["search"].as_str();
                let limit = tc.parameters["limit"].as_i64().unwrap_or(10);

                let mut sql = String::from("SELECT id, caption, department, status, upvote_count, comment_count, DATE(created_at) as day FROM posts WHERE is_private = FALSE");
                let mut binds: Vec<String> = Vec::new();
                if let Some(d) = department { sql.push_str(" AND department = ?"); binds.push(d.to_string()); }
                if let Some(s) = status { sql.push_str(" AND status = ?"); binds.push(s.to_string()); }
                if let Some(t) = tag { sql.push_str(" AND EXISTS (SELECT 1 FROM post_tags WHERE post_id = posts.id AND tag = ?)"); binds.push(t.to_string()); }
                if let Some(q) = search { sql.push_str(" AND caption LIKE ?"); binds.push(format!("%{}%", q)); }
                sql.push_str(" ORDER BY created_at DESC LIMIT ?");

                let mut query = sqlx::query_as::<_, (String, String, String, String, i32, i32, String)>(&sql);
                for b in &binds { query = query.bind(b); }
                query = query.bind(limit);
                let rows = query.fetch_all(pool).await.map_err(AppError::DatabaseError)?;
                let posts: Vec<Value> = rows.iter().map(|r| json!({"id": r.0, "caption": r.1, "department": r.2, "status": r.3, "upvotes": r.4, "comments": r.5, "date": r.6})).collect();
                Ok(json!({"posts": posts, "count": posts.len()}))
            }

            // ── Basic / Dev tools ───────────────────────────────

            "GET_MY_POSTS" => {
                if is_gov_role(user_role) {
                    return Err(AppError::Forbidden("Government accounts don't create posts".into()));
                }
                let status = tc.parameters["status"].as_str();
                let limit = tc.parameters["limit"].as_i64().unwrap_or(10);

                let (sql, bind_status) = match status {
                    Some(s) => ("SELECT id, caption, department, status, upvote_count, comment_count, DATE(created_at) as day FROM posts WHERE user_id = ? AND status = ? ORDER BY created_at DESC LIMIT ?".to_string(), Some(s.to_string())),
                    None => ("SELECT id, caption, department, status, upvote_count, comment_count, DATE(created_at) as day FROM posts WHERE user_id = ? ORDER BY created_at DESC LIMIT ?".to_string(), None),
                };
                let rows: Vec<(String, String, String, String, i32, i32, String)> = if let Some(ref s) = bind_status {
                    sqlx::query_as(&sql).bind(user_id).bind(s).bind(limit).fetch_all(pool).await
                } else {
                    sqlx::query_as(&sql).bind(user_id).bind(limit).fetch_all(pool).await
                }.map_err(AppError::DatabaseError)?;
                let posts: Vec<Value> = rows.iter().map(|r| json!({"id": r.0, "caption": r.1, "department": r.2, "status": r.3, "upvotes": r.4, "comments": r.5, "date": r.6})).collect();
                Ok(json!({"posts": posts, "count": posts.len()}))
            }

            "GET_MY_UNRESPONDED_POSTS" => {
                if is_gov_role(user_role) {
                    return Err(AppError::Forbidden("Government accounts don't create posts".into()));
                }
                let department = tc.parameters["department"].as_str();
                let mut sql = String::from(
                    "SELECT id, caption, department, status, upvote_count, comment_count, DATE(created_at) as day FROM posts WHERE user_id = ? AND status != 'closed'"
                );
                let mut binds: Vec<String> = vec![user_id.to_string()];
                if let Some(d) = department { sql.push_str(" AND department = ?"); binds.push(d.to_string()); }
                sql.push_str(" ORDER BY created_at ASC LIMIT 20");

                let mut query = sqlx::query_as::<_, (String, String, String, String, i32, i32, String)>(&sql);
                for b in &binds { query = query.bind(b); }
                let rows = query.fetch_all(pool).await.map_err(AppError::DatabaseError)?;
                let posts: Vec<Value> = rows.iter().map(|r| json!({"id": r.0, "caption": r.1, "department": r.2, "status": r.3, "upvotes": r.4, "comments": r.5, "date": r.6})).collect();
                Ok(json!({"unresponded_posts": posts, "count": posts.len()}))
            }

            "CREATE_POST_DRAFT" => {
                if is_gov_role(user_role) {
                    return Err(AppError::Forbidden("Government accounts cannot create posts".into()));
                }
                let caption = tc.parameters["caption"].as_str();
                let location = tc.parameters["location"].as_str();
                let department = tc.parameters["department"].as_str();
                let is_private = tc.parameters["is_private"].as_bool().unwrap_or(false);

                let mut missing: Vec<&str> = Vec::new();
                if caption.is_none() || caption.map(|c| c.trim().is_empty()).unwrap_or(true) { missing.push("caption"); }
                if location.is_none() || location.map(|l| l.trim().is_empty()).unwrap_or(true) { missing.push("location"); }

                if !missing.is_empty() {
                    return Ok(json!({
                        "draft_ready": false,
                        "missing_fields": missing,
                        "message": format!("Please provide: {}", missing.join(", ")),
                        "collected": {
                            "caption": caption.unwrap_or(""),
                            "location": location.unwrap_or(""),
                            "department": department.unwrap_or("auto"),
                            "is_private": is_private,
                        }
                    }));
                }

                let caption_text = caption.unwrap();
                let final_dept = match department {
                    Some(d) if !d.is_empty() && d != "auto" => d.to_string(),
                    _ => "auto_classify".to_string(),
                };

                Ok(json!({
                    "draft_ready": true,
                    "draft": {
                        "caption": caption_text,
                        "location": location.unwrap_or(""),
                        "department": final_dept,
                        "is_private": is_private,
                    },
                    "message": "Draft ready. User needs to attach media (1-4 images/videos) and submit via the post form.",
                    "note": "Posts require at least 1 media file. Direct the user to the post creation form with this data pre-filled."
                }))
            }

            // ── Gov-only tools ──────────────────────────────────

            "GET_DEPT_QUEUE" => {
                if !is_gov_role(user_role) {
                    return Err(AppError::Forbidden("Department tools require government role".into()));
                }
                let status = tc.parameters["status"].as_str().unwrap_or("pending");
                let limit = tc.parameters["limit"].as_i64().unwrap_or(20);

                let rows: Vec<(String, String, String, i32, i32, String)> = sqlx::query_as(
                    "SELECT id, caption, status, upvote_count, comment_count, DATE(created_at) as day FROM posts WHERE department = ? AND status = ? ORDER BY created_at ASC LIMIT ?"
                ).bind(user_role).bind(status).bind(limit).fetch_all(pool).await.map_err(AppError::DatabaseError)?;
                let posts: Vec<Value> = rows.iter().map(|r| json!({"id": r.0, "caption": r.1, "status": r.2, "upvotes": r.3, "comments": r.4, "date": r.5})).collect();
                Ok(json!({"department": user_role, "status_filter": status, "posts": posts, "count": posts.len()}))
            }

            "GET_DEPT_TODAY_STATS" => {
                if !is_gov_role(user_role) {
                    return Err(AppError::Forbidden("Department tools require government role".into()));
                }
                let total_today: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE department = ? AND DATE(created_at) = CURDATE()")
                    .bind(user_role).fetch_one(pool).await.map_err(AppError::DatabaseError)?;
                let pending_today: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE department = ? AND DATE(created_at) = CURDATE() AND status = 'pending'")
                    .bind(user_role).fetch_one(pool).await.map_err(AppError::DatabaseError)?;
                let responded_today: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE department = ? AND DATE(created_at) = CURDATE() AND status != 'pending'")
                    .bind(user_role).fetch_one(pool).await.map_err(AppError::DatabaseError)?;

                let total_pending: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE department = ? AND status = 'pending'")
                    .bind(user_role).fetch_one(pool).await.map_err(AppError::DatabaseError)?;
                let total_in_progress: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts WHERE department = ? AND status = 'in_progress'")
                    .bind(user_role).fetch_one(pool).await.map_err(AppError::DatabaseError)?;

                Ok(json!({
                    "department": user_role,
                    "today": { "new_reports": total_today.0, "still_pending": pending_today.0, "responded": responded_today.0 },
                    "backlog": { "total_pending": total_pending.0, "total_in_progress": total_in_progress.0 }
                }))
            }

            "GET_DEPT_RESPONSE_HISTORY" => {
                if !is_gov_role(user_role) {
                    return Err(AppError::Forbidden("Department tools require government role".into()));
                }
                let limit = tc.parameters["limit"].as_i64().unwrap_or(10);
                let rows: Vec<(String, String, String, String, Option<String>, String)> = sqlx::query_as(
                    "SELECT h.id, h.post_id, h.old_status, h.new_status, h.note, DATE(h.created_at) as day FROM post_status_history h JOIN posts p ON p.id = h.post_id WHERE p.department = ? AND h.user_id = ? ORDER BY h.created_at DESC LIMIT ?"
                ).bind(user_role).bind(user_id).bind(limit).fetch_all(pool).await.map_err(AppError::DatabaseError)?;
                let history: Vec<Value> = rows.iter().map(|r| json!({"id": r.0, "post_id": r.1, "from": r.2, "to": r.3, "note": r.4, "date": r.5})).collect();
                Ok(json!({"responses": history, "count": history.len()}))
            }

            "SEARCH_DEPT_POSTS" => {
                if !is_gov_role(user_role) {
                    return Err(AppError::Forbidden("Department tools require government role".into()));
                }
                let status = tc.parameters["status"].as_str();
                let search = tc.parameters["search"].as_str();
                let tag = tc.parameters["tag"].as_str();
                let days = tc.parameters["days"].as_i64();
                let limit = tc.parameters["limit"].as_i64().unwrap_or(15);

                let mut sql = String::from("SELECT id, caption, status, upvote_count, comment_count, DATE(created_at) as day FROM posts WHERE department = ?");
                let mut binds: Vec<String> = vec![user_role.to_string()];
                if let Some(s) = status { sql.push_str(" AND status = ?"); binds.push(s.to_string()); }
                if let Some(q) = search { sql.push_str(" AND caption LIKE ?"); binds.push(format!("%{}%", q)); }
                if let Some(t) = tag { sql.push_str(" AND EXISTS (SELECT 1 FROM post_tags WHERE post_id = posts.id AND tag = ?)"); binds.push(t.to_string()); }
                if let Some(d) = days { sql.push_str(&format!(" AND created_at >= DATE_SUB(NOW(), INTERVAL {} DAY)", d)); }
                sql.push_str(" ORDER BY created_at DESC LIMIT ?");

                let mut query = sqlx::query_as::<_, (String, String, String, i32, i32, String)>(&sql);
                for b in &binds { query = query.bind(b); }
                query = query.bind(limit);
                let rows = query.fetch_all(pool).await.map_err(AppError::DatabaseError)?;
                let posts: Vec<Value> = rows.iter().map(|r| json!({"id": r.0, "caption": r.1, "status": r.2, "upvotes": r.3, "comments": r.4, "date": r.5})).collect();
                Ok(json!({"department": user_role, "posts": posts, "count": posts.len()}))
            }

            "GET_DEPT_TRENDS" => {
                if !is_gov_role(user_role) {
                    return Err(AppError::Forbidden("Department tools require government role".into()));
                }
                let days = tc.parameters["days"].as_i64().unwrap_or(30);
                let rows: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT DATE(created_at) as d, COUNT(*) FROM posts WHERE department = ? AND created_at >= DATE_SUB(NOW(), INTERVAL ? DAY) GROUP BY d ORDER BY d"
                ).bind(user_role).bind(days).fetch_all(pool).await.map_err(AppError::DatabaseError)?;
                let data: Vec<Value> = rows.iter().map(|(date, count)| json!({"date": date, "count": count})).collect();

                let top_tags: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT pt.tag, COUNT(*) as cnt FROM post_tags pt JOIN posts p ON p.id = pt.post_id WHERE p.department = ? AND p.created_at >= DATE_SUB(NOW(), INTERVAL ? DAY) GROUP BY pt.tag ORDER BY cnt DESC LIMIT 10"
                ).bind(user_role).bind(days).fetch_all(pool).await.map_err(AppError::DatabaseError)?;
                let tags: Vec<Value> = top_tags.iter().map(|(tag, count)| json!({"tag": tag, "count": count})).collect();

                Ok(json!({"department": user_role, "days": days, "daily_reports": data, "top_tags": tags}))
            }

            _ => Err(AppError::BadRequest(format!("Unknown tool: {}", tc.tool_name))),
        }
    }

    fn build_system_prompt(user_name: &str, user_role: &str) -> String {
        let role_label = match user_role {
            "city_major_gov" => "City Major Government",
            "fire_department" => "Fire Department",
            "health_department" => "Health Department",
            "environment_department" => "Environment Department",
            "police_department" => "Police Department",
            "dev" => "Platform Developer/Admin",
            _ => "Citizen",
        };

        let shared_tools = r#"
SHARED TOOLS (all roles):
- GET_TRENDING_TAGS — Trending hashtags. Params: (none)
- GET_PLATFORM_STATS — Platform-wide statistics. Params: (none)
- GET_DEPARTMENT_STATS — Per-department breakdown. Params: (none)
- SEARCH_POSTS — Search public posts. Params: department?, status? (pending|in_progress|closed), tag?, search? (text), limit? (number)"#;

        let role_tools = if is_gov_role(user_role) {
            format!(r#"
DEPARTMENT TOOLS (your department: {dept}):
- GET_DEPT_QUEUE — Reports awaiting action. Params: status? (pending|in_progress), limit?
- GET_DEPT_TODAY_STATS — Today's report counts and backlog. Params: (none)
- GET_DEPT_RESPONSE_HISTORY — Your recent status changes. Params: limit?
- SEARCH_DEPT_POSTS — Search within your department. Params: status?, search?, tag?, days?, limit?
- GET_DEPT_TRENDS — Daily volume and top tags for your department. Params: days?

RULES:
- You CANNOT create posts. Government accounts only respond to reports.
- Focus on helping the officer manage their queue efficiently."#, dept = user_role)
        } else {
            r#"
CITIZEN TOOLS:
- GET_MY_POSTS — Your submitted reports. Params: status? (pending|in_progress|closed), limit?
- GET_MY_UNRESPONDED_POSTS — Reports not yet resolved. Params: department?
- CREATE_POST_DRAFT — Draft a new report. Params: caption, location, department? (or "auto"), is_private? (bool)
  If any required field (caption, location) is missing, the tool returns the missing fields. ASK the user to provide them before calling again.
  Posts require at least 1 media file — direct the user to the post form after drafting.

RULES:
- When creating a post draft, ensure caption and location are filled. If the user hasn't provided them, ask before calling the tool.
- For department, use "auto" if the user is unsure — it will be classified automatically."#.to_string()
        };

        format!(
            r#"You are Kirana, a female civic engagement AI assistant for the JogjaWaskita platform.
CRITICAL INSTRUCTION: You MUST speak strictly in polite and formal standard Indonesian (Bahasa Indonesia) at all times. Do not use any regional languages or dialects.
Personality: Warm, professional, and helpful. You address users politely.

User: {name} (Role: {role})
{shared}{role_specific}

RESPONSE FORMAT (when calling tools):
{{"response": "Penjelasan singkat dalam bahasa Indonesia", "tool_calls": [{{"tool_name": "TOOL_NAME", "parameters": {{}}}}]}}

When not calling tools, respond in plain text in indonesian with slight Jogja nuances.
Be concise but incredibly polite and action-oriented."#,
            name = user_name,
            role = role_label,
            shared = shared_tools,
            role_specific = role_tools,
        )
    }
}
