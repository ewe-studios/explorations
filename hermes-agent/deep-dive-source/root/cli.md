# cli.py Deep Dive Exploration

**Source File:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/cli.py`

**Size:** 8,554 lines

**Purpose:** Interactive Terminal Interface for Hermes Agent - a Claude Code-inspired CLI for AI agent interactions

---

## Table of Contents

1. [Module Overview](#module-overview)
2. [Architecture & Design Patterns](#architecture--design-patterns)
3. [Key Classes](#key-classes)
4. [Core Functions](#core-functions)
5. [Line-by-Line Critical Analysis](#line-by-line-critical-analysis)
6. [Data Flow Diagrams](#data-flow-diagrams)
7. [Integration Points](#integration-points)
8. [Configuration System](#configuration-system)
9. [Interactive TUI System](#interactive-tui-system)
10. [Voice Mode System](#voice-mode-system)
11. [Session Management](#session-management)
12. [Security Features](#security-features)

---

## Module Overview

### Purpose

The `cli.py` module provides a **rich interactive terminal interface** for the Hermes Agent framework. It serves as the primary user-facing entry point for CLI-based agent interactions, featuring:

- **prompt_toolkit-based TUI** with persistent input area at bottom of terminal
- **Streaming token display** with real-time response rendering
- **Multi-modal input** supporting text, file paths, clipboard images, and voice
- **Slash command system** for session/tool/model management
- **Git worktree isolation** for parallel agent sessions
- **SQLite session persistence** with resume/branch capabilities
- **Voice mode** with STT/TTS integration
- **Dangerous command approval** workflows

### Key Design Goals

1. **Claude Code-inspired UX**: Familiar interface patterns for users migrating from Anthropic's CLI
2. **Non-blocking I/O**: Agent runs in background thread while UI remains responsive
3. **Interrupt-driven**: User can interrupt agent mid-execution with new input
4. **Extensible**: Plugin system, skill commands, MCP server support
5. **Production-ready**: Logging, error handling, resource cleanup

---

## Architecture & Design Patterns

### Threading Model

```
┌─────────────────────────────────────────────────────────────────┐
│                    Main Thread (prompt_toolkit)                  │
│  - Event loop for UI rendering                                   │
│  - Key binding handlers                                          │
│  - Input area management                                         │
└─────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              │               │               │
              ▼               ▼               ▼
┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐
│  process_loop()  │ │   agent_thread   │ │   spinner_thread │
│  - Polls input   │ │   - run_conversa │   - UI refresh     │
│  - Dispatches    │ │     tion()       │                     │
│    commands      │ │   - Tool calls   │                     │
│                  │ │   - LLM API      │                     │
└──────────────────┘ └──────────────────┘ └──────────────────┘
```

### Key Patterns

1. **Callback Registry**: Tools register callbacks for sudo/approval/secret prompts
2. **Queue-based Communication**: `_pending_input` and `_interrupt_queue` for thread-safe messaging
3. **Conditional Widgets**: prompt_toolkit `ConditionalContainer` for dynamic UI panels
4. **Atomic State Guards**: Thread locks for voice mode state management
5. **Lazy Initialization**: Agent, SessionDB, and auxiliary clients created on first use

---

## Key Classes

### HermesCLI (lines 1190-8336)

The main CLI class managing the entire interactive session.

#### Constructor Signature

```python
def __init__(
    self,
    model: str = None,
    toolsets: List[str] = None,
    provider: str = None,
    api_key: str = None,
    base_url: str = None,
    max_turns: int = None,
    verbose: bool = False,
    compact: bool = False,
    resume: str = None,
    checkpoints: bool = False,
    pass_session_id: bool = False,
)
```

#### Core Instance Attributes

| Attribute | Type | Purpose |
|-----------|------|---------|
| `agent` | `Optional[AIAgent]` | The underlying agent instance |
| `conversation_history` | `List[Dict[str, Any]]` | Message history for current session |
| `session_id` | `str` | Unique session identifier (timestamp_uuid format) |
| `_pending_input` | `queue.Queue` | Queue for user messages awaiting processing |
| `_interrupt_queue` | `queue.Queue` | Queue for messages typed while agent is running |
| `_agent_running` | `bool` | Flag indicating if agent is actively processing |
| `_clarify_state` | `Optional[Dict]` | State for clarify tool questions |
| `_approval_state` | `Optional[Dict]` | State for dangerous command approval |
| `_sudo_state` | `Optional[Dict]` | State for sudo password prompt |
| `_voice_mode` | `bool` | Voice input mode enabled |
| `_stream_buf` | `str` | Buffer for streaming token output |

#### Critical Methods

##### `run()` - Main Event Loop (lines 6921-8336)

```python
def run(self):
    """Run the interactive CLI loop with persistent input at bottom."""
```

**Flow:**
1. Push TUI to bottom of terminal with padding newlines
2. Display welcome banner
3. Initialize prompt_toolkit Application with custom layout
4. Start `process_loop()` thread for message handling
5. Start `spinner_thread()` for UI refresh
6. Run `app.run()` blocking until exit

**Key Bindings Registered:**

```python
@kb.add('enter')
def handle_enter(event):
    # Routes input to correct queue based on state:
    # - Sudo/secret/approval/clarify prompts → response queues
    # - Agent running → _interrupt_queue
    # - Agent idle → _pending_input

@kb.add('c-c')
def handle_ctrl_c(event):
    # Priority: voice recording → cancel prompts → interrupt agent → exit

@kb.add('tab', eager=True)
def handle_tab(event):
    # Accept completion → Accept suggestion → Start completions
```

##### `chat()` - Send Message to Agent (lines 6281-6711)

```python
def chat(self, message, images: list = None) -> Optional[str]:
    """Send a message to the agent and get a response."""
```

**Processing Steps:**

1. **Credential Resolution** (line 6305):
   ```python
   if not self._ensure_runtime_credentials():
       return None
   ```

2. **Model Routing** (line 6308):
   ```python
   turn_route = self._resolve_turn_agent_config(message)
   ```

3. **Image Preprocessing** (lines 6325-6328):
   ```python
   if images:
       message = self._preprocess_images_with_vision(message, images)
   ```

4. **Context Reference Expansion** (lines 6331-6350):
   ```python
   if "@" in message:
       message = preprocess_context_references(message, ...)
   ```

5. **UTF-8 Sanitization** (lines 6355-6357):
   ```python
   message = _sanitize_surrogates(message)  # Fix clipboard surrogate crashes
   ```

6. **Agent Execution** (lines 6444-6471):
   ```python
   def run_agent():
       result = self.agent.run_conversation(
           user_message=agent_message,
           conversation_history=self.conversation_history[:-1],
           stream_callback=stream_callback,
           task_id=self.session_id,
       )
   ```

7. **Interrupt Monitoring** (lines 6483-6510):
   ```python
   while agent_thread.is_alive():
       interrupt_msg = self._interrupt_queue.get(timeout=0.1)
       if interrupt_msg:
           self.agent.interrupt(interrupt_msg)
   ```

##### `process_command()` - Slash Command Handler (lines 4280-4611)

Dispatches slash commands to appropriate handlers:

```python
def process_command(self, command: str) -> bool:
    """Process a slash command. Returns True to continue, False to exit."""
