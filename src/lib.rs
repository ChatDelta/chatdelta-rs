//! # ChatDelta AI Client Library
//!
//! A Rust library for connecting to multiple AI APIs (OpenAI, Google Gemini, Anthropic Claude)
//! with a unified interface. Supports parallel execution, retry logic, and configurable parameters.
//!
//! ## Example
//!
//! ```rust,no_run
//! use chatdelta::{AiClient, ClientConfig, create_client};
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ClientConfig::builder()
//!         .timeout(Duration::from_secs(30))
//!         .retries(3)
//!         .temperature(0.7)
//!         .max_tokens(1024)
//!         .build();
//!     
//!     let client = create_client("openai", "your-api-key", "gpt-4o", config)?;
//!     let response = client.send_prompt("Hello, world!").await?;
//!     println!("{}", response);
//!     
//!     Ok(())
//! }
//! ```

use async_trait::async_trait;
use futures::stream::BoxStream;
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc;

pub mod clients;
pub mod error;
pub mod http;
pub mod metrics;
pub mod orchestration;
pub mod prompt_optimizer;
pub mod utils;
mod sse;

pub use clients::*;
pub use error::*;
pub use http::{HttpConfig, get_provider_client, SHARED_CLIENT};
pub use metrics::{ClientMetrics, MetricsSnapshot, RequestTimer};
pub use orchestration::{AiOrchestrator, FusedResponse, OrchestrationStrategy, ModelCapabilities};
pub use prompt_optimizer::{PromptOptimizer, OptimizedPrompt};
pub use utils::{execute_with_retry, RetryStrategy};

/// Configuration for AI clients
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Timeout for HTTP requests
    pub timeout: Duration,
    /// Number of retry attempts for failed requests
    pub retries: u32,
    /// Temperature for AI responses (0.0-2.0)
    pub temperature: Option<f32>,
    /// Maximum tokens for responses
    pub max_tokens: Option<u32>,
    /// Top-p sampling parameter (0.0-1.0)
    pub top_p: Option<f32>,
    /// Frequency penalty (-2.0 to 2.0)
    pub frequency_penalty: Option<f32>,
    /// Presence penalty (-2.0 to 2.0)
    pub presence_penalty: Option<f32>,
    /// System message for conversation context
    pub system_message: Option<String>,
    /// Custom base URL for API endpoint (e.g., for Azure OpenAI, local models, proxies)
    pub base_url: Option<String>,
    /// Retry strategy for failed requests
    pub retry_strategy: RetryStrategy,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            retries: 0,
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            system_message: None,
            base_url: None,
            retry_strategy: RetryStrategy::default(),
        }
    }
}

impl ClientConfig {
    /// Create a new ClientConfig builder
    pub fn builder() -> ClientConfigBuilder {
        ClientConfigBuilder::default()
    }
}

/// Builder for ClientConfig
#[derive(Debug, Default)]
pub struct ClientConfigBuilder {
    timeout: Option<Duration>,
    retries: Option<u32>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    top_p: Option<f32>,
    frequency_penalty: Option<f32>,
    presence_penalty: Option<f32>,
    system_message: Option<String>,
    base_url: Option<String>,
    retry_strategy: Option<RetryStrategy>,
}

impl ClientConfigBuilder {
    /// Set request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set number of retry attempts
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = Some(retries);
        self
    }

    /// Set temperature (0.0-2.0)
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set maximum tokens
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set top-p sampling (0.0-1.0)
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set frequency penalty (-2.0 to 2.0)
    pub fn frequency_penalty(mut self, penalty: f32) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set presence penalty (-2.0 to 2.0)
    pub fn presence_penalty(mut self, penalty: f32) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Set system message
    pub fn system_message<S: Into<String>>(mut self, message: S) -> Self {
        self.system_message = Some(message.into());
        self
    }

    /// Set custom base URL for API endpoint
    pub fn base_url<S: Into<String>>(mut self, url: S) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set retry strategy
    pub fn retry_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.retry_strategy = Some(strategy);
        self
    }

    /// Build the ClientConfig
    pub fn build(self) -> ClientConfig {
        ClientConfig {
            timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
            retries: self.retries.unwrap_or(0),
            temperature: self.temperature,
            max_tokens: self.max_tokens.or(Some(1024)),
            top_p: self.top_p,
            frequency_penalty: self.frequency_penalty,
            presence_penalty: self.presence_penalty,
            system_message: self.system_message,
            base_url: self.base_url,
            retry_strategy: self.retry_strategy.unwrap_or_default(),
        }
    }
}

