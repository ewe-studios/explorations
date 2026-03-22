# Nanobot: Comprehensive Exploration

## Executive Summary

**nanobot** is an ultra-lightweight personal AI assistant framework written in Python. Inspired by Clawdbot but distilled to its essence, nanobot delivers core agent functionality in just **~3,400 lines of code** — 99% smaller than Clawdbot's 430k+ lines.

### Repository Information
- **Source Location**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.Moltbot/nanobot`
- **Version**: 0.1.3.post4
- **Language**: Python 3.11+
- **License**: MIT

### Key Design Philosophy

1. **Minimalism**: Every line of code earns its place
2. **Readability**: Clean, well-documented code that's easy to understand
3. **Extensibility**: Modular architecture allowing easy additions
4. **Production-Ready**: Despite its size, includes robust error handling and security features

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Core Components](#core-components)
3. [Agent System](#agent-system)
4. [Communication Channels](#communication-channels)
5. [Tool System](#tool-system)
6. [Skills System](#skills-system)
7. [Scheduling & Automation](#scheduling--automation)
8. [Session Management](#session-management)
9. [Provider Integration](#provider-integration)
10. [CLI & User Interface](#cli--user-interface)

---

## Architecture Overview

nanobot follows a clean, event-driven architecture centered around a message bus that decouples components:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Chat Channels                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐            │
│  │ Telegram │  │ WhatsApp │  │ Discord  │  │  Feishu  │            │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘            │
│       │             │             │             │                    │
│       └─────────────┴──────┬──────┴─────────────┘                    │
│                            │                                         │
│                    ┌───────▼────────┐                                │
│                    │  Message Bus   │  ← Async Queue                 │
│                    │  (Inbound/     │                                │
│                    │   Outbound)    │                                │
│                    └───────┬────────┘                                │
│                            │                                         │
│                    ┌───────▼────────┐                                │
│                    │   Agent Loop   │  ← Core Processing             │
│                    └───────┬────────┘                                │
│                            │                                         │
│       ┌────────────────────┼────────────────────┐                   │
│       │                    │                    │                    │
│  ┌────▼─────┐      ┌──────▼───────┐     ┌──────▼──────┐            │
│  │  Tools   │      │   Context    │     │  Provider   │            │
│  │ Registry │      │   Builder    │     │  (LiteLLM)  │            │
│  └──────────┘      └──────────────┘     └─────────────┘            │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │              Supporting Services                             │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐    │   │
│  │  │  Cron    │  │Heartbeat │  │ Session  │  │  Memory  │    │   │
│  │  │ Service  │  │ Service  │  │ Manager  │  │  Store   │    │   │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘    │   │
│  └─────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### Core Data Flow

1. **Inbound**: Channel → Message Bus → Agent Loop → LLM → Tool Execution
2. **Outbound**: Agent Loop → Message Bus → Channel → User

---

## Core Components

### 1. Message Bus (`nanobot/bus/`)

The message bus is the central nervous system, providing async queue-based communication.

**File**: `nanobot/bus/queue.py`

```python
class MessageBus:
    """Async message bus that decouples chat channels from the agent core."""

    def __init__(self):
        self.inbound: asyncio.Queue[InboundMessage] = asyncio.Queue()
        self.outbound: asyncio.Queue[OutboundMessage] = asyncio.Queue()
        self._outbound_subscribers: dict[str, list[Callable]] = {}
```

**Key Features**:
- Separate inbound/outbound queues
- Publisher-subscriber pattern for outbound messages
- Non-blocking async operations
- Timeout-based consumption (1 second default)

**Event Types** (`nanobot/bus/events.py`):
- `InboundMessage`: channel, sender_id, chat_id, content, media, metadata
- `OutboundMessage`: channel, chat_id, content, reply_to, media, metadata

### 2. Configuration System (`nanobot/config/`)

**File**: `nanobot/config/schema.py`

Uses Pydantic for type-safe configuration:

```python
class Config(BaseSettings):
    agents: AgentsConfig
    channels: ChannelsConfig
    providers: ProvidersConfig
    gateway: GatewayConfig
    tools: ToolsConfig
