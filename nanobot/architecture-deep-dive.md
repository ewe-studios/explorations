# Nanobot Architecture Deep-Dive

## Overview

This document provides an in-depth analysis of nanobot's architectural patterns, component interactions, and design decisions. The codebase exemplifies modern async Python design with clean separation of concerns.

---

## System Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           EXTERNAL INTERFACES                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │   Telegram  │  │  WhatsApp   │  │   Discord   │  │    Feishu   │    │
│  │   Channel   │  │   Channel   │  │   Channel   │  │   Channel   │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │                │            │
│         └────────────────┴────────┬───────┴────────────────┘            │
│                                   │                                     │
│                          ┌────────▼────────┐                            │
│                          │  Channel Manager │                            │
│                          │   (Coordinator)  │                            │
│                          └────────┬─────────┘                            │
│                                   │                                     │
│         ┌─────────────────────────┼─────────────────────────┐           │
│         │                         │                         │           │
│  ┌──────▼─────────┐      ┌────────▼────────┐      ┌────────▼──────┐    │
│  │  Inbound Queue │      │   Message Bus   │      │ Outbound Queue│    │
│  │  (asyncio.Queue)│     │  (Pub/Sub Core) │      │ (asyncio.Queue)│   │
│  └──────┬─────────┘      └────────┬────────┘      └────────┬──────┘    │
│         │                         │                         │           │
│         └─────────────────────────┼─────────────────────────┘           │
│                                   │                                     │
│                          ┌────────▼─────────┐                           │
│                          │    Agent Loop    │                           │
│                          │  (Core Processor)│                           │
│                          └────────┬─────────┘                           │
│                                   │                                     │
│         ┌─────────────────────────┼─────────────────────────┐           │
│         │                         │                         │           │
│  ┌──────▼─────────┐      ┌────────▼────────┐      ┌────────▼──────┐    │
│  │  Tool Registry │      │ Context Builder │      │   Provider    │    │
│  │                │      │                 │      │  (LiteLLM)    │    │
│  │  - filesystem  │      │  - Bootstrap    │      │               │    │
│  │  - shell       │      │  - Memory       │      │  - OpenRouter │    │
│  │  - web         │      │  - Skills       │      │  - Anthropic  │    │
│  │  - message     │      │  - History      │      │  - OpenAI     │    │
│  │  - spawn       │      │                 │      │  - etc.       │    │
│  │  - cron        │      │                 │      │               │    │
│  └────────────────┘      └─────────────────┘      └─────────────────┘    │
│                                                                       │
│  ┌─────────────────────────────────────────────────────────────────┐  │
│  │                    SUPPORTING SERVICES                           │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │  │
│  │  │  Cron    │  │Heartbeat │  │ Session  │  │  Memory  │        │  │
│  │  │ Service  │  │ Service  │  │ Manager  │  │  Store   │        │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘        │  │
│  └─────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Message Flow Architecture

### Inbound Message Flow

```
User Message
     │
     ▼
┌─────────────┐
│   Channel   │  1. Receive from external API
│  (Telegram) │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│BaseChannel  │  2. Access control (is_allowed)
│._handle_msg │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Message Bus │  3. Publish to inbound queue
│  (publish)  │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Agent Loop │  4. Consume from inbound queue
│  (consume)  │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Session   │  5. Get/create session for context
│   Manager   │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Context   │  6. Build system prompt + messages
│   Builder   │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Provider  │  7. Call LLM with tools
│  (LiteLLM)  │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Tool Registry│  8. Execute requested tools
│  (execute)  │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Agent     │  9. Iterate until completion
│   Loop      │     (max_iterations)
│ (continue)  │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Session   │  10. Save conversation history
│   Manager   │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Message Bus │  11. Publish response to outbound
│  (publish)  │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Channel   │  12. Send to user
│   Manager   │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Channel   │  13. External API send
│  (Telegram) │
└──────┬──────┘
       │
       ▼
  User Response
```

### Sequence Diagram: Tool Execution

