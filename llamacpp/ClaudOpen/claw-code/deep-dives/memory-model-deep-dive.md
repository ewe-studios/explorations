# Memory Management and Session Persistence Deep-Dive

A comprehensive analysis of how Claw Code manages conversation state, persists sessions, and implements compaction.

## Table of Contents

1. [Overview](#overview)
2. [Session Structure](#session-structure)
3. [Message Types](#message-types)
4. [Content Blocks](#content-blocks)
5. [JSON Serialization](#json-serialization)
6. [Session Persistence](#session-persistence)
7. [Compaction Algorithm](#compaction-algorithm)
8. [Auto-Compaction](#auto-compaction)
9. [Token Estimation](#token-estimation)
10. [Testing](#testing)

---

## Overview

Claw Code maintains conversation state through a session management system that:

- Persists messages to disk as JSON
- Tracks token usage for cost estimation
- Implements automatic compaction to manage context window
- Supports session resumption across restarts

**Location**: `rust/crates/runtime/src/session.rs`, `rust/crates/runtime/src/compact.rs`

**Key Types**:
- `Session` - Top-level conversation container
- `ConversationMessage` - Individual message with role and blocks
- `ContentBlock` - Text, tool use, and tool result content
- `CompactionConfig` - Compaction thresholds and settings

---

## Session Structure

### Session Struct

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    /// Schema version for migration support
    pub version: u32,

    /// Ordered list of conversation messages
    pub messages: Vec<ConversationMessage>,
}

impl Session {
    /// Create a new empty session
    pub fn new() -> Self {
        Self {
            version: 1,
            messages: Vec::new(),
        }
    }

    /// Get total message count
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if session is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Add a user message
    pub fn add_user_message(&mut self, text: impl Into<String>) {
        self.messages.push(ConversationMessage::user_text(text));
    }

    /// Add an assistant message
    pub fn add_assistant_message(&mut self, blocks: Vec<ContentBlock>) {
        self.messages.push(ConversationMessage::assistant(blocks));
    }

    /// Add a tool result
    pub fn add_tool_result(
        &mut self,
        tool_use_id: impl Into<String>,
        tool_name: impl Into<String>,
        output: impl Into<String>,
        is_error: bool,
    ) {
        self.messages.push(ConversationMessage::tool_result(
            tool_use_id,
            tool_name,
            output,
            is_error,
        ));
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}
```

### Session File Location

Sessions are stored in:
- **Linux**: `~/.claude/sessions/<session-id>.json`
- **macOS**: `~/Library/Application Support/Claude/sessions/<session-id>.json`
- **Windows**: `%APPDATA%\Claude\sessions\<session-id>.json`

Session IDs are typically timestamps or UUIDs:
```
~/.claude/sessions/
├── 20260402-231358-abc123.json
├── 20260403-094521-def456.json
└── current.json  # Symlink to most recent
```

---

## Message Types

### MessageRole Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    /// System instructions (not typically used in conversation)
    System,

    /// User input
    User,

    /// Assistant response
    Assistant,

    /// Tool execution result
    Tool,
}
```

### ConversationMessage Struct

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationMessage {
    /// Role of the message sender
    pub role: MessageRole,

    /// Content blocks (text, tool use, tool result)
    pub blocks: Vec<ContentBlock>,

    /// Token usage tracking (assistant messages only)
    pub usage: Option<TokenUsage>,
}

impl ConversationMessage {
    /// Create a user text message
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            blocks: vec![ContentBlock::Text { text: text.into() }],
            usage: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::Assistant,
            blocks,
            usage: None,
        }
    }

    /// Create an assistant message with token usage
    pub fn assistant_with_usage(blocks: Vec<ContentBlock>, usage: Option<TokenUsage>) -> Self {
        Self {
            role: MessageRole::Assistant,
            blocks,
            usage,
        }
    }

    /// Create a tool result message
    pub fn tool_result(
        tool_use_id: impl Into<String>,
        tool_name: impl Into<String>,
        output: impl Into<String>,
        is_error: bool,
    ) -> Self {
        Self {
            role: MessageRole::Tool,
            blocks: vec![ContentBlock::ToolResult {
                tool_use_id: tool_use_id.into(),
                tool_name: tool_name.into(),
                output: output.into(),
                is_error,
            }],
            usage: None,
        }
    }

    /// Get the primary text content (if any)
    pub fn as_text(&self) -> Option<&str> {
        self.blocks.iter().find_map(|block| {
            if let ContentBlock::Text { text } = block {
                Some(text.as_str())
            } else {
                None
            }
        })
    }

    /// Get tool use blocks
    pub fn get_tool_uses(&self) -> Vec<&ContentBlock> {
        self.blocks.iter().filter(|block| {
            matches!(block, ContentBlock::ToolUse { .. })
        }).collect()
    }

    /// Get tool result blocks
    pub fn get_tool_results(&self) -> Vec<&ContentBlock> {
        self.blocks.iter().filter(|block| {
            matches!(block, ContentBlock::ToolResult { .. })
        }).collect()
    }
}
```

---

## Content Blocks

### ContentBlock Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentBlock {
    /// Plain text content
    Text {
        text: String,
    },

    /// Tool invocation request
    ToolUse {
        /// Unique identifier for this tool use
        id: String,

        /// Name of the tool to invoke
        name: String,

        /// JSON input for the tool
        input: String,
    },

    /// Result from tool execution
    ToolResult {
        /// ID of the tool use this is responding to
        tool_use_id: String,

        /// Name of the tool that was executed
        tool_name: String,

        /// Output content from the tool
        output: String,

        /// Whether the tool execution failed
        is_error: bool,
    },
}
```

### Example Content Blocks

```rust
// User text message
ContentBlock::Text {
    text: String::from("What files are in this directory?"),
}

// Assistant tool use
ContentBlock::ToolUse {
    id: String::from("toolu_abc123"),
    name: String::from("bash"),
    input: String::from(r#"{"command": "ls -la"}"#),
}

// Tool execution result
ContentBlock::ToolResult {
    tool_use_id: String::from("toolu_abc123"),
    tool_name: String::from("bash"),
    output: String::from("total 48\ndrwxr-xr-x 4 user user 4096 Apr 2 12:00 .\n..."),
    is_error: false,
}
```

---

## JSON Serialization

### Custom JSON Implementation

Claw Code uses a custom JSON parser (not serde_json) for session persistence to minimize dependencies and have full control over serialization format.

**Location**: `rust/crates/runtime/src/json.rs`

### JsonValue Enum

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}
```

### Session to JSON

```rust
impl Session {
    pub fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();

        // Version
        object.insert(
            "version".to_string(),
            JsonValue::Number(i64::from(self.version)),
        );

        // Messages array
        object.insert(
            "messages".to_string(),
            JsonValue::Array(
                self.messages
                    .iter()
                    .map(ConversationMessage::to_json)
                    .collect(),
            ),
        );

        JsonValue::Object(object)
    }
}

impl ConversationMessage {
    pub fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();

        // Role
        object.insert(
            "role".to_string(),
            JsonValue::String(
                match self.role {
                    MessageRole::System => "system",
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                }
                .to_string(),
            ),
        );

        // Blocks
        object.insert(
            "blocks".to_string(),
            JsonValue::Array(self.blocks.iter().map(ContentBlock::to_json).collect()),
        );

        // Usage (optional)
        if let Some(usage) = self.usage {
            object.insert("usage".to_string(), usage_to_json(usage));
        }

        JsonValue::Object(object)
    }
}

impl ContentBlock {
    pub fn to_json(&self) -> JsonValue {
        let mut object = BTreeMap::new();

        match self {
            ContentBlock::Text { text } => {
                object.insert("type".to_string(), JsonValue::String("text".to_string()));
                object.insert("text".to_string(), JsonValue::String(text.clone()));
            }
            ContentBlock::ToolUse { id, name, input } => {
                object.insert("type".to_string(), JsonValue::String("tool_use".to_string()));
                object.insert("id".to_string(), JsonValue::String(id.clone()));
                object.insert("name".to_string(), JsonValue::String(name.clone()));
                object.insert("input".to_string(), JsonValue::String(input.clone()));
            }
            ContentBlock::ToolResult {
                tool_use_id,
                tool_name,
                output,
                is_error,
            } => {
                object.insert("type".to_string(), JsonValue::String("tool_result".to_string()));
                object.insert("tool_use_id".to_string(), JsonValue::String(tool_use_id.clone()));
                object.insert("tool_name".to_string(), JsonValue::String(tool_name.clone()));
                object.insert("output".to_string(), JsonValue::String(output.clone()));
                object.insert("is_error".to_string(), JsonValue::Bool(*is_error));
            }
        }

        JsonValue::Object(object)
    }
}
```

### JSON to Session

```rust
impl Session {
    pub fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("session must be an object".to_string()))?;

        // Parse version
        let version = object
            .get("version")
            .and_then(JsonValue::as_i64)
            .ok_or_else(|| SessionError::Format("missing version".to_string()))?;

        let version = u32::try_from(version)
            .map_err(|_| SessionError::Format("version out of range".to_string()))?;

        // Parse messages
        let messages = object
            .get("messages")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| SessionError::Format("missing messages".to_string()))?
            .iter()
            .map(ConversationMessage::from_json)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { version, messages })
    }
}

impl ConversationMessage {
    fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("message must be an object".to_string()))?;

        // Parse role
        let role = match object
            .get("role")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| SessionError::Format("missing role".to_string()))?
        {
            "system" => MessageRole::System,
            "user" => MessageRole::User,
            "assistant" => MessageRole::Assistant,
            "tool" => MessageRole::Tool,
            other => {
                return Err(SessionError::Format(format!(
                    "unsupported message role: {other}"
                )))
            }
        };

        // Parse blocks
        let blocks = object
            .get("blocks")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| SessionError::Format("missing blocks".to_string()))?
            .iter()
            .map(ContentBlock::from_json)
            .collect::<Result<Vec<_>, _>>()?;

        // Parse usage (optional)
        let usage = object.get("usage").map(usage_from_json).transpose()?;

        Ok(Self {
            role,
            blocks,
            usage,
        })
    }
}

