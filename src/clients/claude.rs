//! Anthropic Claude client implementation

use crate::{
    execute_with_retry, sse::sse_stream, AiClient, AiResponse, ApiError, ApiErrorType,
    ClientConfig, ClientError, Conversation, Message, ResponseMetadata, StreamChunk,
    StreamError, StreamErrorType,
};
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

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

    async fn send_prompt_with_metadata(&self, prompt: &str) -> Result<AiResponse, ClientError> {
        let mut conversation = Conversation::new();
        if let Some(system_msg) = &self.config.system_message {
            conversation.add_message(Message::system(system_msg));
        }
        conversation.add_user(prompt);
        self.send_conversation_with_metadata(&conversation).await
    }

    async fn send_conversation_with_metadata(
        &self,
        conversation: &Conversation,
    ) -> Result<AiResponse, ClientError> {
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
            #[serde(default)]
            id: Option<String>,
            #[serde(default)]
            model: Option<String>,
            #[serde(default)]
            usage: Option<Usage>,
        }

        #[derive(Deserialize)]
        struct ContentBlock {
            text: String,
        }

        #[derive(Deserialize)]
        struct Usage {
            input_tokens: Option<u32>,
            output_tokens: Option<u32>,
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

        let start_time = Instant::now();

        let (content, resp) = execute_with_retry(self.config.retries, || async {
            let response = self
                .http
                .post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &self.key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                return Err(ClientError::Api(ApiError {
                    message: format!("Claude API error ({}): {}", status, error_text),
                    status_code: Some(status.as_u16()),
                    error_type: ApiErrorType::Other,
                }));
            }

            let resp: Response = response.json().await?;
            let content = resp
                .content
                .first()
                .map(|c| c.text.clone())
                .unwrap_or_else(|| "No response from Claude".to_string());
            
            Ok((content, resp))
        })
        .await?;

        let latency_ms = start_time.elapsed().as_millis() as u64;

        let metadata = ResponseMetadata {
            model_used: resp.model,
            prompt_tokens: resp.usage.as_ref().and_then(|u| u.input_tokens),
            completion_tokens: resp.usage.as_ref().and_then(|u| u.output_tokens),
            total_tokens: resp.usage.as_ref().and_then(|u| {
                u.input_tokens
                    .zip(u.output_tokens)
                    .map(|(i, o)| i + o)
            }),
            finish_reason: None,
            safety_ratings: None,
            request_id: resp.id,
            latency_ms: Some(latency_ms),
        };

        Ok(AiResponse::with_metadata(content, metadata))
    }

    async fn stream_prompt(
        &self,
        prompt: &str,
    ) -> Result<BoxStream<'_, Result<StreamChunk, ClientError>>, ClientError> {
        let mut conversation = Conversation::new();
        if let Some(system_msg) = &self.config.system_message {
            conversation.add_message(Message::system(system_msg));
        }
        conversation.add_user(prompt);
        self.stream_conversation(&conversation).await
    }

    async fn stream_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<BoxStream<'_, Result<StreamChunk, ClientError>>, ClientError> {
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
            stream: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f32>,
            #[serde(skip_serializing_if = "Option::is_none")]
            system: Option<String>,
        }

        #[derive(Deserialize, Debug)]
        #[serde(tag = "type")]
        enum StreamEvent {
            #[serde(rename = "message_start")]
            MessageStart {
                message: MessageInfo,
            },
            #[serde(rename = "content_block_start")]
            ContentBlockStart {
                index: usize,
                content_block: ContentBlock,
            },
            #[serde(rename = "content_block_delta")]
            ContentBlockDelta {
                index: usize,
                delta: Delta,
            },
            #[serde(rename = "content_block_stop")]
            ContentBlockStop {
                index: usize,
            },
            #[serde(rename = "message_delta")]
            MessageDelta {
                delta: MessageDeltaInfo,
                usage: Option<Usage>,
            },
            #[serde(rename = "message_stop")]
            MessageStop,
            #[serde(rename = "ping")]
            Ping,
        }

        #[derive(Deserialize, Debug)]
        struct MessageInfo {
            id: Option<String>,
            model: Option<String>,
            usage: Option<Usage>,
        }

        #[derive(Deserialize, Debug)]
        struct ContentBlock {
            #[serde(rename = "type")]
            block_type: String,
            text: Option<String>,
        }

        #[derive(Deserialize, Debug)]
        struct Delta {
            #[serde(rename = "type")]
            delta_type: Option<String>,
            text: Option<String>,
        }

        #[derive(Deserialize, Debug)]
        struct MessageDeltaInfo {
            stop_reason: Option<String>,
        }

        #[derive(Deserialize, Debug)]
        struct Usage {
            input_tokens: Option<u32>,
            output_tokens: Option<u32>,
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
            stream: true,
            temperature: self.config.temperature,
            system: system_message,
        };

        let response = self
            .http
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ClientError::Api(ApiError {
                message: format!("Claude API error ({}): {}", status, error_text),
                status_code: Some(status.as_u16()),
                error_type: ApiErrorType::Other,
            }));
        }

        // Parse SSE stream
        let sse_stream = sse_stream(response);
        let start_time = Arc::new(std::sync::Mutex::new(Instant::now()));
        let message_info = Arc::new(std::sync::Mutex::new(None));
        let usage_info = Arc::new(std::sync::Mutex::new(None));
        
        let stream = sse_stream
            .filter_map(move |event| {
                let start_time = Arc::clone(&start_time);
                let message_info = Arc::clone(&message_info);
                let usage_info = Arc::clone(&usage_info);
                
                async move {
                    match event {
                        Ok(sse_event) => {
                            // Parse the JSON data
                            match serde_json::from_str::<StreamEvent>(&sse_event.data) {
                                Ok(stream_event) => {
                                    match stream_event {
                                        StreamEvent::MessageStart { message } => {
                                            *message_info.lock().unwrap() = Some(message);
                                            None
                                        }
                                        StreamEvent::ContentBlockDelta { delta, .. } => {
                                            if let Some(text) = delta.text {
                                                Some(Ok(StreamChunk {
                                                    content: text,
                                                    finished: false,
                                                    metadata: None,
                                                }))
                                            } else {
                                                None
                                            }
                                        }
                                        StreamEvent::MessageDelta { delta, usage } => {
                                            if let Some(u) = usage {
                                                *usage_info.lock().unwrap() = Some(u);
                                            }
                                            
                                            // If this has a stop reason, create final chunk with metadata
                                            if delta.stop_reason.is_some() {
                                                let latency_ms = start_time.lock().unwrap().elapsed().as_millis() as u64;
                                                let msg_info = message_info.lock().unwrap();
                                                let usage = usage_info.lock().unwrap();
                                                
                                                let metadata = ResponseMetadata {
                                                    model_used: msg_info.as_ref().and_then(|m| m.model.clone()),
                                                    prompt_tokens: usage.as_ref().and_then(|u| u.input_tokens),
                                                    completion_tokens: usage.as_ref().and_then(|u| u.output_tokens),
                                                    total_tokens: usage.as_ref().and_then(|u| {
                                                        u.input_tokens
                                                            .zip(u.output_tokens)
                                                            .map(|(i, o)| i + o)
                                                    }),
                                                    finish_reason: delta.stop_reason,
                                                    safety_ratings: None,
                                                    request_id: msg_info.as_ref().and_then(|m| m.id.clone()),
                                                    latency_ms: Some(latency_ms),
                                                };
                                                
                                                Some(Ok(StreamChunk {
                                                    content: String::new(),
                                                    finished: true,
                                                    metadata: Some(metadata),
                                                }))
                                            } else {
                                                None
                                            }
                                        }
                                        _ => None,
                                    }
                                }
                                Err(e) => {
                                    // Log parsing error but continue stream
                                    eprintln!("Failed to parse Claude SSE data: {}, data: {}", e, sse_event.data);
                                    None
                                }
                            }
                        }
                        Err(e) => Some(Err(ClientError::Stream(StreamError {
                            message: format!("SSE stream error: {}", e),
                            error_type: StreamErrorType::Other,
                        }))),
                    }
                }
            });

        Ok(Box::pin(stream))
    }
}