```

**Configuration Files**:
- `~/.nanobot/config.json` - Main configuration
- Supports environment variable overrides (`NANOBOT_` prefix)
- Automatic key conversion between camelCase (JSON) and snake_case (Python)

**Provider Auto-Detection**:
```python
def get_api_key(self, model: str | None = None) -> str | None:
    """Get API key for the given model with fallback."""
```

### 3. Utility Functions (`nanobot/utils/`)

Common utilities for directory management, path resolution, and helpers:

```python
def ensure_dir(path: Path) -> Path
def get_data_path() -> Path
def get_workspace_path(workspace: str | None = None) -> Path
def today_date() -> str
def safe_filename(name: str) -> str
```

---

## Agent System

### Agent Loop (`nanobot/agent/loop.py`)

The **AgentLoop** is the heart of nanobot - processing messages and coordinating all agent activities.

**Core Responsibilities**:
1. Consume messages from the inbound bus
2. Build context with history, memory, and skills
3. Call the LLM with tool definitions
4. Execute tool calls
5. Send responses back through the bus

**Key Method**: `_process_message()`

```python
async def _process_message(self, msg: InboundMessage) -> OutboundMessage:
    # 1. Get or create session
    session = self.sessions.get_or_create(msg.session_key)

    # 2. Update tool contexts (channel, chat_id)
    message_tool.set_context(msg.channel, msg.chat_id)

    # 3. Build messages with context
    messages = self.context.build_messages(
        history=session.get_history(),
        current_message=msg.content,
        media=msg.media,
        channel=msg.channel,
        chat_id=msg.chat_id,
    )

    # 4. Agent iteration loop
    while iteration < self.max_iterations:
        # Call LLM
        response = await self.provider.chat(...)

        # Execute tool calls
        if response.has_tool_calls:
            for tool_call in response.tool_calls:
                result = await self.tools.execute(...)
                messages = self.context.add_tool_result(...)
        else:
            # No tool calls = done
            break

    # 5. Save session and return response
    return OutboundMessage(...)
```

### Context Builder (`nanobot/agent/context.py`)

Assembles the system prompt and message context:

**Components**:
1. **Identity**: Core nanobot identity with runtime info
2. **Bootstrap Files**: AGENTS.md, SOUL.md, USER.md, TOOLS.md, IDENTITY.md
3. **Memory**: Long-term and daily memory notes
4. **Skills**: Always-loaded skills + available skills summary

**System Prompt Structure**:
```
# nanobot 🐈
## Current Time: {timestamp}
## Runtime: {platform}
## Workspace: {path}

# Memory
## Long-term Memory
## Today's Notes

# Active Skills
{always-loaded skills content}

# Skills (Progressive Loading)
{XML summary of available skills}
```

### Memory System (`nanobot/agent/memory.py`)

Dual-layer memory for persistent context:

**Daily Memory**:
- Location: `~/.nanobot/workspace/memory/YYYY-MM-DD.md`
- Auto-created each day
- Append-only design

**Long-term Memory**:
- Location: `~/.nanobot/workspace/memory/MEMORY.md`
- Persists important cross-session information

```python
class MemoryStore:
    def get_today_file(self) -> Path
    def read_today(self) -> str
    def append_today(self, content: str) -> None
    def read_long_term(self) -> str
    def get_recent_memories(self, days: int = 7) -> str
```

### Subagent System (`nanobot/agent/subagent.py`)

Lightweight background task execution:

**Key Features**:
- Isolated context per subagent
- No message/spawn tools (focused execution)
- Automatic result announcement
- UUID-based task tracking

**Spawn Process**:
```python
async def spawn(self, task: str, label: str = None) -> str:
    task_id = str(uuid.uuid4())[:8]
    bg_task = asyncio.create_task(self._run_subagent(...))
    self._running_tasks[task_id] = bg_task