impl ContentBlock {
    fn from_json(value: &JsonValue) -> Result<Self, SessionError> {
        let object = value
            .as_object()
            .ok_or_else(|| SessionError::Format("block must be an object".to_string()))?;

        match object
            .get("type")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| SessionError::Format("missing block type".to_string()))?
        {
            "text" => Ok(Self::Text {
                text: required_string(object, "text")?,
            }),
            "tool_use" => Ok(Self::ToolUse {
                id: required_string(object, "id")?,
                name: required_string(object, "name")?,
                input: required_string(object, "input")?,
            }),
            "tool_result" => Ok(Self::ToolResult {
                tool_use_id: required_string(object, "tool_use_id")?,
                tool_name: required_string(object, "tool_name")?,
                output: required_string(object, "output")?,
                is_error: object
                    .get("is_error")
                    .and_then(JsonValue::as_bool)
                    .ok_or_else(|| SessionError::Format("missing is_error".to_string()))?,
            }),
            other => Err(SessionError::Format(format!(
                "unsupported block type: {other}"
            ))),
        }
    }
}
```

### Helper Functions

```rust
fn usage_to_json(usage: TokenUsage) -> JsonValue {
    let mut object = BTreeMap::new();
    object.insert(
        "input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.input_tokens)),
    );
    object.insert(
        "output_tokens".to_string(),
        JsonValue::Number(i64::from(usage.output_tokens)),
    );
    object.insert(
        "cache_creation_input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.cache_creation_input_tokens)),
    );
    object.insert(
        "cache_read_input_tokens".to_string(),
        JsonValue::Number(i64::from(usage.cache_read_input_tokens)),
    );
    JsonValue::Object(object)
}

