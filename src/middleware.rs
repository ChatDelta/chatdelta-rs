//! Common middleware for AI provider clients
//!
//! This module provides reusable components for retry logic, request/response processing,
//! and common HTTP client configuration across all AI providers.

use crate::{ClientError, ClientConfig};
use async_trait::async_trait;
use reqwest::{Client, Response, RequestBuilder};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn, instrument};

/// Trait for provider-specific request processing
#[async_trait]
pub trait RequestMiddleware: Send + Sync {
    /// Process request before sending
    fn process_request(&self, request: RequestBuilder) -> RequestBuilder {
        request
    }

    /// Validate response after receiving
    async fn validate_response(&self, response: Response) -> Result<Response, ClientError> {
        Ok(response)
    }
}

/// Base HTTP client with common retry and timeout logic
pub struct MiddlewareClient {
    client: Client,
    config: ClientConfig,
    provider_name: String,
}

impl MiddlewareClient {
    /// Create a new middleware client
    pub fn new(client: Client, config: ClientConfig, provider_name: String) -> Self {
        Self {
            client,
            config,
            provider_name,
        }
    }

    /// Execute request with retry logic
    #[instrument(skip(self, request_fn), fields(provider = %self.provider_name))]
    pub async fn execute_with_retry<F, Fut, T>(
        &self,
        mut request_fn: F,
    ) -> Result<T, ClientError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, ClientError>>,
    {
        let mut attempts = 0;
        let max_attempts = self.config.retries + 1;

        loop {
            attempts += 1;
            debug!("Attempt {}/{}", attempts, max_attempts);

            match request_fn().await {
                Ok(result) => return Ok(result),
                Err(err) if attempts >= max_attempts => {
                    warn!("All retry attempts exhausted: {}", err);
                    return Err(err);
                }
                Err(err) if self.should_retry(&err) => {
                    let delay = self.get_retry_delay(attempts);
                    warn!("Request failed (attempt {}), retrying in {:?}: {}", attempts, delay, err);
                    sleep(delay).await;
                }
                Err(err) => {
                    debug!("Non-retryable error: {}", err);
                    return Err(err);
                }
            }
        }
    }

    /// Determine if an error is retryable
    fn should_retry(&self, error: &ClientError) -> bool {
        match error {
            ClientError::Network(net_err) => {
                matches!(
                    net_err.error_type,
                    crate::NetworkErrorType::Timeout |
                    crate::NetworkErrorType::ConnectionFailed |
                    crate::NetworkErrorType::ConnectionReset
                )
            }
            ClientError::Api(api_err) => {
                matches!(
                    api_err.error_type,
                    crate::ApiErrorType::RateLimit |
                    crate::ApiErrorType::ServerError
                )
            }
            _ => false,
        }
    }

    /// Calculate retry delay based on strategy
    fn get_retry_delay(&self, attempt: u32) -> Duration {
        self.config.retry_strategy.delay(attempt - 1)
    }

    /// Get the underlying HTTP client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get the configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }
}

/// Common streaming utilities
pub mod streaming {
    use crate::{StreamChunk, ClientError};
    use futures::stream::{Stream, StreamExt};
    
    use tokio::sync::mpsc;
    use tracing::error;

    /// Convert a stream to channel-based interface
    pub async fn stream_to_channel<S>(
        mut stream: S,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), ClientError>
    where
        S: Stream<Item = Result<StreamChunk, ClientError>> + Unpin,
    {
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(chunk) => {
                    let is_finished = chunk.finished;
                    if tx.send(chunk).is_err() {
                        error!("Channel receiver dropped");
                        break;
                    }
                    if is_finished {
                        break;
                    }
                }
                Err(e) => {
                    error!("Stream error: {}", e);
                    // Send error as final chunk
                    let _ = tx.send(StreamChunk {
                        content: format!("Error: {}", e),
                        finished: true,
                        metadata: None,
                    });
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

/// Response validation utilities
pub mod validation {
    use crate::ClientError;
    use serde::de::DeserializeOwned;
    use serde_json::Value;

    /// Validate JSON response structure
    pub fn validate_json_response<T: DeserializeOwned>(
        json: &Value,
        required_fields: &[&str],
    ) -> Result<T, ClientError> {
        // Check required fields exist
        for field in required_fields {
            if json.get(field).is_none() {
                return Err(ClientError::Parse(crate::ParseError {
                    message: format!("Missing required field: {}", field),
                    error_type: crate::ParseErrorType::MissingField,
                    raw_content: Some(json.to_string()),
                }));
            }
        }

        // Deserialize
        serde_json::from_value(json.clone()).map_err(|e| {
            ClientError::Parse(crate::ParseError {
                message: format!("Failed to deserialize response: {}", e),
                error_type: crate::ParseErrorType::JsonParsing,
                raw_content: Some(json.to_string()),
            })
        })
    }

    /// Validate API error response
    pub fn extract_api_error(json: &Value) -> Option<String> {
        json.get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .map(String::from)
            .or_else(|| {
                json.get("message")
                    .and_then(|m| m.as_str())
                    .map(String::from)
            })
    }
}