```

**Command Categories:**

| Command | Handler | Purpose |
|---------|---------|---------|
| `/quit`, `/exit`, `/q` | Inline | Exit CLI |
| `/help` | `show_help()` | Display command reference |
| `/model` | `_handle_model_switch()` | Switch model/provider |
| `/tools` | `_handle_tools_command()` | Enable/disable tools |
| `/skills` | `_handle_skills_command()` | Manage skill plugins |
| `/cron` | `_handle_cron_command()` | Schedule tasks |
| `/rollback` | `_handle_rollback_command()` | Restore checkpoints |
| `/voice` | `_handle_voice_command()` | Voice mode control |
| `/skin` | `_handle_skin_command()` | UI theme selection |
| `/resume` | `_handle_resume_command()` | Restore session |
| `/branch` | `_handle_branch_command()` | Fork session |

##### `_init_agent()` - Agent Initialization (lines 2232-2368)

```python
def _init_agent(self, *, model_override=None, runtime_override=None, route_label=None) -> bool:
    """Initialize the agent on first use. Restores conversation history when resuming."""
```

**Key Steps:**

1. Ensure runtime credentials resolved
2. Initialize SQLite SessionDB if needed
3. Load resumed session history from database
4. Create AIAgent with configuration:
   ```python
   self.agent = AIAgent(
       model=effective_model,
       api_key=runtime.get("api_key"),
       base_url=runtime.get("base_url"),
       provider=runtime.get("provider"),
       max_iterations=self.max_turns,
       enabled_toolsets=self.enabled_toolsets,
       session_id=self.session_id,
       session_db=self._session_db,
       clarify_callback=self._clarify_callback,
       reasoning_callback=self._current_reasoning_callback(),
       stream_delta_callback=self._stream_delta if self.streaming_enabled else None,
       ...
   )
   ```

---

### ChatConsole (lines 973-1000)

Rich Console adapter for prompt_toolkit's patch_stdout context.

```python
class ChatConsole:
    """Rich Console adapter for prompt_toolkit's patch_stdout context.
    
    Captures Rich's rendered ANSI output and routes it through _cprint
    so colors and markup render correctly inside the interactive chat loop.
    """
    
    def __init__(self):
        from io import StringIO
        self._buffer = StringIO()
        self._inner = Console(
            file=self._buffer,
            force_terminal=True,
            color_system="truecolor",
            highlight=False,
        )
    
    def print(self, *args, **kwargs):
        self._buffer.seek(0)
        self._buffer.truncate()
        self._inner.width = shutil.get_terminal_size().columns
        self._inner.print(*args, **kwargs)
        output = self._buffer.getvalue()
        for line in output.rstrip("\n").split("\n"):
            _cprint(line)
```

**Purpose:** Bridge Rich markup to prompt_toolkit's ANSI renderer, preserving colors while avoiding garbled output through `patch_stdout`'s `StdoutProxy`.

---

## Core Functions

### Configuration Loading

#### `load_cli_config()` (lines 180-508)

```python
def load_cli_config() -> Dict[str, Any]:
    """Load CLI configuration from config files.
    
    Config lookup order:
    1. ~/.hermes/config.yaml (user config - preferred)
    2. ./cli-config.yaml (project config - fallback)
    
    Environment variables take precedence over config file values.
    """
```

**Configuration Structure:**

```yaml
model:
  default: "anthropic/claude-opus-4-20250514"
  provider: "auto"
  base_url: ""

terminal:
  env_type: "local"
  cwd: "."
  timeout: 60
  docker_image: "nikolaik/python-nodejs:python3.11-nodejs20"

browser:
  inactivity_timeout: 120
  record_sessions: False

compression:
  enabled: True
  threshold: 0.50
  summary_model: ""

agent:
  max_turns: 90
  system_prompt: ""
  reasoning_effort: ""
  personalities:
    helpful: "You are a helpful, friendly AI assistant."
    concise: "You are a concise assistant..."
    ...

display:
  compact: False
  resume_display: "full"
  show_reasoning: False
  streaming: True
  tool_progress: "all"
  skin: "default"
```

#### `_load_prefill_messages()` (lines 85-111)

```python
def _load_prefill_messages(file_path: str) -> List[Dict[str, Any]]:
    """Load ephemeral prefill messages from a JSON file.
    
    The file should contain a JSON array of {role, content} dicts:
        [{"role": "user", "content": "Hi"}, {"role": "assistant", "content": "Hello!"}]
    
    Relative paths are resolved from ~/.hermes/.
    """
```

**Use Case:** Few-shot priming for specialized agent behaviors without persisting to conversation history.

---

### Git Worktree Isolation

#### `_setup_worktree()` (lines 656-752)

```python
def _setup_worktree(repo_root: str = None) -> Optional[Dict[str, str]]:
    """Create an isolated git worktree for this CLI session.
    
    Returns: dict with worktree metadata {path, branch, repo_root}
    """
```

**Process:**

1. Generate unique worktree name: `hermes-<8-char-uuid>`
2. Create branch: `hermes/hermes-<uuid>`
3. Create worktree at `.worktrees/<name>/`
4. Copy files from `.worktreeinclude` (gitignored files agent needs)
5. Symlink directories to save disk space

**Security Check** (lines 720-731):
```python
# Prevent path traversal and symlink escapes
src_resolved = src.resolve(strict=False)
dst_resolved = dst.resolve(strict=False)
if not _path_is_within_root(src_resolved, repo_root_resolved):
    logger.warning("Skipping .worktreeinclude entry outside repo root: %s", entry)
    continue
```

#### `_cleanup_worktree()` (lines 755-809)

```python
def _cleanup_worktree(info: Dict[str, str] = None) -> None:
    """Remove a worktree and its branch on exit.
    
    If the worktree has uncommitted changes, warn and keep it.
    """
```

**Safety Check** (lines 775-788):
```python
status = subprocess.run(
    ["git", "status", "--porcelain"],
    capture_output=True, text=True, timeout=10, cwd=wt_path,
)
has_changes = bool(status.stdout.strip())

if has_changes:
    print(f"\n⚠ Worktree has uncommitted changes, keeping: {wt_path}")
    return  # Don't delete!
```

---

### File Path Detection

#### `_detect_file_drop()` (lines 926-970)

```python
def _detect_file_drop(user_input: str) -> "dict | None":
    """Detect if user_input is a dragged/pasted file path, not a slash command.
    
    When a user drags a file into the terminal, macOS pastes the absolute path
    (e.g. `/Users/roland/Desktop/file.png`) which starts with `/` and would
    otherwise be mistaken for a slash command.
    
    Returns:
        {"path": Path, "is_image": bool, "remainder": str}
    """
