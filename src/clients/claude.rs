//! Anthropic Claude client implementation

use crate::{execute_with_retry, AiClient, ClientConfig, ClientError, Conversation, Message};
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
    /// Configuration for the client
    config: ClientConfig,
}

impl Claude {
    /// Create a new Claude client
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
impl AiClient for Claude {
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
        struct ClaudeMessage {
            role: String,
            content: String,
        }

        #[derive(Serialize)]
        struct Request {
            model: String,
            messages: Vec<ClaudeMessage>,
            max_tokens: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            system: Option<String>,
        }

        #[derive(Deserialize)]
        struct Response {
            content: Vec<ContentBlock>,
        }

        #[derive(Deserialize)]
        struct ContentBlock {
            text: String,
        }

        // Claude API requires system messages to be handled separately
        let (system_message, messages): (Option<String>, Vec<_>) = {
            let mut system_msg = None;
            let mut regular_messages = Vec::new();

            for msg in &conversation.messages {
                if msg.role == "system" {
                    system_msg = Some(msg.content.clone());
                } else {
                    regular_messages.push(ClaudeMessage {
                        role: msg.role.clone(),
                        content: msg.content.clone(),
                    });
                }
            }
            (system_msg, regular_messages)
        };

        let body = Request {
            model: self.model.clone(),
            messages,
            max_tokens: self.config.max_tokens.unwrap_or(1024),
            temperature: self.config.temperature,
            system: system_message,
        };

        execute_with_retry(self.config.retries, || async {
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

    fn supports_conversations(&self) -> bool {
        true
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        "Claude"
    }

    fn model(&self) -> &str {
        &self.model
    }
}