fn usage_from_json(value: &JsonValue) -> Result<TokenUsage, SessionError> {
    let object = value
        .as_object()
        .ok_or_else(|| SessionError::Format("usage must be an object".to_string()))?;

    Ok(TokenUsage {
        input_tokens: required_u32(object, "input_tokens")?,
        output_tokens: required_u32(object, "output_tokens")?,
        cache_creation_input_tokens: required_u32(object, "cache_creation_input_tokens")?,
        cache_read_input_tokens: required_u32(object, "cache_read_input_tokens")?,
    })
}

fn required_string(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<String, SessionError> {
    object
        .get(key)
        .and_then(JsonValue::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))
}

fn required_u32(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<u32, SessionError> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| SessionError::Format(format!("missing {key}")))?;

    u32::try_from(value).map_err(|_| SessionError::Format(format!("{key} out of range")))
}
```

---

## Session Persistence

### Save to Disk

```rust
impl Session {
    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), SessionError> {
        let json = self.to_json();
        let rendered = json.render();
        fs::write(path, rendered)?;
        Ok(())
    }

    pub fn save_to_default_path(&self) -> Result<PathBuf, SessionError> {
        let session_dir = get_session_directory()?;
        let session_id = generate_session_id();
        let path = session_dir.join(format!("{}.json", session_id));

        self.save_to_path(&path)?;
        Ok(path)
    }
}

