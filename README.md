# ChatDelta

[![Crates.io](https://img.shields.io/crates/v/chatdelta.svg)](https://crates.io/crates/chatdelta)
[![Documentation](https://docs.rs/chatdelta/badge.svg)](https://docs.rs/chatdelta)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A unified Rust library for connecting to multiple AI APIs (OpenAI, Google Gemini, Anthropic Claude) with a common interface. Supports parallel execution, conversations, streaming, retry logic, and extensive configuration options.

## Features

- **Unified Interface**: Single trait (`AiClient`) for all AI providers
- **Multiple Providers**: OpenAI ChatGPT, Google Gemini, Anthropic Claude
- **Conversation Support**: Multi-turn conversations with message history
- **Streaming Responses**: Real-time streaming support (where available)
- **Parallel Execution**: Run multiple AI models concurrently
- **Builder Pattern**: Fluent configuration with `ClientConfig::builder()`
- **Advanced Error Handling**: Detailed error types with specific categories
- **Retry Logic**: Configurable retry attempts with exponential backoff
- **Async/Await**: Built with tokio for efficient async operations
- **Type Safety**: Full Rust type safety with comprehensive error handling
- **Observability**: Built-in metrics collection and tracing support
- **Middleware System**: Reusable components for retry, validation, and streaming
- **Feature Flags**: Optional features like orchestration and prompt optimization
- **Metrics Export**: Prometheus and OpenTelemetry support (optional)

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
chatdelta = "0.8"
tokio = { version = "1", features = ["full"] }
```

## Usage

### Basic Example

```rust
use chatdelta::{AiClient, ClientConfig, create_client};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::builder()
        .timeout(Duration::from_secs(30))
        .retries(3)
        .temperature(0.7)
        .max_tokens(1024)
        .build();
    
    let client = create_client("openai", "your-api-key", "gpt-4o", config)?;
    let response = client.send_prompt("Hello, world!").await?;
    println!("{}", response);
    
    Ok(())
}
```

### Conversation Example

```rust
use chatdelta::{AiClient, ClientConfig, Conversation, create_client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::builder()
        .system_message("You are a helpful assistant")
        .temperature(0.7)
        .build();
    
    let client = create_client("anthropic", "your-api-key", "claude-3-5-sonnet-20241022", config)?;
    
    let mut conversation = Conversation::new();
    conversation.add_user("What's the capital of France?");
    conversation.add_assistant("The capital of France is Paris.");
    conversation.add_user("What's its population?");
    
    let response = client.send_conversation(&conversation).await?;
    println!("{}", response);
    
    Ok(())
}
```

### Parallel Execution

```rust
use chatdelta::{create_client, execute_parallel, ClientConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::builder()
        .retries(2)
        .temperature(0.7)
        .build();
    
    let clients = vec![
        create_client("openai", "openai-key", "gpt-4o", config.clone())?,
        create_client("anthropic", "claude-key", "claude-3-5-sonnet-20241022", config.clone())?,
        create_client("google", "gemini-key", "gemini-1.5-pro", config)?,
    ];
    
    let results = execute_parallel(clients, "Explain quantum computing").await;
    
    for (name, result) in results {
        match result {
            Ok(response) => println!("{}: {}", name, response),
            Err(e) => eprintln!("{} failed: {}", name, e),
        }
    }
    
    Ok(())
}
```

### Streaming Responses

Two methods are available for streaming responses:

#### Using BoxStream (Advanced)

```rust
use chatdelta::{AiClient, ClientConfig, create_client};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::builder()
        .temperature(0.8)
        .build();
    
    let client = create_client("openai", "your-api-key", "gpt-4o", config)?;
    
    if client.supports_streaming() {
        let mut stream = client.stream_prompt("Tell me a story").await?;
        
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(chunk) => {
                    print!("{}", chunk.content);
                    if chunk.finished {
                        println!("\n[Stream finished]");
                        break;
                    }
                }
                Err(e) => eprintln!("Stream error: {}", e),
            }
        }
    } else {
        println!("Client doesn't support streaming");
    }
    
    Ok(())
}
```

#### Using Channel-based Streaming (Simple)

```rust
use chatdelta::{AiClient, ClientConfig, create_client, StreamChunk};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::builder()
        .temperature(0.8)
        .build();
    
    let client = create_client("openai", "your-api-key", "gpt-4o", config)?;
    
    // Create a channel for receiving stream chunks
    let (tx, mut rx) = mpsc::unbounded_channel::<StreamChunk>();
    
    // Start streaming in a task
    let client_clone = client.clone();
    tokio::spawn(async move {
        if let Err(e) = client_clone.send_prompt_streaming("Tell me a story", tx).await {
            eprintln!("Streaming error: {}", e);
        }
    });
    
    // Receive and print chunks
    while let Some(chunk) = rx.recv().await {
        print!("{}", chunk.content);
        if chunk.finished {
            println!("\n[Stream finished]");
            break;
        }
    }
    
    Ok(())
}
```

## Supported Providers

### OpenAI
- Provider: `"openai"`, `"gpt"`, or `"chatgpt"`
- Models: `"gpt-4"`, `"gpt-3.5-turbo"`, etc.
- API Key: OpenAI API key

### Google Gemini
- Provider: `"google"` or `"gemini"`
- Models: `"gemini-1.5-pro"`, `"gemini-1.5-flash"`, etc.
- API Key: Google AI API key

### Anthropic Claude
- Provider: `"anthropic"` or `"claude"`
- Models: `"claude-3-5-sonnet-20241022"`, `"claude-3-haiku-20240307"`, etc.
- API Key: Anthropic API key

## Feature Flags

Optional features can be enabled in your `Cargo.toml`:

```toml
[dependencies]
chatdelta = { version = "0.8", features = ["experimental"] }
# Or specific features:
# chatdelta = { version = "0.8", features = ["orchestration", "metrics-export"] }
```

Available features:
- `orchestration`: Multi-model orchestration and consensus
- `prompt-optimization`: Advanced prompt engineering
- `experimental`: Enables all experimental features
- `metrics-export`: Prometheus and OpenTelemetry metrics export

## Configuration

The `ClientConfig` supports extensive configuration through a builder pattern:

```rust
use chatdelta::ClientConfig;
use std::time::Duration;

