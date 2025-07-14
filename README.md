# ChatDelta

[![Crates.io](https://img.shields.io/crates/v/chatdelta.svg)](https://crates.io/crates/chatdelta)
[![Documentation](https://docs.rs/chatdelta/badge.svg)](https://docs.rs/chatdelta)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A unified Rust library for connecting to multiple AI APIs (OpenAI, Google Gemini, Anthropic Claude) with a common interface. Supports parallel execution, retry logic, and configurable parameters.

## Features

- **Unified Interface**: Single trait (`AiClient`) for all AI providers
- **Multiple Providers**: OpenAI ChatGPT, Google Gemini, Anthropic Claude
- **Parallel Execution**: Run multiple AI models concurrently
- **Retry Logic**: Configurable retry attempts with exponential backoff
- **Async/Await**: Built with tokio for efficient async operations
- **Type Safety**: Full Rust type safety with comprehensive error handling

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
chatdelta = "0.1"
tokio = { version = "1", features = ["full"] }
```

## Usage

### Basic Example

```rust
use chatdelta::{AiClient, ClientConfig, create_client};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig {
        timeout: Duration::from_secs(30),
        retries: 3,
        temperature: Some(0.7),
        max_tokens: Some(1024),
    };
    
    let client = create_client("openai", "your-api-key", "gpt-4", config)?;
    let response = client.send_prompt("Hello, world!").await?;
    println!("{}", response);
    
    Ok(())
}
```

### Parallel Execution

```rust
use chatdelta::{create_client, execute_parallel, ClientConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::default();
    
    let clients = vec![
        create_client("openai", "openai-key", "gpt-4", config.clone())?,
        create_client("anthropic", "claude-key", "claude-3-sonnet-20240229", config.clone())?,
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

## Configuration

```rust
use chatdelta::ClientConfig;
use std::time::Duration;

let config = ClientConfig {
    timeout: Duration::from_secs(60),    // Request timeout
    retries: 3,                          // Number of retry attempts
    temperature: Some(0.8),              // Response creativity (0.0-2.0)
    max_tokens: Some(2048),             // Maximum response length (Claude only)
};
```

## Error Handling

The library provides comprehensive error handling through the `ClientError` enum:

- `ClientError::Network`: Connection and timeout errors
- `ClientError::Api`: API-specific errors and rate limits
- `ClientError::Authentication`: Invalid API keys
- `ClientError::Configuration`: Invalid parameters
- `ClientError::Parse`: Response parsing errors

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