```
Agent Loop          Tool Registry       Tool (Filesystem)      LLM Provider
    │                    │                     │                     │
    │──has_tool_calls──>│                     │                     │
    │                    │                     │                     │
    │──execute(name,───>│                     │                     │
    │   params)          │                     │                     │
    │                    │                     │                     │
    │                    │──validate_params──>│                     │
    │                    │                     │                     │
    │                    │<──errors/ok────────│                     │
    │                    │                     │                     │
    │                    │──execute(**kwargs)─>                     │
    │                    │                     │                     │
    │                    │                     │  _resolve_path()    │
    │                    │                     │  file.read_text()   │
    │                    │                     │                     │
    │                    │<──result string────│                     │
    │<──result string────│                     │                     │
    │                    │                     │                     │
    │──add_tool_result──>│                     │                     │
    │   (to messages)    │                     │                     │
    │                    │                     │                     │
    │──chat(messages)──────────────────────────────────────────────>│
    │                    │                     │                     │
    │<──LLMResponse─────────────────────────────────────────────────│
    │                    │                     │                     │
```

---

## Component Deep-Dive

### 1. Message Bus Pattern

The message bus implements a **dual-queue pub-sub pattern**:

```python
class MessageBus:
    def __init__(self):
        # Two independent async queues
        self.inbound: asyncio.Queue[InboundMessage] = asyncio.Queue()
        self.outbound: asyncio.Queue[OutboundMessage] = asyncio.Queue()

        # Pub-sub for outbound messages
        self._outbound_subscribers: dict[str, list[Callable]] = {}
```

**Design Decisions**:

| Decision | Rationale |
|----------|-----------|
| Separate queues | Clear directionality, easier debugging |
| asyncio.Queue | Non-blocking, natural async integration |
| Subscriber dict per channel | Targeted message delivery |
| 1-second timeout | Prevent infinite blocking, enable graceful shutdown |

**Concurrency Model**:
```
Producer (Channel)          Consumer (Agent)
     │                           │
     │──publish_inbound────────>│
     │    (queue.put)            │    (queue.get)
     │                           │
     │                           │──process_message()
     │                           │
     │<──publish_outbound────────│
     │    (queue.put)            │    (queue.get)
     │                           │
```

### 2. Agent Loop State Machine

The agent loop is a **finite state machine** with iteration limiting:

```python
async def _process_message(self, msg: InboundMessage) -> OutboundMessage:
    iteration = 0

    while iteration < self.max_iterations:  # State: RUNNING
        iteration += 1

        # State: LLM_CALL
        response = await self.provider.chat(...)

        if response.has_tool_calls:  # State: TOOL_EXECUTION
            # Execute all tools
            for tool_call in response.tool_calls:
                result = await self.tools.execute(...)
                messages = self.context.add_tool_result(...)
        else:  # State: DONE
            final_content = response.content
            break

    # State: COMPLETE
    return OutboundMessage(...)
```

**State Diagram**:
```
                    ┌──────────────┐
                    │    START     │
                    └──────┬───────┘
                           │
                           ▼
                    ┌──────────────┐
         ┌─────────│  LLM_CALL    │─────────┐
         │         └──────┬───────┘         │
         │                │                 │
         │         has_tool_calls?          │
         │           ╱    │    ╲            │
         │         ╱      │      ╲          │
         │       YES     NO      ERROR      │
         │       ╱        │        ╲         │
         │      ╱         │         ╲        │
         │     ▼          │          ▼       │
         │ ┌──────────┐   │    ┌──────────┐  │
         │ │TOOL_EXEC │   │    │  ERROR   │  │
         │ └────┬─────┘   │    └────┬─────┘  │
         │      │         │         │        │
         │      └─────────┴─────────┘        │
         │                │                  │
         └────────────────┴──────────────────┘
                          │
                    iteration >= max?
                          │
                    ┌─────┴─────┐
                   YES         NO
                    │           │
                    ▼           │
             ┌──────────┐       │
             │ COMPLETE │       │
             └──────────┘       │
                                │
```

### 3. Context Builder Pipeline

The context builder assembles prompts through a **pipeline pattern**:

```python
def build_system_prompt(self) -> str:
    parts = []

    # Stage 1: Identity (always present)
    parts.append(self._get_identity())

    # Stage 2: Bootstrap files (if exist)
    bootstrap = self._load_bootstrap_files()
    if bootstrap:
        parts.append(bootstrap)

    # Stage 3: Memory (if exists)
    memory = self.memory.get_memory_context()
    if memory:
        parts.append(f"# Memory\n\n{memory}")

    # Stage 4: Skills (progressive loading)
    always_skills = self.skills.get_always_skills()
    if always_skills:
        parts.append(self.skills.load_skills_for_context(always_skills))

    # Stage 5: Available skills summary
    skills_summary = self.skills.build_skills_summary()
    if skills_summary:
        parts.append(f"# Skills\n\n{skills_summary}")

    # Combine with separators
    return "\n\n---\n\n".join(parts)
```

**Progressive Skill Loading Strategy**:

```
┌─────────────────────────────────────────────────────────────┐
│                     SKILL LOADING TIERS                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Tier 1: Always-Loaded                                       │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ Full content included in system prompt              │    │
│  │ Examples: core tools, essential capabilities        │    │
│  │ Token Cost: HIGH                                    │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  Tier 2: Available Summary                                   │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ XML summary with name, description, location        │    │
│  │ Agent uses read_file tool to load full skill        │    │
│  │ Token Cost: LOW (on-demand)                         │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 4. Tool Execution Pipeline

Tools follow a **command pattern with validation**:

```python
class ToolRegistry:
    async def execute(self, name: str, params: dict) -> str:
        tool = self._tools.get(name)
        if not tool:
            return f"Error: Tool '{name}' not found"

        # Stage 1: Validation
        errors = tool.validate_params(params)
        if errors:
            return f"Error: Invalid parameters: " + "; ".join(errors)

        # Stage 2: Execution
        try:
            return await tool.execute(**params)
        except Exception as e:
            return f"Error executing {name}: {str(e)}"
```

**Tool Validation Flow**:
```
┌──────────────┐
│Tool Request  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│JSON Schema   │  parameters = {
│Validation    │      "type": "object",
└──────┬───────┘      "properties": {...},
       │              "required": [...]
       │          }
       ▼
┌──────────────┐
│Type Check    │  string, integer, number,
└──────┬───────┘  boolean, array, object
       │
       ▼
┌──────────────┐
│Constraint    │  minLength, maxLength,
│Validation    │  minimum, maximum, enum
└──────┬───────┘
       │
       ▼
┌──────────────┐
│Required      │  All required fields present?
│Fields Check  │
└──────┬───────┘
       │
       │ Valid → Execute
       │ Invalid → Return error
       ▼
```

### 5. Subagent Lifecycle

Subagents implement a **background task pattern**:

```python
class SubagentManager:
    async def spawn(self, task: str, label: str = None) -> str:
        task_id = str(uuid.uuid4())[:8]

        # Create async task
        bg_task = asyncio.create_task(
            self._run_subagent(task_id, task, label, origin)
        )

        # Store reference
        self._running_tasks[task_id] = bg_task

        # Cleanup on completion
        bg_task.add_done_callback(
            lambda _: self._running_tasks.pop(task_id, None)
        )

        return f"Subagent started (id: {task_id})"
```

**Subagent Lifecycle States**:
```
SPAWN Request
     │
     ▼
┌──────────────┐
│ Create Task  │  asyncio.create_task()
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  RUNNING     │  Execute tool calls
│              │  Limited iterations (15)
└──────┬───────┘
       │
       │ Completion
       ▼
┌──────────────┐
│  ANNOUNCE    │  Send result to main agent
│  RESULT      │  via message bus (system channel)
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  CLEANUP     │  Remove from running_tasks
└──────┬───────┘
```

---

## Data Structures

### Message Types

```python
@dataclass
class InboundMessage:
    """Immutable message from channel to agent."""
    channel: str           # telegram, discord, whatsapp, feishu
    sender_id: str         # User identifier
    chat_id: str           # Channel/group identifier
    content: str           # Message text
    timestamp: datetime    # Auto-set
    media: list[str]       # Image/video URLs
    metadata: dict         # Channel-specific data

    @property
    def session_key(self) -> str:
        return f"{self.channel}:{self.chat_id}"  # Composite key