```

**Subagent Prompt**:
```
# Subagent
You are a subagent spawned by the main agent to complete a specific task.

## Your Task
{task description}

## Rules
1. Stay focused - complete only the assigned task
2. Your final response will be reported back to the main agent
3. Do not initiate conversations or take on side tasks
```

---

## Communication Channels

### Base Channel Interface (`nanobot/channels/base.py`)

Abstract base class for all channel implementations:

```python
class BaseChannel(ABC):
    @abstractmethod
    async def start(self) -> None
    @abstractmethod
    async def stop(self) -> None
    @abstractmethod
    async def send(self, msg: OutboundMessage) -> None

    def is_allowed(self, sender_id: str) -> bool  # Access control
    async def _handle_message(...)  # Internal message routing
```

### Channel Manager (`nanobot/channels/manager.py`)

Coordinates multiple channels:

```python
class ChannelManager:
    def _init_channels(self) -> None  # Dynamic channel initialization
    async def start_all(self) -> None  # Start all channels + dispatcher
    async def _dispatch_outbound(self) -> None  # Route messages
```

### Implemented Channels

#### Telegram (`nanobot/channels/telegram.py`)
- Uses `python-telegram-bot` library
- Supports text, images, voice transcription
- Proxy support (HTTP/SOCKS5)

#### Discord (`nanobot/channels/discord.py`)
- Uses `discord.py` library
- Gateway WebSocket connection
- Intent-based message filtering

#### WhatsApp (`nanobot/channels/whatsapp.py`)
- Uses external bridge (Node.js)
- QR code authentication
- WebSocket communication

#### Feishu/Lark (`nanobot/channels/feishu.py`)
- WebSocket long connection (no public IP needed)
- Event subscription model
- Chinese enterprise integration

---

## Tool System

### Tool Base Class (`nanobot/agent/tools/base.py`)

Abstract tool interface with JSON Schema validation:

```python
class Tool(ABC):
    @property
    @abstractmethod
    def name(self) -> str

    @property
    @abstractmethod
    def description(self) -> str

    @property
    @abstractmethod
    def parameters(self) -> dict[str, Any]  # JSON Schema

    @abstractmethod
    async def execute(self, **kwargs: Any) -> str

    def validate_params(self, params: dict) -> list[str]  # Validation
    def to_schema(self) -> dict  # OpenAI format
```

### Tool Registry (`nanobot/agent/tools/registry.py`)

Dynamic tool management:

```python
class ToolRegistry:
    def register(self, tool: Tool) -> None
    def unregister(self, name: str) -> None
    def get_definitions(self) -> list[dict]  # For LLM
    async def execute(self, name: str, params: dict) -> str
```

### Built-in Tools

#### File Tools (`nanobot/agent/tools/filesystem.py`)

| Tool | Description | Security |
|------|-------------|----------|
| `read_file` | Read file contents | Path validation |
| `write_file` | Write/create files | Creates parent dirs |
| `edit_file` | Replace text in files | Exact match required |
| `list_dir` | List directory contents | Directory validation |

**Workspace Restriction**:
```python
def _resolve_path(path: str, allowed_dir: Path | None = None) -> Path:
    resolved = Path(path).expanduser().resolve()
    if allowed_dir and not str(resolved).startswith(str(allowed_dir.resolve())):
        raise PermissionError(...)
```

#### Shell Tool (`nanobot/agent/tools/shell.py`)

Secure command execution with safety guards:

```python
class ExecTool(Tool):
    deny_patterns = [
        r"\brm\s+-[rf]{1,2}\b",        # rm -rf
        r"\bdel\s+/[fq]\b",            # del /f
        r"\b(format|mkfs|diskpart)\b", # Disk operations
        r"\b(shutdown|reboot|poweroff)\b",
        r":\(\)\s*\{.*\};\s*:",        # Fork bomb
    ]
