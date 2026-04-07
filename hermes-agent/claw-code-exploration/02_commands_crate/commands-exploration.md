# Commands Crate — Line-by-Line Exploration

**Crate:** `commands`  
**Status:** Identical in both claw-code and claw-code-latest  
**Purpose:** Slash command parsing, registry, and handling  
**Total Lines:** 622  
**Files:** `src/lib.rs` (single file crate)

---

## Table of Contents

1. [Module Overview](#module-overview)
2. [Command Registry Types (Lines 3-31)](#command-registry-types)
3. [SlashCommandSpec Definitions (Lines 33-174)](#slashcommandspec-definitions)
4. [SlashCommand Enum (Lines 176-226)](#slashcommand-enum)
5. [Command Parsing (Lines 228-298)](#command-parsing)
6. [Help Rendering (Lines 300-332)](#help-rendering)
7. [Command Handling (Lines 334-388)](#command-handling)
8. [Unit Tests (Lines 390-622)](#unit-tests)
9. [Integration Points](#integration-points)

---

## Module Overview

The commands crate provides the **slash command system** for claw-code's CLI. It handles:

- **Command specification** - Metadata for all 22 slash commands
- **Command parsing** - Convert `/command args` to typed enums
- **Help rendering** - Formatted help output
- **Command handling** - Execute commands that modify state (e.g., `/compact`)

This crate is **pure logic** - it parses and handles commands but doesn't perform I/O.

---

## Command Registry Types (Lines 3-31)

### CommandManifestEntry (Lines 3-7)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandManifestEntry {
    pub name: String,
    pub source: CommandSource,
}
```

**Purpose:** Represents a command extracted from upstream TypeScript source.

**Fields:**

| Field | Type | Purpose |
|-------|------|---------|
| `name` | `String` | Command identifier |
| `source` | `CommandSource` | Where the command comes from |

### CommandSource Enum (Lines 9-14)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    Builtin,
    InternalOnly,
    FeatureGated,
}
```

**Variants:**

| Variant | Meaning |
|---------|---------|
| `Builtin` | Core commands imported in commands.ts |
| `InternalOnly` | Commands in INTERNAL_ONLY_COMMANDS array |
| `FeatureGated` | Commands behind feature() calls |

Used by compat-harness to track upstream command origins.

### CommandRegistry (Lines 16-31)

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandRegistry {
    entries: Vec<CommandManifestEntry>,
}
```

**Methods:**

#### `new()` (Lines 22-25)
```rust
#[must_use]
pub fn new(entries: Vec<CommandManifestEntry>) -> Self {
    Self { entries }
}
```

#### `entries()` (Lines 27-30)
```rust
#[must_use]
pub fn entries(&self) -> &[CommandManifestEntry] {
    &self.entries
}
```
Returns slice of registry entries.

---

## SlashCommandSpec Definitions (Lines 33-174)

### SlashCommandSpec Struct (Lines 33-39)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlashCommandSpec {
    pub name: &'static str,
    pub summary: &'static str,
    pub argument_hint: Option<&'static str>,
    pub resume_supported: bool,
}
```

**Fields:**

| Field | Type | Purpose |
|-------|------|---------|
| `name` | `&'static str` | Command name without `/` |
| `summary` | `&'static str` | One-line description |
| `argument_hint` | `Option<&'static str>` | Syntax hint for args |
| `resume_supported` | `bool` | Works with `--resume` flag |

### SLASH_COMMAND_SPECS (Lines 41-174)

```rust
const SLASH_COMMAND_SPECS: &[SlashCommandSpec] = &[
    SlashCommandSpec {
        name: "help",
        summary: "Show available slash commands",
        argument_hint: None,
        resume_supported: true,
    },
    // ... 21 more specs
];
```

### Complete Command Inventory

| # | Command | Summary | Args | Resume |
|---|---------|---------|------|--------|
| 1 | `/help` | Show available slash commands | — | ✓ |
| 2 | `/status` | Show current session status | — | ✓ |
| 3 | `/compact` | Compact local session history | — | ✓ |
| 4 | `/model` | Show or switch the active model | `[model]` | ✗ |
| 5 | `/permissions` | Show or switch the active permission mode | `[mode]` | ✗ |
| 6 | `/clear` | Start a fresh local session | `[--confirm]` | ✓ |
| 7 | `/cost` | Show cumulative token usage | — | ✓ |
| 8 | `/resume` | Load a saved session into the REPL | `<session-path>` | ✗ |
| 9 | `/config` | Inspect Claude config files | `[section]` | ✓ |
| 10 | `/memory` | Inspect loaded Claude instruction memory files | — | ✓ |
| 11 | `/init` | Create a starter CLAUDE.md for this repo | — | ✓ |
| 12 | `/diff` | Show git diff for current workspace changes | — | ✓ |
| 13 | `/version` | Show CLI version and build information | — | ✓ |
| 14 | `/bughunter` | Inspect the codebase for likely bugs | `[scope]` | ✗ |
| 15 | `/commit` | Generate a commit message and create a git commit | — | ✗ |
| 16 | `/pr` | Draft or create a pull request | `[context]` | ✗ |
| 17 | `/issue` | Draft or create a GitHub issue | `[context]` | ✗ |
| 18 | `/ultraplan` | Run a deep planning prompt | `[task]` | ✗ |
| 19 | `/teleport` | Jump to a file or symbol | `<symbol-or-path>` | ✗ |
| 20 | `/debug-tool-call` | Replay the last tool call with debug details | — | ✗ |
| 21 | `/export` | Export the current conversation | `[file]` | ✓ |
| 22 | `/session` | List or switch managed local sessions | `[action]` | ✗ |

**Statistics:**
- **22 total commands**
- **11 support resume** (50%)
- **12 take arguments** (55%)

---

## SlashCommand Enum (Lines 176-226)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommand {
    Help,
    Status,
    Compact,
    Bughunter {
        scope: Option<String>,
    },
    Commit,
    Pr {
        context: Option<String>,
    },
    Issue {
        context: Option<String>,
    },
    Ultraplan {
        task: Option<String>,
    },
    Teleport {
        target: Option<String>,
    },
    DebugToolCall,
    Model {
        model: Option<String>,
    },
    Permissions {
        mode: Option<String>,
    },
    Clear {
        confirm: bool,
    },
    Cost,
    Resume {
        session_path: Option<String>,
    },
    Config {
        section: Option<String>,
    },
    Memory,
    Init,
    Diff,
    Version,
    Export {
        path: Option<String>,
    },
    Session {
        action: Option<String>,
        target: Option<String>,
    },
    Unknown(String),
}
```

### Design Notes

**Unit variants** (no arguments):
- `Help`, `Status`, `Compact`, `Commit`, `DebugToolCall`, `Cost`, `Memory`, `Init`, `Diff`, `Version`

**Struct variants with single Option<String>**:
- `Bughunter { scope }`, `Pr { context }`, `Issue { context }`, `Ultraplan { task }`, `Teleport { target }`, `Model { model }`, `Permissions { mode }`, `Resume { session_path }`, `Config { section }`, `Export { path }`

**Special variants**:
- `Clear { confirm: bool }` - Boolean flag parsing
- `Session { action, target }` - Two-argument command
- `Unknown(String)` - Fallback for unrecognized commands

---

## Command Parsing (Lines 228-298)

### `parse()` (Lines 229-288)

```rust
#[must_use]
pub fn parse(input: &str) -> Option<Self> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let mut parts = trimmed.trim_start_matches('/').split_whitespace();
    let command = parts.next().unwrap_or_default();
    Some(match command {
        "help" => Self::Help,
        "status" => Self::Status,
        "compact" => Self::Compact,
        "bughunter" => Self::Bughunter {
            scope: remainder_after_command(trimmed, command),
        },
        "commit" => Self::Commit,
        "pr" => Self::Pr {
            context: remainder_after_command(trimmed, command),
        },
        "issue" => Self::Issue {
            context: remainder_after_command(trimmed, command),
        },
        "ultraplan" => Self::Ultraplan {
            task: remainder_after_command(trimmed, command),
        },
        "teleport" => Self::Teleport {
            target: remainder_after_command(trimmed, command),
        },
        "debug-tool-call" => Self::DebugToolCall,
        "model" => Self::Model {
            model: parts.next().map(ToOwned::to_owned),
        },
        "permissions" => Self::Permissions {
            mode: parts.next().map(ToOwned::to_owned),
        },
        "clear" => Self::Clear {
            confirm: parts.next() == Some("--confirm"),
        },
        "cost" => Self::Cost,
        "resume" => Self::Resume {
            session_path: parts.next().map(ToOwned::to_owned),
        },
        "config" => Self::Config {
            section: parts.next().map(ToOwned::to_owned),
        },
        "memory" => Self::Memory,
        "init" => Self::Init,
        "diff" => Self::Diff,
        "version" => Self::Version,
        "export" => Self::Export {
            path: parts.next().map(ToOwned::to_owned),
        },
        "session" => Self::Session {
            action: parts.next().map(ToOwned::to_owned),
            target: parts.next().map(ToOwned::to_owned),
        },
        other => Self::Unknown(other.to_string()),
    })
}
```

**Line-by-line breakdown:**

- Line 231: Trim whitespace from input
- Line 232-234: Return None if doesn't start with `/`
- Line 236: Strip leading `/` and split by whitespace
- Line 237: Extract command name (first token)
- Line 238-287: Match command name to variant

**Parsing strategies:**

1. **No arguments** (lines 239-241, 245, 258, 268, 275-278):
   ```rust
   "help" => Self::Help,
   ```

2. **Remainder capture** (lines 242-244, 246-251, 252-254, 255-257):
   ```rust
   "bughunter" => Self::Bughunter {
       scope: remainder_after_command(trimmed, command),
   },
   ```
   Captures everything after the command (e.g., `/bughunter runtime tests` → `scope: Some("runtime tests")`)

3. **Single token argument** (lines 259-261, 262-264, 269-271, 272-274, 279-281):
   ```rust
   "model" => Self::Model {
       model: parts.next().map(ToOwned::to_owned),
   },
   ```
   Takes only the next whitespace-separated token

4. **Boolean flag** (lines 265-267):
   ```rust
   "clear" => Self::Clear {
       confirm: parts.next() == Some("--confirm"),
   },
   ```

5. **Two arguments** (lines 282-285):
   ```rust
   "session" => Self::Session {
       action: parts.next().map(ToOwned::to_owned),
       target: parts.next().map(ToOwned::to_owned),
   },
   ```

6. **Unknown command** (line 286):
   ```rust
   other => Self::Unknown(other.to_string()),
   ```

### `remainder_after_command()` (Lines 291-298)

```rust
fn remainder_after_command(input: &str, command: &str) -> Option<String> {
    input
        .trim()
        .strip_prefix(&format!("/{command}"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}
```

**Purpose:** Extract all text after a command as a single string.

**Example:**
```
Input:  "/bughunter runtime tests"
Command: "bughunter"
Output: Some("runtime tests")
```

---

## Help Rendering (Lines 300-332)

### `slash_command_specs()` (Lines 300-303)

```rust
#[must_use]
pub fn slash_command_specs() -> &'static [SlashCommandSpec] {
    SLASH_COMMAND_SPECS
}
```
Accessor for the command specs array.

### `resume_supported_slash_commands()` (Lines 305-311)

```rust
#[must_use]
pub fn resume_supported_slash_commands() -> Vec<&'static SlashCommandSpec> {
    slash_command_specs()
        .iter()
        .filter(|spec| spec.resume_supported)
        .collect()
}
```
Returns only commands that work with `--resume`.

### `render_slash_command_help()` (Lines 313-332)

```rust
#[must_use]
pub fn render_slash_command_help() -> String {
    let mut lines = vec![
        "Slash commands".to_string(),
        "  [resume] means the command also works with --resume SESSION.json".to_string(),
    ];
    for spec in slash_command_specs() {
        let name = match spec.argument_hint {
            Some(argument_hint) => format!("/{} {}", spec.name, argument_hint),
            None => format!("/{}", spec.name),
        };
        let resume = if spec.resume_supported {
            " [resume]"
        } else {
            ""
        };
        lines.push(format!("  {name:<20} {}{}", spec.summary, resume));
    }
    lines.join("\n")
}
```

**Output format:**
```
Slash commands
  [resume] means the command also works with --resume SESSION.json
  /help                Show available slash commands [resume]
  /status              Show current session status [resume]
  /compact             Compact local session history [resume]
  /model [model]       Show or switch the active model
  /permissions [read-only|workspace-write|danger-full-access]
```

**Line-by-line:**

- Line 315-318: Header lines
- Line 319: Iterate all specs
- Line 320-323: Format command name with optional argument hint
- Line 324-328: Append `[resume]` tag if supported
- Line 329: Format with left-aligned name (20 chars)
- Line 331: Join with newlines

---

## Command Handling (Lines 334-388)

### SlashCommandResult (Lines 334-338)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandResult {
    pub message: String,
    pub session: Session,
}
```

**Purpose:** Return type for commands that modify state.

### `handle_slash_command()` (Lines 340-388)

```rust
#[must_use]
pub fn handle_slash_command(
    input: &str,
    session: &Session,
    compaction: CompactionConfig,
) -> Option<SlashCommandResult> {
    match SlashCommand::parse(input)? {
        SlashCommand::Compact => {
            let result = compact_session(session, compaction);
            let message = if result.removed_message_count == 0 {
                "Compaction skipped: session is below the compaction threshold.".to_string()
            } else {
                format!(
                    "Compacted {} messages into a resumable system summary.",
                    result.removed_message_count
                )
            };
            Some(SlashCommandResult {
                message,
                session: result.compacted_session,
            })
        }
        SlashCommand::Help => Some(SlashCommandResult {
            message: render_slash_command_help(),
            session: session.clone(),
        }),
        SlashCommand::Status
        | SlashCommand::Bughunter { .. }
        | SlashCommand::Commit
        | SlashCommand::Pr { .. }
        | SlashCommand::Issue { .. }
        | SlashCommand::Ultraplan { .. }
        | SlashCommand::Teleport { .. }
        | SlashCommand::DebugToolCall
        | SlashCommand::Model { .. }
        | SlashCommand::Permissions { .. }
        | SlashCommand::Clear { .. }
        | SlashCommand::Cost
        | SlashCommand::Resume { .. }
        | SlashCommand::Config { .. }
        | SlashCommand::Memory
        | SlashCommand::Init
        | SlashCommand::Diff
        | SlashCommand::Version
        | SlashCommand::Export { .. }
        | SlashCommand::Session { .. }
        | SlashCommand::Unknown(_) => None,
    }
}
```

**Line-by-line:**

- Line 346: Parse input, return None if invalid
- Line 347-361: Handle `/compact`
  - Line 348: Call runtime's `compact_session()`
  - Line 349-356: Format message based on whether compaction occurred
  - Line 357-360: Return modified session
- Line 362-365: Handle `/help`
  - Returns help text, session unchanged
- Line 366-386: All other commands return `None`

**Why None for most commands?**

Commands like `/status`, `/version`, `/commit`, etc. require:
- I/O operations (git, filesystem, API calls)
- Runtime state not available in this crate
- CLI-specific context

These are handled by the `rusty-claude-cli` crate which has full context.

---

## Unit Tests (Lines 390-622)

### Test 1: `parses_supported_slash_commands()` (Lines 398-495)

```rust
#[test]
fn parses_supported_slash_commands() {
    assert_eq!(SlashCommand::parse("/help"), Some(SlashCommand::Help));
    assert_eq!(SlashCommand::parse(" /status "), Some(SlashCommand::Status));
    assert_eq!(
        SlashCommand::parse("/bughunter runtime"),
        Some(SlashCommand::Bughunter {
            scope: Some("runtime".to_string())
        })
    );
    // ... 20 more assertions
}
```

**Coverage:**
- Basic commands: `/help`, `/status`, `/compact`, `/commit`, `/cost`, etc.
- Commands with arguments: `/bughunter runtime`, `/pr ready for review`
- Boolean flag: `/clear`, `/clear --confirm`
- Two arguments: `/session switch abc123`
- Whitespace handling: `" /status "` (leading/trailing spaces)

### Test 2: `renders_help_from_shared_specs()` (Lines 497-525)

```rust
#[test]
fn renders_help_from_shared_specs() {
    let help = render_slash_command_help();
    assert!(help.contains("works with --resume SESSION.json"));
    assert!(help.contains("/help"));
    // ... assertions for all 22 commands
    assert_eq!(slash_command_specs().len(), 22);
    assert_eq!(resume_supported_slash_commands().len(), 11);
}
```

**Verifies:**
- Help output contains expected text
- All 22 commands registered
- Exactly 11 support resume

### Test 3: `compacts_sessions_via_slash_command()` (Lines 527-555)

```rust
#[test]
fn compacts_sessions_via_slash_command() {
    let session = Session {
        version: 1,
        messages: vec![
            ConversationMessage::user_text("a ".repeat(200)),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "b ".repeat(200),
            }]),
            ConversationMessage::tool_result("1", "bash", "ok ".repeat(200), false),
            ConversationMessage::assistant(vec![ContentBlock::Text {
                text: "recent".to_string(),
            }]),
        ],
    };

    let result = handle_slash_command(
        "/compact",
        &session,
        CompactionConfig {
            preserve_recent_messages: 2,
            max_estimated_tokens: 1,
        },
    )
    .expect("slash command should be handled");

    assert!(result.message.contains("Compacted 2 messages"));
    assert_eq!(result.session.messages[0].role, MessageRole::System);
}
```

**Verifies:**
- `/compact` command triggers session compaction
- Old messages are replaced with system summary
- Recent messages are preserved

### Test 4: `help_command_is_non_mutating()` (Lines 557-564)

```rust
#[test]
fn help_command_is_non_mutating() {
    let session = Session::new();
    let result = handle_slash_command("/help", &session, CompactionConfig::default())
        .expect("help command should be handled");
    assert_eq!(result.session, session);
    assert!(result.message.contains("Slash commands"));
}
```

**Verifies:**
- `/help` doesn't modify the session
- Help text is returned

### Test 5: `ignores_unknown_or_runtime_bound_slash_commands()` (Lines 566-621)

```rust
#[test]
fn ignores_unknown_or_runtime_bound_slash_commands() {
    let session = Session::new();
    assert!(handle_slash_command("/unknown", &session, CompactionConfig::default()).is_none());
    assert!(handle_slash_command("/status", &session, CompactionConfig::default()).is_none());
    // ... assertions for 18 commands that return None
}
```

**Verifies:**
- Unknown commands return None
- Runtime-bound commands (require I/O) return None from this crate

---

## Integration Points

### Upstream Dependencies

| Crate | Usage |
|-------|-------|
| `runtime` | `Session`, `compact_session()`, `CompactionConfig` |

### Downstream Dependents

| Crate | How it uses commands |
|-------|---------------------|
| `rusty-claude-cli` | REPL slash command handling |
| `compat-harness` | Command manifest extraction |
| `tools` | Some tools may invoke commands |

### Usage Pattern

```rust
// In rusty-claude-cli REPL loop
if input.starts_with('/') {
    match handle_slash_command(&input, &session, compaction_config) {
        Some(result) => {
            println!("{}", result.message);
            session = result.session;
        }
        None => {
            // Defer to runtime for I/O commands
            handle_runtime_command(&input, &mut session).await?;
        }
    }
}
```

---

## Summary

The commands crate is a **compact, focused module** with clear responsibilities:

| Component | Lines | Purpose |
|-----------|-------|---------|
| CommandRegistry types | 31 | Upstream manifest tracking |
| SlashCommandSpec | 134 | Command metadata (22 specs) |
| SlashCommand enum | 51 | Typed command variants |
| Parsing logic | 71 | `/command args` → enum |
| Help rendering | 33 | Formatted help output |
| Command handling | 49 | Execute state-modifying commands |
| Tests | 233 | Full coverage |

**Key design patterns:**

1. **Parse-handle separation** - Parsing is pure, handling may return None
2. **Spec-driven** - All command metadata in one array
3. **Graceful degradation** - Commands that need I/O return None
4. **Resume awareness** - 50% of commands support session resume

**Comparison: claw-code vs claw-code-latest**

The commands crate is **identical** in both repositories:
- Same 22 slash commands
- Same parsing logic
- Same handling behavior

This stability suggests the slash command interface is mature and complete.
