//! Integration tests for streaming functionality

use chatdelta::{AiClient, StreamChunk};
use futures::stream::StreamExt;

/// Mock client for testing streaming behavior
struct MockStreamingClient {
    chunks: Vec<String>,
}

impl MockStreamingClient {
    fn new(chunks: Vec<String>) -> Self {
        Self { chunks }
    }
}

#[async_trait::async_trait]
impl AiClient for MockStreamingClient {
    async fn send_prompt(&self, _prompt: &str) -> Result<String, chatdelta::ClientError> {
        Ok(self.chunks.join(""))
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_conversations(&self) -> bool {
        false
    }

    fn name(&self) -> &str {
        "MockStreaming"
    }

    fn model(&self) -> &str {
        "mock-stream-1"
    }

    async fn stream_prompt(
        &self,
        _prompt: &str,
    ) -> Result<
        futures::stream::BoxStream<'_, Result<StreamChunk, chatdelta::ClientError>>,
        chatdelta::ClientError,
    > {
        let chunks = self.chunks.clone();
        let total_chunks = chunks.len();
        
        let stream = futures::stream::iter(chunks.into_iter().enumerate()).map(
            move |(idx, content)| {
                Ok(StreamChunk {
                    content,
                    finished: idx == total_chunks - 1,
                    metadata: if idx == total_chunks - 1 {
                        Some(chatdelta::ResponseMetadata {
                            model_used: Some("mock-stream-1".to_string()),
                            prompt_tokens: Some(10),
                            completion_tokens: Some(20),
                            total_tokens: Some(30),
                            finish_reason: Some("stop".to_string()),
                            safety_ratings: None,
                            request_id: Some("test-123".to_string()),
                            latency_ms: Some(100),
                        })
                    } else {
                        None
                    },
                })
            },
        );

        Ok(Box::pin(stream))
    }
}

#[tokio::test]
async fn test_streaming_basic() {
    let client = MockStreamingClient::new(vec![
        "Hello".to_string(),
        " ".to_string(),
        "world".to_string(),
        "!".to_string(),
    ]);

    assert!(client.supports_streaming());
    
    let mut stream = client
        .stream_prompt("test")
        .await
        .expect("Failed to create stream");

    let mut collected = Vec::new();
    let mut finished_count = 0;
    let mut metadata_received = false;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.expect("Failed to get chunk");
        collected.push(chunk.content.clone());
        
        if chunk.finished {
            finished_count += 1;
            if chunk.metadata.is_some() {
                metadata_received = true;
            }
        }
    }

    assert_eq!(collected.join(""), "Hello world!");
    assert_eq!(finished_count, 1, "Should have exactly one finished chunk");
    assert!(metadata_received, "Should receive metadata in final chunk");
}

#[tokio::test]
async fn test_streaming_with_metadata() {
    let client = MockStreamingClient::new(vec!["Response".to_string()]);

    let mut stream = client
        .stream_prompt("test")
        .await
        .expect("Failed to create stream");

    let chunk = stream.next().await.expect("No chunk").expect("Failed to get chunk");
    
    assert!(chunk.finished);
    assert!(chunk.metadata.is_some());
    
    let metadata = chunk.metadata.unwrap();
    assert_eq!(metadata.model_used, Some("mock-stream-1".to_string()));
    assert_eq!(metadata.prompt_tokens, Some(10));
    assert_eq!(metadata.completion_tokens, Some(20));
    assert_eq!(metadata.total_tokens, Some(30));
    assert_eq!(metadata.finish_reason, Some("stop".to_string()));
    assert_eq!(metadata.latency_ms, Some(100));
}

#[tokio::test]
async fn test_streaming_empty() {
    let client = MockStreamingClient::new(vec![]);

    let mut stream = client
        .stream_prompt("test")
        .await
        .expect("Failed to create stream");

    let chunk = stream.next().await;
    assert!(chunk.is_none(), "Empty stream should return None");
}

#[tokio::test]
async fn test_chat_session_streaming() {
    let client = MockStreamingClient::new(vec![
        "Sure".to_string(),
        ", ".to_string(),
        "I can ".to_string(),
        "help!".to_string(),
    ]);

    let mut session = chatdelta::ChatSession::new(Box::new(client));
    
    let collected = {
        let mut stream = session
            .stream("Can you help?")
            .await
            .expect("Failed to stream");

        let mut collected = Vec::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.expect("Failed to get chunk");
            collected.push(chunk.content);
        }
        collected
    };

    assert_eq!(collected.join(""), "Sure, I can help!");
    
    // Verify the user message was added to history
    // Note: Streaming doesn't automatically add the assistant response to history
    let history = session.history();
    assert_eq!(history.messages.len(), 1);
    assert_eq!(history.messages[0].content, "Can you help?");
    assert_eq!(history.messages[0].role, "user");
}

#[test]
fn test_stream_chunk_construction() {
    let chunk = StreamChunk {
        content: "test".to_string(),
        finished: false,
        metadata: None,
    };
    
    assert_eq!(chunk.content, "test");
    assert!(!chunk.finished);
    assert!(chunk.metadata.is_none());
}