/// Represents a single message in a conversation
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Message {
    /// Role of the message sender ("system", "user", "assistant")
    pub role: String,
    /// Content of the message
    pub content: String,
}

impl Message {
    /// Create a new system message
    pub fn system<S: Into<String>>(content: S) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    /// Create a new user message
    pub fn user<S: Into<String>>(content: S) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    /// Create a new assistant message
    pub fn assistant<S: Into<String>>(content: S) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Represents a conversation with message history
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    /// Messages in the conversation
    pub messages: Vec<Message>,
}

impl Conversation {
    /// Create a new empty conversation
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a conversation with a system message
    pub fn with_system<S: Into<String>>(system_message: S) -> Self {
        Self {
            messages: vec![Message::system(system_message)],
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Add a user message to the conversation
    pub fn add_user<S: Into<String>>(&mut self, content: S) {
        self.add_message(Message::user(content));
    }

    /// Add an assistant message to the conversation
    pub fn add_assistant<S: Into<String>>(&mut self, content: S) {
        self.add_message(Message::assistant(content));
    }

    /// Get the last message from the conversation
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }

    /// Clear all messages from the conversation
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Get the number of messages in the conversation
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if the conversation is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

/// Response metadata containing additional information from the AI provider
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ResponseMetadata {
    /// Model version that was actually used (may differ from requested)
    pub model_used: Option<String>,
    /// Number of tokens in the prompt
    pub prompt_tokens: Option<u32>,
    /// Number of tokens in the completion
    pub completion_tokens: Option<u32>,
    /// Total tokens used (prompt + completion)
    pub total_tokens: Option<u32>,
    /// Finish reason (e.g., "stop", "length", "content_filter")
    pub finish_reason: Option<String>,
    /// Safety ratings or content filter results
    pub safety_ratings: Option<serde_json::Value>,
    /// Request ID for debugging
    pub request_id: Option<String>,
    /// Time taken to generate response in milliseconds
    pub latency_ms: Option<u64>,
}

/// AI response with content and metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiResponse {
    /// The actual text content of the response
    pub content: String,
    /// Metadata about the response
    pub metadata: ResponseMetadata,
}

impl AiResponse {
    /// Create a new response with just content (no metadata)
    pub fn new(content: String) -> Self {
        Self {
            content,
            metadata: ResponseMetadata::default(),
        }
    }

    /// Create a response with content and metadata
    pub fn with_metadata(content: String, metadata: ResponseMetadata) -> Self {
        Self { content, metadata }
    }
}

/// Streaming response chunk
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Content of this chunk
    pub content: String,
    /// Whether this is the final chunk
    pub finished: bool,
    /// Metadata (only populated on final chunk)
    pub metadata: Option<ResponseMetadata>,
}

/// A session for managing multi-turn conversations with an AI client.
/// 
/// Automatically maintains conversation history and handles context management.
/// 
/// # Example
/// 
/// ```no_run
/// # use chatdelta::{ChatSession, create_client, ClientConfig};
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = create_client("openai", "key", "gpt-4", ClientConfig::default())?;
/// let mut session = ChatSession::with_system_message(client, "You are a helpful assistant.");
/// 
/// let response1 = session.send("What is Rust?").await?;
/// let response2 = session.send("What are its main features?").await?; // Remembers context
/// # Ok(())
/// # }
/// ```
pub struct ChatSession {
    /// The AI client to use for this session
    client: Box<dyn AiClient>,
    /// The conversation history
    conversation: Conversation,
}

impl ChatSession {
    /// Create a new chat session with the given client
    pub fn new(client: Box<dyn AiClient>) -> Self {
        Self {
            client,
            conversation: Conversation::new(),
        }
    }

    /// Create a new chat session with a system message
    pub fn with_system_message<S: Into<String>>(client: Box<dyn AiClient>, message: S) -> Self {
        Self {
            client,
            conversation: Conversation::with_system(message),
        }
    }

    /// Send a message and get a response
    pub async fn send<S: Into<String>>(&mut self, message: S) -> Result<String, ClientError> {
        let user_msg = message.into();
        self.conversation.add_user(user_msg);
        
        let response = self.client.send_conversation(&self.conversation).await?;
        self.conversation.add_assistant(&response);
        
        Ok(response)
    }

