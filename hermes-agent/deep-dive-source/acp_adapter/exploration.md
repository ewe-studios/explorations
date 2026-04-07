# acp_adapter/ Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/acp_adapter/`

**Status:** complete

---

## Module Overview

The `acp_adapter/` module implements the **Agent Communication Protocol (ACP)** server for Hermes Agent, enabling integration with ACP-compatible clients like VS Code (Cline, Claude Code) and Zed. This ~1,786 line module wraps the Hermes `AIAgent` class and exposes it via the standardized ACP interface.

ACP provides a JSON-RPC based protocol for:
- Session management (create, load, fork, resume)
- Message streaming with tool call visibility
- Authentication and authorization
- Model and tool configuration
- Session state persistence

The module architecture follows a clean separation:
- **server.py** - ACP server implementation
- **session.py** - Session state management
- **tools.py** - ACP tool adaptation
- **auth.py** - Authentication detection
- **events.py** - Event callback adapters
- **permissions.py** - Approval/permission handling

---

## Directory Structure

| File | Lines | Purpose |
|------|-------|---------|
| `__init__.py` | 1 | Package docstring |
| `__main__.py` | 5 | Entry point for `python -m` |
| `auth.py` | 24 | Auth provider detection |
| `entry.py` | 86 | CLI entry point |
| `events.py` | 175 | Event callback adapters |
| `permissions.py` | 77 | Approval callback handling |
| `server.py` | 726 | ACP server implementation |
| `session.py` | 477 | Session manager |
| `tools.py` | 215 | Tool adaptation |

**Total:** ~1,786 lines across 9 files

---

## Key Components

### 1. ACP Server (`server.py`)

Main server implementation extending `acp.Agent` base class.

**Key Class:**
```python
class HermesACPAgent(acp.Agent):
    """ACP Agent implementation wrapping Hermes AIAgent."""
    
    _SLASH_COMMANDS = {
        "help": "Show available commands",
        "model": "Show or change current model",
        "tools": "List available tools",
        "context": "Show conversation context info",
        "reset": "Clear conversation history",
        "compact": "Compress conversation context",
        "version": "Show Hermes version",
    }
```

**Key Methods:**
```python
def on_connect(self, conn: acp.Client) -> None:
    """Store the client connection for sending session updates."""

async def initialize(
    self, 
    request: InitializeRequest
) -> InitializeResponse:
    """Handle ACP initialize handshake."""

async def new_session(self, request: NewSessionRequest) -> NewSessionResponse:
    """Create a new ACP session."""

async def prompt(self, request: PromptRequest) -> PromptResponse:
    """Handle user prompt - main message processing."""

async def load_session(
    self, 
    request: LoadSessionRequest
) -> LoadSessionResponse:
    """Load a session from persistence."""

async def resume_session(
    self, 
    request: ResumeSessionRequest
) -> ResumeSessionResponse:
    """Resume an existing session."""

async def fork_session(
    self, 
    request: ForkSessionRequest
) -> ForkSessionResponse:
    """Fork a session to create a variant."""
```

**Slash Commands Advertised:**
```python
_ADVERTISED_COMMANDS = (
    {"name": "help", "description": "List available commands"},
    {"name": "model", "description": "Show current model", "input_hint": "model name"},
    {"name": "tools", "description": "List available tools"},
    {"name": "context", "description": "Show conversation message counts"},
    {"name": "reset", "description": "Clear conversation history"},
    {"name": "compact", "description": "Compress conversation context"},
    {"name": "version", "description": "Show Hermes version"},
)
```

### 2. Session Manager (`session.py`)

Manages ACP sessions backed by Hermes `AIAgent` instances with database persistence.

**Key Classes:**
```python
@dataclass
class SessionState:
    """Tracks per-session state for an ACP-managed Hermes agent."""
    session_id: str
    agent: Any  # AIAgent instance
    cwd: str = "."
    model: str = ""
    history: List[Dict[str, Any]] = field(default_factory=list)
    cancel_event: Any = None  # threading.Event

class SessionManager:
    """Thread-safe manager for ACP sessions backed by Hermes AIAgent instances.
    
    Sessions are held in-memory for fast access **and** persisted to the
    shared SessionDB so they survive process restarts and are searchable
    via ``session_search``.
    """
```

**SessionManager API:**
```python
def create_session(self, cwd: str = ".") -> SessionState:
    """Create a new session with a unique ID and fresh AIAgent."""

def get_session(self, session_id: str) -> Optional[SessionState]:
    """Return session for *session_id*, or None.
    
    Transparently restores from database if not in memory.
    """

def remove_session(self, session_id: str) -> bool:
    """Remove a session from memory and database."""

def fork_session(self, session_id: str, cwd: str = ".") -> Optional[SessionState]:
    """Deep-copy a session's history into a new session."""

def list_sessions(self) -> List[SessionState]:
    """Return all active sessions."""

def persist(self, state: SessionState) -> None:
    """Persist session to SessionDB."""

def _restore(self, session_id: str) -> Optional[SessionState]:
    """Restore session from SessionDB."""
```