```

**Algorithm:**

1. Check if input starts with `/`
2. Parse first token handling escaped spaces (`\ `)
3. Verify path exists and is a file
4. Check if image extension for auto-attachment
5. Return remainder text after path

**Example:**
```python
# Input: "/Users/foo/code.py:45-46 can you fix this?"
# Output: {"path": Path("/Users/foo/code.py"), "is_image": False, "remainder": ":45-46 can you fix this?"}
```

---

### Streaming Display System

#### `_stream_delta()` (lines 1923-2007)

```python
def _stream_delta(self, text: str) -> None:
    """Line-buffered streaming callback for real-time token rendering.
    
    Receives text deltas from the agent as tokens arrive. Buffers
    partial lines and emits complete lines via _cprint.
    
    Reasoning/thinking blocks are suppressed during streaming since
    they'd display raw XML tags.
    """
```

**Tag Suppression Logic:**

```python
_OPEN_TAGS = ("<REASONING_SCRATCHPAD>", "<think>", "<reasoning>", "<THINKING>", "<thinking>")
_CLOSE_TAGS = ("</REASONING_SCRATCHPAD>", "</think>", "</reasoning>", "</THINKING>", "</thinking>")

# Append to pre-filter buffer
self._stream_prefilt = getattr(self, "_stream_prefilt", "") + text

# Check for open tag
if not getattr(self, "_in_reasoning_block", False):
    for tag in _OPEN_TAGS:
        idx = self._stream_prefilt.find(tag)
        if idx != -1:
            before = self._stream_prefilt[:idx]
            if before:
                self._emit_stream_text(before)
            self._in_reasoning_block = True
            return
```

**When `show_reasoning` is enabled** (lines 1997-2006):
```python
if self.show_reasoning:
    inner = self._stream_prefilt[:idx]
    if inner:
        self._stream_reasoning_delta(inner)
```

#### `_stream_reasoning_delta()` (lines 1874-1909)

```python
def _stream_reasoning_delta(self, text: str) -> None:
    """Stream reasoning/thinking tokens into a dim box above the response."""
```

**Box Rendering:**
```python
if not getattr(self, "_reasoning_box_opened", False):
    self._reasoning_box_opened = True
    w = shutil.get_terminal_size().columns
    r_label = " Reasoning "
    r_fill = w - 2 - len(r_label)
    _cprint(f"\n{_DIM}┌─{r_label}{'─' * max(r_fill - 1, 0)}┐{_RST}")

# Stream line by line
while "\n" in self._reasoning_buf:
    line, self._reasoning_buf = self._reasoning_buf.split("\n", 1)
    _cprint(f"{_DIM}{line}{_RST}")
```

---

### Clarify Tool Integration

#### `_clarify_callback()` (lines 5977-6042)

```python
def _clarify_callback(self, question, choices) -> str:
    """Platform callback for the clarify tool.
    
    Sets up interactive selection UI, blocks until user responds.
    Times out after configured period (default 120s).
    """
```

**State Setup:**
```python
timeout = CLI_CONFIG.get("clarify", {}).get("timeout", 120)
response_queue = queue.Queue()
is_open_ended = not choices or len(choices) == 0

self._clarify_state = {
    "question": question,
    "choices": choices if not is_open_ended else [],
    "selected": 0,
    "response_queue": response_queue,
}
self._clarify_deadline = _time.monotonic() + timeout
self._clarify_freetext = is_open_ended
```

**Polling Loop:**
```python
_last_countdown_refresh = _time.monotonic()
while True:
    try:
        result = response_queue.get(timeout=1)
        self._clarify_deadline = 0
        return result
    except queue.Empty:
        remaining = self._clarify_deadline - _time.monotonic()
        if remaining <= 0:
            break
        # Throttle repaint to avoid flicker
        now = _time.monotonic()
        if now - _last_countdown_refresh >= 5.0:
            _last_countdown_refresh = now
            self._invalidate()
```

**Timeout Behavior:**
```python
self._invalidate()
_cprint(f"\n{_DIM}(clarify timed out after {timeout}s — agent will decide){_RST}")
return "The user did not provide a response within the time limit. Use your best judgement."
```

---

### Dangerous Command Approval

#### `_approval_callback()` (lines 6087-6140)

```python
def _approval_callback(self, command: str, description: str,
                       *, allow_permanent: bool = True) -> str:
    """Prompt for dangerous command approval through prompt_toolkit UI.
    
    Choices: once / session / always / deny
    When allow_permanent=False (tirith warnings), 'always' is hidden.
    """
```

**Lock for Serialization** (lines 6102-6104):
```python
with self._approval_lock:
    # Only one approval prompt at a time
    # Prevents race condition from parallel delegation subtasks
```

**Choice Determination** (lines 6142-6147):
```python
def _approval_choices(self, command: str, *, allow_permanent: bool = True) -> list[str]:
    choices = ["once", "session", "always", "deny"] if allow_permanent else ["once", "session", "deny"]
    if len(command) > 70:
        choices.append("view")  # Expand long commands before deciding
    return choices
```

---

### Voice Mode System

#### `_voice_start_recording()` (lines 5603-5680)

```python
def _voice_start_recording(self):
    """Start capturing audio from the microphone."""
```

**Requirements Check:**
```python
reqs = check_voice_requirements()
if not reqs["audio_available"]:
    raise RuntimeError(
        "Voice mode requires sounddevice and numpy.\n"
        "Install with: pip install sounddevice numpy"
    )
if not reqs.get("stt_available", reqs.get("stt_key_set")):
    raise RuntimeError(
        "Voice mode requires an STT provider for transcription.\n"
        "Option 1: pip install faster-whisper  (free, local)\n"
        "Option 2: Set GROQ_API_KEY (free tier)\n"
        "Option 3: Set VOICE_TOOLS_OPENAI_KEY (paid)"
    )
```

**Silence Detection Config** (lines 5641-5643):
```python
self._voice_recorder._silence_threshold = voice_cfg.get("silence_threshold", 200)
self._voice_recorder._silence_duration = voice_cfg.get("silence_duration", 3.0)
```

**Auto-stop on Silence** (lines 5645-5653):
```python
def _on_silence():
    with self._voice_lock:
        if not self._voice_recording:
            return
    _cprint(f"\n{_DIM}Silence detected, auto-stopping...{_RST}")
    self._voice_stop_and_transcribe()

self._voice_recorder.start(on_silence_stop=_on_silence)
```

#### `_voice_stop_and_transcribe()` (lines 5682-5776)

```python
def _voice_stop_and_transcribe(self):
    """Stop recording, transcribe via STT, and queue transcript as input."""
```

**Atomic Guard** (lines 5687-5691):
```python
with self._voice_lock:
    if not self._voice_recording:
        return
    self._voice_recording = False
    self._voice_processing = True  # Prevents concurrent restart races
```

**No-Speech Detection** (lines 5754-5760):
```python
if not submitted:
    self._no_speech_count = getattr(self, '_no_speech_count', 0) + 1
    if self._no_speech_count >= 3:
        self._voice_continuous = False
        self._no_speech_count = 0
        _cprint(f"{_DIM}No speech detected 3 times, continuous mode stopped.{_RST}")