```

**Features**:
- Configurable timeout (default: 60s)
- Working directory control
- Path traversal detection
- Output truncation (max 10,000 chars)

#### Web Tools (`nanobot/agent/tools/web.py`)

**Web Search** (Brave Search API):
```python
class WebSearchTool(Tool):
    name = "web_search"
    async def execute(self, query: str, count: int = 5) -> str
```

**Web Fetch** (Readability-based extraction):
```python
class WebFetchTool(Tool):
    name = "web_fetch"
    async def execute(self, url: str, extractMode: str = "markdown") -> str
```

Features:
- HTML to Markdown conversion
- JSON detection
- Content truncation (50,000 chars default)
- Redirect limits (5 max)

#### Message Tool (`nanobot/agent/tools/message.py`)

Send messages through channels:
```python
class MessageTool(Tool):
    async def execute(self, content: str, channel: str = None, to: str = None) -> str
```

#### Spawn Tool (`nanobot/agent/tools/spawn.py`)

Create background subagents:
```python
class SpawnTool(Tool):
    async def execute(self, task: str, label: str = None) -> str
```

#### Cron Tool (`nanobot/agent/tools/cron.py`)

Schedule tasks through natural language:
```python
class CronTool(Tool):
    async def execute(self, command: str, name: str = None, ...) -> str
```

---

## Skills System

### Skills Overview

Skills are markdown files (`SKILL.md`) that teach the agent how to use specific tools or perform tasks.

**Location**:
- Built-in: `nanobot/skills/`
- Custom: `~/.nanobot/workspace/skills/`

### Skill Format

```markdown
---
name: skill-name
description: "Brief description"
metadata: {"nanobot":{"emoji":"🔧","requires":{"bins":["tool"]},"always":true}}
---

# Skill Name

Instructions, examples, and best practices for the agent.

## Usage Examples

```bash
command examples
```
```

### Built-in Skills

| Skill | Description | Requirements |
|-------|-------------|--------------|
| `github` | GitHub via `gh` CLI | `gh` binary |
| `weather` | Weather via wttr.in/Open-Meteo | None |
| `summarize` | Summarize URLs, files, YouTube | None |
| `tmux` | Remote-control tmux | `tmux` binary |
| `cron` | Natural language scheduling | None |
| `skill-creator` | Create new skills | None |

### Skills Loader (`nanobot/agent/skills.py`)

Progressive loading strategy:

```python
class SkillsLoader:
    def list_skills(self, filter_unavailable: bool = True) -> list[dict]
    def load_skill(self, name: str) -> str | None
    def build_skills_summary(self) -> str  # XML format
    def get_always_skills(self) -> list[str]
```

**Requirements Checking**:
```python
def _check_requirements(self, skill_meta: dict) -> bool:
    for b in requires.get("bins", []):
        if not shutil.which(b):
            return False  # Missing binary
    for env in requires.get("env", []):
        if not os.environ.get(env):
            return False  # Missing env var
    return True
```

---

## Scheduling & Automation

### Cron Service (`nanobot/cron/service.py`)

Full-featured job scheduling:

**Schedule Types**:
1. **Interval**: `every N seconds`
2. **Cron Expression**: `0 9 * * *`
3. **One-time**: `at YYYY-MM-DDTHH:MM:SS`

```python
class CronService:
    async def start(self) -> None
    def add_job(self, name: str, schedule: CronSchedule, message: str) -> CronJob
    def remove_job(self, job_id: str) -> bool
    def enable_job(self, job_id: str, enabled: bool) -> CronJob | None
    async def run_job(self, job_id: str, force: bool = False) -> bool
```

**Job Structure**:
```python
@dataclass
class CronJob:
    id: str
    name: str
    enabled: bool
    schedule: CronSchedule
    payload: CronPayload  # message, channel, to
    state: CronJobState   # next_run, last_run, status
    delete_after_run: bool
```

### Heartbeat Service (`nanobot/heartbeat/service.py`)

Periodic agent wake-up for proactive tasks:

```python
class HeartbeatService:
    async def start(self) -> None  # Start loop
    async def _tick(self) -> None  # Read HEARTBEAT.md, execute tasks