fn get_session_directory() -> Result<PathBuf, SessionError> {
    // Use XDG directories on Linux
    #[cfg(target_os = "linux")]
    {
        if let Ok(data_dir) = std::env::var("XDG_DATA_HOME") {
            return Ok(PathBuf::from(data_dir).join("claude").join("sessions"));
        }
        if let Ok(home) = std::env::var("HOME") {
            return Ok(PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("claude")
                .join("sessions"));
        }
    }

    // Fallback to ~/.claude/sessions
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home).join(".claude").join("sessions"));
    }

    Err(SessionError::Io(io::Error::new(
        io::ErrorKind::NotFound,
        "could not determine home directory",
    )))
}

fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}", nanos)
}
```

### Load from Disk

```rust
impl Session {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, SessionError> {
        let contents = fs::read_to_string(path)?;
        let json = JsonValue::parse(&contents)?;
        Self::from_json(&json)
    }

    pub fn load_latest() -> Result<Option<Self>, SessionError> {
        let session_dir = get_session_directory()?;
        if !session_dir.exists() {
            return Ok(None);
        }

        // Find most recent session file
        let mut latest: Option<(SystemTime, PathBuf)> = None;

        for entry in fs::read_dir(&session_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(modified) = metadata.modified() {
                        if latest.is_none() || modified > latest.unwrap().0 {
                            latest = Some((modified, path));
                        }
                    }
                }
            }
        }

        if let Some((_, path)) = latest {
            Ok(Some(Self::load_from_path(path)?))
        } else {
            Ok(None)
        }
    }
}
```

### JSON Render Implementation

```rust
impl JsonValue {
    pub fn render(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::String(value) => render_string(value),
            Self::Array(values) => {
                let rendered = values
                    .iter()
                    .map(Self::render)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("[{rendered}]")
            }
            Self::Object(entries) => {
                let rendered = entries
                    .iter()
                    .map(|(key, value)| format!("{}:{}", render_string(key), value.render()))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{rendered}}}")
            }
        }
    }
}

fn render_string(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len() + 2);
    rendered.push('"');
    for ch in value.chars() {
        match ch {
            '"' => rendered.push_str("\\\""),
            '\\' => rendered.push_str("\\\\"),
            '\n' => rendered.push_str("\\n"),
            '\r' => rendered.push_str("\\r"),
            '\t' => rendered.push_str("\\t"),
            '\u{08}' => rendered.push_str("\\b"),
            '\u{0C}' => rendered.push_str("\\f"),
            control if control.is_control() => push_unicode_escape(&mut rendered, control),
            plain => rendered.push(plain),
        }
    }
    rendered.push('"');
    rendered
}