@dataclass
class OutboundMessage:
    """Immutable message from agent to channel."""
    channel: str
    chat_id: str
    content: str
    reply_to: str | None   # Optional reply threading
    media: list[str]
    metadata: dict
```

### Session Storage

**JSONL Format** (line-delimited JSON):
```jsonl
{"_type": "metadata", "created_at": "2025-01-15T10:00:00", "updated_at": "...", "metadata": {}}
{"role": "user", "content": "Hello!", "timestamp": "2025-01-15T10:00:01"}
{"role": "assistant", "content": "Hi there!", "timestamp": "2025-01-15T10:00:02"}
{"role": "user", "content": "How are you?", "timestamp": "2025-01-15T10:00:05"}
{"role": "assistant", "content": "I'm doing well!", "timestamp": "2025-01-15T10:00:06"}
```

**Why JSONL?**
- Append-only writes (efficient)
- Stream reading for partial loads
- Human-readable for debugging
- Easy to truncate old messages

### Cron Job Structure

```python
@dataclass
class CronJob:
    id: str                           # UUID[:8]
    name: str                         # Human-readable name
    enabled: bool                     # Active status
    schedule: CronSchedule            # When to run
    payload: CronPayload              # What to execute
    state: CronJobState               # Execution history
    created_at_ms: int                # Creation timestamp
    updated_at_ms: int                # Last modification
    delete_after_run: bool            # One-shot flag

@dataclass
class CronSchedule:
    kind: Literal["at", "every", "cron"]
    at_ms: int | None                 # One-time execution
    every_ms: int | None              # Interval
    expr: str | None                  # Cron expression
    tz: str | None                    # Timezone

@dataclass
class CronPayload:
    kind: Literal["agent_turn"]       # Currently only agent_turn
    message: str                      # Prompt to agent
    deliver: bool                     # Send response to channel?
    channel: str | None               # Target channel
    to: str | None                    # Target user/chat

@dataclass
class CronJobState:
    next_run_at_ms: int | None        # Next scheduled time
    last_run_at_ms: int | None        # Last execution time
    last_status: str | None           # "ok" or "error"
    last_error: str | None            # Error message if failed
```

---

## Concurrency Model

### Async Architecture

nanobot uses **asyncio** throughout for non-blocking I/O:

```
Main Event Loop
     │
     ├── Agent Loop (async task)
     │    └── Waits on inbound queue
     │
     ├── Channel Manager (async task)
     │    ├── Telegram listener
     │    ├── WhatsApp bridge
     │    ├── Discord gateway
     │    └── Feishu WebSocket
     │
     ├── Cron Service (async task)
     │    └── Timer-based wake-up
     │
     └── Heartbeat Service (async task)
          └── Interval-based wake-up
```

### Task Coordination

```python
async def run():
    # Start services
    await cron.start()
    await heartbeat.start()

    # Run concurrently until shutdown
    await asyncio.gather(
        agent.run(),           #永久的
        channels.start_all(),  #永久的
    )
```

**Shutdown Sequence**:
```python
async def shutdown():
    heartbeat.stop()    # Cancel timer
    cron.stop()         # Cancel timer
    agent.stop()        # Set _running = False
    await channels.stop_all()  # Close connections
```

---

## Security Architecture

### Defense in Depth

```
┌─────────────────────────────────────────────────────────────┐
│                    SECURITY LAYERS                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Layer 1: Access Control                                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ Channel-level allowFrom lists                       │    │
│  │ Sender ID validation                                 │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  Layer 2: Workspace Restriction                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ All file operations validated against allowed_dir   │    │
│  │ Shell commands check for path traversal             │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  Layer 3: Command Guards                                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ Regex patterns block dangerous commands             │    │
│  │ rm -rf, format, shutdown, etc.                      │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  Layer 4: URL Validation                                     │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ Scheme validation (http/https only)                 │    │
│  │ Domain presence check                               │    │
│  │ Redirect limits                                     │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│  Layer 5: Resource Limits                                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ Tool execution timeout (60s default)                │    │
│  │ Output truncation (10,000 chars)                    │    │
│  │ Max iterations (20 default)                         │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Path Resolution Security