    /// Send a message and get a response with metadata
    pub async fn send_with_metadata<S: Into<String>>(
        &mut self,
        message: S,
    ) -> Result<AiResponse, ClientError> {
        let user_msg = message.into();
        self.conversation.add_user(user_msg);
        
        let response = self
            .client
            .send_conversation_with_metadata(&self.conversation)
            .await?;
        self.conversation.add_assistant(&response.content);
        
        Ok(response)
    }

    /// Stream a response for the given message
    pub async fn stream<S: Into<String>>(
        &mut self,
        message: S,
    ) -> Result<BoxStream<'_, Result<StreamChunk, ClientError>>, ClientError> {
        let user_msg = message.into();
        self.conversation.add_user(user_msg);
        
        self.client.stream_conversation(&self.conversation).await
    }

    /// Add a message to the conversation without sending
    pub fn add_message(&mut self, message: Message) {
        self.conversation.add_message(message);
    }

    /// Get the conversation history
    pub fn history(&self) -> &Conversation {
        &self.conversation
    }

    /// Get a mutable reference to the conversation history
    pub fn history_mut(&mut self) -> &mut Conversation {
        &mut self.conversation
    }

    /// Clear the conversation history
    pub fn clear(&mut self) {
        self.conversation.clear();
    }

    /// Reset the session with a new system message
    pub fn reset_with_system<S: Into<String>>(&mut self, message: S) {
        self.conversation = Conversation::with_system(message);
    }
}

/// Common trait implemented by all AI clients
#[async_trait]
pub trait AiClient: Send + Sync {
    /// Sends a prompt and returns the textual response
    async fn send_prompt(&self, prompt: &str) -> Result<String, ClientError>;

    /// Sends a prompt and returns response with metadata
    async fn send_prompt_with_metadata(&self, prompt: &str) -> Result<AiResponse, ClientError> {
        // Default implementation for backward compatibility
        let content = self.send_prompt(prompt).await?;
        Ok(AiResponse::new(content))
    }

    /// Sends a conversation and returns the textual response
    async fn send_conversation(&self, conversation: &Conversation) -> Result<String, ClientError> {
        // Default implementation converts conversation to a single prompt
        let prompt = if conversation.messages.is_empty() {
            return Err(ClientError::config("Empty conversation", None));
        } else if conversation.messages.len() == 1 {
            &conversation.messages[0].content
        } else {
            // For clients that don't support conversations, use the last user message
            conversation
                .messages
                .iter()
                .rev()
                .find(|m| m.role == "user")
                .map(|m| m.content.as_str())
                .unwrap_or(&conversation.messages.last().unwrap().content)
        };
        self.send_prompt(prompt).await
    }
    
    /// Sends a prompt and streams the response in chunks
    async fn send_prompt_streaming(
        &self,
        prompt: &str,
        tx: mpsc::UnboundedSender<StreamChunk>,
    ) -> Result<(), ClientError> {
        // Default implementation: send the whole response as one chunk
        let response = self.send_prompt(prompt).await?;
        tx.send(StreamChunk {
            content: response,
            finished: true,
            metadata: None,
        }).map_err(|_| ClientError::Stream(crate::StreamError {
            message: "Failed to send stream chunk".into(),
            error_type: crate::StreamErrorType::Other,
        }))?;
        Ok(())
    }

    /// Sends a conversation and returns response with metadata
    async fn send_conversation_with_metadata(
        &self,
        conversation: &Conversation,
    ) -> Result<AiResponse, ClientError> {
        // Default implementation for backward compatibility
        let content = self.send_conversation(conversation).await?;
        Ok(AiResponse::new(content))
    }

    /// Sends a prompt and returns a stream of response chunks
    async fn stream_prompt(
        &self,
        _prompt: &str,
    ) -> Result<BoxStream<'_, Result<StreamChunk, ClientError>>, ClientError> {
        // Default implementation falls back to non-streaming
        let response = self.send_prompt(_prompt).await?;
        let chunk = StreamChunk {
            content: response,
            finished: true,
            metadata: None,
        };
        Ok(Box::pin(futures::stream::once(async { Ok(chunk) })))
    }

    /// Sends a conversation and returns a stream of response chunks
    async fn stream_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<BoxStream<'_, Result<StreamChunk, ClientError>>, ClientError> {
        // Default implementation falls back to non-streaming conversation
        let response = self.send_conversation(conversation).await?;
        let chunk = StreamChunk {
            content: response,
            finished: true,
            metadata: None,
        };
        Ok(Box::pin(futures::stream::once(async { Ok(chunk) })))
    }

    /// Returns whether this client supports streaming
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Returns whether this client supports conversation history
    fn supports_conversations(&self) -> bool {
        false
    }

    /// Returns the name/identifier of this AI client
    fn name(&self) -> &str;

    /// Returns the model being used by this client
    fn model(&self) -> &str;
}

