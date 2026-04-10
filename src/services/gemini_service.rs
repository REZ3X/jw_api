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
    parts: Vec<TextPart>,
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
    parts: Vec<TextPart>,
}

#[derive(Debug, Serialize)]
struct TextPart {
    text: String,
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
    api_key: String,
    model: String,
    client: Arc<reqwest::Client>,
}

impl GeminiService {
    pub fn new(api_key: String, model: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .pool_max_idle_per_host(5)
            .build()
            .expect("Failed to build Gemini HTTP client");

        Self {
            api_key,
            model,
            client: Arc::new(client),
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        )
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
                parts: vec![TextPart { text: prompt.into() }],
            }],
            system_instruction: system.map(|s| SystemInstruction {
                parts: vec![TextPart { text: s.into() }],
            }),
            generation_config: Some(GenerationConfig {
                temperature,
                max_output_tokens: max_tokens,
                response_mime_type: None,
            }),
        };

        let response = self.client.post(&self.endpoint()).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Gemini API error {}: {}", status, text);
        }

        let resp: GeminiResponse = response.json().await?;
        Self::extract_text(&resp)
    }

    pub async fn generate_chat_response(
        &self,
        system_prompt: &str,
        history: &[(String, String)],
        user_message: &str,
        temperature: f32,
    ) -> Result<String> {
        let mut contents: Vec<Content> = history
            .iter()
            .map(|(role, content)| Content {
                role: role.clone(),
                parts: vec![TextPart { text: content.clone() }],
            })
            .collect();

        contents.push(Content {
            role: "user".into(),
            parts: vec![TextPart { text: user_message.into() }],
        });

        let body = GeminiRequest {
            contents,
            system_instruction: Some(SystemInstruction {
                parts: vec![TextPart { text: system_prompt.into() }],
            }),
            generation_config: Some(GenerationConfig {
                temperature,
                max_output_tokens: 4096,
                response_mime_type: Some("application/json".to_string()),
            }),
        };

        let response = self.client.post(&self.endpoint()).json(&body).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Gemini API error {}: {}", status, text);
        }

        let resp: GeminiResponse = response.json().await?;
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
