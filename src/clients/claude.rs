//! Anthropic Claude client implementation

use crate::{execute_with_retry, AiClient, ClientConfig, ClientError};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Client for Anthropic's Claude models
pub struct Claude {
    /// Reqwest HTTP client used for requests
    http: Client,
    /// API key for Anthropic
    key: String,
    /// Name of the Claude model to invoke
    model: String,
    /// Maximum number of tokens to generate per response
    max_tokens: u32,
    /// Optional temperature controlling randomness
    temperature: Option<f32>,
    /// Number of retry attempts on failure
    retries: u32,
}

impl Claude {
    /// Create a new Claude client
    pub fn new(http: Client, key: String, model: String, config: ClientConfig) -> Self {
        Self {
            http,
            key,
            model,
            max_tokens: config.max_tokens.unwrap_or(1024),
            temperature: config.temperature,
            retries: config.retries,
        }
    }
}

#[async_trait]
impl AiClient for Claude {
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
            max_tokens: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
        }

        #[derive(Deserialize)]
        struct Response {
            content: Vec<ContentBlock>,
        }

        #[derive(Deserialize)]
        struct ContentBlock {
            text: String,
        }

        let body = Request {
            model: &self.model,
            messages: vec![Message {
                role: "user",
                content: prompt,
            }],
            max_tokens: self.max_tokens,
            temperature: self.temperature,
        };

        execute_with_retry(self.retries, || async {
            let response = self
                .http
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await?;

            let resp: Response = response.json().await?;
            Ok(resp
                .content
                .first()
                .map(|c| c.text.clone())
                .unwrap_or_else(|| "No response from Claude".to_string()))
        })
        .await
    }

    fn name(&self) -> &str {
        "Claude"
    }

    fn model(&self) -> &str {
        &self.model
    }
}