fn push_unicode_escape(rendered: &mut String, control: char) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    rendered.push_str("\\u");
    let value = u32::from(control);
    for shift in [12_u32, 8, 4, 0] {
        let nibble = ((value >> shift) & 0xF) as usize;
        rendered.push(char::from(HEX[nibble]));
    }
}
```

---

## Compaction Algorithm

### CompactionConfig

```rust
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Number of recent messages to always preserve
    pub preserve_recent_messages: usize,

    /// Maximum estimated tokens before compaction triggers
    pub max_estimated_tokens: usize,

    /// Target tokens after compaction
    pub target_tokens_after_compact: usize,

    /// Whether to use summarization for old messages
    pub use_summarization: bool,

    /// Model to use for summarization
    pub summarization_model: String,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            preserve_recent_messages: 4,
            max_estimated_tokens: 10000,
            target_tokens_after_compact: 6000,
            use_summarization: true,
            summarization_model: String::from("claude-haiku-3-5"),
        }
    }
}
```

### Compact Session Function

```rust
pub fn compact_session(
    session: &Session,
    config: &CompactionConfig,
) -> Result<Session, CompactionError> {
    if session.messages.len() <= config.preserve_recent_messages {
        // Nothing to compact
        return Ok(session.clone());
    }

    // Calculate current token estimate
    let current_tokens = estimate_session_tokens(session);
    if current_tokens <= config.max_estimated_tokens {
        // No compaction needed
        return Ok(session.clone());
    }

    // Determine which messages to keep/compact
    let messages_to_compact = session.messages.len() - config.preserve_recent_messages;
    let (old_messages, recent_messages): (Vec<_>, Vec<_>) = session.messages
        .iter()
        .cloned()
        .partition(|(i, _)| i < messages_to_compact);

    // Summarize old messages if enabled
    let summary = if config.use_summarization && !old_messages.is_empty() {
        summarize_messages(&old_messages, config)?
    } else {
        // Just keep the oldest message as anchor
        old_messages.first().cloned().map(|(_, msg)| {
            ConversationMessage {
                role: MessageRole::User,
                blocks: vec![ContentBlock::Text {
                    text: format!(
                        "[Previous conversation with {} messages was summarized]",
                        old_messages.len()
                    ),
                }],
                usage: None,
            }
        })
    };

    // Build compacted session
    let mut compacted_messages = Vec::new();

    // Add summary message
    if let Some(summary) = summary {
        compacted_messages.push(summary);
    }

    // Add recent messages
    compacted_messages.extend(recent_messages);

    Ok(Session {
        version: session.version,
        messages: compacted_messages,
    })
}
```

### Summarize Messages

```rust
fn summarize_messages(
    messages: &[(usize, ConversationMessage)],
    config: &CompactionConfig,
) -> Result<ConversationMessage, CompactionError> {
    // Build context from old messages
    let mut context = String::new();
    for (_, msg) in messages {
        if let Some(text) = msg.as_text() {
            context.push_str(&match msg.role {
                MessageRole::User => format!("User: {}\n", text),
                MessageRole::Assistant => format!("Assistant: {}\n", text),
                MessageRole::Tool => format!("[Tool result: {}]\n", text),
                MessageRole::System => String::new(),
            });
        }
    }

    // Request summarization from API
    // This would normally be async and call the API
    // For now, we create a simple placeholder
    let summary_text = format!(
        "[Conversation summary: {} messages covering the following topics...]",
        messages.len()
    );

    Ok(ConversationMessage {
        role: MessageRole::User,
        blocks: vec![ContentBlock::Text { text: summary_text }],
        usage: None,
    })
}
```

### Token Estimation

```rust
/// Estimate tokens in a session
/// Uses rough heuristic: ~4 characters per token
pub fn estimate_session_tokens(session: &Session) -> usize {
    session.messages.iter().map(estimate_message_tokens).sum()
}

fn estimate_message_tokens(message: &ConversationMessage) -> usize {
    let mut tokens = 0;

    // Base overhead per message
    tokens += 4;  // Role, metadata

    for block in &message.blocks {
        tokens += estimate_block_tokens(block);
    }

    // Add usage if tracked
    if let Some(usage) = message.usage {
        tokens += usage.input_tokens as usize;
        tokens += usage.output_tokens as usize;
    }

    tokens
}

