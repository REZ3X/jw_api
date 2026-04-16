use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    temperature: f32,
    max_output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_mime_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Part {
    Text { text: String },
    InlineData { inline_data: InlineData },
}

#[derive(Debug, Serialize)]
struct InlineData {
    mime_type: String,
    data: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    text: Option<String>,
    #[serde(default)]
    thought: Option<bool>,
}

#[derive(Clone)]
pub struct GeminiService {
    api_keys: Vec<String>,
    models: Vec<String>,
    client: Arc<reqwest::Client>,
}

impl GeminiService {
    // Accepts a comma-separated model list (e.g. "gemini-3.1-flash-lite-preview, gemini-2.5-flash")
    // and a comma-separated api_key list.
    pub fn new(api_key_csv: String, model_csv: String) -> Self {
        let api_keys: Vec<String> = api_key_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert!(!api_keys.is_empty(), "At least one GEMINI_API_KEY must be configured");

        let models: Vec<String> = model_csv
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        assert!(!models.is_empty(), "At least one GEMINI_MODEL must be configured");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .pool_max_idle_per_host(5)
            .build()
            .expect("Failed to build Gemini HTTP client");

        Self {
            api_keys,
            models,
            client: Arc::new(client),
        }
    }

    fn endpoint(&self, model: &str, api_key: &str) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        )
    }

    // Sends a request with automatic failover across configured models and API keys.
    // Retryable errors (5xx, 429, 403, 400) trigger the next model/key.
    async fn send_request(&self, body: &GeminiRequest) -> Result<GeminiResponse> {
        let mut last_error = String::new();

        for api_key in &self.api_keys {
            for model in &self.models {
                let response = self.client.post(&self.endpoint(model, api_key)).json(body).send().await?;

                let status = response.status();
                let text = response.text().await.unwrap_or_default();

                if status.is_success() {
                    match serde_json::from_str::<GeminiResponse>(&text) {
                        Ok(json) => {
                            tracing::debug!("Gemini request served by model: {} with key ...{}", model, &api_key[api_key.len().saturating_sub(4)..]);
                            return Ok(json);
                        }
                        Err(e) => {
                            let raw_preview = &text[..std::cmp::min(200, text.len())];
                            let msg = format!("JSON parse error from {}. Raw: '{}' Error: {}", model, raw_preview, e);
                            tracing::warn!("{}", msg);
                            println!("  [Failover] {} (key ...{})", msg, &api_key[api_key.len().saturating_sub(4)..]);
                            last_error = msg;
                            continue;
                        }
                    }
                }

                last_error = format!("Gemini API error {} (model: {}): {}", status, model, text);

                // Failover on 5xx (server error), 429 (rate limit), 403 (quota/forbidden), 400 (bad request/model not allowed)
                if status.as_u16() >= 500 || status.as_u16() == 429 || status.as_u16() == 403 || status.as_u16() == 400 {
                    let msg = format!("HTTP {} from {}. Body: {}", status, model, &text[..std::cmp::min(150, text.len())]);
                    tracing::warn!("{}", msg);
                    println!("  [Failover] {} (key ...{})", msg, &api_key[api_key.len().saturating_sub(4)..]);
                    continue;
                }

                anyhow::bail!("{}", last_error);
            }
        }

        anyhow::bail!("All models and API keys exhausted. Last error: {}", last_error)
    }

    pub async fn generate_text(&self, prompt: &str) -> Result<String> {
        self.generate_with_system(prompt, None, 1.0, 8192).await
    }

    pub async fn generate_with_system(
        &self,
        prompt: &str,
        system: Option<&str>,
        temperature: f32,
        max_tokens: u32,
    ) -> Result<String> {
        let body = GeminiRequest {
            contents: vec![Content {
                role: "user".into(),
                parts: vec![Part::Text { text: prompt.into() }],
            }],
            system_instruction: system.map(|s| SystemInstruction {
                parts: vec![Part::Text { text: s.into() }],
            }),
            generation_config: Some(GenerationConfig {
                temperature,
                max_output_tokens: max_tokens,
                response_mime_type: None,
            }),
        };

        let resp = self.send_request(&body).await?;
        Self::extract_text(&resp)
    }

    pub async fn generate_chat_response(
        &self,
        system_prompt: &str,
        history: &[(String, String)],
        user_message: &str,
        images: Option<Vec<String>>,
        temperature: f32,
    ) -> Result<String> {
        let mut contents: Vec<Content> = history
            .iter()
            .map(|(role, content)| Content {
                role: role.clone(),
                parts: vec![Part::Text { text: content.clone() }],
            })
            .collect();

        let mut user_parts = vec![Part::Text { text: user_message.into() }];
        if let Some(imgs) = images {
            for img in imgs {
                if let Some(data) = img.strip_prefix("data:image/") {
                    if let Some(idx) = data.find(";base64,") {
                        let mime_ext = &data[..idx];
                        let b64 = &data[idx + 8..];
                        user_parts.push(Part::InlineData {
                            inline_data: InlineData {
                                mime_type: format!("image/{}", mime_ext),
                                data: b64.to_string(),
                            }
                        });
                    }
                }
            }
        }

        contents.push(Content {
            role: "user".into(),
            parts: user_parts,
        });

        let body = GeminiRequest {
            contents,
            system_instruction: Some(SystemInstruction {
                parts: vec![Part::Text { text: system_prompt.into() }],
            }),
            generation_config: Some(GenerationConfig {
                temperature,
                max_output_tokens: 4096,
                response_mime_type: Some("application/json".to_string()),
            }),
        };

        let resp = self.send_request(&body).await?;
        Self::extract_text(&resp)
    }

    fn extract_text(resp: &GeminiResponse) -> Result<String> {
        resp.candidates
            .first()
            .and_then(|c| {
                c.content
                    .parts
                    .iter()
                    .filter(|p| !p.thought.unwrap_or(false))
                    .find_map(|p| p.text.clone())
            })
            .ok_or_else(|| anyhow::anyhow!("Empty response from Gemini"))
    }

    pub async fn generate_chat_title(&self, first_message: &str) -> Result<String> {
        let prompt = format!(
            r#"Generate a concise chat title (max 5 words, no quotes) for a civic engagement conversation starting with:
"{}"
Return ONLY the title text."#,
            first_message
        );
        let raw = self.generate_text(&prompt).await?;
        let title = raw.trim().trim_matches('"').chars().take(60).collect::<String>();
        Ok(if title.is_empty() { "New Chat".into() } else { title })
    }

    pub async fn classify_department(&self, caption: &str) -> Result<String> {
        let prompt = format!(
            r#"Classify this civic report into ONE department. Return ONLY the enum value.

Departments:
- city_major_gov: general city issues, roads, infrastructure, public facilities, government misconduct
- fire_department: fire hazards, fire trucks, fire hydrants, fire alarms
- health_department: health issues, hospitals, clinics, sanitation
- environment_department: pollution, waste management, environmental damage
- police_department: criminal activity, law violations, public safety

Report: "{}"

Return ONLY one of: city_major_gov, fire_department, health_department, environment_department, police_department"#,
            caption
        );

        let raw = self.generate_with_system(
            &prompt,
            Some("You are a civic report classifier. Return ONLY the department enum value."),
            0.2,
            50,
        ).await?;

        let dept = raw.trim().to_lowercase();
        let valid = [
            "city_major_gov",
            "fire_department",
            "health_department",
            "environment_department",
            "police_department",
        ];

        if valid.contains(&dept.as_str()) {
            Ok(dept)
        } else {
            Ok("city_major_gov".to_string())
        }
    }
}