```python
def _resolve_path(path: str, allowed_dir: Path | None = None) -> Path:
    """Resolve path with optional directory restriction."""
    resolved = Path(path).expanduser().resolve()

    if allowed_dir:
        # Check if resolved path starts with allowed directory
        if not str(resolved).startswith(str(allowed_dir.resolve())):
            raise PermissionError(
                f"Path {path} is outside allowed directory {allowed_dir}"
            )

    return resolved
```

### Shell Command Guard

```python
def _guard_command(self, command: str, cwd: str) -> str | None:
    deny_patterns = [
        r"\brm\s+-[rf]{1,2}\b",        # rm -rf, rm -r, rm -fr
        r"\bdel\s+/[fq]\b",            # del /f, del /q
        r"\b(format|mkfs|diskpart)\b", # Disk operations
        r"\bdd\s+if=",                 # dd command
        r">\s*/dev/sd",                # Write to disk
        r"\b(shutdown|reboot|poweroff)\b",
        r":\(\)\s*\{.*\};\s*:",        # Fork bomb
    ]

    for pattern in deny_patterns:
        if re.search(pattern, command.lower()):
            return "Error: Command blocked by safety guard"

    # Path traversal check
    if self.restrict_to_workspace:
        if "..\\" in command or "../" in command:
            return "Error: Path traversal detected"

    return None
```

---

## Configuration Architecture

### Hierarchical Configuration

```
┌─────────────────────────────────────────────────────────────┐
│                  CONFIGURATION SOURCES                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Priority 1: Environment Variables (highest)                │
│  NANOBOT_PROVIDERS__OPENROUTER__API_KEY=sk-or-xxx           │
│                                                              │
│  Priority 2: Config File (~/.nanobot/config.json)           │
│  {                                                           │
│    "providers": {                                            │
│      "openrouter": { "apiKey": "sk-or-xxx" }                 │
│    }                                                         │
│  }                                                           │
│                                                              │
│  Priority 3: Defaults (lowest)                              │
│  Pydantic model defaults                                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Key Transformation Pipeline

```python
# JSON (camelCase) → Pydantic (snake_case)
def convert_keys(data: Any) -> Any:
    if isinstance(data, dict):
        return {camel_to_snake(k): convert_keys(v) for k, v in data.items()}
    return data

# Pydantic (snake_case) → JSON (camelCase)
def convert_to_camel(data: Any) -> Any:
    if isinstance(data, dict):
        return {snake_to_camel(k): convert_to_camel(v) for k, v in data.items()}
    return data
```

---

## Provider Abstraction

### LiteLLM Adapter Pattern

```python
class LiteLLMProvider(LLMProvider):
    """Adapter for multi-provider support via LiteLLM."""

    def __init__(self, api_key: str, api_base: str, default_model: str):
        # Detect provider type
        self.is_openrouter = (
            api_key.startswith("sk-or-") or
            (api_base and "openrouter" in api_base)
        )

        self.is_vllm = bool(api_base) and not self.is_openrouter

        # Set environment variables for LiteLLM
        if self.is_openrouter:
            os.environ["OPENROUTER_API_KEY"] = api_key
        elif self.is_vllm:
            os.environ["OPENAI_API_KEY"] = api_key
```

**Model Prefix Handling**:
```
User Input: "claude-opus-4-5"
     │
     ▼
┌─────────────────┐
│ Provider Detect │  → is_openrouter = True
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Add Prefix      │  → "openrouter/claude-opus-4-5"
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ LiteLLM Call    │  acompletion(model="openrouter/...")
└─────────────────┘
```

---

## Extension Points

### Adding a New Channel

1. Create `nanobot/channels/newchannel.py`
2. Implement `BaseChannel` abstract methods
3. Add config schema in `ChannelsConfig`
4. Register in `ChannelManager._init_channels()`

```python
class NewChannel(BaseChannel):
    name = "newchannel"

    async def start(self) -> None:
        self._running = True
        # Connect to platform
        # Listen for messages
        # Call _handle_message() for each

    async def stop(self) -> None:
        self._running = False
        # Close connection

    async def send(self, msg: OutboundMessage) -> None:
        # Send message via platform API
