---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.NousResearch/hermes-agent
repository: git@github.com:NousResearch/hermes-agent.git
explored_at: 2026-03-25
---

# Tools System and Composition Deep Dive

Hermes Agent has 40+ tools organized into composable toolsets. This document explores the tools system architecture, tool composition patterns, and how tools are interconnected.

## Tool Organization

```
tools/
├── registry.py           # Central registry (~200 lines)
├── approval.py           # Dangerous command detection
├── terminal_tool.py      # Terminal orchestration (~1700 lines)
├── process_registry.py   # Background process management
├── file_tools.py         # File read/write/search/patch
├── web_tools.py          # Web search/extract
├── browser_tool.py       # Browser automation
├── code_execution_tool.py # Code sandbox
├── delegate_tool.py      # Subagent delegation
├── mcp_tool.py           # MCP client
├── memory_tool.py        # Persistent memory
├── todo_tool.py          # Task planning
├── skills_hub.py         # Skill marketplace
├── tts_tool.py           # Text-to-speech
├── vision_tools.py       # Image analysis
├── image_generation_tool.py # Image generation
├── mixture_of_agents_tool.py # MoA orchestration
├── cronjob_tools.py      # Scheduled tasks
├── send_message_tool.py  # Cross-platform messaging
├── honcho_tools.py       # Honcho user modeling
├── homeassistant_tool.py # Smart home control
├── rl_training_tool.py   # RL training environments
├── session_search_tool.py # Session history search
└── environments/         # Terminal backends
    ├── local.py
    ├── docker.py
    ├── ssh.py
    ├── modal.py
    ├── daytona.py
    └── singularity.py
```

## Tool Classification

### By Execution Pattern

| Pattern | Tools | Description |
|---------|-------|-------------|
| **Sync** | file_tools, memory, todo | Return immediately |
| **Async** | web_tools, browser_tool | Await external API |
| **Long-running** | terminal, process | Managed lifecycle |
| **Interactive** | clarify | Requires user input |

### By Safety Level

| Level | Tools | Approval Required |
|-------|-------|-------------------|
| **Safe** | web_search, vision_analyze | Never |
| **Read-only** | read_file, search_files | Never |
| **Write** | write_file, terminal | Context-dependent |
| **Destructive** | rm, chmod 777 | Always |

### By Scope

| Scope | Tools | Sharing |
|-------|-------|---------|
| **Per-session** | todo, memory | Isolated |
| **Per-process** | process_registry | Shared |
| **Global** | send_message | Shared |

## Tool Composition

### Parallel Execution Safety

```python
# model_tools.py

# Tools that must NEVER run concurrently
_NEVER_PARALLEL_TOOLS = frozenset({"clarify"})

# Read-only tools safe for parallel execution
_PARALLEL_SAFE_TOOLS = frozenset({
    "ha_get_state",
    "ha_list_entities",
    "ha_list_services",
    "honcho_context",
    "honcho_profile",
    "honcho_search",
    "read_file",
    "search_files",
    "session_search",
    "skill_view",
    "skills_list",
    "vision_analyze",
    "web_extract",
    "web_search",
})

# File tools can run concurrently on independent paths
_PATH_SCOPED_TOOLS = frozenset({"read_file", "write_file", "patch"})
```

### Path Overlap Detection

```python
def _paths_overlap(left: Path, right: Path) -> bool:
    """Return True when two paths may refer to same subtree."""
    left_parts = left.parts
    right_parts = right.parts

    if not left_parts or not right_parts:
        return bool(left_parts) == bool(right_parts) and bool(left_parts)

    common_len = min(len(left_parts), len(right_parts))
    return left_parts[:common_len] == right_parts[:common_len]


def _extract_parallel_scope_path(
    tool_name: str, function_args: dict
) -> Path | None:
    """Return normalized file target for path-scoped tools."""
    if tool_name not in _PATH_SCOPED_TOOLS:
        return None

    raw_path = function_args.get("path")
    if not isinstance(raw_path, str) or not raw_path.strip():
        return None

    return Path(raw_path).expanduser()
```

