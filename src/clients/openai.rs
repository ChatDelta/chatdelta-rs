//! OpenAI ChatGPT client implementation

use crate::{execute_with_retry, AiClient, ClientConfig, ClientError};
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
    /// Optional temperature parameter controlling response creativity
    temperature: Option<f32>,
    /// Number of times to retry a failed request
    retries: u32,
}

impl ChatGpt {
    /// Create a new ChatGPT client
    pub fn new(http: Client, key: String, model: String, config: ClientConfig) -> Self {
        Self {
            http,
            key,
            model,
            temperature: config.temperature,
            retries: config.retries,
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
            temperature: self.temperature,
        };

        execute_with_retry(self.retries, || async {
            let response = self
                .http
                .post("https://api.openai.com/v1/chat/completions")
                .bearer_auth(&self.key)
                .json(&body)
                .send()
                .await?;

            // Check for HTTP error status codes
            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Err(ClientError::Api(format!("OpenAI API error {}: {}", status, error_text)));
            }

            let resp: Response = response.json().await?;
            
            // Check for error in response body
            if let Some(error) = resp.error {
                return Err(ClientError::Api(format!("OpenAI API error: {}", error.message)));
            }
            
            // Check for missing or empty choices
            let choices = resp.choices.ok_or_else(|| 
                ClientError::Parse("OpenAI response missing 'choices' field".to_string()))?;
            
            if choices.is_empty() {
                return Err(ClientError::Api("OpenAI returned empty choices array".to_string()));
            }
            
            Ok(choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_else(|| "No response from ChatGPT".to_string()))
        })
        .await
    }

    fn name(&self) -> &str {
        "ChatGPT"
    }

    fn model(&self) -> &str {
        &self.model
    }
}
