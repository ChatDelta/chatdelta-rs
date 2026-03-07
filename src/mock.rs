//! Mock AI client for use in tests.
//!
//! Enable with the `mock` Cargo feature:
//! ```toml
//! [dev-dependencies]
//! chatdelta = { version = "*", features = ["mock"] }
//! ```

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::{AiClient, ClientError, Conversation};

/// A mock AI client for use in tests.
///
/// Pre-load it with canned responses; each call to [`send_prompt`] or
/// [`send_conversation`] pops the next response off the queue. When the queue
/// is exhausted it returns a default string so tests don't panic.
///
/// [`send_prompt`]: AiClient::send_prompt
/// [`send_conversation`]: AiClient::send_conversation
///
/// # Example
///
/// ```rust
/// use chatdelta::mock::MockClient;
/// use chatdelta::{AiClient, ClientError};
///
/// # #[tokio::main]
/// # async fn main() {
/// let client = MockClient::new(
///     "test-model",
///     vec![Ok("hello".to_string()), Err(ClientError::config("oops", None))],
/// );
///
/// // First call returns the canned Ok response
/// let r = client.send_prompt("anything").await;
/// assert_eq!(r.unwrap(), "hello");
///
/// // Second call returns the canned Err
/// let r = client.send_prompt("anything").await;
/// assert!(r.is_err());
/// # }
/// ```
pub struct MockClient {
    pub name: String,
    pub model: String,
    pub responses: Arc<Mutex<VecDeque<Result<String, ClientError>>>>,
}

impl MockClient {
    /// Create a new `MockClient` with a queue of canned responses.
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
