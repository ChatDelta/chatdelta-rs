# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ChatDelta is a unified Rust library for connecting to multiple AI APIs (OpenAI, Google Gemini, Anthropic Claude) with a common interface. The library supports parallel execution, retry logic, and configurable parameters.

## Core Architecture

### Main Components
- **`src/lib.rs`**: Main library entry point with `AiClient` trait, `ClientConfig`, factory functions, and parallel execution utilities
- **`src/clients/`**: Individual AI provider implementations
  - `openai.rs`: ChatGPT client implementation
  - `gemini.rs`: Google Gemini client implementation  
  - `claude.rs`: Anthropic Claude client implementation
- **`src/error.rs`**: Unified error handling with `ClientError` enum covering Network, API, Authentication, Configuration, and Parse errors

### Key Patterns
- **Trait-based design**: All clients implement the `AiClient` trait with `send_prompt()`, `name()`, and `model()` methods
- **Factory pattern**: `create_client()` function creates appropriate client based on provider string
- **Async/await**: All API calls are async using tokio runtime
- **Retry logic**: Configurable retry attempts with exponential backoff
- **Parallel execution**: `execute_parallel()` function runs multiple clients concurrently using `futures::join_all`

## Common Development Commands

### Build and Test
```bash
# Build the project
cargo build

# Run all tests (includes unit tests in lib.rs)
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_execute_parallel

# Build in release mode
cargo build --release
```

### Code Quality
```bash
# Run Clippy linter
cargo clippy

# Fix Clippy suggestions automatically
cargo clippy --fix

# Format code
cargo fmt

# Check formatting without applying
cargo fmt --check
```

### Documentation
```bash
# Generate and open documentation
cargo doc --open

# Generate docs without opening
cargo doc
```

## Testing Strategy

The codebase includes comprehensive unit tests using a `MockClient` implementation for testing parallel execution and summary generation without making actual API calls.