/// Factory function to create AI clients
///
/// # Arguments
///
/// * `provider` - The AI provider: "openai", "google"/"gemini", or "anthropic"/"claude"
/// * `api_key` - The API key for the provider
/// * `model` - The model name (e.g., "gpt-4", "claude-3-sonnet-20240229", "gemini-1.5-pro")
/// * `config` - Configuration for timeouts, retries, and generation parameters
///
/// # Example
///
/// ```rust,no_run
/// use chatdelta::{create_client, ClientConfig};
/// use std::time::Duration;
///
/// let config = ClientConfig::default();
/// let client = create_client("openai", "your-api-key", "gpt-4", config)?;
/// # Ok::<(), chatdelta::ClientError>(())
/// ```
pub fn create_client(
    provider: &str,
    api_key: &str,
    model: &str,
    config: ClientConfig,
) -> Result<Box<dyn AiClient>, ClientError> {
    let http_client = Client::builder()
        .timeout(config.timeout)
        .build()
        .map_err(|e| ClientError::config(format!("Failed to create HTTP client: {e}"), None))?;

    match provider.to_lowercase().as_str() {
        "openai" | "gpt" | "chatgpt" => Ok(Box::new(ChatGpt::new(
            http_client,
            api_key.to_string(),
            model.to_string(),
            config,
        ))),
        "google" | "gemini" => Ok(Box::new(Gemini::new(
            http_client,
            api_key.to_string(),
            model.to_string(),
            config,
        ))),
        "anthropic" | "claude" => Ok(Box::new(Claude::new(
            http_client,
            api_key.to_string(),
            model.to_string(),
            config,
        ))),
        _ => Err(ClientError::config(
            format!("Unknown provider: {provider}. Supported providers: openai, google, anthropic"),
            Some("provider".to_string()),
        )),
    }
}

/// Execute multiple AI clients in parallel and return all results
///
/// This function runs all provided clients concurrently and returns the results
/// in the order they complete, not necessarily the order they were provided.
///
/// # Arguments
///
/// * `clients` - Vector of AI clients to execute
/// * `prompt` - The prompt to send to all clients
///
/// # Returns
///
/// A vector of tuples containing the client name and either the response or an error
///
/// # Example
///
/// ```rust,no_run
/// use chatdelta::{create_client, execute_parallel, ClientConfig};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ClientConfig::default();
/// let clients = vec![
///     create_client("openai", "key1", "gpt-4", config.clone())?,
///     create_client("anthropic", "key2", "claude-3-sonnet-20240229", config)?,
/// ];
///
/// let results = execute_parallel(clients, "Hello, world!").await;
/// for (name, result) in results {
///     match result {
///         Ok(response) => println!("{}: {}", name, response),
///         Err(e) => eprintln!("{} failed: {}", name, e),
///     }
/// }
/// # Ok(())
/// # }
/// ```
pub async fn execute_parallel(
    clients: Vec<Box<dyn AiClient>>,
    prompt: &str,
) -> Vec<(String, Result<String, ClientError>)> {
    use futures::future;

    let futures: Vec<_> = clients
        .iter()
        .map(|client| {
            let name = client.name().to_string();
            let prompt = prompt.to_string();
            async move {
                let result = client.send_prompt(&prompt).await;
                (name, result)
            }
        })
        .collect();

    future::join_all(futures).await
}

/// Execute multiple AI clients in parallel with a conversation and return all results
///
/// This function runs all provided clients concurrently using conversation history
/// and returns the results in the order they complete.
///
/// # Arguments
///
/// * `clients` - Vector of AI clients to execute
/// * `conversation` - The conversation to send to all clients
///
/// # Returns
///
/// A vector of tuples containing the client name and either the response or an error
pub async fn execute_parallel_conversation(
    clients: Vec<Box<dyn AiClient>>,
    conversation: &Conversation,
) -> Vec<(String, Result<String, ClientError>)> {
    use futures::future;

    let futures: Vec<_> = clients
        .iter()
        .map(|client| {
            let name = client.name().to_string();
            let conversation = conversation.clone();
            async move {
                let result = client.send_conversation(&conversation).await;
                (name, result)
            }
        })
        .collect();

    future::join_all(futures).await
}

