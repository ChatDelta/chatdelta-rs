# ChatDelta Crate Wishlist & Implementation Status

*Last Updated: 2026-03-07 — audited against chatdelta-rs source at v0.8.0*

---

## ✅ Fully Delivered (no further action needed)

### RetryStrategy — v0.3.0
- `RetryStrategy::Exponential(Duration)`, `Linear(Duration)`, `Fixed(Duration)`
- Set via `ClientConfig::builder().retry_strategy(strategy)`
- CLI wires up via `--retry-strategy exponential|linear|fixed`

### Response Metadata — delivered post-v0.3.0, confirmed in source
- `AiResponse` with `ResponseMetadata` (model_used, prompt_tokens, completion_tokens,
  total_tokens, finish_reason, latency_ms, safety_ratings, request_id)
- `client.send_prompt_with_metadata(prompt) -> Result<AiResponse>`
- `client.send_conversation_with_metadata(&conv) -> Result<AiResponse>`
- `ChatSession::send_with_metadata(msg) -> Result<AiResponse>` — full metadata in session context
- Final `StreamChunk` carries `Option<ResponseMetadata>` — metadata available on streaming too
- **Note for CLI:** The CLI has not yet wired `--show-usage` to `send_with_metadata`. It's
  ready to use now.

### System Prompt Support — delivered post-v0.3.0
- `ClientConfig::builder().system_message("You are...")`
- `ChatSession::with_system_message(client, msg)`
- `ChatSession::reset_with_system(msg)` — switch persona without rebuilding client
- Works across all three providers

### Structured Error Handling — delivered post-v0.3.0
- `ClientError` enum: `Network`, `Api`, `Authentication`, `Configuration`, `Parse`, `Stream`
- Convenience constructors: `ClientError::timeout(...)`, `::rate_limit(...)`, `::config(...)`

### Custom API Endpoints — delivered post-v0.3.0, confirmed in source
- `ClientConfig::builder().base_url("https://my-proxy/v1")`
- The `base_url` field flows through `create_client` via the `ClientConfig` argument
- Honoured by the OpenAI client (Azure OpenAI, LiteLLM proxies, local Ollama via
  OpenAI-compatible endpoint all work)
- **Note:** The CLI's WISHLIST previously said this was "still missing" — it is not.
  Wire up a `--base-url` flag mapped to `config_builder.base_url(url)`.

### Streaming Responses — v0.4.0 / v0.7.0
- `client.stream_prompt(prompt) -> BoxStream<Result<StreamChunk>>`
- `client.send_prompt_streaming(prompt, tx)` where `tx: UnboundedSender<StreamChunk>`
- `ChatSession::stream(msg)` returns `BoxStream`
- CLI wires up `--stream` via the channel API (`mpsc::unbounded_channel`)

### ChatSession::clear() — confirmed in source
- `ChatSession::clear()` is implemented and public
- **Note:** The CLI conversation loop has a stale `// TODO: Use ChatSession::clear() once
  available` comment at the `"clear"` match arm. The method exists — remove the TODO and
  call `session.clear()`.

### ChatSession::history() — confirmed in source (different name than requested)
- `ChatSession::history() -> &Conversation` — access full history
- `ChatSession::history_mut() -> &mut Conversation` — mutate history
- `Conversation.messages: Vec<Message>` is public, so `session.history().messages` gives
  `&[Message]`
- **Note:** The CLI `"save"` branch has a stale `// TODO: Use ChatSession::get_history()
  once available` comment. The method exists as `history()` — remove the TODO.

### ModelCapabilities — v0.6.0 (requires `orchestration` feature)
- `ModelCapabilities` struct with `supports_streaming`, `max_tokens`, etc.
- Available on clients via the orchestration layer

---

## 🔴 Open — Real Gaps Found in Source Audit

### 1. `execute_parallel` has no metadata variant ⭐⭐⭐⭐
- `execute_parallel(clients, prompt)` calls `send_prompt` internally and returns
  `Vec<(String, Result<String>)>` — no token counts, no latency, no model info
- The CLI can get metadata from single-client calls via `send_prompt_with_metadata`,
  but the parallel path has no equivalent
- **Ideal API:**
  ```rust
  pub async fn execute_parallel_with_metadata(
      clients: Vec<Box<dyn AiClient>>,
      prompt: &str,
  ) -> Vec<(String, Result<AiResponse, ClientError>)>
  ```
- **CLI use case:** `--show-usage` flag that reports per-model token counts after a
  parallel run

### 2. `Conversation` lacks `Serialize`/`Deserialize` ⭐⭐⭐⭐
- `Message` derives `serde::Serialize, serde::Deserialize` — but `Conversation` does not
- `ChatSession::history()` returns `&Conversation`, which cannot be serialized to JSON
  for saving/resuming sessions
- This is the actual blocker for `--save-conversation` / `--load-conversation` in the CLI
- **Fix needed:**
  ```rust
  #[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
  pub struct Conversation { ... }
  ```
- With that in place the CLI can do:
  ```rust
  // save
  let json = serde_json::to_string(session.history())?;
  fs::write(path, json)?;
  // load
  let conv: Conversation = serde_json::from_str(&fs::read_to_string(path)?)?;
  session.history_mut().messages = conv.messages;
  ```

### 3. `MockClient` is not publicly exported ⭐⭐⭐
- `MockClient` exists in `src/lib.rs` but only inside `#[cfg(test)]` — it is invisible
  to downstream crates
- The CLI cannot use it in its own integration tests without live API keys
- **Fix:** Move `MockClient` out of the test module and export it, ideally gated behind
  a `testing` or `mock` Cargo feature so it doesn't bloat production builds:
  ```rust
  #[cfg(feature = "testing")]
  pub struct MockClient { ... }
  ```
- **CLI use case:** Test debate logic, conversation flow, and error-handling paths
  without hitting the API

### 4. `ChatSession::load_history()` missing ⭐⭐
- `history_mut()` lets you swap messages, but it's awkward for the common pattern of
  restoring a saved session
- **Ideal API:**
  ```rust
  impl ChatSession {
      pub fn load_history(&mut self, messages: Vec<Message>);
  }
  ```

---

## 📊 Lower Priority

### Debate / Orchestration Integration
- The CLI's `debate` subcommand is hand-rolled on top of raw `send_prompt` calls
- v0.6.0 ships `AiOrchestrator`, consensus strategies, response fusion, and confidence
  scoring that the CLI doesn't use at all
- A `DebateSession` or `ModeratedDebate` struct in the crate would let the CLI delegate
  the debate protocol to the library and keep non-CLI consumers in sync

### Request Cancellation
- Ability to abort an in-flight request (e.g. Ctrl-C during a stream)

### Per-model Progress Callbacks for `execute_parallel`
- A channel or callback that fires when each individual model finishes would let the CLI
  show a live progress display during parallel runs (currently the CLI blocks until all
  models return)

---

## 🐛 Ongoing Issues

1. **ChatSession takes client by value** — the client is moved into the session; there
   is no way to extract or replace it without rebuilding the entire session (present since
   v0.3.0, still true in v0.8.0)

2. **Stale TODO comments in CLI source** — `main.rs` lines ~827 and ~846 still say
   "once ChatSession::clear() / get_history() is available". Both methods now exist.
   Those comments should be removed and the code wired up.

---

*This wishlist is maintained by the chatdelta-cli project as a communication channel
with chatdelta-rs. For the reverse channel (crate → CLI), see `WHATSNEW.md` in
chatdelta-rs.*