let config = ClientConfig::builder()
    .timeout(Duration::from_secs(60))    // Request timeout
    .retries(3)                          // Number of retry attempts
    .temperature(0.8)                    // Response creativity (0.0-2.0)
    .max_tokens(2048)                    // Maximum response length
    .top_p(0.9)                         // Top-p sampling (0.0-1.0)
    .frequency_penalty(0.1)              // Frequency penalty (-2.0 to 2.0)
    .presence_penalty(0.1)               // Presence penalty (-2.0 to 2.0)
    .system_message("You are a helpful assistant") // System message for conversation context
    .build();
```

### Configuration Options

| Parameter | Description | Default | Supported By |
|-----------|-------------|---------|--------------|
| `timeout` | HTTP request timeout | 30 seconds | All |
| `retries` | Number of retry attempts | 0 | All |
| `temperature` | Response creativity (0.0-2.0) | None | All |
| `max_tokens` | Maximum response length | 1024 | All |
| `top_p` | Top-p sampling (0.0-1.0) | None | OpenAI |
| `frequency_penalty` | Frequency penalty (-2.0 to 2.0) | None | OpenAI |
| `presence_penalty` | Presence penalty (-2.0 to 2.0) | None | OpenAI |
| `system_message` | System message for conversations | None | All |

## Error Handling

The library provides comprehensive error handling through the `ClientError` enum with detailed error types:

```rust
use chatdelta::{ClientError, ApiErrorType, NetworkErrorType};

match result {
    Err(ClientError::Network(net_err)) => {
        match net_err.error_type {
            NetworkErrorType::Timeout => println!("Request timed out"),
            NetworkErrorType::ConnectionFailed => println!("Connection failed"),
            _ => println!("Network error: {}", net_err.message),
        }
    }
    Err(ClientError::Api(api_err)) => {
        match api_err.error_type {
            ApiErrorType::RateLimit => println!("Rate limit exceeded"),
            ApiErrorType::QuotaExceeded => println!("API quota exceeded"),
            ApiErrorType::InvalidModel => println!("Invalid model specified"),
            _ => println!("API error: {}", api_err.message),
        }
    }
    Err(ClientError::Authentication(auth_err)) => {
        println!("Authentication failed: {}", auth_err.message);
    }
    Err(ClientError::Configuration(config_err)) => {
        println!("Configuration error: {}", config_err.message);
    }
    Err(ClientError::Parse(parse_err)) => {
        println!("Parse error: {}", parse_err.message);
    }
    Err(ClientError::Stream(stream_err)) => {
        println!("Stream error: {}", stream_err.message);
    }
    Ok(response) => println!("Success: {}", response),
}
```

### Error Categories

- **Network**: Connection issues, timeouts, DNS resolution failures
- **API**: Rate limits, quota exceeded, invalid models, server errors
- **Authentication**: Invalid API keys, expired tokens, insufficient permissions
- **Configuration**: Invalid parameters, missing required fields
- **Parse**: JSON parsing errors, missing response fields
- **Stream**: Streaming-specific errors, connection lost, invalid chunks

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

We welcome contributions! To get started, clone the repository and install the Rust toolchain. Before opening a pull request, run the following commands:

```bash
# Check formatting
cargo fmt -- --check

# Run the linter
cargo clippy -- -D warnings

# Execute tests
cargo test
```

This project uses GitHub Actions to run the same checks automatically on every pull request.