fn estimate_block_tokens(block: &ContentBlock) -> usize {
    match block {
        ContentBlock::Text { text } => {
            // ~4 chars per token
            text.len() / 4
        }
        ContentBlock::ToolUse { id, name, input } => {
            // Tool calls have structure overhead
            10 + (id.len() / 4) + (name.len() / 4) + (input.len() / 4)
        }
        ContentBlock::ToolResult { tool_use_id, tool_name, output, is_error } => {
            // Tool results have structure overhead
            8 + (tool_use_id.len() / 4) + (tool_name.len() / 4) + (output.len() / 4)
        }
    }
}
```

---

## Auto-Compaction

### Configuration from Environment

```rust
/// Check if auto-compaction should trigger
pub fn should_auto_compact(
    session: &Session,
    config: &CompactionConfig,
) -> bool {
    let tokens = estimate_session_tokens(session);
    tokens > config.max_estimated_tokens
}

/// Generate continuation message after compaction
pub fn auto_compaction_continuation_message() -> ConversationMessage {
    ConversationMessage::user_text(
        "I've summarized our earlier conversation to save space. Please continue helping me with my task."
    )
}
```

### Runtime Integration

```rust
impl ConversationRuntime {
    pub fn run_turn(&mut self, input: &str) -> Result<ConversationTurn> {
        // Check if compaction needed before processing
        if should_auto_compact(&self.session, &self.compaction_config) {
            self.session = compact_session(&self.session, &self.compaction_config)?;

            // Add continuation message
            self.session.messages.push(auto_compaction_continuation_message());
        }

        // Continue with normal turn processing
        // ...
    }
}
```

### Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLAWD_AUTO_COMPACT_THRESHOLD` | Token threshold for auto-compaction | 200000 |
| `CLAWD_COMPACT_TARGET` | Target tokens after compaction | 100000 |
| `CLAWD_PRESERVE_MESSAGES` | Number of messages to preserve | 4 |

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn persists_and_restores_session_json() {
        let mut session = Session::new();
        session.add_user_message("hello");
        session.add_assistant_message(vec![
            ContentBlock::Text {
                text: "thinking".to_string(),
            },
            ContentBlock::ToolUse {
                id: "tool-1".to_string(),
                name: "bash".to_string(),
                input: "echo hi".to_string(),
            },
        ]);
        session.add_tool_result("tool-1", "bash", "hi", false);

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("runtime-session-{nanos}.json"));

        session.save_to_path(&path).expect("session should save");
        let restored = Session::load_from_path(&path).expect("session should load");
        fs::remove_file(&path).expect("temp file should be removable");

        assert_eq!(restored, session);
        assert_eq!(restored.messages[2].role, MessageRole::Tool);
        assert_eq!(
            restored.messages[1].usage.expect("usage").total_tokens(),
            17
        );
    }

    #[test]
    fn compaction_reduces_tokens() {
        let mut session = Session::new();

        // Add many messages
        for i in 0..50 {
            session.add_user_message(format!("Question {}", i));
            session.add_assistant_message(vec![ContentBlock::Text {
                text: format!("Answer {}", i),
            }]);
        }

        let config = CompactionConfig::default();
        let compacted = compact_session(&session, &config).expect("compaction should succeed");

        assert!(compacted.messages.len() < session.messages.len());
        assert!(estimate_session_tokens(&compacted) < estimate_session_tokens(&session));
    }

    #[test]
    fn preserves_recent_messages_after_compaction() {
        let mut session = Session::new();
        for i in 0..20 {
            session.add_user_message(format!("Message {}", i));
        }

        let config = CompactionConfig {
            preserve_recent_messages: 4,
            ..Default::default()
        };

        let compacted = compact_session(&session, &config).expect("compaction should succeed");

        // Last 4 messages should be preserved
        assert!(compacted.messages.len() >= 4);
    }
}
```

---

## Related Files

| File | Purpose |
|------|---------|
| `rust/crates/runtime/src/session.rs` | Session and message types |
| `rust/crates/runtime/src/compact.rs` | Compaction algorithm |
| `rust/crates/runtime/src/json.rs` | Custom JSON parser |
| `rust/crates/runtime/src/usage.rs` | Token usage tracking |
| `rust/crates/runtime/src/conversation.rs` | Runtime integration |
