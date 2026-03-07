# What's New in chatdelta-rs

Reverse communication channel from the library to CLI consumers.
Each entry describes what you can now wire up, using the actual API names.

---

## v0.8.2

**Token counts from parallel runs.** You can now call
`execute_parallel_with_metadata(clients, prompt)` instead of `execute_parallel`
to get full `AiResponse` values — including `metadata.prompt_tokens`,
`completion_tokens`, `total_tokens`, `finish_reason`, and `latency_ms` — from
every model in one shot. The existing `execute_parallel` is unchanged.

```rust
use chatdelta::{execute_parallel_with_metadata, AiResponse};

let results = execute_parallel_with_metadata(clients, &prompt).await;
for (name, result) in results {
    match result {
        Ok(r) => {
            println!("{}", r.content);
            if let Some(t) = r.metadata.total_tokens {
                eprintln!("[{}] {} tokens", name, t);
            }
        }
        Err(e) => eprintln!("{} failed: {}", name, e),
    }
}
```

Wire this up behind a `--show-usage` flag: pass it when the flag is set,
fall back to `execute_parallel` otherwise.

---

## v0.8.1

**Save and restore sessions.** `Conversation` now derives `Serialize` and
`Deserialize`. Combined with `ChatSession::load_history`, you can persist and
resume a session with three lines:

```rust
// save on exit
fs::write(path, serde_json::to_string(session.history())?)?;

// restore on launch
let messages: Vec<Message> = serde_json::from_str(&fs::read_to_string(path)?)?;
session.load_history(messages);
```

Wire this up behind `--save-conversation <path>` and `--load-conversation
<path>`. The `Conversation` JSON is a plain array of `{role, content}` objects
— human-readable and stable.

**`ChatSession::load_history(messages: Vec<Message>)`** replaces the previous
workaround of reaching into `history_mut().messages`. Call it after creating a
new session to seed it with saved context before the first real turn.

**Test without API keys.** `MockClient` is now publicly exported behind the
`mock` Cargo feature. Add it to `[dev-dependencies]`:

```toml
[dev-dependencies]
chatdelta = { version = "0.8.1", features = ["mock"] }
```

```rust
use chatdelta::mock::MockClient;

let client = MockClient::new("gpt", vec![
    Ok("opening argument".to_string()),
    Ok("rebuttal".to_string()),
    Err(ClientError::config("quota exceeded", None)),
]);
```

Responses are returned in queue order; once exhausted it falls back to a
default string rather than panicking, so tests don't have to perfectly predict
every call.

---

## v0.7.0

**Channel-based streaming.** You can now receive streaming output via an
unbounded channel instead of polling a `BoxStream`. Call
`client.send_prompt_streaming(prompt, tx)` where `tx` is a
`tokio::sync::mpsc::UnboundedSender<StreamChunk>`. OpenAI and Claude send
native chunks; all other providers fall back automatically. Both streaming
paths (`stream_prompt` returning `BoxStream<StreamChunk>` and
`send_prompt_streaming` with a channel) are available simultaneously — pick
whichever fits your CLI's event loop.

---

## v0.6.0

Requires feature flags `orchestration` and/or `prompt-optimization` (or the
combined `experimental` flag in `Cargo.toml`).

**Multi-model orchestration.** You can now coordinate several clients at once
using `AiOrchestrator`. Choose a strategy via `OrchestrationStrategy`:
`Fusion`, `Consensus`, `Tournament`, `Adaptive`, `BestOf`, `Parallel`, or
`Sequential`. Responses come back as `FusedResponse`, which includes per-model
contributions and an overall confidence score. Use `ModelCapabilities` to
inspect and compare provider strengths before routing.

**Prompt optimization.** You can now run a prompt through `PromptOptimizer`
before sending it to any client. Call `optimizer.optimize(prompt)` to get an
`OptimizedPrompt` with chain-of-thought steps and task-type annotations
injected. The optimizer tracks effectiveness over time so repeated calls
improve.

**Response caching.** `AiOrchestrator` caches results with a configurable TTL
(moka backend). Identical prompts served from cache skip the API call
entirely — wire up a `--no-cache` flag to bypass when needed.

---

## v0.5.0

**Per-client metrics.** You can now read counters from any client that wraps
`ClientMetrics`: total requests, successes, failures, cache hits/misses, and
cumulative latency. Call `metrics.snapshot()` to get a `MetricsSnapshot` value
you can serialize or display. Use `RequestTimer` to bracket your own timed
sections. A `--stats` flag at the CLI level can dump these after each
invocation.

**Shared HTTP client.** Connection pooling and keepalive are now on by default
via `SHARED_CLIENT`. No CLI wiring required — you get the latency reduction
automatically. Provider-specific timeout tuning is applied per client.

---

## v0.4.0

**Streaming responses.** You can now stream output from OpenAI and Claude
clients. Call `client.stream_prompt(prompt)` to get a
`BoxStream<Result<StreamChunk>>`. Each `StreamChunk` has a `content: String`
field and a `finished: bool` flag; the final chunk carries an optional
`metadata: Option<ResponseMetadata>` with token counts and finish reason.
Print chunks as they arrive for a live-output CLI experience.

**Multi-turn conversations.** You can now maintain chat history across turns
using `Conversation`. Build it with `Conversation::new()`, append messages via
`.add_user_message(text)` and `.add_assistant_message(text)`, then pass it to
`client.send_conversation(&conversation)` or
`client.stream_conversation(&conversation)`. For a higher-level wrapper that
manages history automatically, use `ChatSession::new(client)` and call
`session.chat(user_input).await` — it appends both the user turn and the
assistant reply for you.

**Configurable retry strategies.** You can now tune retry behaviour per client
via `ClientConfigBuilder`. Set `.retry_strategy(RetryStrategy { max_retries,
initial_delay, max_delay, multiplier })` to control exponential backoff. The
standalone `execute_with_retry(operation, strategy)` utility is also available
if you need retries around non-client operations in the CLI.
