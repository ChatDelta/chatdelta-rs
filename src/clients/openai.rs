//! OpenAI ChatGPT client implementation

use crate::{
    execute_with_retry, AiClient, AiResponse, ApiError, ApiErrorType, ClientConfig,
    ClientError, Conversation, Message, ParseError, ParseErrorType, ResponseMetadata,
    StreamChunk,
};
use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

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
        let response = self.send_prompt_with_metadata(prompt).await?;
        Ok(response.content)
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
    
    async fn send_prompt_with_metadata(&self, prompt: &str) -> Result<AiResponse, ClientError> {
        let conversation = Conversation {
            messages: vec![Message::user(prompt)],
        };
        self.send_conversation_with_metadata(&conversation).await
    }

    async fn send_conversation_with_metadata(
        &self,
        conversation: &Conversation,
    ) -> Result<AiResponse, ClientError> {
        #[derive(Serialize)]
        struct ApiMessage<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Serialize)]
        struct Request<'a> {
            model: &'a str,
            messages: Vec<ApiMessage<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            top_p: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            frequency_penalty: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            presence_penalty: Option<f32>,
        }

        #[derive(Deserialize)]
        struct Response {
            choices: Option<Vec<Choice>>,
            error: Option<ErrorInfo>,
            usage: Option<Usage>,
            model: Option<String>,
            id: Option<String>,
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
            finish_reason: Option<String>,
        }

        #[derive(Deserialize)]
        struct RespMessage {
            content: String,
        }

        #[derive(Deserialize)]
        struct Usage {
            prompt_tokens: Option<u32>,
            completion_tokens: Option<u32>,
            total_tokens: Option<u32>,
        }

        let mut messages = Vec::new();
        
        // Add system message if configured
        if let Some(system_msg) = &self.config.system_message {
            messages.push(ApiMessage {
                role: "system",
                content: system_msg,
            });
        }
        
        // Add conversation messages
        for msg in &conversation.messages {
            messages.push(ApiMessage {
                role: &msg.role,
                content: &msg.content,
            });
        }

        let body = Request {
            model: &self.model,
            messages,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            top_p: self.config.top_p,
            frequency_penalty: self.config.frequency_penalty,
            presence_penalty: self.config.presence_penalty,
        };

        let start_time = Instant::now();

        let (content, resp) = execute_with_retry(self.config.retries, || async {
            let url = if let Some(base_url) = &self.config.base_url {
                format!("{}/chat/completions", base_url.trim_end_matches('/'))
            } else {
                "https://api.openai.com/v1/chat/completions".to_string()
            };
            
            let response = self
                .http
                .post(&url)
                .bearer_auth(&self.key)
                .json(&body)
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(response.error_for_status().unwrap_err().into());
            }

            let resp: Response = response.json().await?;

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

            let choices = resp.choices.as_ref().ok_or_else(|| {
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

            let content = choices
                .first()
                .map(|c| c.message.content.clone())
                .unwrap_or_else(|| "No response from ChatGPT".to_string());

            Ok((content, resp))
        })
        .await?;

        let latency_ms = start_time.elapsed().as_millis() as u64;

        let metadata = ResponseMetadata {
            model_used: resp.model,
            prompt_tokens: resp.usage.as_ref().and_then(|u| u.prompt_tokens),
            completion_tokens: resp.usage.as_ref().and_then(|u| u.completion_tokens),
            total_tokens: resp.usage.as_ref().and_then(|u| u.total_tokens),
            finish_reason: resp
                .choices
                .and_then(|c| c.first().and_then(|ch| ch.finish_reason.clone())),
            safety_ratings: None,
            request_id: resp.id,
            latency_ms: Some(latency_ms),
        };

        Ok(AiResponse::with_metadata(content, metadata))
    }

    async fn send_conversation(&self, conversation: &Conversation) -> Result<String, ClientError> {
        let response = self.send_conversation_with_metadata(conversation).await?;
        Ok(response.content)
    }

    async fn stream_prompt(
        &self,
        prompt: &str,
    ) -> Result<BoxStream<'_, Result<StreamChunk, ClientError>>, ClientError> {
        let conversation = Conversation {
            messages: vec![Message::user(prompt)],
        };
        self.stream_conversation(&conversation).await
    }

    async fn stream_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<BoxStream<'_, Result<StreamChunk, ClientError>>, ClientError> {
        #[derive(Serialize)]
        struct ApiMessage<'a> {
            role: &'a str,
            content: &'a str,
        }

        #[derive(Serialize)]
        struct Request<'a> {
            model: &'a str,
            messages: Vec<ApiMessage<'a>>,
            stream: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            max_tokens: Option<u32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            top_p: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            frequency_penalty: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            presence_penalty: Option<f32>,
        }

        let mut messages = Vec::new();
        
        // Add system message if configured
        if let Some(system_msg) = &self.config.system_message {
            messages.push(ApiMessage {
                role: "system",
                content: system_msg,
            });
        }
        
        // Add conversation messages
        for msg in &conversation.messages {
            messages.push(ApiMessage {
                role: &msg.role,
                content: &msg.content,
            });
        }

        let body = Request {
            model: &self.model,
            messages,
            stream: true,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            top_p: self.config.top_p,
            frequency_penalty: self.config.frequency_penalty,
            presence_penalty: self.config.presence_penalty,
        };

        let url = if let Some(base_url) = &self.config.base_url {
            format!("{}/chat/completions", base_url.trim_end_matches('/'))
        } else {
            "https://api.openai.com/v1/chat/completions".to_string()
        };

        let response = self
            .http
            .post(&url)
            .bearer_auth(&self.key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(response.error_for_status().unwrap_err().into());
        }

        // For streaming, OpenAI returns server-sent events
        // For now, we'll provide a basic implementation that falls back to non-streaming
        // A full implementation would parse SSE events
        let content = self.send_conversation(conversation).await?;
        let chunk = StreamChunk {
            content,
            finished: true,
            metadata: None,
        };
        Ok(Box::pin(stream::once(async { Ok(chunk) })))
    }
}