```

---

### Session Management

#### SQLite Session Storage

The CLI integrates with `hermes_state.SessionDB` for persistent session tracking:

```python
from hermes_state import SessionDB
self._session_db = SessionDB()
```

**Session Creation** (from `new_session()` lines 3280-3334):
```python
self._session_db.create_session(
    session_id=self.session_id,
    source=os.environ.get("HERMES_SESSION_SOURCE", "cli"),
    model=self.model,
    model_config={
        "max_iterations": self.max_turns,
        "reasoning_config": self.reasoning_config,
    },
)
```

**Message Persistence** (during conversation):
```python
self._session_db.append_message(
    session_id=self.session_id,
    role=msg.get("role", "user"),
    content=msg.get("content"),
    tool_name=msg.get("tool_name") or msg.get("name"),
    tool_calls=msg.get("tool_calls"),
    reasoning=msg.get("reasoning"),
)
```

#### Session Branching

#### `_handle_branch_command()` (lines 3414-3524)

```python
def _handle_branch_command(self, cmd_original: str) -> None:
    """Handle /branch [name] — fork current session into new independent copy."""
```

**Process:**

1. Generate new session ID
2. End current session with reason "branched"
3. Create new session with `parent_session_id` link
4. Copy all messages to new session
5. Switch to new session

```python
# Copy conversation history
for msg in self.conversation_history:
    self._session_db.append_message(
        session_id=new_session_id,
        role=msg.get("role"),
        content=msg.get("content"),
        ...
    )
```

---

### MCP Server Management

#### `_reload_mcp()` (lines 5427-5511)

```python
def _reload_mcp(self):
    """Reload MCP servers: disconnect all, re-read config.yaml, reconnect."""
```

**Process:**

1. Capture old server names
2. Shutdown existing connections
3. Re-read config.yaml fresh
4. Compute added/removed/reconnected servers
5. Refresh agent's tool list
6. Inject system message about tool changes

```python
# Refresh agent tool list
if self.agent is not None:
    from model_tools import get_tool_definitions
    self.agent.tools = get_tool_definitions(
        enabled_toolsets=self.agent.enabled_toolsets,
        quiet_mode=True,
    )
    self.agent.valid_tool_names = {
        tool["function"]["name"] for tool in self.agent.tools
    }

# Inject context message
self.conversation_history.append({
    "role": "user",
    "content": f"[SYSTEM: MCP servers have been reloaded. Added servers: {added}. Tool list updated.]",
})
```

#### Config File Watcher

#### `_check_config_mcp_changes()` (lines 5370-5425)

```python
def _check_config_mcp_changes(self) -> None:
    """Detect mcp_servers changes in config.yaml and auto-reload."""
```

**Debounce Logic:**
```python
CONFIG_WATCH_INTERVAL = 5.0  # seconds

now = time.monotonic()
if now - self._last_config_check < CONFIG_WATCH_INTERVAL:
    return  # Too soon
self._last_config_check = now

# Check mtime
mtime = cfg_path.stat().st_mtime
if mtime == self._config_mtime:
    return  # File unchanged
```

**Reload in Background Thread:**
```python
_reload_thread = threading.Thread(target=self._reload_mcp, daemon=True)
_reload_thread.start()
_reload_thread.join(timeout=30)  # Hard timeout prevents TUI freeze
```

---

## Line-by-Line Critical Analysis

### Imports and Setup (lines 1-80)

```python
#!/usr/bin/env python3
"""
Hermes Agent CLI - Interactive Terminal Interface

A beautiful command-line interface for the Hermes Agent, inspired by Claude Code.
Features ASCII art branding, interactive REPL, toolset selection, and rich formatting.
"""

import logging
import os
import shutil
import sys
import json
import atexit
import tempfile
import time
import uuid
import textwrap
from contextlib import contextmanager
from pathlib import Path
from datetime import datetime
from typing import List, Dict, Any, Optional

logger = logging.getLogger(__name__)

# Suppress startup messages for clean CLI experience
os.environ["HERMES_QUIET"] = "1"

import yaml

# prompt_toolkit for fixed input area TUI
from prompt_toolkit.history import FileHistory
from prompt_toolkit.styles import Style as PTStyle
from prompt_toolkit.patch_stdout import patch_stdout
from prompt_toolkit.application import Application
from prompt_toolkit.layout import Layout, HSplit, Window, FormattedTextControl, ConditionalContainer
from prompt_toolkit.layout.processors import Processor, Transformation, PasswordProcessor, ConditionalProcessor
from prompt_toolkit.filters import Condition
from prompt_toolkit.layout.dimension import Dimension
from prompt_toolkit.layout.menus import CompletionsMenu
from prompt_toolkit.widgets import TextArea
from prompt_toolkit.key_binding import KeyBindings
from prompt_toolkit import print_formatted_text as _pt_print
from prompt_toolkit.formatted_text import ANSI as _PT_ANSI
```

**Key Observation:** Extensive prompt_toolkit imports indicate heavy customization of the TUI layer. The `patch_stdout` import is critical - it allows Rich/text output to render correctly inside the prompt_toolkit event loop.

```python
try:
    from prompt_toolkit.cursor_shapes import CursorShape
    _STEADY_CURSOR = CursorShape.BLOCK  # Non-blinking block cursor
except (ImportError, AttributeError):
    _STEADY_CURSOR = None
```

**Compatibility Note:** Graceful fallback for older prompt_toolkit versions without cursor shape support.

```python
from agent.usage_pricing import (
    CanonicalUsage,
    estimate_usage_cost,
    format_duration_compact,
    format_token_count_compact,
)
from hermes_cli.banner import _format_context_length
```

**Cost Tracking:** Token usage and cost estimation imported early for session summary display.

```python
# Load .env from ~/.hermes/.env first, then project root as dev fallback.
_hermes_home = get_hermes_home()
_project_env = Path(__file__).parent / '.env'
load_hermes_dotenv(hermes_home=_hermes_home, project_env=_project_env)
```

**Environment Priority:** User's `~/.hermes/.env` takes precedence over project `.env` - important for multi-project setups.

---

### Security: Secret Capture System

#### `_secret_capture_callback()` and Related (lines 7000-7022, 6252-6264)

```python
def _secret_capture_callback(self, var_name: str, prompt: str, metadata=None) -> dict:
    """Capture secrets through secure prompt_toolkit UI (password masking)."""
    return prompt_for_secret(self, var_name, prompt, metadata)
```

**Registration** (lines 7020-7022):
```python
set_sudo_password_callback(self._sudo_password_callback)
set_approval_callback(self._approval_callback)
set_secret_capture_callback(self._secret_capture_callback)
```

**Why Callbacks?** Tools run in agent thread but prompts need main thread UI. Callbacks provide thread-safe bridge.

---

### Streaming TTS Integration

#### Streaming Setup in `chat()` (lines 6376-6433)

```python
# Streaming TTS: when ElevenLabs + sounddevice available, stream sentence-by-sentence
use_streaming_tts = False
if self._voice_tts:
    from tools.tts_tool import _load_tts_config, _get_provider, _import_elevenlabs, _import_sounddevice
    from tools.tts_tool import stream_tts_to_speaker
    
    _tts_cfg = _load_tts_config()
    if _get_provider(_tts_cfg) == "elevenlabs":
        _import_elevenlabs()
        _import_sounddevice()
        use_streaming_tts = True
