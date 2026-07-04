---
title: Goose Session & Conversation — SQLite Persistence, Message Model, Token Tracking
---

# Goose Session & Conversation — SQLite Persistence, Message Model, Token Tracking

**Every goose conversation is persisted in SQLite with a v5 schema tracking sessions, messages, token usage, extensions, and scheduled jobs.** The `SessionManager` provides full CRUD with automatic schema migration — older database versions are transparently upgraded. The `Conversation` type is a newtype over `Vec<Message>` that enforces structural invariants through validation on construction and repair on every agent turn. Messages carry dual-visibility metadata (`user_visible` / `agent_visible`) that enables the compaction system to hide LLM context from humans and hide compaction artifacts from the LLM.

Source: `crates/goose/src/session/session_manager.rs` — 1,359 lines, `crates/goose/src/conversation/mod.rs` — 1,109 lines, `crates/goose/src/conversation/message.rs` — 1,215 lines

## SQLite Schema

### Global Singleton

```rust
// crates/goose/src/session/session_manager.rs:63
static SESSION_STORAGE: OnceCell<Arc<SessionStorage>> = OnceCell::const_new();
```

Initialized once, shared across the entire process via `Arc<SessionStorage>`. All session operations go through this singleton — no connection pooling needed since goose is single-user.

### Schema Version History (v1 → v5)

| Version | Migration |
|---------|-----------|
| v1 | Initial: `sessions` table (id, name, working_dir, created_at, updated_at, conversation_json) |
| v2 | Added `session_type` (User/Scheduled/SubAgent/Hidden), `schedule_id` |
| v3 | Added accumulated token fields: `accumulated_total_tokens`, `accumulated_input_tokens`, `accumulated_output_tokens` |
| v4 | Added `recipe` (JSON blob), `user_recipe_values` (JSON blob) |
| v5 | Added `extension_data` (JSON blob tracking enabled extensions), `message_count` |

All migrations are additive — columns are added, never dropped. The `conversation` is stored as a JSON blob in the `conversation_json` column.

### The Session Struct

```rust
// crates/goose/src/session/session_manager.rs:65-90
pub struct Session {
    pub id: String,                                          // UUID v4
    pub working_dir: PathBuf,                                // Session-scoped filesystem root
    pub name: String,                                        // Human-readable (auto-generated or user-set)
    pub user_set_name: bool,                                 // false = auto-generated from first prompt
    pub session_type: SessionType,                           // User | Scheduled | SubAgent | Hidden
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub extension_data: ExtensionData,                       // Which extensions were enabled
    pub total_tokens: Option<i32>,                           // Current session tokens
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub accumulated_total_tokens: Option<i32>,               // Lifetime tokens (never reset)
    pub accumulated_input_tokens: Option<i32>,
    pub accumulated_output_tokens: Option<i32>,
    pub schedule_id: Option<String>,                         // If cron-triggered
    pub recipe: Option<Recipe>,                              // Recipe configuration
    pub user_recipe_values: Option<HashMap<String, String>>, // User-provided recipe parameters
    pub conversation: Option<Conversation>,                  // Full message history (JSON blob)
    pub message_count: usize,                                // Denormalized for fast listing
}
```

**Aha:** Token tracking exists at TWO levels. `total_tokens` resets per session (each new conversation starts at 0). `accumulated_total_tokens` increments monotonically across all sessions and never resets. This means goose can tell you both "this session used 15K tokens" and "you've used 2.3M tokens total across all sessions" — critical for cost tracking without querying every historical session.

### Session Types

```rust
pub enum SessionType {
    User,        // Normal interactive CLI/HTTP session
    Scheduled,   // Cron-triggered (no human in the loop)
    SubAgent,    // Sub-agent task delegation (restricted tools)
    Hidden,      // Internal operations (not shown in session list)
}
```

## Conversation — The Message Container

```rust
// crates/goose/src/conversation/mod.rs:11-12
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
pub struct Conversation(Vec<Message>);
```

### The push() Method — Streaming Accumulation

```rust
// crates/goose/src/conversation/mod.rs:44-63
pub fn push(&mut self, message: Message) {
    if let Some(last) = self.0.last_mut()
        .filter(|m| m.id.is_some() && m.id == message.id)
    {
        // SAME message ID → merge content (streaming accumulation)
        match (last.content.last_mut(), message.content.last()) {
            (Some(MessageContent::Text(ref mut last)),
             Some(MessageContent::Text(new)))
                if message.content.len() == 1 =>
            {
                last.text.push_str(&new.text);  // Concatenate streaming text
            }
            (_, _) => {
                last.content.extend(message.content);  // Extend with new content blocks
            }
        }
    } else {
        self.0.push(message);  // New message ID → append
    }
}
```

**Aha:** The ID-based merge is how goose handles streaming. As the LLM streams tokens, each chunk arrives with the same message UUID. `push()` detects the matching ID and appends to the existing message rather than creating duplicates. This means the UI can display progressively building messages without a separate buffering layer, AND the conversation stored in SQLite is always a clean deduplicated view.

