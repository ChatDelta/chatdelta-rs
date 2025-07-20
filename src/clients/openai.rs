//! OpenAI ChatGPT client implementation

use crate::{
    execute_with_retry, AiClient, ApiError, ApiErrorType, ClientConfig, ClientError,
    ParseError, ParseErrorType,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Client for OpenAI's ChatGPT models
pub struct ChatGpt {
    /// Reqwest HTTP client used for requests
    http: Client,
    /// API key for authenticating with OpenAI
    key: String,
    /// Model name to call, e.g. `"gpt-4"`
    model: String,
    /// Configuration for the client
    config: ClientConfig,
}

impl ChatGpt {
    /// Create a new ChatGPT client
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
impl AiClient for ChatGpt {
    async fn send_prompt(&self, prompt: &str) -> Result<String, ClientError> {
        #[derive(Serialize)]
        struct Message<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Serialize)]
        struct Request<'a> {
            model: &'a str,
            messages: Vec<Message<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
        }

        #[derive(Deserialize)]
        struct Response {
            choices: Option<Vec<Choice>>,
            error: Option<ErrorInfo>,
        }

        #[derive(Deserialize)]
        struct ErrorInfo {
            message: String,
            #[serde(rename = "type")]
            error_type: Option<String>,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: RespMessage,
        }

        #[derive(Deserialize)]
        struct RespMessage {
            content: String,
        }

        let body = Request {
            model: &self.model,
            messages: vec![Message {
                role: "user",
                content: prompt,
            }],
            temperature: self.config.temperature,
        };

        execute_with_retry(self.config.retries, || async {
            let response = self
                .http
                .post("https://api.openai.com/v1/chat/completions")
                .bearer_auth(&self.key)
                .json(&body)
                .send()
                .await?;

            // Check for HTTP error status codes
            if !response.status().is_success() {
                return Err(response.error_for_status().unwrap_err().into());
            }

            let resp: Response = response.json().await?;

            // Check for error in response body
            if let Some(error) = resp.error {
                let error_type = match error.error_type.as_deref() {
                    Some("insufficient_quota") => ApiErrorType::QuotaExceeded,
                    Some("model_not_found") => ApiErrorType::InvalidModel,
                    Some("content_filter") => ApiErrorType::ContentFilter,
                    _ => ApiErrorType::Other,
                };
                return Err(ClientError::Api(ApiError {
                    message: format!("OpenAI API error: {}", error.message),
                    status_code: None,
                    error_type,
                }));
            }

            // Check for missing or empty choices
            let choices = resp.choices.ok_or_else(|| {
                ClientError::Parse(ParseError {
                    message: "OpenAI response missing 'choices' field".to_string(),
                    error_type: ParseErrorType::MissingField,
                })
            })?;

            if choices.is_empty() {
                return Err(ClientError::Api(ApiError {
                    message: "OpenAI returned empty choices array".to_string(),
                    status_code: None,
                    error_type: ApiErrorType::Other,
                }));
            }

            Ok(choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_else(|| "No response from ChatGPT".to_string()))
        })
        .await
    }

    fn supports_conversations(&self) -> bool {
        true
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "ChatGPT"
    }

    fn model(&self) -> &str {
        &self.model
    }
}