```

### Adding a New Tool

1. Create `nanobot/agent/tools/newtool.py`
2. Inherit from `Tool` base class
3. Implement abstract methods
4. Register in `AgentLoop._register_default_tools()`

```python
class NewTool(Tool):
    @property
    def name(self) -> str:
        return "new_tool"

    @property
    def description(self) -> str:
        return "Description of what the tool does"

    @property
    def parameters(self) -> dict[str, Any]:
        return {
            "type": "object",
            "properties": {
                "param1": {"type": "string", "description": "..."}
            },
            "required": ["param1"]
        }

    async def execute(self, param1: str, **kwargs) -> str:
        # Implementation
        return result
```

### Adding a New Skill

1. Create directory `nanobot/skills/skillname/`
2. Create `SKILL.md` with frontmatter and instructions

```markdown
---
name: skillname
description: "What this skill does"
metadata: {"nanobot":{"emoji":"🔧","requires":{"bins":["tool"]}}}
---

# Skill Name

Instructions and examples for using this skill.
```

---

## Performance Considerations

### Memory Management

1. **Session Caching**: In-memory cache with disk persistence
2. **Message Truncation**: 50 messages max for LLM context
3. **Tool Output Limits**: 10,000 chars max

### Async Efficiency

1. **Queue-based Processing**: Non-blocking message handling
2. **Concurrent Channels**: All channels run as async tasks
3. **Timeout Protection**: All I/O has timeouts

### Token Optimization

1. **Progressive Skill Loading**: Only load skills on demand
2. **Recent Memory**: Last 7 days by default
3. **History Limiting**: Configurable max messages

---

## Testing Strategy

### Test Categories

```
tests/
├── test_agent/          # Agent loop, context, memory
├── test_channels/       # Channel implementations
├── test_tools/          # Tool execution, validation
├── test_config/         # Config loading, validation
└── test_integration/    # End-to-end tests
```

### Mocking Strategy

```python
# Mock LLM provider for testing
class MockProvider(LLMProvider):
    async def chat(self, messages, tools=None, model=None):
        return LLMResponse(content="Mock response")

# Mock message bus
class MockBus(MessageBus):
    async def publish_outbound(self, msg):
        self.sent_messages.append(msg)
```

---

## Deployment Patterns

### Docker Deployment

```dockerfile
FROM python:3.11-slim

WORKDIR /app
COPY pyproject.toml .
RUN pip install .

COPY nanobot/ /app/nanobot/

CMD ["nanobot", "gateway"]
```

### Volume Mounting

```bash
docker run -v ~/.nanobot:/root/.nanobot nanobot gateway
```

### Process Management

Recommended process layout:
```
┌─────────────────────────────────────┐
│         Process Supervisor          │
│         (systemd, supervisor, etc.) │
└────────────────┬────────────────────┘
                 │
        ┌────────┼────────┐
        │        │        │
        ▼        ▼        ▼
   ┌────────┐ ┌────────┐ ┌──────────┐
   │Gateway │ │ Bridge │ │  Watchdog│
   │  :18790│ │ :3001  │ │ (health) │
   └────────┘ └────────┘ └──────────┘
```

---

## Conclusion

nanobot's architecture demonstrates:

1. **Clean Separation**: Message bus decouples all components
2. **Async-First**: Non-blocking I/O throughout
3. **Security Layers**: Defense in depth approach
4. **Extensibility**: Clear patterns for adding channels, tools, skills
5. **Observability**: Structured logging, clear error handling
6. **Resource Management**: Timeouts, limits, cleanup

The codebase serves as an excellent reference for:
- Async Python application design
- Event-driven architectures
- LLM agent implementation
- Multi-platform chatbot development
