# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

ChatDelta is a unified Rust library for connecting to multiple AI APIs (OpenAI, Google Gemini, Anthropic Claude) with a common interface. The library supports streaming responses, multi-turn conversations, parallel execution, metrics collection, and advanced orchestration.

## Core Architecture

### Key Types (src/lib.rs)
- **`AiClient` trait**: The core abstraction all provider clients implement. Only `send_prompt`, `name()`, and `model()` are required — all other methods (`send_conversation`, `stream_prompt`, `stream_conversation`, `send_prompt_streaming`, etc.) have default fallback implementations.
- **`ClientConfig` / `ClientConfigBuilder`**: Configuration via builder pattern. Supports timeout, retries, temperature, max_tokens, top_p, penalties, system_message, base_url, and retry_strategy.
- **`Conversation` / `Message`**: Multi-turn message history. Messages have roles: "system", "user", "assistant".
- **`ChatSession`**: High-level wrapper that owns a client and `Conversation`, automatically appending history. Preferred for interactive use.
- **`AiResponse` / `ResponseMetadata`**: Rich response type with token counts, finish reason, latency, request ID, safety ratings.
- **`StreamChunk`**: Unit of streaming output — has `content`, `finished` flag, and optional `metadata` on the final chunk.

### Provider Clients (src/clients/)
- `ChatGpt` ("openai" | "gpt" | "chatgpt"), `Gemini` ("google" | "gemini"), `Claude` ("anthropic" | "claude")
- Instantiated via `create_client(provider, api_key, model, config)` factory function
- All three re-exported from `src/clients/mod.rs`

### Top-level Functions (src/lib.rs)
- `execute_parallel(clients, prompt)` — runs all clients concurrently, returns `Vec<(name, Result)>`
- `execute_parallel_conversation(clients, conversation)` — same but with conversation history
- `generate_summary(client, responses)` — uses one client to summarize responses from multiple models

### Supporting Modules
- **`src/middleware.rs`**: `MiddlewareClient` with `execute_with_retry` and `RequestMiddleware` trait for provider-specific request/response hooks
- **`src/utils.rs`**: `RetryStrategy` (exponential backoff config) and standalone `execute_with_retry`
- **`src/metrics.rs`**: `ClientMetrics` (atomic counters), `RequestTimer`, `MetricsSnapshot`
- **`src/observability.rs`**: Prometheus/OpenTelemetry export and tracing setup (requires `metrics-export` feature)
- **`src/http.rs`**: `SHARED_CLIENT` (global reqwest client), `get_provider_client`, `HttpConfig`
- **`src/sse.rs`**: SSE parsing for streaming responses (private module)
- **`src/error.rs`**: `ClientError` enum with structured variants: `Network`, `Api`, `Authentication`, `Configuration`, `Parse`, `Stream` — each with typed sub-enums and convenience constructors (`ClientError::timeout`, `::rate_limit`, `::config`, etc.)

### Feature Flags (Cargo.toml)
- `orchestration` — enables `src/orchestration.rs`: `AiOrchestrator`, `FusedResponse`, `OrchestrationStrategy`, `ModelCapabilities`
- `prompt-optimization` — enables `src/prompt_optimizer.rs`: `PromptOptimizer`, `OptimizedPrompt`
- `experimental` — enables both `orchestration` and `prompt-optimization`
- `metrics-export` — enables Prometheus and OpenTelemetry export via optional dependencies

## Common Development Commands

```bash
# Build
cargo build
cargo build --release

# Test
cargo test --all
cargo test test_execute_parallel          # run a specific test
cargo test -- --nocapture                 # show println! output

# Lint and format (both required by CI)
cargo clippy -- -D warnings
cargo fmt
cargo fmt -- --check                      # CI check only

# Docs
cargo doc --open
```

## CI/CD Pipeline

GitHub Actions (`.github/workflows/ci.yml`) runs on all PRs and main branch pushes:
1. `cargo fmt -- --check`
2. `cargo clippy -- -D warnings`
3. `cargo test --all`

## Testing Strategy

Unit tests live within each module file. `MockClient` (defined in `src/lib.rs` tests) implements `AiClient` with a `VecDeque` of pre-set responses — use it for testing parallel execution, conversations, and orchestration without real API calls.