HEARTBEAT_PROMPT = """Read HEARTBEAT.md in your workspace.
Follow any instructions or tasks listed there."""
```

**Features**:
- Configurable interval (default: 30 minutes)
- Reads `HEARTBEAT.md` for tasks
- Skips execution if file is empty
- `HEARTBEAT_OK` token indicates no action needed

---

## Session Management

### Session Manager (`nanobot/session/manager.py`)

Persistent conversation history using JSONL files:

```python
class SessionManager:
    def get_or_create(self, key: str) -> Session
    def save(self, session: Session) -> None
    def delete(self, key: str) -> bool
    def list_sessions(self) -> list[dict]
```

**Session Structure**:
```python
@dataclass
class Session:
    key: str  # channel:chat_id
    messages: list[dict]
    created_at: datetime
    updated_at: datetime
    metadata: dict
```

**Storage Format** (JSONL):
```jsonl
{"_type": "metadata", "created_at": "...", "metadata": {...}}
{"role": "user", "content": "Hello!", "timestamp": "..."}
{"role": "assistant", "content": "Hi there!", "timestamp": "..."}
```

**Features**:
- File-based persistence (`~/.nanobot/sessions/`)
- In-memory caching
- Recent message limiting (50 default for LLM context)
- Safe filename encoding

---

## Provider Integration

### LiteLLM Provider (`nanobot/providers/litellm_provider.py`)

Multi-provider support through LiteLLM abstraction:

**Supported Providers**:
- OpenRouter (unified access to all models)
- Anthropic (Claude direct)
- OpenAI (GPT direct)
- DeepSeek
- Groq (includes Whisper transcription)
- Gemini
- Zhipu (GLM)
- Moonshot (Kimi)
- vLLM (self-hosted)

**Provider Detection**:
```python
class LiteLLMProvider(LLMProvider):
    def __init__(self, api_key: str, api_base: str, default_model: str):
        # Auto-detect provider type
        self.is_openrouter = api_key.startswith("sk-or-") or "openrouter" in api_base
        self.is_vllm = bool(api_base) and not self.is_openrouter

        # Set environment variables for LiteLLM
        if self.is_openrouter:
            os.environ["OPENROUTER_API_KEY"] = api_key
```

**Model Prefix Handling**:
```python
# Ensure correct provider prefix
if "gemini" in model.lower() and not model.startswith("gemini/"):
    model = f"gemini/{model}"

if self.is_vllm:
    model = f"hosted_vllm/{model}"
```

### Provider Base (`nanobot/providers/base.py`)

Abstract interface:

```python
class LLMProvider(ABC):
    @abstractmethod
    async def chat(
        self,
        messages: list[dict],
        tools: list[dict] | None = None,
        model: str | None = None,
        max_tokens: int = 4096,
        temperature: float = 0.7,
    ) -> LLMResponse
```

**Response Structure**:
```python
@dataclass
class LLMResponse:
    content: str | None
    tool_calls: list[ToolCallRequest]
    finish_reason: str
    usage: dict
```

### Transcription (`nanobot/providers/transcription.py`)

Voice-to-text via Groq Whisper:

```python
async def transcribe_audio(file_path: str) -> str:
    """Transcribe audio file using Groq's Whisper."""
