# Main Entry Points and CLI Flow

A deep-dive into how Claw Code starts up, parses arguments, and executes commands.

## Table of Contents

1. [Overview](#overview)
2. [Main Entry Point (`main.rs`)](#main-entry-point-mainrs)
3. [CLI Argument Parsing (`args.rs`)](#cli-argument-parsing-argsrs)
4. [Application State (`app.rs`)](#application-state-apprs)
5. [REPL Implementation (`input.rs`)](#repl-implementation-inputrs)
6. [Terminal Rendering (`render.rs`)](#terminal-rendering-renderrs)
7. [Initialization (`init.rs`)](#initialization-initrs)
8. [Command Flow Diagram](#command-flow-diagram)

---

## Overview

The Claw Code CLI is implemented in the `rusty-claude-cli` crate, which serves as the main binary entry point. The CLI supports multiple modes of operation:

| Mode | Command | Description |
|------|---------|-------------|
| **REPL** | `claw` | Interactive chat session with streaming |
| **Prompt** | `claw prompt "text"` | One-shot question/answer |
| **Login** | `claw login` | OAuth authentication flow |
| **Logout** | `claw logout` | Clear stored credentials |
| **Init** | `claw init` | Initialize repository config |
| **Resume** | `claw resume <session>` | Continue previous session |
| **System Prompt** | `claw system-prompt` | Dump constructed system prompt |
| **Dump Manifests** | `claw dump-manifests` | Extract TS command/tool manifests |
| **Bootstrap Plan** | `claw bootstrap-plan` | Generate bootstrap plan |

---

## Main Entry Point (`main.rs`)

**Location**: `rust/crates/rusty-claude-cli/src/main.rs`

### Complete Flow

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse CLI arguments
    let args = Args::parse();

    // 2. Handle login/logout separately (no API needed)
    match args.command.as_ref() {
        Some(Command::Login) => return oauth::login_flow(),
        Some(Command::Logout) => return oauth::logout(),
        _ => {}
    }

    // 3. Load configuration
    let config = ConfigLoader::load()?;

    // 4. Create API client
    let api_client = AnthropicClient::new(config.oauth_credentials()?)?;

    // 5. Create runtime with tool executor
    let tool_executor = StaticToolExecutor::new();
    let mut runtime = ConversationRuntime::new(api_client, tool_executor);

    // 6. Execute requested command
    match args.command {
        None => runtime.run_repl()?,           // Interactive mode
        Some(Command::Prompt { text }) => runtime.run_prompt(&text)?,
        Some(Command::Init) => init_repository()?,
        Some(Command::Resume { session }) => runtime.resume_session(&session)?,
        // ... other commands
    }

    Ok(())
}
```

### Key Responsibilities

1. **Argument Parsing**: Delegates to `args::Args::parse()`
2. **Auth Handling**: OAuth login/logout happen before config loading
3. **Configuration**: Loads and merges 5 config files via `ConfigLoader`
4. **API Client**: Creates authenticated Anthropic API client
5. **Tool Executor**: Creates static tool executor for built-in tools
6. **Runtime Creation**: Instantiates `ConversationRuntime` with client and executor
7. **Command Dispatch**: Routes to appropriate handler based on command

### Error Handling

The main function returns `Result<(), Box<dyn std::error::Error>>`, allowing any error to bubble up and be printed to stderr. This is appropriate for a CLI binary where detailed error reporting is sufficient.

---

## CLI Argument Parsing (`args.rs`)

**Location**: `rust/crates/rusty-claude-cli/src/args.rs`

### Argument Structure

```rust
#[derive(Parser, Debug, Clone)]
#[command(name = "claw")]
#[command(about = "CLAW: Command Line AI Worker")]
pub struct Args {
    /// Working directory
    #[arg(short, long)]
    pub dir: Option<PathBuf>,

    /// Permission mode
    #[arg(short = 'm', long, default_value = "workspace-write")]
    pub permission_mode: PermissionMode,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub output_format: OutputFormat,

    /// Subcommand
    #[command(subcommand)]
    pub command: Option<Command>,
}
```

### Permission Mode Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PermissionMode {
    /// Read-only access
    ReadOnly,
    /// Can write within workspace
    WorkspaceWrite,
    /// Full system access
    DangerFullAccess,
}
```

### Output Format Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Plain text output
    Text,
    /// JSON formatted output
    Json,
}
```

### Command Enum

```rust
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Start OAuth login flow
    Login,
    /// Clear stored credentials
    Logout,
    /// Initialize repository configuration
    Init,
    /// Send a single prompt and exit
    Prompt {
        /// The prompt text
        text: String,
    },
    /// Resume a previous session
    Resume {
        /// Session ID or path
        session: String,
    },
    /// Dump the system prompt
    SystemPrompt,
    /// Extract command/tool manifests from TS source
    DumpManifests,
    /// Generate bootstrap plan
    BootstrapPlan,
}
```

### clap Configuration

The CLI uses `clap` with the following features:

- **Derive macros**: `#[derive(Parser, Subcommand)]` for declarative parsing
- **Help messages**: Each argument and command has descriptive help text
- **Default values**: Sensible defaults for optional arguments
- **Value validation**: Enum variants restrict allowed values

---

## Application State (`app.rs`)

**Location**: `rust/crates/rusty-claude-cli/src/app.rs`

### SessionConfig

```rust
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub working_dir: PathBuf,
    pub permission_mode: PermissionMode,
    pub output_format: OutputFormat,
}
```

### SessionState

```rust
#[derive(Debug)]
pub struct SessionState {
    pub session: Session,
    pub total_cost: f64,
    pub total_tokens: TokenUsage,
}
```

### CliApp Structure

```rust
pub struct CliApp {
    pub config: SessionConfig,
    pub state: SessionState,
    pub renderer: TerminalRenderer,
    pub editor: LineEditor,
}
```

### Key Methods

#### `run_repl(&mut self)`

Main interactive loop:

```rust
pub fn run_repl(&mut self) -> Result<()> {
    self.print_welcome();

    loop {
        // 1. Read user input
        let input = self.editor.read_line()?;

        // 2. Check for exit commands
        if input.trim() == "exit" || input.trim() == "/quit" {
            break;
        }

        // 3. Handle slash commands
        if input.starts_with('/') {
            self.handle_slash_command(&input)?;
            continue;
        }

        // 4. Run conversation turn
        let response = self.runtime.run_turn(&input)?;

        // 5. Render response
        self.renderer.render_markdown(&response.content)?;

        // 6. Update token usage
        self.state.total_tokens += response.usage;
    }

    Ok(())
}
```

#### `run_turn(&mut self, input: &str)`

Executes a single conversation turn:

```rust
pub fn run_turn(&mut self, input: &str) -> Result<ConversationTurn> {
    // 1. Add user message to session
    self.state.session.add_user_message(input);

    // 2. Build system prompt
    let system_prompt = self.prompt_builder.build(&self.state.session);

    // 3. Call API
    let response = self.api_client.send_message(system_prompt, &self.state.session)?;

    // 4. Process tool calls if any
    for tool_call in response.tool_calls {
        let result = self.tool_executor.execute(&tool_call)?;
        self.state.session.add_tool_result(&tool_call.id, result);
    }

    // 5. Return final response
    Ok(ConversationTurn {
        content: response.content,
        usage: response.usage,
    })
}
```

---

## REPL Implementation (`input.rs`)

**Location**: `rust/crates/rusty-claude-cli/src/input.rs`

### LineEditor Structure

```rust
pub struct LineEditor {
    editor: Editor<LineEditorHelper>,
    history_path: PathBuf,
}

struct LineEditorHelper {
    completer: SlashCommandCompleter,
}
```

### Configuration

```rust
let mut editor = Editor::new()?;
editor.set_helper(Some(LineEditorHelper {
    completer: SlashCommandCompleter::new(),
}));

// Key bindings
editor.bind_sequence(
    KeyEvent(KeyCode::Char('j'), KeyModifiers::CONTROL),
    InputAction::Newline,  // Ctrl+J for newline
);
editor.bind_sequence(
    KeyEvent(KeyCode::Enter, KeyModifiers::SHIFT),
    InputAction::Newline,  // Shift+Enter for newline
);
```

### Slash Command Completion

```rust
pub struct SlashCommandCompleter {
    commands: Vec<&'static str>,
}

impl SlashCommandCompleter {
    pub fn new() -> Self {
        Self {
            commands: SLASH_COMMAND_SPECS.iter().map(|s| s.name).collect(),
        }
    }
}

impl Completer for SlashCommandCompleter {
    fn complete(&self, line: &str, pos: usize) -> Vec<(usize, String)> {
        // Find partial command
        let partial = &line[1..pos];  // Skip leading '/'

        // Filter matching commands
        self.commands
            .iter()
            .filter(|cmd| cmd.starts_with(partial))
            .map(|cmd| (1, cmd.to_string()))
            .collect()
    }
}
```

### Input Reading

```rust
pub fn read_line(&mut self) -> Result<String> {
    // Add to history if not empty
    if let Some(last) = self.history.last() {
        if !last.is_empty() {
            self.editor.add_history_entry(last)?;
        }
    }

    // Read line with completion
    let input = self.editor.readline("> ")?;

    // Handle multiline (Ctrl+J, Shift+Enter)
    let mut full_input = input;
    while self.needs_continuation(&full_input) {
        full_input.push('\n');
        let continuation = self.editor.readline("... ")?;
        full_input.push_str(&continuation);
    }

    Ok(full_input)
}
```

---

## Terminal Rendering (`render.rs`)

**Location**: `rust/crates/rusty-claude-cli/src/render.rs`

### TerminalRenderer

```rust
pub struct TerminalRenderer {
    stdout: Stdout,
    markdown_parser: pulldown_cmark::Parser,
    syntax_set: SyntaxSet,
    theme: Theme,
}
```

### Markdown to ANSI

```rust
pub fn markdown_to_ansi(&mut self, markdown: &str) -> Result<()> {
    // Parse markdown
    let events = pulldown_cmark::Parser::new(markdown);

    // Convert to ANSI
    for event in events {
        match event {
            Event::Start(tag) => self.handle_tag_start(tag)?,
            Event::End(tag) => self.handle_tag_end(tag)?,
            Event::Text(text) => self.write_text(&text)?,
            Event::Code(code) => self.write_code(&code)?,
            Event::Html(html) => self.write_html(&html)?,
            _ => {}
        }
    }

    Ok(())
}
```

### Syntax Highlighting

```rust
pub fn highlight_code(&self, code: &str, language: &str) -> String {
    // Find syntax definition
    let syntax = self.syntax_set
        .find_syntax_by_token(language)
        .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

    // Highlight with theme
    let highlighter = Highlighter::new(&self.theme);
    let ops = highlighter.highlight(code, syntax);

    // Convert to ANSI
    terminal_as_string(&ops, code)
}
```

### Spinner Animation

```rust
pub struct Spinner {
    frames: Vec<&'static str>,
    current: usize,
    message: String,
}

impl Spinner {
    const FRAMES: &'static [&'static str] = &[
        "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏",
    ];

    pub fn new(message: impl Into<String>) -> Self {
        Self {
            frames: Self::FRAMES.to_vec(),
            current: 0,
            message: message.into(),
        }
    }

    pub fn tick(&mut self) {
        self.current = (self.current + 1) % self.frames.len();
        print!("\r{} {}", self.frames[self.current], self.message);
        stdout().flush().unwrap();
    }
}
```

### MarkdownStreamState

For handling streaming markdown responses:

```rust
pub struct MarkdownStreamState {
    buffer: String,
    in_code_block: bool,
    code_fence: Option<String>,
    pending_text: String,
}

impl MarkdownStreamState {
    pub fn process_chunk(&mut self, chunk: &str) -> Vec<RenderEvent> {
        self.buffer.push_str(chunk);

        let mut events = Vec::new();

        // Handle code block boundaries
        if let Some(fence) = self.detect_code_fence(&self.buffer) {
            if self.in_code_block {
                // End code block
                self.in_code_block = false;
                events.push(RenderEvent::CodeBlockEnd);
            } else {
                // Start code block
                self.in_code_block = true;
                self.code_fence = Some(fence);
                events.push(RenderEvent::CodeBlockStart(fence));
            }
        }

        // Handle text events
        if !self.in_code_block {
            events.push(RenderEvent::Text(self.buffer.clone()));
            self.buffer.clear();
        }

        events
    }
}
```

---

## Initialization (`init.rs`)

**Location**: `rust/crates/rusty-claude-cli/src/init.rs`

### Repository Initialization

```rust
pub fn init_repository() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let claude_dir = cwd.join(".claude");

    // Create .claude/ directory
    if !claude_dir.exists() {
        std::fs::create_dir_all(&claude_dir)?;
        println!("Created .claude/ directory");
    }

    // Create .claude.json config
    let config_path = claude_dir.join("settings.json");
    if !config_path.exists() {
        let default_config = serde_json::json!({
            "permissionMode": "workspace-write",
            "model": {
                "maxSessionTurns": 100
            }
        });
        std::fs::write(&config_path, serde_json::to_string_pretty(&default_config)?)?;
        println!("Created .claude/settings.json");
    }

    // Update .gitignore
    let gitignore_path = cwd.join(".gitignore");
    let mut gitignore_content = String::new();
    if gitignore_path.exists() {
        gitignore_content = std::fs::read_to_string(&gitignore_path)?;
    }

    let claude_entries = [".claude.json", ".claude/commands/", ".claude/sessions/"];
    let mut modified = false;

    for entry in &claude_entries {
        if !gitignore_content.contains(entry) {
            gitignore_content.push_str(entry);
            gitignore_content.push('\n');
            modified = true;
        }
    }

    if modified {
        std::fs::write(&gitignore_path, gitignore_content)?;
        println!("Updated .gitignore");
    }

    // Create CLAUDE.md template
    let claude_md_path = cwd.join("CLAUDE.md");
    if !claude_md_path.exists() {
        let template = include_str!("../templates/CLAUDE.md.template");
        std::fs::write(&claude_md_path, template)?;
        println!("Created CLAUDE.md template");
    }

    Ok(())
}
```

### CLAUDE.md Template

```markdown
# Project Context

## Overview
[Project description]

## Architecture
[Key architectural decisions]

## Development
[Build and test instructions]

## Conventions
[Coding standards and patterns]
```

---

## Command Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Input                              │
│              (CLI args or REPL stdin)                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    args.rs - Argument Parser                    │
│                    (clap derive macros)                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      main.rs - Entry Point                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  1. Parse args                                           │   │
│  │  2. Handle login/logout (early exit)                     │   │
│  │  3. Load configuration (ConfigLoader)                    │   │
│  │  4. Create API client (AnthropicClient)                  │   │
│  │  5. Create tool executor (StaticToolExecutor)            │   │
│  │  6. Create runtime (ConversationRuntime)                 │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
              ▼                               ▼
┌──────────────────────────┐      ┌──────────────────────────┐
│    REPL Mode (None)      │      │   Prompt Mode            │
│  ┌────────────────────┐  │      │  ┌────────────────────┐  │
│  │ Loop:              │  │      │  │ Single turn        │  │
│  │  - Read input      │  │      │  │ - Add to session   │  │
│  │  - Handle slash    │  │      │  │ - Call API         │  │
│  │  - Run turn        │  │      │  │ - Execute tools    │  │
│  │  - Render response │  │      │  │ - Print response   │  │
│  │  - Update usage    │  │      │  └────────────────────┘  │
│  └────────────────────┘  │      └──────────────────────────┘
└──────────────────────────┘                  │
              │                               │
              └───────────────┬───────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│              app.rs - CliApp::run_turn()                        │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  1. Add user message to session                          │   │
│  │  2. Build system prompt (prompt.rs)                      │   │
│  │  3. Call API (api/client.rs)                             │   │
│  │  4. Process tool calls (tools crate)                     │   │
│  │  5. Update session with results                          │   │
│  │  6. Return response                                      │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  render.rs - TerminalRenderer                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  1. Parse markdown (pulldown-cmark)                      │   │
│  │  2. Highlight code (syntect)                             │   │
│  │  3. Convert to ANSI                                      │   │
│  │  4. Write to stdout                                      │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         Output                                  │
│              (ANSI-formatted text in terminal)                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Startup Sequence Timeline

```
T=0ms     User runs: claw prompt "hello"
           │
T=1ms     ┌─► main() enters
           │
T=2ms     ├─► Args::parse() - clap parses arguments
           │
T=5ms     ├─► ConfigLoader::load() - discover and merge configs
           │    └─► ~/.claude.json
           │    └─► .claude/settings.json
           │    └─► .claude/settings.local.json
           │    └─► CLAUDE.md (instructions)
           │
T=15ms    ├─► AnthropicClient::new() - OAuth token retrieval
           │
T=20ms    ├─► StaticToolExecutor::new() - register built-in tools
           │
T=25ms    ├─► ConversationRuntime::new() - create runtime
           │
T=30ms    └─► run_turn("hello")
                │
                ├─► PromptBuilder::build() - construct system prompt
                │
                ├─► api_client.send_message() - API call
                │
                ├─► StaticToolExecutor::execute() - if tool calls
                │
                └─► TerminalRenderer::render() - display response

T=500ms   Response displayed to user
```

---

## Environment Variables

The CLI checks these environment variables during startup:

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLAUDE_CODE_API_KEY` | Direct API key (bypasses OAuth) | None |
| `CLAUDE_CODE_UPSTREAM` | Path to TypeScript source for manifest extraction | Auto-discover |
| `CLAWD_CONFIG` | Override config file path | ~/.claude.json |
| `NO_COLOR` | Disable ANSI color output | false |
| `CLAWD_DEBUG` | Enable debug logging | false |

---

## Related Files

| File | Purpose |
|------|---------|
| `rust/crates/rusty-claude-cli/src/main.rs` | Main entry point |
| `rust/crates/rusty-claude-cli/src/args.rs` | CLI argument parsing |
| `rust/crates/rusty-claude-cli/src/app.rs` | Application state and REPL loop |
| `rust/crates/rusty-claude-cli/src/input.rs` | Line editor with completion |
| `rust/crates/rusty-claude-cli/src/render.rs` | Terminal rendering |
| `rust/crates/rusty-claude-cli/src/init.rs` | Repository initialization |
| `rust/crates/runtime/src/config.rs` | Configuration loading |
| `rust/crates/api/src/client.rs` | API client creation |