### fix_conversation() — Structural Repair

```rust
// crates/goose/src/conversation/mod.rs
pub fn fix_conversation(conv: Conversation) -> (Conversation, Vec<ConversationIssue>) {
    // Runs on EVERY reply() invocation. Detects and fixes:
    // 1. Duplicate system messages → keep only the LAST one
    // 2. Tool responses without matching tool requests → REMOVE orphan response
    // 3. Tool requests without matching responses → mark as pending
    // 4. Non-alternating roles (two User messages in a row) → insert assistant placeholder
    // 5. Empty messages (content = []) → remove
    // 6. Messages where ALL content blocks are empty → remove
    // Returns the fixed conversation + list of issues found (for logging)
}
```

**Why this exists:** LLMs are not guaranteed to produce structurally valid conversations. They can hallucinate tool response IDs that don't match any request, produce duplicate system messages, or emit empty content blocks. `fix_conversation()` acts as the safety net — it ensures the conversation is always structurally valid before the next LLM call, preventing cascading failures from a single malformed response.

## The Message Model

```rust
// crates/goose/src/conversation/message.rs
pub struct Message {
    pub id: Option<String>,                  // UUID — same ID across streaming chunks
    pub role: Role,                          // User | Assistant | System
    pub content: Vec<MessageContent>,        // MULTIPLE content blocks per message
    pub metadata: Option<MessageMetadata>,
    pub created_at: DateTime<Utc>,
    pub provider_data: Option<ProviderData>,  // Raw provider response for debugging
}

pub enum MessageContent {
    Text(TextContent),                              // { text: String }
    ToolRequest(ToolRequestContent),                // { id, tool_call: Result<ToolCall, Error> }
    ToolResponse(ToolResponseContent),              // { id, tool_result: Result<Vec<Content>, Error> }
    Thinking(ThinkingContent),                      // { thinking: String } — Claude extended thinking
    RedactedThinking(RedactedThinkingContent),       // Thinking hidden by provider
    SystemNotification(SystemNotificationContent),  // { msg: String, notification_type }
    ToolConfirmationRequest(ToolConfirmationRequestContent),
    FrontendToolRequest(FrontendToolRequestContent),
    FrontendToolResponse(FrontendToolResponseContent),
    Image(ImageContent),                            // { mime_type, data (base64) }
}
```

### MessageMetadata — The Visibility System

```rust
pub struct MessageMetadata {
    pub tool_call_id: Option<String>,    // Links ToolResponse to ToolRequest
    pub model: Option<String>,           // Which model produced this message
    pub tokens: Option<TokenUsage>,
    pub cost: Option<f64>,
    pub extension_name: Option<String>,  // Which MCP server handled the tool call
    pub user_visible: Option<bool>,      // Can the human see this?
    pub agent_visible: Option<bool>,     // Can the LLM see this?
}

impl MessageMetadata {
    pub fn agent_only() -> Self {        // For compaction summaries
        Self { user_visible: Some(false), agent_visible: Some(true), ..Default::default() }
    }
    pub fn with_agent_invisible(self) -> Self {  // For compacted originals
        Self { agent_visible: Some(false), ..self }
    }
}
```

**Aha:** This dual-visibility system is the foundation of goose's compaction. During compaction, original messages are marked `user_visible=true, agent_visible=false` — the human can still scroll back and see past conversation, but the LLM doesn't see them (reducing token usage). The compaction summary is marked `agent_visible=true, user_visible=false` — the LLM gets context continuity but the human doesn't see summary clutter. Everywhere in the codebase, `is_agent_visible()` / `is_user_visible()` gates determine what goes to the LLM vs the UI.

## Session Name Generation

```rust
// crates/goose/src/providers/base.rs:35
pub static MSG_COUNT_FOR_SESSION_NAME_GENERATION: usize = 3;

// After 3 messages, goose calls the LLM to generate a session name
// The session name is derived from the conversation topic, not a random UUID
// Example: "Building a React dashboard with D3 charts" instead of "Session a3f2c1b9"
```

## Token Tracking

```rust
// Token counts stored per-session AND accumulated:
// Per-session: total_tokens, input_tokens, output_tokens (reset each session)
// Accumulated: accumulated_* (monotonically increasing, never reset)
// 
// Updated after every provider.complete() call via ProviderUsage
// SessionManager.update_session() persists to SQLite
```

## Cross-References

- [Compaction & Context](06-compaction-context.md) — How the visibility system enables LLM-based summarization
- [Architecture](01-architecture.md) — Where fix_conversation() and push() are called in the reply loop
- [CLI Deep Dive](12-cli.md) — Session CRUD commands

## Next

→ [Compaction & Context](06-compaction-context.md) — Token counting, threshold calculation, LLM-based conversation summarization.
