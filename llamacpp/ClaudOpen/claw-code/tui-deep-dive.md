# Claw Code TUI Implementation Deep Dive

Analysis and implementation guide for the Terminal User Interface in rusty-claude-cli.

## Table of Contents

1. [Current TUI Architecture](#current-tui-architecture)
2. [TUI Enhancement Plan](#tui-enhancement-plan)
3. [Component Implementation Guide](#component-implementation-guide)
4. [Phase 0: Structural Cleanup](#phase-0-structural-cleanup)
5. [Phase 1: Status Bar & Live HUD](#phase-1-status-bar--live-hud)
6. [Phase 2: Enhanced Streaming](#phase-2-enhanced-streaming)
7. [Phase 3: Tool Visualization](#phase-3-tool-visualization)
8. [Phase 4-6: Advanced Features](#phase-4-6-advanced-features)

---

## Current TUI Architecture

### File Structure

```
rust/crates/rusty-claude-cli/src/
├── main.rs         # 3,159 lines - CLI entrypoint, REPL loop, rendering
├── input.rs        # 269 lines - rustyline line editor
├── render.rs       # 641 lines - Markdown rendering, spinner
└── init.rs         # Repository initialization
```

### Current Components

| Component | File | Lines | Quality |
|-----------|------|-------|---------|
| **Input (rustyline)** | input.rs | 269 | ✅ Solid |
| **Markdown Rendering** | render.rs | 641 | ✅ Good |
| **REPL Loop** | main.rs | 3,159 | ⚠️ Monolithic |
| **Tool Display** | main.rs | (embedded) | ✅ Functional |
| **Spinner** | render.rs | ~100 | ✅ Good |

### Dependencies

```toml
# TUI-related dependencies
crossterm = "0.28"        # Terminal control (cursor, colors, clear)
pulldown-cmark = "0.13"   # Markdown parsing
syntect = "5"             # Syntax highlighting
rustyline = "15"          # Line editing with completion
```

### Current Rendering Flow

```
User Input (rustyline)
       │
       ▼
Slash Command Parse
       │
       ▼
API Request → Stream Events
       │
       ▼
┌──────────────────────────────────────┐
│  TerminalRenderer                    │
│  ├─ render_markdown()                │
│  │   ├─ Parse pulldown-cmark events  │
│  │   ├─ Apply ColorTheme styling     │
│  │   └─ Handle tables, code, links   │
│  ├─ highlight_code()                 │
│  │   └─ syntect highlighting         │
│  └─ Spinner                          │
│      └─ Braille dot animation        │
└──────────────────────────────────────┘
       │
       ▼
crossterm::execute! / queue!
       │
       ▼
stdout (terminal)
```

### Strengths

1. **Clean rendering pipeline** - Markdown rendering is well-structured
2. **Syntax highlighting** - Uses syntect for code blocks
3. **Rich tool display** - Box-drawing borders, ✓/✗ icons
4. **Comprehensive tests** - Every formatting function tested
5. **Session management** - Full persistence and resume

### Weaknesses

1. **Monolithic main.rs** - 3,159 lines in single file
2. **No alternate-screen mode** - Everything is inline scrolling
3. **No progress bars** - Only spinner, no token progress
4. **No visual diff** - `/diff` dumps raw text
5. **No status bar** - Model, tokens not visible during interaction
6. **Artificial delay** - 8ms sleep per chunk in streaming
7. **No resize handling** - Terminal size not tracked
8. **No pager** - Long outputs overflow viewport
9. **No collapsible output** - Tool results flood screen
10. **No color themes** - Hardcoded ColorTheme::default()

---

## TUI Enhancement Plan

### Phase Overview

| Phase | Focus | Effort | Impact |
|-------|-------|--------|--------|
| **Phase 0** | Structural Cleanup | Medium | High (maintainability) |
| **Phase 1** | Status Bar & HUD | Medium | High (UX) |
| **Phase 2** | Enhanced Streaming | Large | High (UX) |
| **Phase 3** | Tool Visualization | Medium | High (UX) |
| **Phase 4** | Navigation & Commands | Medium | Medium (UX) |
| **Phase 5** | Color Themes | Medium | Low (polish) |
| **Phase 6** | Full-Screen Mode | XLarge | Medium (power users) |

---

## Phase 0: Structural Cleanup

### Goal

Break the 3,159-line monolith into focused, testable modules.

### Target Structure

```
rust/crates/rusty-claude-cli/src/
├── main.rs              # ~100 lines - Entrypoint, arg dispatch only
├── args.rs              # CLI argument parsing (consolidate parsers)
├── app.rs               # LiveCli struct, REPL loop, turn execution
├── format.rs            # Status/cost/model/permissions formatting
├── session_mgr.rs       # Session CRUD operations
├── init.rs              # Repository initialization (unchanged)
├── input.rs             # Line editor (minor extensions)
├── render.rs            # TerminalRenderer, Spinner (extended)
└── tui/
    ├── mod.rs           # TUI module root
    ├── status_bar.rs    # Persistent bottom status line
    ├── tool_panel.rs    # Tool call visualization
    ├── diff_view.rs     # Colored diff rendering
    ├── pager.rs         # Internal pager for long outputs
    └── theme.rs         # Color theme definitions
```

### Step 1: Extract LiveCli to app.rs

```rust
// New file: src/app.rs

use api::{AnthropicClient, AuthSource};
use commands::SlashCommand;
use runtime::{ConfigLoader, PermissionPolicy, Session};
use crate::render::TerminalRenderer;
use crate::input::LineEditor;

pub struct LiveCli {
    pub model: String,
    pub session: Session,
    pub client: AnthropicClient,
    pub permission_policy: PermissionPolicy,
    pub system_prompt: Vec<String>,
    pub config_loader: ConfigLoader,
    pub allowed_tools: Option<BTreeSet<String>>,
    pub renderer: TerminalRenderer,
}

impl LiveCli {
    pub fn new(
        model: String,
        enable_tools: bool,
        allowed_tools: Option<BTreeSet<String>>,
        permission_mode: PermissionMode,
    ) -> Result<Self, RuntimeError> {
        // Initialize all fields (moved from main.rs)
        Ok(Self { /* ... */ })
    }

    pub fn run_turn(&mut self, prompt: &str) -> Result<(), RuntimeError> {
        // Single turn execution (moved from main.rs)
    }

    pub fn run_turn_with_output(
        &mut self,
        prompt: &str,
        output_format: CliOutputFormat,
    ) -> Result<(), RuntimeError> {
        // Turn execution with formatted output
    }

    pub fn handle_slash_command(&mut self, input: &str) -> Result<(), RuntimeError> {
        // Parse and execute slash commands
    }

    pub fn stream_response(
        &mut self,
        events: Vec<AssistantEvent>,
    ) -> Result<(), io::Error> {
        // Stream and render API response
    }
}
```

### Step 2: Extract Formatting to format.rs

```rust
// New file: src/format.rs

use runtime::{Session, TokenUsage, UsageTracker};

/// Format session status report
pub fn format_session_status(
    session: &Session,
    usage: &UsageTracker,
    model: &str,
) -> String {
    let mut lines = vec![
        format!("Model: {model}"),
        format!("Messages: {}", session.messages.len()),
        format!("Input tokens: {}", usage.total().input_tokens),
        format!("Output tokens: {}", usage.total().output_tokens),
        format!("Total cost: {}", format_usd(usage.estimate_cost(model).total_cost)),
    ];
    lines.join("\n")
}

/// Format cost breakdown
pub fn format_cost_breakdown(usage: &UsageTracker, model: &str) -> String {
    let estimate = usage.estimate_cost(model);
    let pricing = pricing_for_model(model);

    let mut lines = vec![
        "Token Usage:".to_string(),
        format!("  Input:  {}", usage.total().input_tokens),
        format!("  Output: {}", usage.total().output_tokens),
        "".to_string(),
        "Cost Breakdown:".to_string(),
        format!("  Input:  {}", format_usd(estimate.input_cost)),
        format!("  Output: {}", format_usd(estimate.output_cost)),
        format!("  Total:  {}", format_usd(estimate.total_cost)),
        "".to_string(),
        "Pricing:".to_string(),
        format!("  Input:  {} per 1M tokens", format_usd(pricing.input_per_million)),
        format!("  Output: {} per 1M tokens", format_usd(pricing.output_per_million)),
    ];
    lines.join("\n")
}

/// Format model information
pub fn format_model_info(current_model: &str) -> String {
    let resolved = resolve_model_alias(current_model);
    let max_tokens = max_tokens_for_model(resolved);

    format!(
        "Current model: {} ({})\nMax output tokens: {}",
        current_model, resolved, max_tokens
    )
}

/// Format permission mode
pub fn format_permission_mode(mode: PermissionMode) -> String {
    let description = match mode {
        PermissionMode::ReadOnly => "Read-only (safe inspection)",
        PermissionMode::WorkspaceWrite => "Workspace write (file modifications)",
        PermissionMode::DangerFullAccess => "Danger: Full access (all tools)",
    };
    format!("Permission mode: {mode:?}\n{description}")
}

/// Format USD amounts
pub fn format_usd(cost: f64) -> String {
    format!("${:.4}", cost)
}
```

### Step 3: Extract Session Management to session_mgr.rs

```rust
// New file: src/session_mgr.rs

use runtime::Session;
use std::path::{Path, PathBuf};

pub struct SessionManager {
    sessions_dir: PathBuf,
}

impl SessionManager {
    pub fn new() -> Result<Self, SessionError> {
        let sessions_dir = get_sessions_directory()?;
        fs::create_dir_all(&sessions_dir)?;
        Ok(Self { sessions_dir })
    }

    pub fn create_session(&self) -> Result<Session, SessionError> {
        let session = Session::new();
        self.save_session(&session)?;
        Ok(session)
    }

    pub fn load_session(&self, session_id: &str) -> Result<Session, SessionError> {
        let path = self.sessions_dir.join(format!("{session_id}.json"));
        Session::load(&path)
    }

    pub fn save_session(&self, session: &Session) -> Result<(), SessionError> {
        let path = self.sessions_dir.join(format!("{}.json", session.id));
        session.save(&path)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>, SessionError> {
        let mut sessions = Vec::new();

        for entry in fs::read_dir(&self.sessions_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(session) = Session::load(&path) {
                    sessions.push(SessionInfo {
                        id: session.id.clone(),
                        created: session.created,
                        message_count: session.messages.len(),
                        path,
                    });
                }
            }
        }

        sessions.sort_by(|a, b| b.created.cmp(&a.created));
        Ok(sessions)
    }

    pub fn switch_session(&self, session_id: &str) -> Result<Session, SessionError> {
        self.load_session(session_id)
    }
}

pub struct SessionInfo {
    pub id: String,
    pub created: u64,
    pub message_count: usize,
    pub path: PathBuf,
}
```

### Step 4: Remove Legacy Code

The existing `app.rs` contains a `CliApp` struct that appears unused. Audit and either:
1. Merge unique features into `LiveCli`
2. Delete entirely

```rust
// Current legacy code in app.rs - TO BE REMOVED OR MERGED
pub struct CliApp {
    // ... unused fields
}

// Audit checklist:
// - Does CliApp have features LiveCli doesn't?
// - Is the stream event handler pattern different?
// - Does TerminalRenderer in app.rs differ from render.rs?
```

---

## Phase 1: Status Bar & Live HUD

### Goal

Add persistent information display during interaction.

### Implementation: status_bar.rs

```rust
// New file: src/tui/status_bar.rs

use crossterm::{
    cursor::{MoveTo, MoveToColumn},
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
    execute,
};
use std::io::{Write, stdout};

pub struct StatusBar {
    pub model: String,
    pub permission_mode: String,
    pub session_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub estimated_cost: f64,
    pub git_branch: Option<String>,
    pub turn_duration_secs: u64,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            model: String::new(),
            permission_mode: String::new(),
            session_id: String::new(),
            input_tokens: 0,
            output_tokens: 0,
            estimated_cost: 0.0,
            git_branch: None,
            turn_duration_secs: 0,
        }
    }

    pub fn render(&self) -> io::Result<()> {
        let mut stdout = stdout();

        // Get terminal size for full-width bar
        let (width, _height) = crossterm::terminal::size()?;

        // Move to bottom line
        let (_, height) = crossterm::terminal::size()?;
        execute!(stdout, MoveTo(0, height - 1))?;

        // Background
        execute!(
            stdout,
            SetBackgroundColor(Color::DarkGrey),
            Clear(ClearType::CurrentLine)
        )?;

        // Left section: Model and session
        let left = format!(
            " {} | {} | {}",
            self.model, self.permission_mode, self.session_id
        );
        execute!(
            stdout,
            SetForegroundColor(Color::White),
            Print(&left),
        )?;

        // Center section: Tokens
        let center = format!(
            " | In: {} | Out: {} ",
            self.input_tokens, self.output_tokens
        );
        let center_x = (width as usize).saturating_sub(center.len()) / 2;
        execute!(
            stdout,
            MoveToColumn(center_x as u16),
            SetForegroundColor(Color::Cyan),
            Print(&center),
        )?;

        // Right section: Cost and git branch
        let right = if let Some(branch) = &self.git_branch {
            format!(" ${:.4} | {} ", self.estimated_cost, branch)
        } else {
            format!(" ${:.4} ", self.estimated_cost)
        };
        let right_x = width.saturating_sub(right.len() as u16 + 1);
        execute!(
            stdout,
            MoveToColumn(right_x),
            SetForegroundColor(Color::Yellow),
            Print(&right),
        )?;

        // Reset
        execute!(stdout, ResetColor, MoveToColumn(0))?;

        Ok(())
    }

    pub fn update_tokens(&mut self, input: u32, output: u32) {
        self.input_tokens = input;
        self.output_tokens = output;
    }

    pub fn update_cost(&mut self, cost: f64) {
        self.estimated_cost = cost;
    }

    pub fn update_git_branch(&mut self, branch: Option<String>) {
        self.git_branch = branch;
    }
}

/// Parse git branch from git command
pub fn parse_git_branch() -> Option<String> {
    std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
}
```

### Integration with LiveCli

```rust
// In src/app.rs

use crate::tui::status_bar::StatusBar;

pub struct LiveCli {
    // ... existing fields
    status_bar: StatusBar,
}

impl LiveCli {
    pub fn run_turn(&mut self, prompt: &str) -> Result<(), RuntimeError> {
        let start = Instant::now();

        // Execute turn
        let summary = self.runtime.run_turn(prompt)?;

        // Update status bar
        self.status_bar.update_tokens(
            summary.usage.input_tokens,
            summary.usage.output_tokens,
        );
        self.status_bar.update_cost(
            self.usage_tracker.estimate_cost(&self.model).total_cost
        );
        self.status_bar.turn_duration_secs = start.elapsed().as_secs();

        // Re-render status bar
        self.status_bar.render()?;

        Ok(())
    }
}
```

---

## Phase 2: Enhanced Streaming

### Goal

Make the main response stream visually rich and responsive.

### Remove Artificial Delay

```rust
// Current (main.rs - TO REMOVE)
fn stream_markdown(text: &str, renderer: &mut TerminalRenderer) -> io::Result<()> {
    for chunk in text.split_whitespace() {
        renderer.render(chunk)?;
        std::thread::sleep(Duration::from_millis(8));  // REMOVE
    }
    Ok(())
}

// Optimized
fn stream_markdown(text: &str, renderer: &mut TerminalRenderer) -> io::Result<()> {
    renderer.render(text)?;  // Immediate rendering
    Ok(())
}
```

### Incremental Markdown Rendering

```rust
// Enhanced render.rs

pub struct IncrementalMarkdownState {
    buffer: String,
    last_rendered_paragraph: String,
    in_code_block: bool,
    in_heading: bool,
}

impl TerminalRenderer {
    pub fn render_markdown_incremental(
        &mut self,
        delta: &str,
        out: &mut impl Write,
    ) -> io::Result<()> {
        self.stream_state.buffer.push_str(delta);

        // Check for complete paragraphs (double newline)
        if let Some(paragraph_end) = self.stream_state.buffer.find("\n\n") {
            let paragraph = self.stream_state.buffer[..paragraph_end].to_string();
            self.stream_state.buffer = self.stream_state.buffer[paragraph_end + 2..].to_string();

            // Render complete paragraph
            self.render_complete_paragraph(&paragraph, out)?;
        }

        // Render remaining buffer (partial paragraph)
        if !self.stream_state.buffer.is_empty() {
            self.render_partial_paragraph(&self.stream_state.buffer, out)?;
        }

        Ok(())
    }

    fn render_complete_paragraph(&mut self, paragraph: &str, out: &mut impl Write) -> io::Result<()> {
        // Full markdown parsing for complete paragraphs
        let parser = Parser::new_ext(paragraph, Options::all());
        self.render_events(parser, out)
    }

    fn render_partial_paragraph(&mut self, text: &str, out: &mut impl Write) -> io::Result<()> {
        // Simple rendering without final formatting
        // Don't apply heading styles until paragraph is complete
        // Don't close list items until paragraph is complete
        write!(out, "{}", text)
    }
}
```

### Thinking/Reasoning Indicator

```rust
// New file: src/tui/thinking_indicator.rs

use crossterm::{
    style::{Color, Print, ResetColor, SetForegroundColor},
    execute,
};
use std::io::{Write, stdout};

pub struct ThinkingIndicator {
    frame_index: usize,
    is_reasoning: bool,
}

impl ThinkingIndicator {
    const THINKING_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    const REASONING_FRAMES: [&str; 8] = ["🧠", "💭", "🤔", "💡", "🤔", "💭", "🧠", "✨"];

    pub fn new(is_reasoning: bool) -> Self {
        Self {
            frame_index: 0,
            is_reasoning,
        }
    }

    pub fn tick(&mut self, label: &str) -> io::Result<()> {
        let mut stdout = stdout();

        let (frame, color) = if self.is_reasoning {
            (
                Self::REASONING_FRAMES[self.frame_index % Self::REASONING_FRAMES.len()],
                Color::Magenta,
            )
        } else {
            (
                Self::THINKING_FRAMES[self.frame_index % Self::THINKING_FRAMES.len()],
                Color::Blue,
            )
        };

        self.frame_index += 1;

        execute!(
            stdout,
            crossterm::cursor::SavePosition,
            crossterm::cursor::MoveToColumn(0),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
            SetForegroundColor(color),
            Print(format!("{frame} {label}")),
            ResetColor,
            crossterm::cursor::RestorePosition
        )?;

        stdout.flush()
    }

    pub fn finish(&mut self, label: &str) -> io::Result<()> {
        let mut stdout = stdout();
        self.frame_index = 0;

        execute!(
            stdout,
            crossterm::cursor::MoveToColumn(0),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
            SetForegroundColor(Color::Green),
            Print(format!("✔ {label}\n")),
            ResetColor
        )?;

        stdout.flush()
    }
}
```

---

## Phase 3: Tool Visualization

### Goal

Make tool execution legible and navigable.

### Collapsible Tool Output

```rust
// New file: src/tui/tool_panel.rs

use crossterm::{
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
    execute,
};
use std::io::{Write, stdout};

pub struct ToolOutputPanel {
    tool_name: String,
    success: bool,
    output_lines: Vec<String>,
    expanded: bool,
    max_preview_lines: usize,
}

impl ToolOutputPanel {
    pub fn new(tool_name: String, success: bool, output: String) -> Self {
        let lines: Vec<String> = output.lines().map(|s| s.to_string()).collect();
        Self {
            tool_name,
            success,
            output_lines: lines,
            expanded: false,
            max_preview_lines: 15,
        }
    }

    pub fn render(&self) -> io::Result<()> {
        let mut stdout = stdout();

        // Box drawing header
        let icon = if self.success { "✓" } else { "✗" };
        let color = if self.success { Color::Green } else { Color::Red };

        execute!(
            stdout,
            Print(format!("╭─ {icon} ")),
            SetForegroundColor(color),
            Print(&self.tool_name),
            ResetColor,
        )?;

        // Line count indicator
        if self.output_lines.len() > self.max_preview_lines {
            execute!(
                stdout,
                Print(format!(" ({} lines)", self.output_lines.len())),
            )?;
        }

        execute!(stdout, Print(" ─╮\n"))?;

        // Output content
        let lines_to_show = if self.expanded {
            self.output_lines.len()
        } else {
            self.output_lines.len().min(self.max_preview_lines)
        };

        for line in &self.output_lines[..lines_to_show] {
            execute!(stdout, Print(format!("│ {line}\n")))?;
        }

        // Truncation indicator
        if !self.expanded && self.output_lines.len() > self.max_preview_lines {
            execute!(
                stdout,
                Print(format!(
                    "│ ... ({} more lines hidden, use <expand> to show all)\n",
                    self.output_lines.len() - self.max_preview_lines
                )),
            )?;
        }

        execute!(stdout, Print("╰"))?;
        for _ in 0..40 {
            execute!(stdout, Print("─"))?;
        }
        execute!(stdout, Print("╯\n"))?;

        Ok(())
    }

    pub fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }
}

/// Tool call timeline summary
pub struct ToolTimeline {
    tools: Vec<ToolExecution>,
}

pub struct ToolExecution {
    pub name: String,
    pub success: bool,
    pub duration_ms: u64,
}

impl ToolTimeline {
    pub fn render(&self) -> io::Result<()> {
        let mut stdout = stdout();

        let total_duration: u64 = self.tools.iter().map(|t| t.duration_ms).sum();

        execute!(stdout, Print("Tool execution: "))?;

        for (i, tool) in self.tools.iter().enumerate() {
            let icon = if tool.success { "✓" } else { "✗" };
            let color = if tool.success { Color::Green } else { Color::Red };

            execute!(
                stdout,
                SetForegroundColor(color),
                Print(format!("{icon} {}", tool.name)),
                ResetColor,
            )?;

            if i < self.tools.len() - 1 {
                execute!(stdout, Print(" → "))?;
            }
        }

        execute!(
            stdout,
            Print(format!(" ({} tools, {:.1}s)\n", self.tools.len(), total_duration as f64 / 1000.0)),
        )?;

        Ok(())
    }
}
```

### Diff-Aware edit_file Display

```rust
// New file: src/tui/diff_view.rs

use crossterm::{
    style::{Color, Print, ResetColor, SetForegroundColor},
    execute,
};
use std::io::{Write, stdout};

/// Render unified diff with colors
pub fn render_unified_diff(diff: &str) -> io::Result<()> {
    let mut stdout = stdout();

    for line in diff.lines() {
        match line.chars().next() {
            Some('+') if !line.starts_with("+++") => {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Green),
                    Print(line),
                    ResetColor,
                    Print("\n")
                )?;
            }
            Some('-') if !line.starts_with("---") => {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Red),
                    Print(line),
                    ResetColor,
                    Print("\n")
                )?;
            }
            Some('@') => {
                execute!(
                    stdout,
                    SetForegroundColor(Color::Cyan),
                    Print(line),
                    ResetColor,
                    Print("\n")
                )?;
            }
            _ => {
                execute!(stdout, Print(line), Print("\n"))?;
            }
        }
    }

    Ok(())
}

/// Format edit_file result with diff
pub fn format_edit_result(
    path: &str,
    old_string: &str,
    new_string: &str,
) -> String {
    // Generate unified diff
    let diff = generate_unified_diff(path, old_string, new_string);

    let mut output = String::new();
    output.push_str(&format!("✓ edit_file: {path}\n\n"));
    output.push_str(&diff);

    output
}
```

---

## Phase 4-6: Advanced Features

### Phase 4: Enhanced Navigation

#### Colored /diff Output

```rust
// In src/format.rs or src/tui/diff_view.rs

pub fn format_git_diff(dump: &str) -> String {
    // Parse raw git diff output
    // Apply ANSI color codes for +, -, @ lines
    // Return colored string
}
```

#### Pager for Long Outputs

```rust
// New file: src/tui/pager.rs

pub struct Pager {
    lines: Vec<String>,
    viewport_start: usize,
    viewport_size: usize,
}

impl Pager {
    pub fn new(lines: Vec<String>) -> Self {
        let (_, height) = crossterm::terminal::size().unwrap_or((80, 24));
        Self {
            lines,
            viewport_start: 0,
            viewport_size: height.saturating_sub(2) as usize, // Reserve status bar
        }
    }

    pub fn render(&self) -> io::Result<()> {
        let viewport_end = (self.viewport_start + self.viewport_size)
            .min(self.lines.len());

        for (i, line) in self.lines[self.viewport_start..viewport_end].iter().enumerate() {
            crossterm::execute!(std::io::stdout(), crossterm::cursor::MoveTo(0, i as u16))?;
            print!("{line}");
        }

        // Page indicator
        let total_pages = (self.lines.len() + self.viewport_size - 1) / self.viewport_size;
        let current_page = self.viewport_start / self.viewport_size + 1;
        println!("\nPage {current_page}/{total_pages} (j/k: scroll, q: quit)");

        Ok(())
    }

    pub fn scroll_down(&mut self) {
        if self.viewport_start + self.viewport_size < self.lines.len() {
            self.viewport_start += self.viewport_size;
        }
    }

    pub fn scroll_up(&mut self) {
        self.viewport_start = self.viewport_start.saturating_sub(self.viewport_size);
    }
}
```

### Phase 5: Color Themes

```rust
// New file: src/tui/theme.rs

#[derive(Debug, Clone, Copy)]
pub struct ColorTheme {
    pub heading: Color,
    pub emphasis: Color,
    pub strong: Color,
    pub inline_code: Color,
    pub link: Color,
    pub quote: Color,
    pub table_border: Color,
    pub code_block_border: Color,
    pub spinner_active: Color,
    pub spinner_done: Color,
    pub spinner_failed: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
}

impl ColorTheme {
    pub fn dark() -> Self {
        Self {
            heading: Color::Cyan,
            emphasis: Color::Magenta,
            strong: Color::Yellow,
            inline_code: Color::Green,
            link: Color::Blue,
            quote: Color::DarkGrey,
            table_border: Color::DarkCyan,
            code_block_border: Color::DarkGrey,
            spinner_active: Color::Blue,
            spinner_done: Color::Green,
            spinner_failed: Color::Red,
            status_bar_bg: Color::DarkGrey,
            status_bar_fg: Color::White,
        }
    }

    pub fn light() -> Self {
        Self {
            heading: Color::DarkBlue,
            emphasis: Color::DarkMagenta,
            strong: Color::DarkYellow,
            inline_code: Color::DarkGreen,
            link: Color::DarkBlue,
            quote: Color::DarkGrey,
            table_border: Color::DarkCyan,
            code_block_border: Color::DarkGrey,
            spinner_active: Color::Blue,
            spinner_done: Color::Green,
            spinner_failed: Color::Red,
            status_bar_bg: Color::White,
            status_bar_fg: Color::Black,
        }
    }

    pub fn catppuccin() -> Self {
        // Catppuccin theme colors (RGB)
        Self {
            heading: Color::Rgb { r: 137, g: 180, b: 250 }, // Blue
            emphasis: Color::Rgb { r: 245, g: 189, b: 47 }, // Yellow
            strong: Color::Rgb { r: 245, g: 189, b: 47 },
            inline_code: Color::Rgb { r: 166, g: 227, b: 161 }, // Green
            link: Color::Rgb { r: 137, g: 180, b: 250 },
            quote: Color::Rgb { r: 166, g: 173, b: 247 }, // Mauve
            table_border: Color::Rgb { r: 148, g: 153, b: 165 }, // Overlay1
            code_block_border: Color::Rgb { r: 166, g: 173, b: 247 },
            spinner_active: Color::Rgb { r: 137, g: 180, b: 250 },
            spinner_done: Color::Rgb { r: 166, g: 227, b: 161 },
            spinner_failed: Color::Rgb { r: 243, g: 139, b: 168 }, // Red
            status_bar_bg: Color::Rgb { r: 48, g: 49, b: 66 }, // Surface0
            status_bar_fg: Color::Rgb { r: 205, g: 214, b: 244 }, // Text
        }
    }
}
```

### Phase 6: Full-Screen TUI Mode (ratatui)

```rust
// Optional feature gate
# Cargo.toml
[features]
full-tui = ["ratatui"]

[dependencies]
ratatui = { version = "0.26", optional = true }

// New file: src/tui/fullscreen.rs

#[cfg(feature = "full-tui")]
mod fullscreen {
    use ratatui::{
        backend::CrosstermBackend,
        Terminal,
        widgets::{Block, Borders, Paragraph},
        Frame,
    };

    pub struct FullScreenApp {
        conversation: Vec<String>,
        input: String,
        scroll_offset: usize,
    }

    impl FullScreenApp {
        pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
            // Setup terminal
            crossterm::terminal::enable_raw_mode()?;
            let mut stdout = std::io::stdout();
            crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
            let backend = CrosstermBackend::new(stdout);
            let mut terminal = Terminal::new(backend)?;

            // Main loop
            loop {
                terminal.draw(|f| self.ui(f))?;

                // Handle input
                // ...
            }

            // Restore terminal
            crossterm::terminal::disable_raw_mode()?;
            crossterm::execute!(
                terminal.backend_mut(),
                crossterm::terminal::LeaveAlternateScreen
            )?;

            Ok(())
        }

        fn ui(&mut self, f: &mut Frame) {
            // Split screen: conversation view + input
            // Right sidebar: tool status, todos
            // Bottom: status bar
        }
    }
}
```

---

## Testing TUI Components

```rust
// Tests should not require actual terminal

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn status_bar_renders_without_error() {
        let status = StatusBar {
            model: "claude-sonnet-4-6".to_string(),
            permission_mode: "workspace-write".to_string(),
            session_id: "abc123".to_string(),
            input_tokens: 1000,
            output_tokens: 500,
            estimated_cost: 0.0123,
            git_branch: Some("main".to_string()),
            turn_duration_secs: 5,
        };

        // Render to buffer (not actual stdout)
        // Verify no panic, correct structure
    }

    #[test]
    fn tool_panel_collapses_long_output() {
        let output = "line\n".repeat(100);
        let panel = ToolOutputPanel::new(
            "bash".to_string(),
            true,
            output,
        );

        assert!(!panel.expanded);
        // Verify preview shows max_preview_lines
    }

    #[test]
    fn diff_view_colors_additions_and_removals() {
        let diff = r#"--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,3 @@
 line1
-removed
+added
 line3
"#;

        let mut output = Cursor::new(Vec::new());
        render_unified_diff(diff, &mut output).unwrap();

        // Verify ANSI codes present for + and - lines
    }
}
```

---

*Last updated: 2026-04-02*
*Based on TUI-ENHANCEMENT-PLAN.md revision*