```

**Display Callback** (lines 6411-6421):
```python
def display_callback(sentence: str):
    """Called by TTS consumer when sentence ready to display + speak."""
    nonlocal _streaming_box_opened
    if not _streaming_box_opened:
        _streaming_box_opened = True
        w = self.console.width
        label = " ⚕ Hermes "
        fill = w - 2 - len(label)
        _cprint(f"\n{_GOLD}╭─{label}{'─' * max(fill - 1, 0)}╮{_RST}")
    _cprint(sentence.rstrip())
```

**Queue-based Streaming** (lines 6407-6428):
```python
if use_streaming_tts:
    text_queue = queue.Queue()
    stop_event = threading.Event()
    
    tts_thread = threading.Thread(
        target=stream_tts_to_speaker,
        args=(text_queue, stop_event, self._voice_tts_done),
        kwargs={"display_callback": display_callback},
        daemon=True,
    )
    tts_thread.start()
    
    def stream_callback(delta: str):
        if text_queue is not None:
            text_queue.put(delta)
```

---

### Context Reference Expansion

#### @-mentions Processing (lines 6331-6350)

```python
if isinstance(message, str) and "@" in message:
    from agent.context_references import preprocess_context_references
    from agent.model_metadata import get_model_context_length
    
    _ctx_len = get_model_context_length(self.model, ...)
    _ctx_result = preprocess_context_references(
        message, cwd=os.getcwd(), context_length=_ctx_len)
    
    if _ctx_result.references:
        _cprint(f"  {@ context: {len(_ctx_result.references)} ref(s), {_ctx_result.injected_tokens} tokens}")
    
    if _ctx_result.blocked:
        return "\n".join(_ctx_result.warnings) or "Context injection refused."
    
    message = _ctx_result.message
```

**Purpose:** Expand `@file:main.py`, `@diff`, `@folder:src/` into actual file content before sending to agent.

**Safety:** `_ctx_result.blocked` prevents context injection attacks by refusing malformed references.

---

### UTF-8 Sanitization

#### Surrogate Character Handling (lines 6352-6357)

```python
# Sanitize surrogate characters from clipboard paste (Google Docs, Word, etc.)
# Lone surrogates are invalid UTF-8 and crash JSON serialization in OpenAI SDK
if isinstance(message, str):
    from run_agent import _sanitize_surrogates
    message = _sanitize_surrogates(message)
```

**Why This Matters:** Rich-text editors often paste text with UTF-16 surrogate pairs that Python's JSON encoder cannot handle. This prevents crashes when serializing messages for API calls.

---

### Interrupt Handling Deep Dive

#### The Interrupt Queue Pattern (lines 6476-6510)

```python
interrupt_msg = None
while agent_thread.is_alive():
    if hasattr(self, '_interrupt_queue'):
        try:
            interrupt_msg = self._interrupt_queue.get(timeout=0.1)
            if interrupt_msg:
                if self._clarify_state or self._clarify_freetext:
                    continue  # Don't steal clarify input
                print("\n⚡ New message detected, interrupting...")
                
                # Signal TTS to stop
                if stop_event is not None:
                    stop_event.set()
                
                self.agent.interrupt(interrupt_msg)
                
                # Debug logging
                try:
                    _dbg = _hermes_home / "interrupt_debug.log"
                    with open(_dbg, "a") as _f:
                        import time as _t
                        _f.write(f"{_t.strftime('%H:%M:%S')} interrupt fired: msg={str(interrupt_msg)[:60]!r}\n")
                except Exception:
                    pass
                break
        except queue.Empty:
            # Force prompt_toolkit to flush stdout buffer
            self._invalidate(min_interval=0.15)
```

**Key Design Decisions:**

1. **Separate Queue:** `_interrupt_queue` is distinct from `_pending_input` - prevents race between process_loop and interrupt monitoring
2. **Clarify Priority:** Clarify tool input takes precedence - user answering questions shouldn't be treated as interrupts
3. **TTS Interruption:** `stop_event.set()` stops audio playback immediately
4. **Debug Logging:** All interrupts logged to `~/.hermes/interrupt_debug.log` for troubleshooting

---

### Tool Preview System

#### Inline Diff Rendering (lines 5572-5597)

```python
def _on_tool_start(self, tool_call_id: str, function_name: str, function_args: dict):
    """Capture local before-state for write-capable tools."""
    from agent.display import capture_local_edit_snapshot
    snapshot = capture_local_edit_snapshot(function_name, function_args)
    if snapshot is not None:
        self._pending_edit_snapshots[tool_call_id] = snapshot

def _on_tool_complete(self, tool_call_id: str, function_name: str, function_args: dict, function_result: str):
    """Render file edits with inline diff after write-capable tools complete."""
    snapshot = self._pending_edit_snapshots.pop(tool_call_id, None)
    from agent.display import render_edit_diff_with_delta
    render_edit_diff_with_delta(
        function_name, function_result, function_args=function_args,
        snapshot=snapshot, print_fn=_cprint,
    )