**Task CWD Override Helpers:**
```python
def _register_task_cwd(task_id: str, cwd: str) -> None:
    """Bind a task/session id to editor's working directory for tools."""

def _clear_task_cwd(task_id: str) -> None:
    """Remove task-specific cwd overrides for an ACP session."""
```

### 3. Tools Adapter (`tools.py`)

Adapts Hermes tools for ACP consumption.

**Key Functions:**
```python
def hermes_tools_to_acp_tools() -> List[acp.Tool]:
    """Convert Hermes tool definitions to ACP format."""

def execute_hermes_tool(
    tool_name: str, 
    args: dict
) -> Tuple[bool, str]:
    """Execute a Hermes tool and return (success, result)."""
```

**Tool Translation:**
- Hermes tool schemas (JSON Schema) -> ACP `InputSchema`
- Hermes tool handlers -> ACP tool execution callbacks
- Tool output formatting for ACP content blocks

### 4. Event Callbacks (`events.py`)

Adapts Hermes event callbacks to ACP session update format.

**Key Functions:**
```python
def make_message_cb(session_state: SessionState, conn: acp.Client):
    """Create callback for assistant message chunks."""

def make_tool_progress_cb(session_state: SessionState, conn: acp.Client):
    """Create callback for tool call progress events."""

def make_thinking_cb(session_state: SessionState, conn: acp.Client):
    """Create callback for thinking/reasoning content."""

def make_step_cb(session_state: SessionState, conn: acp.Client):
    """Create callback for step transitions."""
```

**Callback Behavior:**
- Streams incremental updates to ACP client via `conn.session.message()`
- Handles thinking content separately from final responses
- Formats tool call arguments for display

### 5. Permissions (`permissions.py`)

Approval callback for tool execution permissions.

**Key Function:**
```python
def make_approval_callback(session_state: SessionState):
    """Create approval callback for tool execution.
    
    Returns async function that checks session config for:
    - Auto-approve list
    - Manual approval requirements
    - Platform-specific rules
    """
```

### 6. Authentication (`auth.py`)

Detects and validates authentication credentials.

**Key Functions:**
```python
def detect_provider() -> str:
    """Detect active provider from environment/config."""

def has_provider(provider: str) -> bool:
    """Check if a specific provider is configured."""
```

**Detection Order:**
1. Environment variables (`OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY`, etc.)
2. Hermes auth file (`~/.hermes/auth.json`)
3. Config file (`~/.hermes/config.yaml`)

### 7. Entry Point (`entry.py`)

CLI entry point for running the ACP server.

**Usage:**
```bash
# Via module
python -m acp_adapter

# Via hermes CLI
hermes acp-serve
```

**Key Code:**
```python
def main():
    """Run the ACP server."""
    session_manager = SessionManager()
    agent = HermesACPAgent(session_manager)
    acp.run(agent)
```

---

## ACP Protocol Support

### Capabilities Advertised

```python
AgentCapabilities(
    prompts=PromptCapabilities(listChanged=True),
    tools=ToolCapabilities(listChanged=True),
    sessions=SessionCapabilities(
        list=SessionListCapabilities(),
        fork=SessionForkCapabilities(),
    ),
)
```

### Session Lifecycle

1. **Initialize** - Handshake with capabilities exchange
2. **New Session** - Create fresh session with unique ID
3. **Prompt** - Process user messages, stream responses
4. **Load/Resume** - Restore persisted sessions
5. **Fork** - Create variant sessions
6. **List** - Enumerate active sessions

### Content Block Types

| Type | ACP Schema | Hermes Source |
|------|-----------|---------------|
| Text | `TextContentBlock` | Assistant text chunks |
| Image | `ImageContentBlock` | Generated images |
| Audio | `AudioContentBlock` | TTS output |
| Resource | `ResourceContentBlock` | File references |
| Embedded Resource | `EmbeddedResourceContentBlock` | Inline file content |

---

## Integration Points

### With Hermes Core (`run_agent.py`)
- `SessionManager._make_agent()` - Creates `AIAgent` instances
- Event callbacks - Stream `AIAgent` output to ACP client
- Tool execution - Calls Hermes tool handlers

### With Session Database
- Sessions persisted to `~/.hermes/state.db`
- Survives process restarts
- Searchable via `session_search` tool

### With Tool System (`tools/`)
- Tool schemas exposed via ACP
- Dynamic tool registration/deregistration
- Tool approval via permissions callback

### With Terminal Tool
- CWD override registration for task isolation
- Ensures terminal commands run in editor's working directory

---

## Related Files

**Individual File Explorations:**
- [server.md](./acp_adapter/server.md)
- [session.md](./acp_adapter/session.md)
- [tools.md](./acp_adapter/tools.md)
- [auth.md](./acp_adapter/auth.md)
- [entry.md](./acp_adapter/entry.md)
- [events.md](./acp_adapter/events.md)
- [permissions.md](./acp_adapter/permissions.md)

**Related Modules:**
- [copilot_acp_client.md](../agent/copilot_acp_client.md) - GitHub Copilot ACP client (agent/)
- [agent/exploration.md](../agent/exploration.md) - Main agent module

---

*Deep dive created: 2026-04-07*
