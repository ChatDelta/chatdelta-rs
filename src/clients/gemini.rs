//! Google Gemini client implementation

use crate::{
    execute_with_retry, AiClient, ApiErrorType, ClientConfig, ClientError, Conversation,
    Message,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Client for Google Gemini models
pub struct Gemini {
    /// Reqwest HTTP client used for requests
    http: Client,
    /// API key for Gemini access
    key: String,
    /// Model identifier such as `"gemini-1.5-pro"`
    model: String,
    /// Configuration for the client
    config: ClientConfig,
}

impl Gemini {
    /// Create a new Gemini client
    pub fn new(http: Client, key: String, model: String, config: ClientConfig) -> Self {
        Self {
            http,
            key,
            model,
            config,
        }
    }
}

#[async_trait]
impl AiClient for Gemini {
    async fn send_prompt(&self, prompt: &str) -> Result<String, ClientError> {
        let mut conversation = Conversation::new();
        if let Some(system_msg) = &self.config.system_message {
            conversation.add_message(Message::system(system_msg));
        }
        conversation.add_user(prompt);
        self.send_conversation(&conversation).await
    }

    async fn send_conversation(&self, conversation: &Conversation) -> Result<String, ClientError> {
        #[derive(Serialize)]
        struct Part<'a> {
            text: &'a str,
        }

        #[derive(Serialize)]
        struct Content<'a> {
            parts: Vec<Part<'a>>,
        }

        #[derive(Serialize)]
        struct Request<'a> {
            contents: Vec<Content<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            generation_config: Option<GenerationConfig>,
        }

        #[derive(Serialize)]
        struct GenerationConfig {
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
        }

        #[derive(Deserialize)]
        struct Response {
            #[serde(default)]
            candidates: Vec<Candidate>,
            error: Option<ApiError>,
        }

        #[derive(Deserialize)]
        struct ApiError {
            code: u32,
            message: String,
            #[allow(dead_code)]
            status: String,
        }

        #[derive(Deserialize)]
        struct Candidate {
            content: CandContent,
        }

        #[derive(Deserialize)]
        struct CandContent {
            parts: Vec<CandPart>,
        }

        #[derive(Deserialize)]
        struct CandPart {
            text: String,
        }

        // Convert conversation to Gemini format - for now just use the last user message
        let user_content = conversation
            .messages
            .iter()
            .rev()
            .find(|msg| msg.role == "user")
            .map(|msg| msg.content.as_str())
            .unwrap_or("");

        let body = Request {
            contents: vec![Content {
                parts: vec![Part { text: user_content }],
            }],
            generation_config: self.config.temperature.map(|temp| GenerationConfig {
                temperature: Some(temp),
            }),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        );

        execute_with_retry(self.config.retries, || async {
            let response = self
                .http
                .post(&url)
                .header("X-goog-api-key", &self.key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await?;

            let response_text = response.text().await?;
            let resp: Response = serde_json::from_str(&response_text)?;

            if let Some(error) = resp.error {
                let error_type = match error.code {
                    429 => ApiErrorType::RateLimit,
                    403 => ApiErrorType::QuotaExceeded,
                    400 => ApiErrorType::BadRequest,
                    _ => ApiErrorType::Other,
                };
                return Err(ClientError::Api(crate::ApiError {
                    message: format!("Gemini API Error ({}): {}", error.code, error.message),
                    status_code: Some(error.code as u16),
                    error_type,
                }));
            }

            Ok(resp
                .candidates
                .first()
                .and_then(|c| c.content.parts.first())
                .map(|p| p.text.clone())
                .unwrap_or_else(|| "No response from Gemini".to_string()))
        })
        .await
    }

    fn supports_conversations(&self) -> bool {
        true
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "Gemini"
    }

    fn model(&self) -> &str {
        &self.model
    }
}
