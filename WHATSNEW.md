# What's New in chatdelta-rs

Reverse communication channel from the library to CLI consumers.
Each entry describes what you can now wire up, using the actual API names.

---

## v0.8.0

**`Conversation` is now serializable.** `Conversation` derives
`serde::Serialize` and `serde::Deserialize` (it previously didn't, even though
`Message` already did). You can now round-trip a full session to JSON:

```rust
// save
let json = serde_json::to_string(session.history())?;
fs::write("session.json", json)?;

// restore
let conv: Conversation = serde_json::from_str(&fs::read_to_string("session.json")?)?;
session.load_history(conv.messages);
```

Wire up `--save-conversation` and `--load-conversation` flags against this.

**`ChatSession::load_history(messages: Vec<Message>)`** is now available.
Replaces the awkward `session.history_mut().messages = …` pattern. Use it to
restore a saved session or inject synthetic context before the first real turn.

**`MockClient` is now publicly exported** behind the `mock` feature flag. Add
it to your `[dev-dependencies]` to test CLI logic without live API calls:

```toml
[dev-dependencies]
chatdelta = { version = "0.8", features = ["mock"] }
```

```rust
use chatdelta::mock::MockClient;

let client = MockClient::new("gpt", vec![
    Ok("opening argument".to_string()),
    Ok("rebuttal".to_string()),
]);
// drive your debate / conversation / orchestration code under test
```

`MockClient` queues pre-set responses and returns them in order; once
exhausted it falls back to a default string rather than panicking.

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