```

---

## CLI & User Interface

### Commands (`nanobot/cli/commands.py`)

Full CLI using Typer:

| Command | Description |
|---------|-------------|
| `nanobot onboard` | Initialize config & workspace |
| `nanobot agent -m "..."` | Single message mode |
| `nanobot agent` | Interactive chat mode |
| `nanobot gateway` | Start server for channels |
| `nanobot status` | Show configuration status |
| `nanobot cron add/list/remove` | Manage scheduled jobs |
| `nanobot channels status` | Show channel status |
| `nanobot channels login` | WhatsApp QR authentication |

### Workspace Templates

Onboard creates default structure:

```
~/.nanobot/
├── config.json          # Configuration
├── workspace/
│   ├── AGENTS.md        # Agent instructions
│   ├── SOUL.md          # Personality/values
│   ├── USER.md          # User preferences
│   ├── MEMORY.md        # Long-term memory
│   ├── HEARTBEAT.md     # Proactive tasks
│   ├── skills/          # Custom skills
│   └── memory/
│       ├── YYYY-MM-DD.md  # Daily notes
│       └── MEMORY.md      # (symlink/reference)
└── sessions/
    └── *.jsonl          # Conversation history
```

### Rich Console Output

```python
console.print(f"{__logo__} nanobot v{__version__}")
console.print("[green]✓[/green] Created config")
console.print("[yellow]Warning: No channels enabled[/yellow]")
```

---

## Security Features

### 1. Access Control

**Channel-level**:
```python
def is_allowed(self, sender_id: str) -> bool:
    allow_list = getattr(self.config, "allow_from", [])
    if not allow_list:
        return True  # Allow everyone if no list
    return sender_id in allow_list
```

### 2. Workspace Restriction

**Config Option**:
```json
{
  "tools": {
    "restrictToWorkspace": true
  }
}
```

**Effect**:
- All file operations restricted to workspace
- Shell commands cannot access outside workspace
- Path traversal detection (`../`, `..\\`)

### 3. Shell Command Guards

**Blocked Patterns**:
- `rm -rf`, `del /f` (destructive deletion)
- `format`, `mkfs`, `diskpart` (disk operations)
- `shutdown`, `reboot`, `poweroff` (system control)
- Fork bombs

### 4. URL Validation

```python
def _validate_url(url: str) -> tuple[bool, str]:
    if p.scheme not in ('http', 'https'):
        return False, "Only http/https allowed"
    if not p.netloc:
        return False, "Missing domain"
```

---

## Testing Infrastructure

### Test Structure (`tests/`)

```
tests/
├── test_agent/
├── test_channels/
├── test_tools/
└── test_config/
```

### Pytest Configuration

```toml
[tool.pytest.ini_options]
asyncio_mode = "auto"
testpaths = ["tests"]
```

---

## Dependencies

### Core Dependencies

```toml
dependencies = [
    "typer>=0.9.0",           # CLI framework
    "litellm>=1.0.0",         # LLM abstraction
    "pydantic>=2.0.0",        # Config validation
    "pydantic-settings>=2.0.0",
    "websockets>=12.0",       # WebSocket support
    "httpx>=0.25.0",          # HTTP client
    "loguru>=0.7.0",          # Logging
    "readability-lxml>=0.8.0", # Web extraction
    "rich>=13.0.0",           # Console output
    "croniter>=2.0.0",        # Cron expressions
    "python-telegram-bot>=21.0",
    "lark-oapi>=1.0.0",       # Feishu API
]
```

### Development Dependencies

```toml
[tool.optional-dependencies.dev]
pytest>=7.0.0
pytest-asyncio>=0.21.0
ruff>=0.1.0
```

---

## File Structure Summary

```
nanobot/
├── pyproject.toml           # Package configuration
├── README.md                # User documentation
├── Dockerfile               # Container definition
├── bridge/                  # WhatsApp bridge (Node.js)
│   ├── package.json
│   └── src/
│       ├── index.ts
│       ├── server.ts
│       ├── types.d.ts
│       └── whatsapp.ts
├── nanobot/
│   ├── __init__.py
│   ├── __main__.py
│   ├── agent/
│   │   ├── __init__.py
│   │   ├── context.py       # Prompt builder
│   │   ├── loop.py          # Core agent loop
│   │   ├── memory.py        # Memory store
│   │   ├── skills.py        # Skills loader
│   │   ├── subagent.py      # Background tasks
│   │   └── tools/
│   │       ├── __init__.py
│   │       ├── base.py      # Tool ABC
│   │       ├── registry.py  # Tool registry
│   │       ├── cron.py
│   │       ├── filesystem.py
│   │       ├── message.py
│   │       ├── shell.py
│   │       ├── spawn.py
│   │       └── web.py
│   ├── bus/
│   │   ├── __init__.py
│   │   ├── events.py        # Message types
│   │   └── queue.py         # Message bus
│   ├── channels/
│   │   ├── __init__.py
│   │   ├── base.py          # Channel ABC
│   │   ├── discord.py
│   │   ├── feishu.py
│   │   ├── manager.py       # Channel coordinator
│   │   ├── telegram.py
│   │   └── whatsapp.py
│   ├── cli/
│   │   ├── __init__.py
│   │   └── commands.py      # CLI commands
│   ├── config/
│   │   ├── __init__.py
│   │   ├── loader.py        # Config loading
│   │   └── schema.py        # Pydantic models
│   ├── cron/
│   │   ├── __init__.py
│   │   ├── service.py       # Scheduling
│   │   └── types.py         # Cron types
│   ├── heartbeat/
│   │   ├── __init__.py
│   │   └── service.py       # Periodic wake-up
│   ├── providers/
│   │   ├── __init__.py
│   │   ├── base.py          # Provider ABC
│   │   ├── litellm_provider.py
│   │   └── transcription.py
│   ├── session/
│   │   ├── __init__.py
│   │   └── manager.py       # Session storage
│   ├── skills/
│   │   ├── README.md
│   │   ├── github/SKILL.md
│   │   ├── weather/SKILL.md
│   │   ├── summarize/SKILL.md
│   │   ├── tmux/SKILL.md
│   │   └── cron/SKILL.md
│   └── utils/
│       ├── __init__.py
│       └── helpers.py
└── tests/
    └── ...