### Destructive Command Detection

```python
_DESTRUCTIVE_PATTERNS = re.compile(
    r"""(?:^|\s|&&|\|\||;|`)(?:
        rm\s|rmdir\s|
        mv\s|
        sed\s+-i|
        truncate\s|
        dd\s|
        shred\s|
        git\s+(?:reset|clean|checkout)\s
    )""",
    re.VERBOSE,
)

# Output redirects that overwrite files
_REDIRECT_OVERWRITE = re.compile(r'[^>]>[^>]')


def _is_destructive_command(cmd: str) -> bool:
    """Heuristic: does this command modify/delete files?"""
    if not cmd:
        return False
    if _DESTRUCTIVE_PATTERNS.search(cmd):
        return True
    if _REDIRECT_OVERWRITE.search(cmd):
        return True
    return False
```

### Parallel Batch Safety Check

```python
def _should_parallelize_tool_batch(tool_calls) -> bool:
    """Return True when batch is safe to run concurrently."""
    if len(tool_calls) <= 1:
        return False

    tool_names = [tc.function.name for tc in tool_calls]

    # Check for interactive tools
    if any(name in _NEVER_PARALLEL_TOOLS for name in tool_names):
        return False

    reserved_paths: list[Path] = []
    for tool_call in tool_calls:
        tool_name = tool_call.function.name
        try:
            function_args = json.loads(tool_call.function.arguments)
        except Exception:
            logger.debug("Could not parse args — sequential")
            return False

        if tool_name in _PATH_SCOPED_TOOLS:
            scoped_path = _extract_parallel_scope_path(
                tool_name, function_args
            )
            if scoped_path is None:
                return False
            if any(_paths_overlap(scoped_path, existing)
                   for existing in reserved_paths):
                return False
            reserved_paths.append(scoped_path)
            continue

        if tool_name not in _PARALLEL_SAFE_TOOLS:
            return False

    return True
```

## Toolsets System

### Core Toolsets

```python
# toolsets.py

_HERMES_CORE_TOOLS = [
    # Web
    "web_search", "web_extract",
    # Terminal + process management
    "terminal", "process",
    # File manipulation
    "read_file", "write_file", "patch", "search_files",
    # Vision + image generation
    "vision_analyze", "image_generate",
    # MoA
    "mixture_of_agents",
    # Skills
    "skills_list", "skill_view", "skill_manage",
    # Browser automation
    "browser_navigate", "browser_snapshot", "browser_click",
    "browser_type", "browser_scroll", "browser_back",
    "browser_press", "browser_close", "browser_get_images",
    "browser_vision", "browser_console",
    # Text-to-speech
    "text_to_speech",
    # Planning & memory
    "todo", "memory",
    # Session history search
    "session_search",
    # Clarifying questions
    "clarify",
    # Code execution + delegation
    "execute_code", "delegate_task",
    # Cronjob management
    "cronjob",
    # Cross-platform messaging
    "send_message",
    # Honcho memory tools
    "honcho_context", "honcho_profile", "honcho_search", "honcho_conclude",
    # Home Assistant
    "ha_list_entities", "ha_get_state", "ha_list_services", "ha_call_service",
]
```

### Toolset Definitions

```python
TOOLSETS = {
    "web": {
        "description": "Web research and content extraction tools",
        "tools": ["web_search", "web_extract"],
        "includes": []
    },

    "terminal": {
        "description": "Terminal/command execution and process management",
        "tools": ["terminal", "process"],
        "includes": []
    },

    "file": {
        "description": "File manipulation: read, write, patch, search",
        "tools": ["read_file", "write_file", "patch", "search_files"],
        "includes": []
    },

    "debugging": {
        "description": "Debugging and troubleshooting toolkit",
        "tools": ["terminal", "process"],
        "includes": ["web", "file"]  # Composition!
    },

    "hermes-cli": {
        "description": "Full interactive CLI toolset",
        "tools": _HERMES_CORE_TOOLS,
        "includes": []
    },

    "hermes-telegram": {
        "description": "Telegram bot toolset",
        "tools": _HERMES_CORE_TOOLS,
        "includes": []
    },
}
```

### Recursive Resolution

```python
def resolve_toolset(name: str, visited: Set[str] = None) -> List[str]:
    """Recursively resolve a toolset to get all tool names.

    Handles toolset composition by recursively resolving
    included toolsets and combining all tools.
    """
    if visited is None:
        visited = set()

    # Special alias: all tools across every toolset
    if name in {"all", "*"}:
        all_tools: Set[str] = set()
        for toolset_name in get_toolset_names():
            resolved = resolve_toolset(toolset_name, visited.copy())
            all_tools.update(resolved)
        return list(all_tools)

    # Cycle detection
    if name in visited:
        return []  # Diamond dep or genuine cycle — safe to skip

    visited.add(name)

    toolset = TOOLSETS.get(name)
    if not toolset:
        # Fall back to plugin-provided toolsets
        if name in _get_plugin_toolset_names():
            try:
                from tools.registry import registry
                return [
                    e.name for e in registry._tools.values()
                    if e.toolset == name
                ]
            except Exception:
                pass
        return []

    # Collect direct tools
    tools = set(toolset.get("tools", []))

    # Recursively resolve included toolsets
    for included_name in toolset.get("includes", []):
        included_tools = resolve_toolset(included_name, visited)
        tools.update(included_tools)

    return list(tools)
```

### Multiple Toolset Resolution

```python
def resolve_multiple_toolsets(toolset_names: List[str]) -> List[str]:
    """Resolve multiple toolsets and combine their tools."""
    all_tools = set()

    for name in toolset_names:
        tools = resolve_toolset(name)
        all_tools.update(tools)

    return list(all_tools)
```

## Tool Interconnections

### Cross-Tool References

Some tools reference other tools dynamically:

```python
# browser_tool.py — post-processing in model_tools.py

def get_tool_definitions(enabled_toolsets, disabled_toolsets, quiet_mode):
    # ... collect tool definitions ...

    # Inject cross-references dynamically
    if "browser_navigate" in tool_defs:
        if "execute_code" in available_tools:
            tool_defs["browser_navigate"]["description"] += (
                " For complex automation, consider using execute_code "
                "to script browser actions in Python."
            )

    if "execute_code" in tool_defs:
        if "terminal" in available_tools:
            tool_defs["execute_code"]["description"] += (
                " For shell commands, use terminal instead."
            )

    return tool_defs
```

### Shared State: Process Registry

```python
# tools/process_registry.py

class ProcessRegistry:
    """Track background processes across tool calls.

    Used by terminal tool for background=true processes.
    Singleton shared across AIAgent instances.
    """

    _instance: Optional["ProcessRegistry"] = None
    _lock = threading.Lock()

    def __new__(cls):
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = super().__new__(cls)
                    cls._instance._processes = {}
                    cls._instance._locks = defaultdict(threading.Lock)
        return cls._instance

    def register(self, pid: int, info: dict):
        with self._lock:
            self._processes[pid] = {
                **info,
                "started_at": time.time(),
                "status": "running",
            }

    def get(self, pid: int) -> Optional[dict]:
        return self._processes.get(pid)

    def list_processes(self) -> List[dict]:
        return list(self._processes.values())
```

### Shared State: Last Resolved Tool Names

```python
# model_tools.py — process-global

_last_resolved_tool_names: Optional[Set[str]] = None


def get_tool_definitions(enabled_toolsets, disabled_toolsets, quiet_mode):
    """Get tool definitions for current toolset configuration."""
    global _last_resolved_tool_names

    # Resolve tool names from toolsets
    tool_names = resolve_multiple_toolsets(enabled_toolsets)
    if disabled_toolsets:
        disabled_names = resolve_multiple_toolsets(disabled_toolsets)
        tool_names = [n for n in tool_names if n not in disabled_names]

    _last_resolved_tool_names = set(tool_names)

    # Get schemas from registry
    return registry.get_definitions(tool_names, quiet=quiet_mode)
```

This global is saved/restored during subagent execution:

```python
# delegate_tool.py

def delegate_task(task: str, subagent_model: str = None, ...) -> str:
    """Spawn subagent with isolated context."""
    global _last_resolved_tool_names

    # Save parent's tool state
    parent_tool_names = _last_resolved_tool_names

    try:
        # Run subagent (may change _last_resolved_tool_names)
        result = _run_single_child(task, ...)
        return result
    finally:
        # Restore parent's tool state
        _last_resolved_tool_names = parent_tool_names
```

## Tool Handler Patterns

### Standard Handler

```python
# tools/memory_tool.py

def memory_handler(args: dict, task_id: str = None) -> str:
    """Handle memory tool calls."""
    action = args.get("action", "read")
    content = args.get("content")
    target = args.get("target", "memory")

    memory_store = MemoryStore()
    memory_store.load_from_disk()

    if action == "add":
        # Security scan
        threat = _scan_memory_content(content)
        if threat:
            return json.dumps({"error": threat})

        memory_store.add(content, target)
        return json.dumps({"success": True, "action": "add"})

    elif action == "read":
        return json.dumps({
            "memory": memory_store.memory_entries,
            "user": memory_store.user_entries,
        })

    # ... other actions
```

### Async Handler

```python
# tools/web_tools.py

async def web_search_async(query: str, task_id: str = None) -> str:
    """Async web search handler."""
    parallel = ParallelWeb(api_key=os.getenv("PARALLEL_API_KEY"))
    results = await parallel.search(query)
    return json.dumps({"results": results})


# Registration with is_async=True
registry.register(
    name="web_search",
    toolset="web",
    schema={...},
    handler=lambda args, **kw: web_search_async(
        args.get("query", ""),
        task_id=kw.get("task_id")
    ),
    is_async=True,  # <-- Flag for async bridging
)
```

### Handler with Environment Check

```python
# tools/homeassistant_tool.py

def check_ha_requirements() -> bool:
    """Check if Home Assistant is configured."""
    return bool(os.getenv("HASS_TOKEN"))


registry.register(
    name="ha_get_state",
    toolset="homeassistant",
    schema={...},
    handler=ha_get_state_handler,
    check_fn=check_ha_requirements,
    requires_env=["HASS_TOKEN"],
)
```

## Tool Output Format

All handlers return JSON strings:

```python
# Success
json.dumps({"success": True, "data": ...})

# Error
json.dumps({"error": "Error message"})

# Structured result
json.dumps({
    "success": True,
    "result": {...},
    "metadata": {...}
})
```

## Tool Progress Callbacks

```python
# run_agent.py

class AIAgent:
    def __init__(self, ..., tool_progress_callback=None, ...):
        self.tool_progress_callback = tool_progress_callback

    def _execute_tool(self, tool_call):
        if self.tool_progress_callback:
            self.tool_progress_callback(
                type="tool_start",
                tool_name=tool_call.function.name,
                args=tool_call.function.arguments,
            )

        result = handle_function_call(...)

        if self.tool_progress_callback:
            self.tool_progress_callback(
                type="tool_complete",
                tool_name=tool_call.function.name,
                result=result,
            )
```

Used by CLI and gateway for streaming tool output.

## Summary

The tools system provides:

1. **40+ tools** organized into composable toolsets
2. **Parallel execution** with safety checks for path conflicts
3. **Destructive command detection** with pattern matching
4. **Recursive toolset resolution** with cycle detection
5. **Shared state management** via ProcessRegistry and globals
6. **Async bridging** for seamless sync/async execution
7. **Cross-tool references** injected dynamically
8. **JSON output format** for consistent result handling