/// Generate a summary using one of the provided clients
///
/// Takes the responses from multiple AI models and uses another AI client
/// to generate a summary highlighting key differences and commonalities.
///
/// # Arguments
///
/// * `client` - The AI client to use for generating the summary
/// * `responses` - Vector of tuples containing (model_name, response) pairs
///
/// # Example
///
/// ```rust,no_run
/// use chatdelta::{create_client, generate_summary, ClientConfig};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let config = ClientConfig::default();
/// let summarizer = create_client("openai", "your-key", "gpt-4", config)?;
///
/// let responses = vec![
///     ("GPT-4".to_string(), "Response from GPT-4...".to_string()),
///     ("Claude".to_string(), "Response from Claude...".to_string()),
/// ];
///
/// let summary = generate_summary(&*summarizer, &responses).await?;
/// println!("Summary: {}", summary);
/// # Ok(())
/// # }
/// ```
pub async fn generate_summary(
    client: &dyn AiClient,
    responses: &[(String, String)],
) -> Result<String, ClientError> {
    let mut summary_prompt = "Given these AI model responses:\n".to_string();
    for (name, response) in responses {
        summary_prompt.push_str(&format!("{name}:\n{response}\n---\n"));
    }
    summary_prompt.push_str("Summarize the key differences and commonalities.");

    client.send_prompt(&summary_prompt).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    // Mock client for testing
    pub struct MockClient {
        pub name: String,
        pub model: String,
        pub responses: Arc<Mutex<VecDeque<Result<String, ClientError>>>>,
    }

    impl MockClient {
        pub fn new(name: &str, responses: Vec<Result<String, ClientError>>) -> Self {
            Self {
                name: name.to_string(),
                model: "mock-model".to_string(),
                responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            }
        }
    }

    #[async_trait]
    impl AiClient for MockClient {
        async fn send_prompt(&self, _prompt: &str) -> Result<String, ClientError> {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok("mock response".to_string()))
        }

        async fn send_conversation(
            &self,
            _conversation: &Conversation,
        ) -> Result<String, ClientError> {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| Ok("mock conversation response".to_string()))
        }

        fn supports_conversations(&self) -> bool {
            true
        }

        fn supports_streaming(&self) -> bool {
            false
        }

        fn name(&self) -> &str {
            &self.name
        }

        fn model(&self) -> &str {
            &self.model
        }
    }

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.retries, 0);
        assert_eq!(config.temperature, None);
        assert_eq!(config.max_tokens, Some(1024));
    }

    #[tokio::test]
    async fn test_execute_parallel() {
        let clients: Vec<Box<dyn AiClient>> = vec![
            Box::new(MockClient::new(
                "client1",
                vec![Ok("response1".to_string())],
            )),
            Box::new(MockClient::new(
                "client2",
                vec![Ok("response2".to_string())],
            )),
        ];

        let results = execute_parallel(clients, "test prompt").await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "client1");
        assert!(results[0].1.is_ok());
        assert_eq!(results[1].0, "client2");
        assert!(results[1].1.is_ok());
    }

    #[tokio::test]
    async fn test_generate_summary() {
        let client = MockClient::new("summarizer", vec![Ok("summary response".to_string())]);
        let responses = vec![
            ("AI1".to_string(), "response1".to_string()),
            ("AI2".to_string(), "response2".to_string()),
        ];

        let summary = generate_summary(&client, &responses).await;
        assert!(summary.is_ok());
        assert_eq!(summary.unwrap(), "summary response");
    }

    #[tokio::test]
    async fn test_execute_parallel_conversation() {
        let clients: Vec<Box<dyn AiClient>> = vec![
            Box::new(MockClient::new(
                "client1",
                vec![Ok("conversation response1".to_string())],
            )),
            Box::new(MockClient::new(
                "client2",
                vec![Ok("conversation response2".to_string())],
            )),
        ];

        let mut conversation = Conversation::new();
        conversation.add_user("Hello");
        conversation.add_assistant("Hi there!");
        conversation.add_user("How are you?");

        let results = execute_parallel_conversation(clients, &conversation).await;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "client1");
        assert!(results[0].1.is_ok());
        assert_eq!(results[1].0, "client2");
        assert!(results[1].1.is_ok());
    }

    #[tokio::test]
    async fn test_mock_client_conversation_support() {
        let client = MockClient::new("test", vec![Ok("conversation test".to_string())]);
        assert!(client.supports_conversations());
        assert!(!client.supports_streaming());

        let mut conversation = Conversation::new();
        conversation.add_user("Test message");

        let result = client.send_conversation(&conversation).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "conversation test");
    }
}
