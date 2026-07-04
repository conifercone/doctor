use crate::ai::summary::StructuredSummary;
use crate::error::{DoctorError, DoctorResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageContent,
}

#[derive(Debug, Deserialize)]
struct ChatMessageContent {
    content: String,
}

#[derive(Debug, Clone)]
pub struct AiExplanation {
    pub raw_response: String,
    pub model_used: String,
}

fn build_system_prompt() -> String {
    r#"You are a Spring Boot diagnostic expert. You explain diagnostic findings to developers in Chinese (zh-CN).

For each issue, provide:
1. **问题描述**: What is happening in plain language
2. **根因分析**: Why this problem occurs, referencing specific configuration or code patterns
3. **影响范围**: What parts of the system are affected
4. **修复建议**: Specific, actionable steps to resolve the issue

Rules:
- Base your explanation ONLY on the provided diagnostic data
- If evidence is insufficient, explicitly state: "警告：证据不足，以下分析基于部分信息"
- Do NOT guess or fabricate causes — if uncertain, say so
- Reference issue IDs when citing specific findings
- Be concise but thorough
- Format output in Markdown with clear headings"#
        .to_string()
}

pub async fn explain(
    summary: &StructuredSummary,
    api_url: &str,
    api_key: &str,
    model: &str,
) -> DoctorResult<AiExplanation> {
    let user_message = serde_json::to_string_pretty(summary)
        .map_err(|e| DoctorError::ConfigError(format!("Failed to serialize summary: {e}")))?;

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: build_system_prompt(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Please explain the following Spring Boot diagnostic findings:\n\n```json\n{}\n```",
                    user_message
                ),
            },
        ],
        max_tokens: Some(4096),
        temperature: 0.1,
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| DoctorError::NetworkError {
            url: api_url.to_string(),
            source: e,
        })?;

    let response = client
        .post(api_url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| DoctorError::NetworkError {
            url: api_url.to_string(),
            source: e,
        })?;

    let response = response
        .error_for_status()
        .map_err(|e| DoctorError::NetworkError {
            url: api_url.to_string(),
            source: e,
        })?;

    let chat_response: ChatResponse = response.json().await.map_err(|e| {
        DoctorError::ParseError {
            file: "LLM API response".to_string(),
            message: format!("Failed to parse response: {e}"),
        }
    })?;

    let content = chat_response
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_else(|| "No explanation generated.".to_string());

    Ok(AiExplanation {
        raw_response: content,
        model_used: model.to_string(),
    })
}