```

---

## Line Count

```bash
# Core agent lines (excluding tests, docs, dependencies)
bash core_agent_lines.sh
# Result: ~3,428 lines
```

---

## Design Patterns Used

1. **Abstract Factory**: Tool creation with validation
2. **Observer**: Message bus pub-sub pattern
3. **Strategy**: Provider abstraction (LiteLLM)
4. **Template Method**: Channel base class
5. **Singleton**: Session manager per key
6. **Command**: Tool execution pattern
7. **Repository**: Cron job storage

---

## Key Architectural Decisions

### 1. Async-First Design
All I/O operations use asyncio for concurrency without threading overhead.

### 2. Message Bus Decoupling
Channels and agent communicate through queues, enabling:
- Independent scaling
- Easy channel additions
- Clean separation of concerns

### 3. Progressive Skill Loading
- Always-loaded skills: Full content in prompt
- Available skills: XML summary (agent reads on demand)
- Reduces token usage while maintaining discoverability

### 4. File-Based Persistence
- Sessions: JSONL for easy streaming
- Memory: Markdown for human readability
- Config: JSON with Pydantic validation

### 5. Security by Default
- Workspace restriction optional but recommended
- Shell command guards always active
- Access control per channel

---

## Comparison with Clawdbot

| Aspect | Clawdbot | Nanobot |
|--------|----------|---------|
| Lines of Code | 430,000+ | ~3,400 |
| Language | TypeScript/Python | Python |
| Architecture | Microservices | Monolithic (modular) |
| Target Use | Enterprise | Personal/Research |
| Learning Curve | Steep | Gentle |
| Extensibility | Plugin system | Skills + code |
| Deploy | Kubernetes | Single command |

---

## Conclusion

nanobot demonstrates that a production-grade AI assistant can be built with minimal code while maintaining:
- Full feature parity for core use cases
- Clean, maintainable architecture
- Security and access control
- Multi-provider and multi-channel support
- Scheduling and automation
- Persistent memory and sessions

The codebase serves as an excellent educational resource for understanding:
1. Async Python patterns
2. Event-driven architecture
3. LLM agent design
4. Tool integration
5. Multi-platform chat bot development
