# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ChatDelta is a unified Rust library for connecting to multiple AI APIs (OpenAI, Google Gemini, Anthropic Claude) with a common interface. The library supports streaming responses, multi-turn conversations, parallel execution, metrics collection, and advanced orchestration.

## Core Architecture

### Main Components
- **`src/lib.rs`**: Main library entry point with `AiClient` trait, `ClientConfig` with builder pattern, factory functions, and module exports
- **`src/clients/`**: Individual AI provider implementations
  - `openai.rs`: ChatGPT client with streaming support
  - `gemini.rs`: Google Gemini client implementation
  - `claude.rs`: Anthropic Claude client with conversation support
  - `mod.rs`: Module exports and shared types
- **`src/error.rs`**: Comprehensive error handling with `ClientError` enum covering Network, API, Authentication, Configuration, Parse, and Stream errors
- **`src/middleware.rs`**: Reusable middleware components for retry logic, request/response processing, and streaming utilities
- **`src/observability.rs`**: Metrics export (Prometheus, OpenTelemetry) and structured logging with tracing
- **`src/orchestration.rs`**: [Feature-gated] Advanced multi-model orchestration with response fusion, confidence scoring, and consensus algorithms
- **`src/prompt_optimizer.rs`**: [Feature-gated] Prompt optimization engine with templates and performance tracking
- **`src/metrics.rs`**: Performance metrics collection with atomic counters for requests, latency, tokens, and cache hits
- **`src/http.rs`**: Shared HTTP client configuration and provider-specific client management
- **`src/utils.rs`**: Utility functions including `execute_with_retry` and `RetryStrategy`
- **`src/sse.rs`**: Server-sent events handling for streaming responses

### Key Features
- **Streaming Support**: Both BoxStream and channel-based streaming interfaces
- **Conversation Management**: Multi-turn conversation support with message history via `Conversation` struct
- **Trait-based design**: All clients implement the `AiClient` trait with methods for prompts, conversations, and streaming
- **Builder Pattern**: `ClientConfig::builder()` for fluent configuration
- **Factory pattern**: `create_client()` function creates appropriate client based on provider string
- **Async/await**: All API calls are async using tokio runtime with full feature set
- **Retry logic**: Configurable retry attempts with customizable `RetryStrategy`
- **Parallel execution**: `execute_parallel()` function runs multiple clients concurrently
- **Caching**: Response caching using moka cache library
- **Metrics**: Detailed performance tracking with `ClientMetrics` and `RequestTimer`
- **Middleware System**: `MiddlewareClient` provides common retry/timeout logic across providers
- **Observability**: Structured logging with tracing and optional metrics export (Prometheus/OpenTelemetry)
- **Feature Flags**: Optional experimental features (orchestration, prompt-optimization) can be enabled selectively

## Common Development Commands

### Build and Test
```bash
# Build the project
cargo build

# Run all tests
cargo test --all

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_execute_parallel

# Build in release mode
cargo build --release
```

### Code Quality
```bash
# Run Clippy linter (CI requirement)
cargo clippy -- -D warnings

# Format code
cargo fmt

# Check formatting (CI requirement)
cargo fmt -- --check
```

### Documentation
```bash
# Generate and open documentation
cargo doc --open

# Generate docs without opening
cargo doc
```

## CI/CD Pipeline

GitHub Actions workflow (.github/workflows/ci.yml) runs on all PRs and main branch pushes:
1. Format check: `cargo fmt -- --check`
2. Clippy linter: `cargo clippy -- -D warnings`
3. Test suite: `cargo test --all`

## Testing Strategy

The codebase includes comprehensive unit tests using `MockClient` implementations for testing parallel execution, streaming, conversations, and orchestration without making actual API calls. Tests are located within each module file.