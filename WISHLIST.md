# ChatDelta Crate Wishlist & Implementation Status

*Last Updated: After integrating chatdelta v0.3.0*

## 🎉 Successfully Implemented in v0.3.0

Thank you for implementing these features! They're working great in the CLI:

### ✅ RetryStrategy (Wishlist #6)
- **Status**: Fully implemented and integrated
- **What we got**: `RetryStrategy::Exponential(Duration)`, `Linear(Duration)`, `Fixed(Duration)`
- **CLI Integration**: Added `--retry-strategy` flag allowing users to choose strategy
- **Feedback**: Works perfectly! The Duration parameter for base delay is intuitive

### ✅ ChatSession (Wishlist #3) 
- **Status**: Partially implemented
- **What we got**: `ChatSession::new(client)` and `session.send(message)` 
- **CLI Integration**: Added `--conversation` mode for interactive chat
- **Current Limitations**: 
  - Can't extract client from session for resetting
  - No built-in serialization for saving/loading history
  - No apparent methods for accessing conversation history
- **Suggestions for v0.4.0**:
  ```rust
  impl ChatSession {
      pub fn clear(&mut self) // Reset history keeping same client
      pub fn get_history(&self) -> &[Message] // Access messages
      pub fn load_history(&mut self, messages: Vec<Message>)
      pub fn set_system_prompt(&mut self, prompt: &str)
  }
  ```

## ✅ Recently Implemented (Post v0.3.0)

### Response Metadata ⭐⭐⭐⭐⭐
- **Status**: ✅ FULLY IMPLEMENTED
- **What we got**: `AiResponse` with comprehensive `ResponseMetadata`
- **Implementation**:
  ```rust
  pub struct ResponseMetadata {
      pub model_used: Option<String>,
      pub prompt_tokens: Option<u32>,
      pub completion_tokens: Option<u32>,
      pub total_tokens: Option<u32>,
      pub finish_reason: Option<String>,
      pub safety_ratings: Option<Vec<SafetyRating>>,
      pub request_id: Option<String>,
      pub latency_ms: Option<u64>,
  }
  ```
- **Methods**: `send_prompt_with_metadata()`, `send_conversation_with_metadata()`
- **Integration**: Metadata also available in streaming final chunks

### System Prompt Support ⭐⭐⭐⭐
- **Status**: ✅ FULLY IMPLEMENTED
- **What we got**: System message support in `ClientConfig`
- **Implementation**: `ClientConfig::builder().system_message("You are...")`
- **Integration**: Works with all clients (OpenAI, Claude, Gemini)
- **ChatSession**: Can create sessions with system messages via `ChatSession::with_system_message()`

### Better Error Types ⭐⭐⭐
- **Status**: ✅ FULLY IMPLEMENTED
- **What we got**: Comprehensive `ClientError` enum with specific error types
- **Implementation**:
  ```rust
  pub enum ClientError {
      Network(NetworkError),
      Api(ApiError),
      Authentication(AuthError),
      Configuration(ConfigError),
      Parse(ParseError),
      Stream(StreamError),
  }
  ```
- **Features**: Detailed error types with context, automatic conversion from common errors
- **API Errors**: RateLimit, QuotaExceeded, InvalidModel, ContentFilter, etc.

### Custom API Endpoints ⭐⭐⭐
- **Status**: ✅ FULLY IMPLEMENTED
- **What we got**: `base_url` field in `ClientConfig`
- **Implementation**: `ClientConfig::builder().base_url("https://my-api.com")`
- **Use Cases**: Azure OpenAI, local models, proxies
- **Integration**: Works with OpenAI client for compatible endpoints

### Streaming Response Support ⭐⭐⭐⭐⭐
- **Status**: ✅ FULLY IMPLEMENTED
- **What we got**: Real SSE streaming for OpenAI and Claude clients
- **Implementation**:
  ```rust
  async fn stream_prompt(&self, prompt: &str) -> Result<BoxStream<'_, Result<StreamChunk, ClientError>>, ClientError>
  async fn stream_conversation(&self, conversation: &Conversation) -> Result<BoxStream<'_, Result<StreamChunk, ClientError>>, ClientError>
  ```
- **Features**:
  - Proper Server-Sent Events (SSE) parsing
  - Streaming with metadata in final chunk
  - Support for both OpenAI and Claude streaming APIs
  - Integration with ChatSession for streaming conversations
- **Note**: Gemini streaming is technically supported by the API but not yet implemented

## 🚀 High Priority for Next Release (v0.4.0)

Based on real-world CLI usage, these would have the most impact:

### 1. Complete Gemini Streaming Implementation ⭐⭐⭐⭐
- **Current State**: Gemini client exists but doesn't implement streaming
- **Ideal**: Implement `streamGenerateContent` endpoint support
- **CLI Use Case**: Consistent streaming experience across all providers


## 📊 Medium Priority Features


### 5. Model Capability Discovery ⭐⭐⭐
- **Use Case**: Dynamically adjust parameters based on model
- **Ideal API**:
  ```rust
  client.get_capabilities() -> ModelCapabilities {
      max_tokens: usize,
      supports_streaming: bool,
      supports_functions: bool,
      // etc.
  }
  ```


## 💭 Lower Priority (Nice to Have)

### 7. Progress Callbacks
- For long operations, ability to show progress

### 8. Request Cancellation  
- Ability to abort in-flight requests

### 9. Mock Client for Testing
- Would help with CLI testing

## 🐛 Issues/Observations in v0.3.0

1. **ChatSession Constructor**: Takes client by value, making it impossible to reuse the client or reset the session without recreating everything

2. **Missing re-exports**: Would be helpful to re-export common types at crate root

3. **Documentation**: More examples would help, especially for ChatSession usage patterns

## 💡 Implementation Suggestions

### For Streaming (High Priority)
Consider using `tokio_stream` and async generators:
```rust
use tokio_stream::Stream;

pub trait AiClient {
    fn send_prompt_streaming(&self, prompt: &str) 
        -> impl Stream<Item = Result<String>>;
}
```

### For Response Metadata
A non-breaking approach could be:
```rust
// Keep existing method for compatibility
async fn send_prompt(&self, prompt: &str) -> Result<String>;

// Add new method that returns Response
async fn send_prompt_detailed(&self, prompt: &str) -> Result<Response>;
```

## 🎯 Summary for v0.4.0

**Already Implemented** (Available now!):
1. ✅ Streaming responses (OpenAI & Claude)
2. ✅ Response metadata with token usage
3. ✅ System prompt support
4. ✅ Better error types
5. ✅ Custom API endpoints
6. ✅ ChatSession with history management
7. ✅ Retry strategies

**Top Remaining Priorities**:
1. Complete Gemini streaming implementation
2. Model capability discovery
3. Enhanced ChatSession methods (clear, load_history, etc.)

**Quick Wins**:
- Add `ChatSession::clear()` method
- Export `Message` type for conversation history
- Better error types enum

## 🙏 Thank You!

The v0.3.0 release with RetryStrategy and ChatSession has been fantastic! The retry strategies in particular have made the CLI much more robust. Looking forward to continued collaboration!

---
*Note: This wishlist is actively maintained by the chatdelta-cli project as a communication channel with the upstream crate.*