```

**How It Works:**

1. Before tool runs: capture file's current content as `snapshot`
2. Tool executes: `write_file` modifies the file
3. After tool completes: compute diff between snapshot and new content
4. Render inline diff showing exactly what changed

**Benefits:** User sees actual file changes in real-time instead of just "write_file succeeded" messages.

---

### TUI Layout System

#### Building the Layout (lines 7960-7983)

```python
layout = Layout(
    HSplit(
        self._build_tui_layout_children(
            sudo_widget=sudo_widget,
            secret_widget=secret_widget,
            approval_widget=approval_widget,
            clarify_widget=clarify_widget,
            spinner_widget=spinner_widget,
            spacer=spacer,
            status_bar=status_bar,
            input_rule_top=input_rule_top,
            image_bar=image_bar,
            input_area=input_area,
            input_rule_bot=input_rule_bot,
            voice_status_bar=voice_status_bar,
            completions_menu=completions_menu,
        )
    )
)
```

**Layout Structure:**

```
┌─────────────────────────────────────────┐
│  (Dynamic: clarify/approval/sudo panels)│
│                                         │
│  [spinner_widget] - "Thinking..."       │
│                                         │
│  ─────────────────────────────────────  │ ← input_rule_top
│  [📎 Image #1] [📎 Image #2]            │ ← image_bar
│  ❯ user input here...                   │ ← input_area (TextArea)
│  ─────────────────────────────────────  │ ← input_rule_bot
│  🎤 Voice mode | TTS on | Continuous    │ ← voice_status_bar
│  ─────────────────────────────────────  │
│  ⚕ claude-opus │ 4.2k/128k │ 3% │ 2m  │ ← status_bar
└─────────────────────────────────────────┘
```

#### Status Bar Construction (lines 1616-1684)

```python
def _get_status_bar_fragments(self):
    """Return styled fragments for status bar rendering."""
    snapshot = self._get_status_bar_snapshot()
    
    # Width calculation from prompt_toolkit's terminal size
    try:
        from prompt_toolkit.application import get_app
        width = get_app().output.get_size().columns
    except Exception:
        width = shutil.get_terminal_size((80, 24)).columns
    
    # Context percentage with color coding
    percent = snapshot["context_percent"]
    bar_style = self._status_bar_context_style(percent)
    # Returns: green (good) → yellow (warn) → orange (bad) → red (critical)
    
    frags = [
        ("class:status-bar", " ⚕ "),
        ("class:status-bar-strong", snapshot["model_short"]),
        ("class:status-bar-dim", " │ "),
        ("class:status-bar-dim", context_label),
        ("class:status-bar-dim", " │ "),
        (bar_style, self._build_context_bar(percent)),  # Visual progress bar
        ("class:status-bar-dim", " "),
        (bar_style, percent_label),
        ("class:status-bar-dim", " │ "),
        ("class:status-bar-dim", duration_label),
        ("class:status-bar", " "),
    ]
```

**Overflow Protection** (lines 1554-1580):

```python
@classmethod
def _trim_status_bar_text(cls, text: str, max_width: int) -> str:
    """Trim status-bar text to single terminal row."""
    if cls._status_bar_display_width(text) <= max_width:
        return text
    
    ellipsis = "..."
    # Use character cell width (not len()) for CJK characters
    for ch in text:
        ch_width = get_cwidth(ch) if get_cwidth else len(ch)
        if width + ch_width + ellipsis_width > max_width:
            break
        out.append(ch)
    return "".join(out).rstrip() + ellipsis
```

---

### Resize Handling Fix

#### Ghost Line Prevention (lines 8048-8089)

```python
# Fix ghost status-bar lines on terminal resize
_original_on_resize = app._on_resize

def _resize_clear_ghosts():
    from prompt_toolkit.data_structures import Point as _Pt
    renderer = app.renderer
    try:
        old_size = renderer._last_size
        new_size = renderer.output.get_size()
        if (
            old_size
            and new_size.columns < old_size.columns
            and new_size.columns > 0
        ):
            # Terminal got narrower - lines reflow to multiple rows
            reflow_factor = (
                (old_size.columns + new_size.columns - 1)
                // new_size.columns
            )
            last_h = renderer._last_screen.height if renderer._last_screen else 0
            extra = last_h * (reflow_factor - 1)
            if extra > 0:
                renderer._cursor_pos = _Pt(
                    x=renderer._cursor_pos.x,
                    y=renderer._cursor_pos.y + extra,
                )
    except Exception:
        pass  # Never break resize handling
    _original_on_resize()

app._on_resize = _resize_clear_ghosts
```

**Problem Solved:** When terminal shrinks, previously-rendered wide lines reflow to multiple narrower rows. prompt_toolkit's default handler only moves cursor up by original line count, leaving "ghost" duplicates visible. This fix calculates the reflow multiplier and moves cursor up far enough to erase all ghosts.

---

## Data Flow Diagrams

### User Input Flow

```
┌─────────────┐
│ User types  │
│ + Enter     │
└──────┬──────┘
       │
       ▼
┌─────────────────────────────────────────┐
│  KeyBinding: handle_enter(event)        │
│  - Check active state (sudo/clarify/etc)│
│  - Route to appropriate queue           │
└─────────────────────────────────────────┘
       │
       ├─── If _sudo_state ───► _sudo_state["response_queue"].put(password)
       │
       ├─── If _clarify_state ──► _clarify_state["response_queue"].put(answer)
       │
       ├─── If _approval_state ─► _approval_state["response_queue"].put(choice)
       │
       ├─── If _agent_running ──► _interrupt_queue.put(payload)
       │                            │
       │                            ▼
       │                     ┌──────────────────┐
       │                     │ chat() monitors  │
       │                     │ _interrupt_queue │
       │                     │ during agent run │
       │                     └──────────────────┘
       │
       └─── If agent idle ────► _pending_input.put(payload)
                                │
                                ▼
                         ┌──────────────────┐
                         │ process_loop()   │
                         │ polls queue      │
                         └──────────────────┘
```

### Agent Response Flow

```
┌─────────────────────┐
│ AIAgent generates   │
│ token via LLM API   │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────────────────────┐
│ stream_delta_callback(text: str)    │
│ - Route to _stream_delta()          │
└─────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────┐
│ _stream_delta()                     │
│ 1. Check for reasoning tags         │
│ 2. Buffer partial lines             │
│ 3. Emit complete lines via _cprint  │
└─────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────┐
│ _cprint(text)                       │
│ - Wrap in prompt_toolkit ANSI()     │
│ - print_formatted_text(ANSI(text))  │
└─────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────┐
│ prompt_toolkit renderer             │
│ - Paints colored text to terminal   │
│ - Handles line wrapping, scrolling  │
└─────────────────────────────────────┘
```

### Session Persistence Flow

```
┌──────────────────────────────┐
│ User sends message           │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ conversation_history.append()│
│ - Add user message           │
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ agent.run_conversation()     │
│ - Execute tool calls         │
│ - Get LLM response           │
│ - Update conversation_history│
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ _persist_session()           │
│ - Iterate over new messages  │
│ - For each message:          │
│   session_db.append_message()│
└──────────────┬───────────────┘
               │
               ▼
┌──────────────────────────────┐
│ SQLite INSERT into messages  │
│ - session_id                 │
│ - role (user/assistant/tool) │
│ - content                    │
│ - tool_calls (JSON)          │
│ - reasoning (JSON)           │
│ - timestamp                  │
└──────────────────────────────┘
```

---

## Integration Points

### External Modules Imported

| Module | Purpose | Key Functions |
|--------|---------|---------------|
| `run_agent` | Core agent logic | `AIAgent`, `run_conversation()` |
| `model_tools` | Tool definitions | `get_tool_definitions()`, `get_toolset_for_tool()` |
| `hermes_cli.*` | CLI utilities | `setup_logging()`, `load_cli_config()`, `banner` |
| `tools.*` | Individual tools | `terminal_tool`, `browser_tool`, `skills_tool`, etc. |
| `agent.*` | Agent subsystems | `usage_pricing`, `smart_model_routing`, `title_generator` |
| `gateway.*` | Messaging platforms | `start_gateway()`, `Platform` enum |
| `hermes_state` | SQLite persistence | `SessionDB` class |
| `cron` | Scheduled tasks | `get_job()`, cron job management |

### Callback Registration

```python
# Tools register callbacks so CLI handles interactive prompts
from tools.terminal_tool import set_sudo_password_callback, set_approval_callback
from tools.skills_tool import set_secret_capture_callback
from hermes_cli.callbacks import prompt_for_secret

set_sudo_password_callback(self._sudo_password_callback)
set_approval_callback(self._approval_callback)
set_secret_capture_callback(self._secret_capture_callback)
```

**Why This Pattern?** Tools run in agent thread but need to display UI prompts on main thread. Callbacks provide thread-safe bridge.

### Plugin Integration

```python
# Plugin manager gets CLI reference for message injection
from hermes_cli.plugins import get_plugin_manager
get_plugin_manager()._cli_ref = self

# Plugin commands dispatched via name registry
_plugin_cmd_handlers = _get_plugin_cmd_handler_names()
if base_cmd in _plugin_cmd_handlers:
    plugin_handler = get_plugin_command_handler(base_cmd)
    result = plugin_handler(user_args)
```

---

## Configuration System

### Config Loading Priority

```
1. CLI arguments (--model, --provider, etc.)
2. Environment variables (OPENAI_API_KEY, HERMES_*)
3. ~/.hermes/config.yaml (user config)
4. ./cli-config.yaml (project config)
5. Hardcoded defaults
```

### Key Configuration Sections

#### Model Configuration

```yaml
model:
  default: "anthropic/claude-opus-4-20250514"  # or just a string like "gpt-4"
  provider: "auto"  # auto, openrouter, nous, openai-codex, etc.
  base_url: ""      # Custom endpoint (llama.cpp, ollama, vLLM)
```

**Code Handling** (lines 1266-1286):
```python
_model_config = CLI_CONFIG.get("model", {})
_config_model = (_model_config.get("default") or _model_config.get("model") or "") if isinstance(_model_config, dict) else (_model_config or "")
self.model = model or _config_model or _DEFAULT_CONFIG_MODEL

# Track if model is default or explicitly chosen
self._model_is_default = not model and (
    not _config_model or _config_model == _DEFAULT_CONFIG_MODEL
)
```

#### Tool Progress Modes

```yaml
display:
  tool_progress: "off"    # Silent, just final response
               | "new"    # Show each new tool (skip repeats)
               | "all"    # Show every tool call
               | "verbose" # Full args, results, think blocks
```

**Cycle Command** (lines 5116-5142):
```python
def _toggle_verbose(self):
    cycle = ["off", "new", "all", "verbose"]
    self.tool_progress_mode = cycle[(idx + 1) % len(cycle)]
    self.verbose = self.tool_progress_mode == "verbose"
```

---

## Interactive TUI System

### Prompt Symbol System

#### `_get_tui_prompt_symbols()` (lines 6755-6792)

```python
def _get_tui_prompt_symbols(self) -> tuple[str, str]:
    """Return (normal_prompt, state_suffix) for active skin.
    
    When profile is active (not "default"), profile name is prepended:
    "coder ❯" instead of just "❯"
    """
    from hermes_cli.skin_engine import get_active_prompt_symbol
    symbol = get_active_prompt_symbol("❯ ")
    
    # Prepend profile name
    from hermes_cli.profiles import get_active_profile_name
    profile = get_active_profile_name()
    if profile not in ("default", "custom"):
        symbol = f"{profile} {symbol}"
    
    # Extract arrow character for state suffix
    parts = symbol.split()
    candidate = parts[-1] if parts else ""
    arrow_chars = ("❯", ">", "$", "#", "›", "»", "→")
    if any(ch in candidate for ch in arrow_chars):
        return symbol, candidate.rstrip() + " "
    
    return symbol, symbol
```

**State-Specific Prompts:**
```python
def _get_tui_prompt_fragments(self):
    symbol, state_suffix = self._get_tui_prompt_symbols()
    if self._voice_recording:
        return [("class:voice-recording", f"● {bar} {state_suffix}")]
    if self._sudo_state:
        return [("class:sudo-prompt", f"🔐 {state_suffix}")]
    if self._clarify_state:
        return [("class:prompt-working", f"? {state_suffix}")]
    if self._agent_running:
        return [("class:prompt-working", f"⚕ {state_suffix}")]
    return [("class:prompt", symbol)]
```

### Auto-Completion System

#### SlashCommandCompleter (lines 7488-7505)

```python
_completer = SlashCommandCompleter(
    skill_commands_provider=lambda: _skill_commands,
)
input_area = TextArea(
    completer=_completer,
    complete_while_typing=True,
    auto_suggest=SlashCommandAutoSuggest(
        history_suggest=AutoSuggestFromHistory(),
        completer=_completer,
    ),
)
```

**Completion Flow:**
```
User types "/" → Completer shows built-in commands
User types "/s" → Completer filters to /skills, /save, /statusbar, etc.
User types "/ski" → Completer narrows to /skills
User types "/skills " → Completer shows skill subcommands (search, browse, install, etc.)
```

---

## Voice Mode System

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Voice Mode Pipeline                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌────────┐│
│  │ Microphone│───►│AudioRec- │───►│  STT     │───►│ Text   ││
│  │  Input   │    │  order   │    │(Whisper/ │    │ Queue  ││
│  │          │    │          │    │  Groq)   │    │        ││
│  └──────────┘    └──────────┘    └──────────┘    └────────┘│
│       ▲                                              │      │
│       │                                              ▼      │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌────────┐│
│  │ Speaker  │◄───│  TTS     │◄───│  Agent   │◄───│_pending││
│  │  Output  │    │(Eleven-  │    │ Response │    │ _input ││
│  │          │    │  labs)   │    │          │    │        ││
│  └──────────┘    └──────────┘    └──────────┘    └────────┘│
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Key Bindings for Voice

```python
@kb.add(_voice_key)  # Default: Ctrl+B (c-b)
def handle_voice_record(event):
    """Toggle voice recording when voice mode is active."""
    if not cli_ref._voice_mode:
        return
    
    if cli_ref._voice_recording:
        # Stop recording
        cli_ref._voice_continuous = False
        threading.Thread(target=cli_ref._voice_stop_and_transcribe, daemon=True).start()
    else:
        # Guard: don't start during agent run or prompts
        if cli_ref._agent_running:
            return
        if cli_ref._clarify_state or cli_ref._sudo_state or cli_ref._approval_state:
            return
        if cli_ref._voice_processing:
            return  # Previous cycle still running
        
        # Interrupt TTS if playing
        if not cli_ref._voice_tts_done.is_set():
            stop_playback()
            cli_ref._voice_tts_done.set()
        
        threading.Thread(target=_start_recording, daemon=True).start()
```

**Why Thread Dispatch?** (lines 7396-7407)

```python
# Dispatch to daemon thread so play_beep(sd.wait),
# AudioRecorder.start(lock acquire), and config I/O
# never block the prompt_toolkit event loop.
def _start_recording():
    try:
        cli_ref._voice_start_recording()
        if hasattr(cli_ref, '_app') and cli_ref._app:
            cli_ref._app.invalidate()
    except Exception as e:
        _cprint(f"\n{_DIM}Voice recording failed: {e}{_RST}")

threading.Thread(target=_start_recording, daemon=True).start()
```

**Problem Prevented:** `sd.wait()` in `play_beep()` and lock acquisition in `AudioRecorder.start()` would freeze the entire UI if run on main thread.

---

## Security Features

### Secret Redaction

```python
# Security settings from config
security_config = defaults.get("security", {})
if isinstance(security_config, dict):
    redact = security_config.get("redact_secrets")
    if redact is not None:
        os.environ["HERMES_REDACT_SECRETS"] = str(redact).lower()
```

**Effect:** When enabled, API keys and credentials are masked in logs and output.

### Tirith Security Scanner

```python
# Ensure tirith security scanner is available (downloads if needed)
try:
    from tools.tirith_security import ensure_installed
    tirith_path = ensure_installed(log_failures=False)
    if tirith_path is None:
        security_cfg = self.config.get("security", {}) or {}
        tirith_enabled = security_cfg.get("tirith_enabled", True)
        if tirith_enabled:
            _cprint(f"  {_DIM}⚠ tirith security scanner enabled but not available{_RST}")
except Exception:
    pass  # Non-fatal — fail-open at scan time
```

**Purpose:** Scans shell commands for dangerous patterns before execution (e.g., `rm -rf /`, `dd if=/dev/zero`).

### Worktree Isolation

```python
# Create isolated git worktree for parallel agent sessions
wt_info = _setup_worktree()
if wt_info:
    os.environ["TERMINAL_CWD"] = wt_info["path"]
    atexit.register(_cleanup_worktree, wt_info)
```

**Security Benefit:** Agents working in parallel cannot accidentally overwrite each other's changes or access unrelated project files.

---

## Error Handling Patterns

### Graceful Degradation

Throughout the codebase, optional features fail gracefully:

```python
try:
    from hermes_cli.skin_engine import get_active_skin
    _skin = get_active_skin()
except Exception:
    pass  # Skin engine is optional — default skin used if unavailable

try:
    from hermes_state import SessionDB
    self._session_db = SessionDB()
except Exception as e:
    logger.warning("SQLite session store not available: %s", e)
    # Session continues without persistence
```

### Cleanup on Exit

```python
def _run_cleanup():
    """Run resource cleanup exactly once."""
    global _cleanup_done
    if _cleanup_done:
        return
    _cleanup_done = True
    
    try:
        _cleanup_all_terminals()  # Close VM instances
    except Exception:
        pass
    
    try:
        _cleanup_all_browsers()  # Close browser sessions
    except Exception:
        pass
    
    try:
        from tools.mcp_tool import shutdown_mcp_servers
        shutdown_mcp_servers()
    except Exception:
        pass
    
    try:
        from agent.auxiliary_client import shutdown_cached_clients
        shutdown_cached_clients()  # Close HTTP connections
    except Exception:
        pass
```

**Registration:**
```python
atexit.register(_run_cleanup)
```

---

## Testing Considerations

### Testability Hooks

1. **Pure Functions:** `_detect_file_drop()`, `_parse_reasoning_config()`, `_looks_like_slash_command()` are pure and easily testable

2. **Dependency Injection:** Callbacks (`_clarify_callback`, `_approval_callback`) allow mocking for tests

3. **State Exposure:** Internal state (`_clarify_state`, `_approval_state`) accessible for assertions

### Known Testing Challenges

1. **prompt_toolkit Event Loop:** Requires special handling in tests - cannot easily simulate key presses without full application context

2. **Thread Coordination:** `_interrupt_queue` and `_pending_input` require careful timing in tests

3. **External Dependencies:** STT/TTS/MCP servers require mocks or skip markers

---

## Performance Optimizations

### Throttled Repaints

```python
def _invalidate(self, min_interval: float = 0.25) -> None:
    """Throttled UI repaint — prevents terminal flickering on slow/SSH connections."""
    import time as _time
    now = _time.monotonic()
    if hasattr(self, "_app") and self._app and (now - self._last_invalidate) >= min_interval:
        self._last_invalidate = now
        self._app.invalidate()
```

**Why Important:** Without throttling, rapid updates (e.g., streaming tokens) cause visible flicker especially over SSH.

### Lazy Client Initialization

```python
def _ensure_runtime_credentials(self) -> bool:
    """Ensure runtime credentials are resolved before agent use.
    
    Re-resolves provider credentials so key rotation and token refresh
    are picked up without restarting the CLI.
    """
```

**Benefit:** Doesn't create OpenAI/Auxiliary clients until first actual use, saving memory and startup time.

---

## Known Issues and Workarounds

### Event Loop is Closed Errors

**Problem:** httpx/AsyncOpenAI clients' `__del__` tries to close connections on dead event loop.

**Fix** (lines 548-552):
```python
# Neuter AsyncHttpxClientWrapper.__del__ before any AsyncOpenAI clients are created
try:
    from agent.auxiliary_client import neuter_async_httpx_del
    neuter_async_httpx_del()
except Exception:
    pass
```

### Patch Stdout Swallowing ANSI

**Problem:** `patch_stdout`'s `StdoutProxy` swallows raw ANSI escape sequences.

**Fix** (lines 906-913):
```python
def _cprint(text: str):
    """Print ANSI-colored text through prompt_toolkit's native renderer."""
    _pt_print(_PT_ANSI(text))
```

### Clipboard Surrogate Crashes

**Problem:** Rich-text editors paste UTF-16 surrogates that crash JSON serialization.

**Fix** (lines 6352-6357):
```python
from run_agent import _sanitize_surrogates
message = _sanitize_surrogates(message)
```

---

## Related Files

| File | Purpose |
|------|---------|
| `run_agent.py` | Core `AIAgent` class and `run_conversation()` logic |
| `model_tools.py` | Tool definitions and toolset management |
| `hermes_cli/config.py` | Configuration loading and validation |
| `hermes_cli/skin_engine.py` | UI theming and color schemes |
| `hermes_state.py` | SQLite session persistence |
| `tools/terminal_tool.py` | Terminal/shell execution |
| `tools/browser_tool.py` | Browser automation |
| `tools/skills_tool.py` | Skill/plugin management |
| `tools/voice_mode.py` | Voice STT/TTS integration |
| `agent/context_references.py` | @-mention expansion |
| `agent/title_generator.py` | Auto-generated session titles |

---

## Summary

The `cli.py` module is a **sophisticated interactive terminal interface** that balances:

- **Rich UX** with streaming tokens, colored output, and dynamic UI panels
- **Responsiveness** through multi-threading and non-blocking I/O
- **Extensibility** via plugins, skills, and MCP servers
- **Security** with worktree isolation, secret capture, and command approval
- **Persistence** through SQLite session storage and resume capabilities

At ~8,500 lines, it's a substantial codebase that demonstrates advanced Python techniques including:

- Async/thread coordination patterns
- Custom prompt_toolkit widget development
- Callback-based architecture for cross-thread communication
- Graceful error handling and resource cleanup
- Configuration-driven feature flags

The code follows a **defensive programming style** with extensive try/except blocks, state validation, and fallback behaviors - appropriate for a user-facing tool that must handle diverse environments and unexpected conditions.

---

*Deep Dive Exploration Complete*

*Generated: 2026-04-07*
*Source Lines Analyzed: 8,554*
*Exploration Lines: 600+*
