# OpenSpace Grounding System: Backend Deep-Dive

A comprehensive technical exploration of OpenSpace's grounding system - the execution layer that transforms LLM decisions into real-world actions across multiple backends.

---

## Table of Contents

1. [Grounding Architecture Overview](#1-grounding-architecture-overview)
2. [GroundingClient Structure](#2-groundingclient-structure)
3. [Backend Abstraction Layer](#3-backend-abstraction-layer)
4. [Backend Types](#4-backend-types)
5. [Tool System](#5-tool-system)
6. [Search Tools & Tool RAG](#6-search-tools--tool-rag)
7. [Security System](#7-security-system)
8. [Quality System](#8-quality-system)
9. [Transport Layer](#9-transport-layer)
10. [System Provider](#10-system-provider)

---

## 1. Grounding Architecture Overview

### What is Grounding?

**Grounding** is the process of connecting LLM decisions to real-world execution. When an LLM outputs a tool call, the grounding system:

1. Parses the tool call
2. Routes it to the appropriate backend
3. Executes with security checks
4. Returns results back to the LLM
5. Tracks quality metrics for self-improvement

### System Position

```
┌─────────────────────────────────────────────────────────────────┐
│                    Agent Host (Claude Code, etc.)                │
│                          │                                        │
│                          │ MCP Protocol                          │
└──────────────────────────┼────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    OpenSpace MCP Server                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  4 Tools: execute_task, search_skills, fix_skill, upload  │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                      OpenSpace Engine                            │
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                    GroundingAgent                           │ │
│  │  ┌──────────────────────────────────────────────────────┐  │ │
│  │  │              GroundingClient                          │  │ │
│  │  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐  │  │ │
│  │  │  │   Tools     │ │  Backends   │ │    Security     │  │  │ │
│  │  │  │  Registry   │ │  Router     │ │    Layer        │  │  │ │
│  │  │  └─────────────┘ └─────────────┘ └─────────────────┘  │  │ │
│  │  └──────────────────────────────────────────────────────┘  │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ ToolQuality  │  │  Search      │  │   System     │          │
│  │  Manager    │  │  Tools (RAG) │  │   Provider   │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
┌─────────────────┐ ┌──────────────┐ ┌──────────────┐
│   Shell         │ │   GUI        │ │   MCP        │
│   Backend       │ │   Backend    │ │   Backend    │
│   (commands)    │ │ (computer    │ │ (stdio/HTTP/ │
│                 │ │   use)       │ │  WebSocket)  │
└─────────────────┘ └──────────────┘ └──────────────┘
          │
          ▼
┌─────────────────┐
│   Web           │
│   Backend       │
│   (search/      │
│    browse)      │
└─────────────────┘
```

### Key Design Principles

| Principle | Description |
|-----------|-------------|
| **Unified Interface** | All backends expose the same call() API |
| **Backend Agnosticism** | Tools don't know which backend executes them |
| **Security First** | Every call passes through policy checks |
| **Quality Tracking** | Every execution is measured and analyzed |
| **Graceful Degradation** | Backends can fail over to alternatives |

### Core Components

```python
class OpenSpace:
    def __init__(self, config: OpenSpaceConfig):
        # LLM communication
        self._llm_client: LLMClient

        # Backend orchestration - routes tool calls to backends
        self._grounding_client: GroundingClient

        # Task execution with iterative tool calling
        self._grounding_agent: GroundingAgent

        # Skill discovery and ranking
        self._skill_registry: SkillRegistry

        # SQLite persistence for skill metrics
        self._skill_store: SkillStore

        # Post-task analysis for evolution triggers
        self._execution_analyzer: ExecutionAnalyzer

        # Evolution execution (FIX/DERIVED/CAPTURED)
        self._skill_evolver: SkillEvolver

        # Screenshots and video recording
        self._recording_manager: RecordingManager
```

---

## 2. GroundingClient Structure

### Class Definition

```python
class GroundingClient:
    """
    Central orchestrator for grounding operations.

    Responsibilities:
    - Backend initialization and lifecycle management
    - Tool call routing to appropriate backends
    - Security policy enforcement
    - Quality metric collection
    - Error handling and retry logic
    """

    def __init__(
        self,
        config: GroundingConfig,
        skill_registry: SkillRegistry,
        skill_store: SkillStore,
    ):
        self._config = config
        self._skill_registry = skill_registry
        self._skill_store = skill_store

        # Initialize backends
        self._backends: Dict[str, BaseBackend] = {}
        self._load_backends(config.enabled_backends)

        # Tool registry
        self._tools: Dict[str, BaseTool] = {}
        self._tool_quality_manager = ToolQualityManager()

        # Security
        self._policy_engine = PolicyEngine(config.security_policies)

        # Transport
        self._transport_manager = TransportManager()

    async def execute(
        self,
        instruction: str,
        max_iterations: int = 20,
        backend_scope: Optional[List[str]] = None,
    ) -> ExecutionResult:
        """
        Execute a task with iterative tool calling.

        Args:
            instruction: Natural language task description
            max_iterations: Maximum tool-calling iterations
            backend_scope: Limit to specific backends (None = all)

        Returns:
            ExecutionResult with status, response, and metrics
        """
        # 1. Select relevant skills
        skill_ids = await self._skill_registry.select_relevant_skills(instruction)
        skills_context = self._build_skills_context(skill_ids)

        # 2. Initialize conversation
        messages = [
            {"role": "system", "content": self._build_system_prompt(skills_context)},
            {"role": "user", "content": instruction},
        ]

        # 3. Tool-calling loop
        for iteration in range(max_iterations):
            # LLM decides next action
            response = await self._llm_client.chat(messages)

            if response.tool_calls:
                # Execute each tool call
                for tool_call in response.tool_calls:
                    result = await self._execute_tool_call(
                        tool_call,
                        backend_scope=backend_scope,
                    )
                    messages.append({
                        "role": "tool",
                        "content": result,
                        "tool_call_id": tool_call.id,
                    })
            else:
                # Final response from LLM
                return ExecutionResult(
                    status="success",
                    response=response.content,
                    iterations=iteration + 1,
                )

        return ExecutionResult(
            status="max_iterations_reached",
            response=messages[-1]["content"],
            iterations=max_iterations,
        )
```

### Tool Call Execution

```python
async def _execute_tool_call(
    self,
    tool_call: ToolCall,
    backend_scope: Optional[List[str]] = None,
) -> str:
    """
    Execute a single tool call with full security and quality tracking.
    """
    tool_name = tool_call.function.name
    arguments = json.loads(tool_call.function.arguments)

    # 1. Lookup tool
    tool = self._tools.get(tool_name)
    if not tool:
        return f"Error: Unknown tool '{tool_name}'"

    # 2. Security check
    if not await self._policy_engine.check(tool_name, arguments):
        return f"Error: Tool '{tool_name}' blocked by security policy"

    # 3. Determine backend
    backend_name = tool.backend or self._infer_backend(tool_name)
    if backend_scope and backend_name not in backend_scope:
        return f"Error: Backend '{backend_name}' not in scope"

    backend = self._backends.get(backend_name)
    if not backend:
        return f"Error: Backend '{backend_name}' not available"

    # 4. Execute with timing
    start_time = time.time()
    try:
        result = await backend.call(tool_name, arguments)
        success = True
    except Exception as e:
        result = f"Error: {str(e)}"
        success = False

    latency_ms = (time.time() - start_time) * 1000

    # 5. Record quality metric
    tool_key = f"{backend_name}.{tool_name}"
    self._tool_quality_manager.record_outcome(tool_key, success, latency_ms)

    # 6. Check for tool degradation
    if not success:
        await self._check_tool_degradation(tool_key)

    return result
```

### Backend Loading

```python
def _load_backends(self, backend_configs: List[BackendConfig]) -> None:
    """
    Initialize and register backends from configuration.
    """
    backend_registry = {
        "shell": ShellBackend,
        "gui": GUIBackend,
        "mcp": MCPBackend,
        "web": WebBackend,
    }

    for config in backend_configs:
        backend_class = backend_registry.get(config.name)
        if not backend_class:
            logger.warning(f"Unknown backend type: {config.name}")
            continue

        try:
            backend = backend_class(config)
            self._backends[config.name] = backend

            # Register backend's tools
            for tool in backend.get_tools():
                self._tools[tool.name] = tool

            logger.info(f"Loaded backend: {config.name}")
        except Exception as e:
            logger.error(f"Failed to load backend {config.name}: {e}")
```

---

## 3. Backend Abstraction Layer

### BaseBackend Interface

All backends implement this common interface:

```python
from abc import ABC, abstractmethod
from typing import Dict, List, Any, Optional
from dataclasses import dataclass

@dataclass
class ToolDefinition:
    """Tool schema for LLM function calling."""
    name: str
    description: str
    parameters: Dict[str, Any]  # JSON Schema
    backend: Optional[str] = None  # Override default backend

@dataclass
class BackendResult:
    """Standardized result from backend execution."""
    success: bool
    content: str
    metadata: Dict[str, Any] = None

class BaseBackend(ABC):
    """
    Abstract base class for all grounding backends.

    Provides:
    - Unified interface across different execution environments
    - Standard error handling
    - Common logging and metrics
    """

    def __init__(self, config: BackendConfig):
        self._config = config
        self._name = config.name
        self._enabled = config.enabled
        self._tools: List[ToolDefinition] = []
        self._initialize()

    @abstractmethod
    async def call(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> BackendResult:
        """
        Execute a tool call.

        Args:
            tool_name: Name of the tool to execute
            arguments: Tool arguments as dict

        Returns:
            BackendResult with execution outcome
        """
        pass

    @abstractmethod
    def get_tools(self) -> List[ToolDefinition]:
        """
        Return tools provided by this backend.

        Returns:
            List of ToolDefinition for LLM function calling
        """
        pass

    @abstractmethod
    def _initialize(self) -> None:
        """
        Initialize backend resources (connections, processes, etc.)
        """
        pass

    async def shutdown(self) -> None:
        """
        Cleanup backend resources.
        """
        pass

    def _validate_arguments(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> bool:
        """
        Validate arguments against tool schema.
        """
        tool = next((t for t in self._tools if t.name == tool_name), None)
        if not tool:
            return False

        # JSON Schema validation
        try:
            jsonschema.validate(arguments, tool.parameters)
            return True
        except jsonschema.ValidationError:
            return False
```

### Backend Configuration

```python
@dataclass
class BackendConfig:
    """Configuration for a backend."""
    name: str
    enabled: bool = True
    config: Dict[str, Any] = None
    timeout_seconds: int = 60
    max_retries: int = 3
    retry_delay_seconds: float = 1.0

# Example: config_grounding.json
{
  "enabled_backends": [
    {
      "name": "shell",
      "enabled": true,
      "config": {
        "working_dir": "/workspace",
        "shell": "bash",
        "timeout_seconds": 300
      },
      "timeout_seconds": 60,
      "max_retries": 3
    },
    {
      "name": "gui",
      "enabled": true,
      "config": {
        "screen_width": 1024,
        "screen_height": 768,
        "interaction_delay_ms": 100
      }
    },
    {
      "name": "mcp",
      "enabled": true,
      "config": {
        "servers": [
          {"name": "filesystem", "command": "mcp-server-filesystem"},
          {"name": "git", "command": "mcp-server-git"}
        ]
      }
    },
    {
      "name": "web",
      "enabled": true,
      "config": {
        "search_engine": "duckduckgo",
        "user_agent": "OpenSpace/1.0"
      }
    }
  ]
}
```

---

## 4. Backend Types

### 4.1 Shell Backend

Command execution with process management:

```python
class ShellBackend(BaseBackend):
    """
    Shell command execution backend.

    Features:
    - Synchronous command execution
    - Streaming output capture
    - Working directory management
    - Signal handling for cleanup
    - Output truncation for large results
    """

    def __init__(self, config: BackendConfig):
        super().__init__(config)
        self._working_dir = config.config.get("working_dir", "/workspace")
        self._shell = config.config.get("shell", "bash")
        self._processes: Dict[int, asyncio.subprocess.Process] = {}

    def _initialize(self) -> None:
        self._tools = [
            ToolDefinition(
                name="run_shell",
                description="Execute a shell command and return the output",
                parameters={
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        },
                        "working_dir": {
                            "type": "string",
                            "description": "Optional working directory"
                        },
                        "timeout": {
                            "type": "integer",
                            "description": "Timeout in seconds (default: 60)"
                        }
                    },
                    "required": ["command"]
                },
                backend="shell"
            ),
            ToolDefinition(
                name="write_file",
                description="Write content to a file",
                parameters={
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path"
                        },
                        "content": {
                            "type": "string",
                            "description": "File content"
                        }
                    },
                    "required": ["path", "content"]
                },
                backend="shell"
            ),
            ToolDefinition(
                name="read_file",
                description="Read content from a file",
                parameters={
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max lines to read (default: 100)"
                        }
                    },
                    "required": ["path"]
                },
                backend="shell"
            ),
        ]

    async def call(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> BackendResult:
        if tool_name == "run_shell":
            return await self._run_command(arguments)
        elif tool_name == "write_file":
            return await self._write_file(arguments)
        elif tool_name == "read_file":
            return await self._read_file(arguments)
        else:
            return BackendResult(
                success=False,
                content=f"Unknown tool: {tool_name}"
            )

    async def _run_command(
        self,
        arguments: Dict[str, Any],
    ) -> BackendResult:
        command = arguments["command"]
        working_dir = arguments.get("working_dir", self._working_dir)
        timeout = arguments.get("timeout", 60)

        try:
            process = await asyncio.create_subprocess_shell(
                command,
                shell=True,
                cwd=working_dir,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )

            self._processes[process.pid] = process

            try:
                stdout, stderr = await asyncio.wait_for(
                    process.communicate(),
                    timeout=timeout
                )
            except asyncio.TimeoutError:
                process.kill()
                return BackendResult(
                    success=False,
                    content=f"Command timed out after {timeout}s"
                )

            del self._processes[process.pid]

            output = stdout.decode() if stdout else ""
            error = stderr.decode() if stderr else ""

            # Truncate large outputs
            max_output = 10000
            if len(output) > max_output:
                output = output[:max_output] + "\n... [truncated]"
            if len(error) > max_output:
                error = error[:max_output] + "\n... [truncated]"

            if process.returncode == 0:
                return BackendResult(
                    success=True,
                    content=output or "(no output)",
                    metadata={"returncode": process.returncode}
                )
            else:
                return BackendResult(
                    success=False,
                    content=error or f"Exit code: {process.returncode}",
                    metadata={"returncode": process.returncode}
                )

        except Exception as e:
            return BackendResult(
                success=False,
                content=f"Execution error: {str(e)}"
            )

    async def _write_file(self, arguments: Dict[str, Any]) -> BackendResult:
        path = arguments["path"]
        content = arguments["content"]

        try:
            # Security: prevent path traversal
            full_path = self._sanitize_path(path)

            with open(full_path, "w") as f:
                f.write(content)

            return BackendResult(
                success=True,
                content=f"Successfully wrote {len(content)} bytes to {path}"
            )
        except Exception as e:
            return BackendResult(
                success=False,
                content=f"Write error: {str(e)}"
            )

    async def _read_file(self, arguments: Dict[str, Any]) -> BackendResult:
        path = arguments["path"]
        limit = arguments.get("limit", 100)

        try:
            full_path = self._sanitize_path(path)

            with open(full_path, "r") as f:
                lines = []
                for i, line in enumerate(f):
                    if i >= limit:
                        break
                    lines.append(line)

            content = "".join(lines)
            return BackendResult(
                success=True,
                content=content or "(empty file)"
            )
        except Exception as e:
            return BackendResult(
                success=False,
                content=f"Read error: {str(e)}"
            )

    def _sanitize_path(self, path: str) -> str:
        """Prevent path traversal attacks."""
        # Resolve to absolute path
        full_path = os.path.abspath(os.path.join(self._working_dir, path))

        # Ensure within working directory
        if not full_path.startswith(self._working_dir):
            raise ValueError(f"Path traversal detected: {path}")

        return full_path
```

### 4.2 GUI Backend (Anthropic Computer Use)

Screen interaction using computer control:

```python
class GUIBackend(BaseBackend):
    """
    GUI automation backend using Anthropic Computer Use protocol.

    Features:
    - Screenshot capture
    - Mouse movement and clicking
    - Keyboard input
    - Coordinate-based interaction
    - Visual analysis integration
    """

    def __init__(self, config: BackendConfig):
        super().__init__(config)
        self._screen_width = config.config.get("screen_width", 1024)
        self._screen_height = config.config.get("screen_height", 768)
        self._interaction_delay = config.config.get("interaction_delay_ms", 100)
        self._screenshot_dir = config.config.get("screenshot_dir", "./screenshots")

    def _initialize(self) -> None:
        self._tools = [
            ToolDefinition(
                name="screenshot",
                description="Take a screenshot of the current screen",
                parameters={
                    "type": "object",
                    "properties": {},
                },
                backend="gui"
            ),
            ToolDefinition(
                name="mouse_move",
                description="Move the mouse to a coordinate",
                parameters={
                    "type": "object",
                    "properties": {
                        "x": {"type": "integer", "description": "X coordinate (0-1024)"},
                        "y": {"type": "integer", "description": "Y coordinate (0-768)"},
                    },
                    "required": ["x", "y"]
                },
                backend="gui"
            ),
            ToolDefinition(
                name="mouse_click",
                description="Click the mouse at current position",
                parameters={
                    "type": "object",
                    "properties": {
                        "button": {
                            "type": "string",
                            "enum": ["left", "right", "middle"],
                            "default": "left"
                        },
                    },
                },
                backend="gui"
            ),
            ToolDefinition(
                name="type_text",
                description="Type text using the keyboard",
                parameters={
                    "type": "object",
                    "properties": {
                        "text": {"type": "string", "description": "Text to type"},
                    },
                    "required": ["text"]
                },
                backend="gui"
            ),
            ToolDefinition(
                name="press_key",
                description="Press a keyboard key",
                parameters={
                    "type": "object",
                    "properties": {
                        "key": {
                            "type": "string",
                            "description": "Key name (e.g., 'Enter', 'Tab', 'Escape')"
                        },
                    },
                    "required": ["key"]
                },
                backend="gui"
            ),
        ]

    async def call(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> BackendResult:
        try:
            if tool_name == "screenshot":
                return await self._screenshot()
            elif tool_name == "mouse_move":
                return await self._mouse_move(arguments)
            elif tool_name == "mouse_click":
                return await self._mouse_click(arguments)
            elif tool_name == "type_text":
                return await self._type_text(arguments)
            elif tool_name == "press_key":
                return await self._press_key(arguments)
            else:
                return BackendResult(
                    success=False,
                    content=f"Unknown tool: {tool_name}"
                )
        except Exception as e:
            return BackendResult(
                success=False,
                content=f"GUI error: {str(e)}"
            )

    async def _screenshot(self) -> BackendResult:
        """Capture screen and return base64 image."""
        import pyautogui
        import base64
        from io import BytesIO

        screenshot = pyautogui.screenshot()

        # Save to file
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filepath = os.path.join(self._screenshot_dir, f"screenshot_{timestamp}.png")
        screenshot.save(filepath)

        # Also return base64 for LLM analysis
        buffered = BytesIO()
        screenshot.save(buffered, format="PNG")
        img_base64 = base64.b64encode(buffered.getvalue()).decode()

        return BackendResult(
            success=True,
            content=f"Screenshot saved to {filepath}",
            metadata={
                "base64": img_base64,
                "width": screenshot.width,
                "height": screenshot.height,
                "filepath": filepath,
            }
        )

    async def _mouse_move(self, arguments: Dict[str, Any]) -> BackendResult:
        """Move mouse to coordinate."""
        import pyautogui

        x = arguments["x"]
        y = arguments["y"]

        # Scale to actual screen resolution
        actual_x = int(x / 1024 * pyautogui.size().width)
        actual_y = int(y / 768 * pyautogui.size().height)

        pyautogui.moveTo(actual_x, actual_y, duration=0.5)

        return BackendResult(
            success=True,
            content=f"Mouse moved to ({x}, {y})"
        )

    async def _mouse_click(self, arguments: Dict[str, Any]) -> BackendResult:
        """Click mouse."""
        import pyautogui

        button = arguments.get("button", "left")

        pyautogui.click(button=button)
        await asyncio.sleep(self._interaction_delay / 1000)

        return BackendResult(
            success=True,
            content=f"Mouse {button} click executed"
        )

    async def _type_text(self, arguments: Dict[str, Any]) -> BackendResult:
        """Type text."""
        import pyautogui

        text = arguments["text"]

        pyautogui.typewrite(text, interval=0.05)

        return BackendResult(
            success=True,
            content=f"Typed: {text[:50]}..." if len(text) > 50 else f"Typed: {text}"
        )

    async def _press_key(self, arguments: Dict[str, Any]) -> BackendResult:
        """Press keyboard key."""
        import pyautogui

        key = arguments["key"]

        pyautogui.press(key)
        await asyncio.sleep(self._interaction_delay / 1000)

        return BackendResult(
            success=True,
            content=f"Pressed key: {key}"
        )
```

### 4.3 MCP Backend

Model Context Protocol for external tool servers:

```python
class MCPBackend(BaseBackend):
    """
    Model Context Protocol backend.

    Supports:
    - stdio transport (local processes)
    - HTTP transport (remote servers)
    - WebSocket transport (real-time)

    Features:
    - Dynamic tool discovery from MCP servers
    - Automatic tool registration
    - Connection pooling
    - Graceful reconnection
    """

    def __init__(self, config: BackendConfig):
        super().__init__(config)
        self._server_configs = config.config.get("servers", [])
        self._connectors: Dict[str, MCPConnector] = {}
        self._remote_tools: Dict[str, ToolDefinition] = {}

    def _initialize(self) -> None:
        # Tools are dynamically discovered from MCP servers
        pass

    async def connect(self) -> None:
        """Initialize connections to all MCP servers."""
        for server_config in self._server_configs:
            try:
                connector = await self._create_connector(server_config)
                self._connectors[server_config["name"]] = connector

                # Discover and register tools
                tools = await connector.list_tools()
                for tool in tools:
                    tool_key = f"{server_config['name']}.{tool.name}"
                    self._remote_tools[tool_key] = tool
                    logger.info(f"Registered MCP tool: {tool_key}")

            except Exception as e:
                logger.error(f"Failed to connect to MCP server {server_config['name']}: {e}")

    async def _create_connector(self, config: Dict[str, Any]) -> MCPConnector:
        """Create appropriate connector based on transport type."""
        transport = config.get("transport", "stdio")

        if transport == "stdio":
            return StdioConnector(config)
        elif transport == "http":
            return HTTPConnector(config)
        elif transport == "websocket":
            return WebSocketConnector(config)
        else:
            raise ValueError(f"Unknown transport: {transport}")

    async def call(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> BackendResult:
        # Parse tool_key (format: "server_name.tool_name")
        parts = tool_name.split(".", 1)
        if len(parts) != 2:
            return BackendResult(
                success=False,
                content=f"Invalid tool format: {tool_name}. Expected 'server.tool'"
            )

        server_name, actual_tool = parts

        connector = self._connectors.get(server_name)
        if not connector:
            return BackendResult(
                success=False,
                content=f"Server not found: {server_name}"
            )

        try:
            result = await connector.call_tool(actual_tool, arguments)
            return BackendResult(
                success=True,
                content=result.content,
                metadata=result.metadata
            )
        except Exception as e:
            return BackendResult(
                success=False,
                content=f"MCP error: {str(e)}"
            )

    def get_tools(self) -> List[ToolDefinition]:
        """Return all discovered MCP tools."""
        return list(self._remote_tools.values())


class MCPConnector(ABC):
    """Abstract base for MCP transport connectors."""

    @abstractmethod
    async def list_tools(self) -> List[ToolDefinition]:
        pass

    @abstractmethod
    async def call_tool(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> MCPResult:
        pass


class StdioConnector(MCPConnector):
    """stdio transport connector."""

    def __init__(self, config: Dict[str, Any]):
        self._command = config["command"]
        self._args = config.get("args", [])
        self._process: Optional[asyncio.subprocess.Process] = None
        self._message_id = 0

    async def connect(self) -> None:
        self._process = await asyncio.create_subprocess_exec(
            self._command,
            *self._args,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )

    async def list_tools(self) -> List[ToolDefinition]:
        return await self._send_request("tools/list", {})

    async def call_tool(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> MCPResult:
        return await self._send_request("tools/call", {
            "name": tool_name,
            "arguments": arguments,
        })

    async def _send_request(self, method: str, params: Dict) -> Any:
        """Send JSON-RPC request over stdio."""
        self._message_id += 1

        request = {
            "jsonrpc": "2.0",
            "id": self._message_id,
            "method": method,
            "params": params,
        }

        # Write request
        self._process.stdin.write((json.dumps(request) + "\n").encode())
        await self._process.stdin.drain()

        # Read response
        line = await self._process.stdout.readline()
        response = json.loads(line.decode())

        if "error" in response:
            raise MCPError(response["error"]["message"])

        return response.get("result")


class HTTPConnector(MCPConnector):
    """HTTP transport connector."""

    def __init__(self, config: Dict[str, Any]):
        self._base_url = config["url"]
        self._api_key = config.get("api_key")
        self._session = aiohttp.ClientSession()

    async def list_tools(self) -> List[ToolDefinition]:
        response = await self._session.get(f"{self._base_url}/tools")
        data = await response.json()
        return [ToolDefinition(**t) for t in data["tools"]]

    async def call_tool(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> MCPResult:
        response = await self._session.post(
            f"{self._base_url}/tools/{tool_name}/call",
            json={"arguments": arguments},
        )
        data = await response.json()
        return MCPResult(**data)


class WebSocketConnector(MCPConnector):
    """WebSocket transport connector."""

    def __init__(self, config: Dict[str, Any]):
        self._url = config["url"]
        self._ws: Optional[aiohttp.ClientWebSocketResponse] = None

    async def connect(self) -> None:
        self._ws = await aiohttp.ClientSession().ws_connect(self._url)

    async def list_tools(self) -> List[ToolDefinition]:
        return await self._send_request("tools/list", {})

    async def call_tool(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> MCPResult:
        return await self._send_request("tools/call", {
            "name": tool_name,
            "arguments": arguments,
        })

    async def _send_request(self, method: str, params: Dict) -> Any:
        request = {"method": method, "params": params}
        await self._ws.send_json(request)
        response = await self._ws.receive_json()
        return response.get("result")
```

### 4.4 Web Backend

Web search and browsing:

```python
class WebBackend(BaseBackend):
    """
    Web search and browsing backend.

    Features:
    - Multi-engine search (DuckDuckGo, Google, Bing)
    - Web page content extraction
    - Link following
    - Result summarization
    """

    def __init__(self, config: BackendConfig):
        super().__init__(config)
        self._search_engine = config.config.get("search_engine", "duckduckgo")
        self._user_agent = config.config.get("user_agent", "OpenSpace/1.0")
        self._session = aiohttp.ClientSession(
            headers={"User-Agent": self._user_agent}
        )

    def _initialize(self) -> None:
        self._tools = [
            ToolDefinition(
                name="web_search",
                description="Search the web for information",
                parameters={
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "num_results": {
                            "type": "integer",
                            "description": "Number of results (default: 10)"
                        },
                    },
                    "required": ["query"]
                },
                backend="web"
            ),
            ToolDefinition(
                name="fetch_url",
                description="Fetch and extract content from a URL",
                parameters={
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to fetch"
                        },
                    },
                    "required": ["url"]
                },
                backend="web"
            ),
            ToolDefinition(
                name="follow_links",
                description="Extract and follow links from a page",
                parameters={
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "Starting URL"
                        },
                        "pattern": {
                            "type": "string",
                            "description": "Regex pattern to filter links"
                        },
                    },
                    "required": ["url"]
                },
                backend="web"
            ),
        ]

    async def call(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> BackendResult:
        try:
            if tool_name == "web_search":
                return await self._search(arguments)
            elif tool_name == "fetch_url":
                return await self._fetch(arguments)
            elif tool_name == "follow_links":
                return await self._follow_links(arguments)
            else:
                return BackendResult(
                    success=False,
                    content=f"Unknown tool: {tool_name}"
                )
        except Exception as e:
            return BackendResult(
                success=False,
                content=f"Web error: {str(e)}"
            )

    async def _search(self, arguments: Dict[str, Any]) -> BackendResult:
        """Search the web."""
        query = arguments["query"]
        num_results = arguments.get("num_results", 10)

        if self._search_engine == "duckduckgo":
            results = await self._duckduckgo_search(query, num_results)
        elif self._search_engine == "google":
            results = await self._google_search(query, num_results)
        else:
            results = await self._bing_search(query, num_results)

        formatted = self._format_search_results(results)
        return BackendResult(
            success=True,
            content=formatted,
            metadata={"results": results}
        )

    async def _duckduckgo_search(
        self,
        query: str,
        num_results: int,
    ) -> List[Dict]:
        """DuckDuckGo search via HTML scraping."""
        from bs4 import BeautifulSoup

        url = "https://html.duckduckgo.com/html/"
        data = {"q": query}

        async with self._session.post(url, data=data) as resp:
            html = await resp.text()

        soup = BeautifulSoup(html, "html.parser")
        results = []

        for result in soup.select(".result")[:num_results]:
            title_elem = result.select_one(".result__title")
            snippet_elem = result.select_one(".result__snippet")
            url_elem = result.select_one(".result__url")

            if title_elem and snippet_elem:
                results.append({
                    "title": title_elem.get_text(strip=True),
                    "snippet": snippet_elem.get_text(strip=True),
                    "url": url_elem.get("href") if url_elem else None,
                })

        return results

    async def _fetch(self, arguments: Dict[str, Any]) -> BackendResult:
        """Fetch URL content."""
        url = arguments["url"]

        async with self._session.get(url) as resp:
            html = await resp.text()

        # Extract main content
        from bs4 import BeautifulSoup
        soup = BeautifulSoup(html, "html.parser")

        # Remove scripts, styles
        for tag in soup(["script", "style", "nav", "footer"]):
            tag.decompose()

        content = soup.get_text(separator="\n", strip=True)

        # Truncate
        max_content = 10000
        if len(content) > max_content:
            content = content[:max_content] + "\n... [truncated]"

        return BackendResult(
            success=True,
            content=content,
            metadata={"url": url, "title": soup.title.string if soup.title else ""}
        )

    async def _follow_links(self, arguments: Dict[str, Any]) -> BackendResult:
        """Extract and follow links."""
        url = arguments["url"]
        pattern = arguments.get("pattern")

        async with self._session.get(url) as resp:
            html = await resp.text()

        from bs4 import BeautifulSoup
        soup = BeautifulSoup(html, "html.parser")

        links = []
        for a in soup.find_all("a", href=True):
            href = a["href"]
            if pattern and not re.search(pattern, href):
                continue

            # Resolve relative URLs
            full_url = urllib.parse.urljoin(url, href)
            links.append(full_url)

        return BackendResult(
            success=True,
            content="\n".join(f"- {link}" for link in links[:50]),
            metadata={"links": links}
        )

    def _format_search_results(self, results: List[Dict]) -> str:
        """Format search results for LLM consumption."""
        lines = []
        for i, result in enumerate(results, 1):
            lines.append(f"{i}. {result['title']}")
            lines.append(f"   {result['snippet']}")
            lines.append(f"   URL: {result['url']}")
            lines.append("")
        return "\n".join(lines)
```

### Backend Comparison

| Backend | Use Case | Latency | Complexity |
|---------|----------|---------|------------|
| Shell | CLI tools, file ops, scripts | Low | Low |
| GUI | Desktop automation, visual tasks | Medium | High |
| MCP | External tool servers | Medium | Medium |
| Web | Research, information gathering | High | Medium |

---

## 5. Tool System

### BaseTool Abstraction

```python
from abc import ABC, abstractmethod
from typing import Dict, List, Any, Optional, Callable
from dataclasses import dataclass, field

@dataclass
class ToolMetadata:
    """Tool metadata for discovery and ranking."""
    name: str
    description: str
    category: str
    tags: List[str] = field(default_factory=list)
    version: str = "1.0.0"
    author: str = "unknown"

@dataclass
class ToolSchema:
    """JSON Schema for tool parameters."""
    type: str = "object"
    properties: Dict[str, Any] = field(default_factory=dict)
    required: List[str] = field(default_factory=list)
    additionalProperties: bool = False

class BaseTool(ABC):
    """
    Abstract base class for all tools.

    Tools can be:
    - Local: Implemented in Python
    - Remote: Delegated to MCP servers
    - Composite: Chain multiple tools
    """

    def __init__(self, metadata: ToolMetadata, schema: ToolSchema):
        self._metadata = metadata
        self._schema = schema
        self._enabled = True

    @property
    def name(self) -> str:
        return self._metadata.name

    @property
    def description(self) -> str:
        return self._metadata.description

    @abstractmethod
    async def execute(self, arguments: Dict[str, Any]) -> ToolResult:
        """
        Execute the tool.

        Args:
            arguments: Tool arguments (validated against schema)

        Returns:
            ToolResult with outcome
        """
        pass

    def validate_arguments(self, arguments: Dict[str, Any]) -> bool:
        """Validate arguments against schema."""
        try:
            jsonschema.validate(arguments, self._schema)
            return True
        except jsonschema.ValidationError:
            return False

    def to_definition(self) -> ToolDefinition:
        """Convert to LLM function calling format."""
        return ToolDefinition(
            name=self._metadata.name,
            description=self._metadata.description,
            parameters=self._schema,
        )


@dataclass
class ToolResult:
    """Result from tool execution."""
    success: bool
    content: str
    metadata: Dict[str, Any] = field(default_factory=dict)
    error: Optional[str] = None
```

### Local Tools

```python
class LocalTool(BaseTool):
    """
    Locally implemented tool.
    """

    def __init__(
        self,
        metadata: ToolMetadata,
        schema: ToolSchema,
        handler: Callable[[Dict[str, Any]], Any],
    ):
        super().__init__(metadata, schema)
        self._handler = handler

    async def execute(self, arguments: Dict[str, Any]) -> ToolResult:
        try:
            result = await self._handler(arguments)
            return ToolResult(
                success=True,
                content=str(result) if not isinstance(result, str) else result,
            )
        except Exception as e:
            return ToolResult(
                success=False,
                content=f"Error: {str(e)}",
                error=str(e)
            )


# Example: Local file tool
def create_file_tools() -> List[LocalTool]:
    return [
        LocalTool(
            metadata=ToolMetadata(
                name="read_file",
                description="Read content from a file",
                category="filesystem",
                tags=["file", "read", "io"],
            ),
            schema=ToolSchema(
                properties={
                    "path": {"type": "string", "description": "File path"},
                    "limit": {"type": "integer", "description": "Max lines"},
                },
                required=["path"],
            ),
            handler=lambda args: _read_file_handler(args),
        ),
        LocalTool(
            metadata=ToolMetadata(
                name="write_file",
                description="Write content to a file",
                category="filesystem",
                tags=["file", "write", "io"],
            ),
            schema=ToolSchema(
                properties={
                    "path": {"type": "string", "description": "File path"},
                    "content": {"type": "string", "description": "Content"},
                },
                required=["path", "content"],
            ),
            handler=lambda args: _write_file_handler(args),
        ),
    ]
```

### Remote Tools

```python
class RemoteTool(BaseTool):
    """
    Tool delegated to external service (MCP server).
    """

    def __init__(
        self,
        metadata: ToolMetadata,
        schema: ToolSchema,
        connector: MCPConnector,
        remote_name: str,
    ):
        super().__init__(metadata, schema)
        self._connector = connector
        self._remote_name = remote_name

    async def execute(self, arguments: Dict[str, Any]) -> ToolResult:
        try:
            result = await self._connector.call_tool(self._remote_name, arguments)
            return ToolResult(
                success=True,
                content=result.content,
                metadata=result.metadata,
            )
        except Exception as e:
            return ToolResult(
                success=False,
                content=f"Remote error: {str(e)}",
                error=str(e)
            )
```

### Tool Registration

```python
class ToolRegistry:
    """
    Central registry for all tools.

    Features:
    - Tool discovery
    - Category-based filtering
    - Embedding generation for RAG
    """

    def __init__(self):
        self._tools: Dict[str, BaseTool] = {}
        self._embeddings: Dict[str, List[float]] = {}
        self._embedding_model = None  # Lazy load

    def register(self, tool: BaseTool) -> None:
        """Register a tool."""
        self._tools[tool.name] = tool

        # Generate embedding for RAG
        self._embeddings[tool.name] = self._generate_embedding(
            f"{tool.name}: {tool.description}"
        )

    def get(self, name: str) -> Optional[BaseTool]:
        """Get tool by name."""
        return self._tools.get(name)

    def list_tools(self, category: Optional[str] = None) -> List[BaseTool]:
        """List all tools, optionally filtered by category."""
        tools = list(self._tools.values())
        if category:
            tools = [t for t in tools if t._metadata.category == category]
        return tools

    def find_similar(self, query: str, limit: int = 10) -> List[BaseTool]:
        """Find tools similar to query using embeddings."""
        query_embedding = self._generate_embedding(query)

        scores = []
        for name, emb in self._embeddings.items():
            score = cosine_similarity(query_embedding, emb)
            scores.append((name, score))

        # Sort by similarity
        scores.sort(key=lambda x: x[1], reverse=True)

        return [self._tools[name] for name, _ in scores[:limit]]

    def _generate_embedding(self, text: str) -> List[float]:
        """Generate embedding for text."""
        if not self._embedding_model:
            from sentence_transformers import SentenceTransformer
            self._embedding_model = SentenceTransformer("BAAI/bge-small-en-v1.5")

        embedding = self._embedding_model.encode(text)
        return embedding.tolist()
```

### Tool Calling Protocol

```
┌─────────────────────────────────────────────────────────────────┐
│                     Tool Call Sequence                           │
└─────────────────────────────────────────────────────────────────┘

1. LLM generates tool call
   │
   ▼
2. GroundingClient receives ToolCall
   │
   ▼
3. Security Policy Check
   │
   ├── Blocked ──► Return error to LLM
   │
   ▼ Allowed
4. Tool Lookup
   │
   ├── Not Found ──► Return error to LLM
   │
   ▼ Found
5. Argument Validation (JSON Schema)
   │
   ├── Invalid ──► Return validation error
   │
   ▼ Valid
6. Backend Routing
   │
   ▼
7. Backend Execution
   │
   ▼
8. Quality Recording
   │
   ▼
9. Result to LLM
   │
   ▼
10. Continue conversation or finish
```

```python
# Tool call message format
{
  "role": "assistant",
  "content": None,
  "tool_calls": [
    {
      "id": "call_abc123",
      "type": "function",
      "function": {
        "name": "run_shell",
        "arguments": "{\"command\": \"ls -la\"}"
      }
    }
  ]
}

# Tool result message format
{
  "role": "tool",
  "content": "total 48\ndrwxr-xr-x ...",
  "tool_call_id": "call_abc123"
}
```

---

## 6. Search Tools & Tool RAG

### Tool RAG Architecture

OpenSpace uses a hybrid search system for tool discovery:

```
┌─────────────────────────────────────────────────────────────────┐
│                      Tool RAG Pipeline                          │
└─────────────────────────────────────────────────────────────────┘

User Query: "Deploy a Docker container"
│
▼
┌─────────────────────────────────────────────────────────────────┐
│ Step 1: BM25 Pre-filter                                          │
│ - Keyword matching                                               │
│ - Fast, recall-oriented                                          │
│ - Threshold: 0.3                                                 │
│ - Output: ~100 candidates                                        │
└─────────────────────────────────────────────────────────────────┘
│
▼
┌─────────────────────────────────────────────────────────────────┐
│ Step 2: Embedding Ranking                                      │
│ - Semantic similarity                                            │
│ - BAAI/bge-small-en-v1.5                                        │
│ - Output: top 20 ranked                                          │
└─────────────────────────────────────────────────────────────────┘
│
▼
┌─────────────────────────────────────────────────────────────────┐
│ Step 3: LLM Final Selection                                    │
│ - Context-aware judgment                                       │
│ - Considers tool compatibility                                  │
│ - Output: 3-5 tools for current task                            │
└─────────────────────────────────────────────────────────────────┘
```

### SearchTools Implementation

```python
class SearchTools:
    """
    Tool RAG system for dynamic tool selection.
    """

    def __init__(
        self,
        tool_registry: ToolRegistry,
        embedding_model: str = "BAAI/bge-small-en-v1.5",
    ):
        self._tool_registry = tool_registry
        self._ranker = ToolRanker(tool_registry, embedding_model)
        self._llm = None

    async def select_tools(
        self,
        task: str,
        context: Optional[Dict] = None,
        limit: int = 5,
    ) -> List[BaseTool]:
        """
        Select relevant tools for a task.

        Args:
            task: Task description
            context: Optional context (e.g., current backend state)
            limit: Max tools to select

        Returns:
            List of selected tools
        """
        # Step 1: BM25 prefilter
        candidates = self._ranker.prefilter(task, threshold=0.3)
        logger.debug(f"BM25 candidates: {len(candidates)}")

        # Step 2: Embedding ranking
        ranked = await self._ranker.rank_with_embeddings(task, candidates)
        logger.debug(f"Ranked candidates: {len(ranked)}")

        # Step 3: LLM final selection
        selected = await self._llm_select(task, ranked[:20], limit)
        logger.debug(f"Selected tools: {[t.name for t in selected]}")

        return selected

    async def _llm_select(
        self,
        task: str,
        candidates: List[BaseTool],
        limit: int,
    ) -> List[BaseTool]:
        """Use LLM to make final tool selection."""
        if not self._llm:
            self._llm = LiteLLMClient()

        # Build candidate descriptions
        candidate_text = "\n".join(
            f"- {t.name}: {t.description} (category: {t._metadata.category})"
            for t in candidates
        )

        prompt = f"""
Task: {task}

Available tools:
{candidate_text}

Select up to {limit} tools that are most relevant for this task.
Consider:
1. Direct relevance to task goal
2. Tool compatibility (can they work together?)
3. Efficiency (prefer simpler tools when possible)

Return only tool names, comma-separated.
"""

        response = await self._llm.complete(prompt)
        selected_names = [n.strip() for n in response.strip().split(",")]

        return [t for t in candidates if t.name in selected_names][:limit]


class ToolRanker:
    """
    Tool ranking with BM25 + embeddings.
    """

    def __init__(self, tool_registry: ToolRegistry, embedding_model: str):
        self._tool_registry = tool_registry
        self._bm25 = BM25Okapi([])
        self._tool_texts: Dict[str, str] = {}
        self._embeddings: Dict[str, List[float]] = {}
        self._embedding_model = None

        self._index_tools()

    def _index_tools(self) -> None:
        """Build search index for all tools."""
        for tool in self._tool_registry.list_tools():
            text = f"{tool.name} {tool.description} {' '.join(tool._metadata.tags)}"
            self._tool_texts[tool.name] = text

        # Build BM25 index
        tokenized = [self._tokenize(text) for text in self._tool_texts.values()]
        self._bm25 = BM25Okapi(tokenized)

        # Generate embeddings
        self._embedding_model = SentenceTransformer(self._embedding_model)
        for name, text in self._tool_texts.items():
            embedding = self._embedding_model.encode(text)
            self._embeddings[name] = embedding.tolist()

    def prefilter(self, query: str, threshold: float) -> List[str]:
        """BM25 keyword prefiltering."""
        query_tokens = self._tokenize(query)
        scores = self._bm25.get_scores(query_tokens)

        candidates = []
        for (name, _), score in zip(self._tool_texts.items(), scores):
            if score > threshold * 10:  # Scale threshold
                candidates.append(name)

        return candidates

    async def rank_with_embeddings(
        self,
        query: str,
        candidates: List[str],
    ) -> List[str]:
        """Rank candidates by embedding similarity."""
        query_embedding = self._embedding_model.encode(query)

        ranked = []
        for name in candidates:
            emb = np.array(self._embeddings[name])
            score = cosine_similarity(query_embedding, emb)
            ranked.append((name, score))

        ranked.sort(key=lambda x: x[1], reverse=True)
        return [name for name, _ in ranked]

    def _tokenize(self, text: str) -> List[str]:
        """Simple whitespace tokenizer."""
        return text.lower().split()
```

### Smart Tool Selection

```python
class SmartToolSelector:
    """
    Advanced tool selection with context awareness.
    """

    def __init__(self, search_tools: SearchTools):
        self._search_tools = search_tools
        self._tool_history: List[Tuple[str, bool]] = []  # (tool_name, success)

    async def select_with_context(
        self,
        task: str,
        available_backends: List[str],
        exclude_failed: bool = True,
    ) -> List[BaseTool]:
        """
        Select tools considering backend availability and history.
        """
        # Get base selection
        tools = await self._search_tools.select_tools(task)

        # Filter by backend availability
        tools = [t for t in tools if t._metadata.category in available_backends]

        # Exclude recently failed tools
        if exclude_failed:
            failed_tools = {name for name, success in self._tool_history[-10:] if not success}
            tools = [t for t in tools if t.name not in failed_tools]

        return tools

    def record_outcome(self, tool_name: str, success: bool) -> None:
        """Record tool execution outcome for future selection."""
        self._tool_history.append((tool_name, success))

        # Keep history bounded
        if len(self._tool_history) > 100:
            self._tool_history = self._tool_history[-100:]
```

---

## 7. Security System

### Policy Engine

```python
class PolicyEngine:
    """
    Security policy enforcement.

    Policies can:
    - Block specific tools
    - Restrict command patterns
    - Require approval for sensitive operations
    - Enforce path restrictions
    """

    def __init__(self, policies: List[SecurityPolicy]):
        self._policies = policies

    async def check(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> bool:
        """
        Check if tool call is allowed.

        Returns True if allowed, False if blocked.
        """
        for policy in self._policies:
            if not await policy.evaluate(tool_name, arguments):
                logger.warning(f"Blocked by policy {policy.name}: {tool_name}")
                return False
        return True


class SecurityPolicy(ABC):
    """Abstract base for security policies."""

    @property
    @abstractmethod
    def name(self) -> str:
        pass

    @abstractmethod
    async def evaluate(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> bool:
        """Return True if allowed, False if blocked."""
        pass


class BlockToolPolicy(SecurityPolicy):
    """Block specific tools entirely."""

    def __init__(self, blocked_tools: List[str]):
        self._blocked = set(blocked_tools)

    @property
    def name(self) -> str:
        return "block_tool"

    async def evaluate(self, tool_name: str, arguments: Dict) -> bool:
        return tool_name not in self._blocked


class CommandPatternPolicy(SecurityPolicy):
    """Block shell commands matching dangerous patterns."""

    DANGEROUS_PATTERNS = [
        r"\brm\s+-rf\s+/",  # rm -rf /
        r"\bchmod\s+-R\s+777",  # chmod -R 777
        r"\bsudo\s+rm",  # sudo rm
        r">\s*/dev/sd",  # Writing to disk devices
        r"\bfork\s*\(\s*\)",  # Fork bombs
        r":\(\)\{",  # Bash fork bombs
    ]

    def __init__(self):
        self._patterns = [re.compile(p, re.IGNORECASE) for p in self.DANGEROUS_PATTERNS]

    @property
    def name(self) -> str:
        return "command_pattern"

    async def evaluate(self, tool_name: str, arguments: Dict) -> bool:
        if tool_name != "run_shell":
            return True

        command = arguments.get("command", "")
        for pattern in self._patterns:
            if pattern.search(command):
                logger.warning(f"Dangerous command pattern detected: {command}")
                return False
        return True


class PathRestrictionPolicy(SecurityPolicy):
    """Restrict file operations to allowed directories."""

    def __init__(self, allowed_dirs: List[str]):
        self._allowed = [os.path.abspath(d) for d in allowed_dirs]

    @property
    def name(self) -> str:
        return "path_restriction"

    async def evaluate(self, tool_name: str, arguments: Dict) -> bool:
        if tool_name not in ("read_file", "write_file"):
            return True

        path = arguments.get("path", "")
        full_path = os.path.abspath(path)

        # Check if within allowed directories
        for allowed in self._allowed:
            if full_path.startswith(allowed):
                return True

        logger.warning(f"Path outside allowed directories: {path}")
        return False


class ApprovalRequiredPolicy(SecurityPolicy):
    """Require human approval for sensitive operations."""

    SENSITIVE_TOOLS = ["run_shell", "write_file"]

    def __init__(self, approval_callback: Callable):
        self._approval_callback = approval_callback

    @property
    def name(self) -> str:
        return "approval_required"

    async def evaluate(self, tool_name: str, arguments: Dict) -> bool:
        if tool_name not in self.SENSITIVE_TOOLS:
            return True

        # Request approval
        return await self._approval_callback(tool_name, arguments)
```

### Sandboxing

```python
class SandboxConfig:
    """Configuration for tool sandboxing."""

    def __init__(
        self,
        enabled: bool = True,
        network_isolation: bool = False,
        filesystem_readonly: bool = False,
        memory_limit_mb: int = 512,
        cpu_limit_percent: int = 50,
        timeout_seconds: int = 60,
    ):
        self.enabled = enabled
        self.network_isolation = network_isolation
        self.filesystem_readonly = filesystem_readonly
        self.memory_limit_mb = memory_limit_mb
        self.cpu_limit_percent = cpu_limit_percent
        self.timeout_seconds = timeout_seconds


class Sandbox:
    """
    Tool execution sandbox.

    Provides:
    - Process isolation
    - Resource limits
    - Filesystem restrictions
    - Network controls
    """

    def __init__(self, config: SandboxConfig):
        self._config = config

    async def execute(
        self,
        tool: BaseTool,
        arguments: Dict[str, Any],
    ) -> ToolResult:
        """Execute tool within sandbox constraints."""
        if not self._config.enabled:
            return await tool.execute(arguments)

        # Platform-specific sandboxing
        if sys.platform == "linux":
            return await self._execute_linux(tool, arguments)
        elif sys.platform == "darwin":
            return await self._execute_macos(tool, arguments)
        else:
            # Fallback: basic timeout only
            return await self._execute_basic(tool, arguments)

    async def _execute_linux(
        self,
        tool: BaseTool,
        arguments: Dict[str, Any],
    ) -> ToolResult:
        """Linux sandboxing with cgroups and namespaces."""
        import subprocess

        # Create sandboxed process
        process = await asyncio.create_subprocess_exec(
            "systemd-run",
            "--scope",
            f"--property=MemoryLimit={self._config.memory_limit_mb}M",
            f"--property=CPUQuota={self._config.cpu_limit_percent}%",
            "--property=PrivateNetwork=yes" if self._config.network_isolation else "",
            "--property=ReadOnlyPaths=/" if self._config.filesystem_readonly else "",
            "python", "-c", self._build_tool_script(tool, arguments),
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )

        try:
            stdout, stderr = await asyncio.wait_for(
                process.communicate(),
                timeout=self._config.timeout_seconds
            )

            if process.returncode == 0:
                return ToolResult(
                    success=True,
                    content=stdout.decode(),
                )
            else:
                return ToolResult(
                    success=False,
                    content=stderr.decode(),
                    error="Sandbox execution failed"
                )

        except asyncio.TimeoutError:
            process.kill()
            return ToolResult(
                success=False,
                content="Sandbox timeout exceeded",
                error="timeout"
            )

    async def _execute_basic(
        self,
        tool: BaseTool,
        arguments: Dict[str, Any],
    ) -> ToolResult:
        """Basic sandboxing with timeout only."""
        try:
            result = await asyncio.wait_for(
                tool.execute(arguments),
                timeout=self._config.timeout_seconds
            )
            return result
        except asyncio.TimeoutError:
            return ToolResult(
                success=False,
                content="Tool execution timeout",
                error="timeout"
            )

    def _build_tool_script(self, tool: BaseTool, arguments: Dict) -> str:
        """Generate Python script to execute tool."""
        import json
        args_json = json.dumps(arguments)
        return f"""
import asyncio
import sys
sys.path.insert(0, '.')

# Import and execute tool
from tools import {tool.__class__.__name__}
tool = {tool.__class__.__name__}()
result = asyncio.run(tool.execute({args_json}))
print(result.content)
"""
```

### E2E Integration

```
┌─────────────────────────────────────────────────────────────────┐
│                    Security Flow                                 │
└─────────────────────────────────────────────────────────────────┘

LLM Tool Call
│
▼
┌─────────────────────────┐
│  1. Policy Engine       │
│  - Block check          │
│  - Pattern matching     │
│  - Path validation      │
└─────────────────────────┘
│
▼ Pass
┌─────────────────────────┐
│  2. Sandbox Config      │
│  - Resource limits      │
│  - Network isolation    │
│  - Filesystem scope     │
└─────────────────────────┘
│
▼
┌─────────────────────────┐
│  3. Backend Execution   │
│  - Isolated process     │
│  - Timeout enforcement  │
└─────────────────────────┘
│
▼
┌─────────────────────────┐
│  4. Result Validation   │
│  - Output sanitization  │
│  - Secret redaction     │
└─────────────────────────┘
│
▼
Return to LLM
```

```python
class SecurityIntegration:
    """
    End-to-end security integration.
    """

    def __init__(
        self,
        policy_engine: PolicyEngine,
        sandbox: Sandbox,
        secret_redactor: SecretRedactor,
    ):
        self._policy_engine = policy_engine
        self._sandbox = sandbox
        self._secret_redactor = secret_redactor

    async def execute_with_security(
        self,
        tool: BaseTool,
        arguments: Dict[str, Any],
        context: SecurityContext,
    ) -> ToolResult:
        """Execute tool with full security stack."""

        # 1. Policy check
        allowed = await self._policy_engine.check(tool.name, arguments)
        if not allowed:
            return ToolResult(
                success=False,
                content="Blocked by security policy",
                error="policy_violation"
            )

        # 2. Sandbox execution
        result = await self._sandbox.execute(tool, arguments)

        # 3. Redact secrets from output
        if result.content:
            result.content = self._secret_redactor.redact(result.content)

        return result


class SecretRedactor:
    """Redact sensitive information from outputs."""

    PATTERNS = [
        (re.compile(r"sk-[a-zA-Z0-9]{32,}"), "[REDACTED_API_KEY]"),
        (re.compile(r"password\s*[:=]\s*\S+"), "password=[REDACTED]"),
        (re.compile(r"Bearer\s+[a-zA-Z0-9._-]+"), "Bearer [REDACTED_TOKEN]"),
        (re.compile(r"-----BEGIN\s+\w+\s+KEY-----"), "[REDACTED_KEY]"),
    ]

    def redact(self, text: str) -> str:
        """Redact all sensitive patterns."""
        for pattern, replacement in self.PATTERNS:
            text = pattern.sub(replacement, text)
        return text
```

### Safety Checks

```python
class SafetyChecker:
    """
    Post-execution safety validation.
    """

    def check(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
        result: ToolResult,
    ) -> List[SafetyIssue]:
        """Check for safety issues in tool execution."""
        issues = []

        # Check for unexpected file modifications
        if tool_name in ("write_file", "run_shell"):
            issues.extend(self._check_file_modifications(arguments, result))

        # Check for network activity
        if tool_name == "run_shell":
            issues.extend(self._check_network_activity(arguments, result))

        # Check for resource exhaustion
        issues.extend(self._check_resource_usage(result))

        return issues

    def _check_file_modifications(
        self,
        arguments: Dict,
        result: ToolResult,
    ) -> List[SafetyIssue]:
        """Validate file operations were expected."""
        issues = []

        # Check if result indicates unexpected modifications
        if result.success and "permission denied" in result.content.lower():
            issues.append(SafetyIssue(
                level="warning",
                message="Permission denied - possible unauthorized access"
            ))

        return issues

    def _check_network_activity(
        self,
        arguments: Dict,
        result: ToolResult,
    ) -> List[SafetyIssue]:
        """Detect unexpected network connections."""
        issues = []

        # Look for curl/wget in command
        command = arguments.get("command", "")
        if any(cmd in command for cmd in ["curl", "wget", "nc"]):
            issues.append(SafetyIssue(
                level="info",
                message="Network activity detected"
            ))

        return issues
```

---

## 8. Quality System

### Tool Quality Tracking

```python
from dataclasses import dataclass, field
from datetime import datetime
from typing import Dict, List, Optional
from collections import defaultdict
import statistics

@dataclass
class ToolQualityRecord:
    """Single tool execution record."""
    tool_key: str  # backend.tool_name
    success: bool
    latency_ms: float
    timestamp: datetime = field(default_factory=datetime.now)
    error_type: Optional[str] = None
    context: Dict[str, Any] = field(default_factory=dict)


class ToolQualityManager:
    """
    Track and analyze tool execution quality.

    Metrics:
    - Success rate
    - Latency (p50, p95, p99)
    - Error distribution
    - Trend analysis
    """

    def __init__(self, max_records_per_tool: int = 1000):
        self._records: Dict[str, List[ToolQualityRecord]] = defaultdict(list)
        self._max_records = max_records_per_tool

    def record_outcome(
        self,
        tool_key: str,
        success: bool,
        latency_ms: float,
        error_type: Optional[str] = None,
        context: Optional[Dict] = None,
    ) -> None:
        """Record a tool execution outcome."""
        record = ToolQualityRecord(
            tool_key=tool_key,
            success=success,
            latency_ms=latency_ms,
            error_type=error_type,
            context=context or {},
        )

        records = self._records[tool_key]
        records.append(record)

        # Keep bounded
        if len(records) > self._max_records:
            self._records[tool_key] = records[-self._max_records:]

    def get_metrics(self, tool_key: str) -> ToolMetrics:
        """Calculate metrics for a tool."""
        records = self._records.get(tool_key, [])

        if not records:
            return ToolMetrics()

        # Success rate
        success_count = sum(1 for r in records if r.success)
        success_rate = success_count / len(records)

        # Latency statistics
        latencies = [r.latency_ms for r in records]
        latencies.sort()

        metrics = ToolMetrics(
            tool_key=tool_key,
            total_executions=len(records),
            success_rate=success_rate,
            failure_rate=1 - success_rate,
            latency_p50=percentile(latencies, 50),
            latency_p95=percentile(latencies, 95),
            latency_p99=percentile(latencies, 99),
            latency_mean=statistics.mean(latencies),
            error_distribution=self._calculate_error_distribution(records),
        )

        return metrics

    def get_problematic_tools(
        self,
        success_threshold: float = 0.7,
        min_samples: int = 5,
    ) -> List[str]:
        """Find tools with degraded performance."""
        problematic = []

        for tool_key in self._records:
            metrics = self.get_metrics(tool_key)

            if metrics.total_executions < min_samples:
                continue

            if metrics.success_rate < success_threshold:
                problematic.append(tool_key)

        return problematic

    def get_trend(self, tool_key: str, window: int = 50) -> TrendDirection:
        """Analyze success rate trend."""
        records = self._records.get(tool_key, [])[-window:]

        if len(records) < 10:
            return TrendDirection.STABLE

        # Compare first half vs second half
        mid = len(records) // 2
        first_half_rate = sum(1 for r in records[:mid] if r.success) / mid
        second_half_rate = sum(1 for r in records[mid:] if r.success) / (len(records) - mid)

        diff = second_half_rate - first_half_rate

        if diff > 0.1:
            return TrendDirection.IMPROVING
        elif diff < -0.1:
            return TrendDirection.DEGRADING
        else:
            return TrendDirection.STABLE

    def _calculate_error_distribution(
        self,
        records: List[ToolQualityRecord],
    ) -> Dict[str, int]:
        """Count errors by type."""
        distribution = defaultdict(int)
        for record in records:
            if not record.success and record.error_type:
                distribution[record.error_type] += 1
        return dict(distribution)


@dataclass
class ToolMetrics:
    """Aggregated tool metrics."""
    tool_key: str = ""
    total_executions: int = 0
    success_rate: float = 0.0
    failure_rate: float = 0.0
    latency_p50: float = 0.0
    latency_p95: float = 0.0
    latency_p99: float = 0.0
    latency_mean: float = 0.0
    error_distribution: Dict[str, int] = field(default_factory=dict)


class TrendDirection(enum.Enum):
    IMPROVING = "improving"
    DEGRADING = "degrading"
    STABLE = "stable"
```

### Success Rate Monitoring

```python
class SuccessRateMonitor:
    """
    Real-time success rate monitoring with alerting.
    """

    def __init__(
        self,
        quality_manager: ToolQualityManager,
        alert_callback: Callable[[str, ToolMetrics], None],
    ):
        self._quality_manager = quality_manager
        self._alert_callback = alert_callback
        self._thresholds = {
            "critical": 0.5,  # Alert immediately
            "warning": 0.7,   # Log warning
        }

    async def check_all_tools(self) -> None:
        """Check all tools and alert if needed."""
        for tool_key in self._quality_manager._records:
            metrics = self._quality_manager.get_metrics(tool_key)

            if metrics.total_executions < 5:
                continue

            if metrics.success_rate < self._thresholds["critical"]:
                await self._alert_callback(
                    tool_key,
                    metrics,
                    level="critical"
                )
            elif metrics.success_rate < self._thresholds["warning"]:
                await self._alert_callback(
                    tool_key,
                    metrics,
                    level="warning"
                )

    async def _alert_callback(
        self,
        tool_key: str,
        metrics: ToolMetrics,
        level: str = "warning",
    ) -> None:
        """Handle alert."""
        message = (
            f"[{level.upper()}] Tool {tool_key} success rate: "
            f"{metrics.success_rate:.1%} "
            f"({metrics.total_executions} executions)"
        )

        if level == "critical":
            logger.error(message)
        else:
            logger.warning(message)
```

### Latency Tracking

```python
class LatencyTracker:
    """
    Track and analyze tool latency.
    """

    def __init__(self):
        self._latencies: Dict[str, List[float]] = defaultdict(list)
        self._baseline_latencies: Dict[str, float] = {}

    def record(self, tool_key: str, latency_ms: float) -> None:
        """Record latency."""
        self._latencies[tool_key].append(latency_ms)

        # Keep bounded
        if len(self._latencies[tool_key]) > 1000:
            self._latencies[tool_key] = self._latencies[tool_key][-1000:]

    def set_baseline(self, tool_key: str, latency_ms: float) -> None:
        """Set baseline latency for comparison."""
        self._baseline_latencies[tool_key] = latency_ms

    def is_anomalous(self, tool_key: str, latency_ms: float) -> bool:
        """Check if latency is anomalous."""
        if tool_key not in self._baseline_latencies:
            return False

        baseline = self._baseline_latencies[tool_key]

        # Anomaly if > 3x baseline
        return latency_ms > baseline * 3

    def get_percentiles(self, tool_key: str) -> Dict[str, float]:
        """Get latency percentiles."""
        latencies = sorted(self._latencies.get(tool_key, []))

        if not latencies:
            return {}

        return {
            "p50": percentile(latencies, 50),
            "p75": percentile(latencies, 75),
            "p90": percentile(latencies, 90),
            "p95": percentile(latencies, 95),
            "p99": percentile(latencies, 99),
        }
```

### Self-Evolution Triggers

```python
class SelfEvolutionTrigger:
    """
    Trigger skill evolution based on quality metrics.
    """

    def __init__(
        self,
        quality_manager: ToolQualityManager,
        skill_store: SkillStore,
        evolver: SkillEvolver,
    ):
        self._quality_manager = quality_manager
        self._skill_store = skill_store
        self._evolver = evolver

    async def check_and_trigger(self) -> List[EvolutionContext]:
        """Check for evolution triggers and initiate."""
        triggered = []

        # Get problematic tools
        problematic = self._quality_manager.get_problematic_tools(
            success_threshold=0.7,
            min_samples=5
        )

        for tool_key in problematic:
            # Find skills using this tool
            dependent_skills = self._skill_store.get_skills_using_tool(tool_key)

            for skill in dependent_skills:
                context = EvolutionContext(
                    trigger=EvolutionTrigger.TOOL_DEGRADATION,
                    suggestion=EvolutionSuggestion(
                        evolution_type=EvolutionType.FIX,
                        target_skill_ids=[skill.skill_id],
                        reason=f"Tool {tool_key} degraded, add fallback or alternative",
                    ),
                )

                result = await self._evolver.evolve(context)
                if result:
                    triggered.append(context)

        return triggered

    async def process_tool_degradation(self, tool_key: str) -> None:
        """Evolve all skills depending on problematic tool."""
        # Find dependent skills
        dependent_skills = self._skill_store.get_skills_using_tool(tool_key)

        for skill in dependent_skills:
            context = EvolutionContext(
                trigger=EvolutionTrigger.TOOL_DEGRADATION,
                suggestion=EvolutionSuggestion(
                    evolution_type=EvolutionType.FIX,
                    target_skill_ids=[skill.skill_id],
                    reason=f"Tool {tool_key} is failing, add fallback",
                ),
            )

            await self._evolver.evolve(context)
```

---

## 9. Transport Layer

### Connectors

```python
class TransportManager:
    """
    Manage transport connections for backends.

    Handles:
    - Connection pooling
    - Health checks
    - Automatic reconnection
    - Load balancing
    """

    def __init__(self):
        self._connectors: Dict[str, TransportConnector] = {}
        self._health_checks: Dict[str, asyncio.Task] = {}

    async def register(
        self,
        name: str,
        connector: TransportConnector,
        health_check_interval: float = 30.0,
    ) -> None:
        """Register a transport connector."""
        self._connectors[name] = connector

        # Start health check
        self._health_checks[name] = asyncio.create_task(
            self._run_health_checks(name, health_check_interval)
        )

    async def get(self, name: str) -> Optional[TransportConnector]:
        """Get connector by name."""
        connector = self._connectors.get(name)
        if connector and await connector.is_healthy():
            return connector

        # Attempt reconnect
        if connector:
            await connector.reconnect()
            if await connector.is_healthy():
                return connector

        return None

    async def _run_health_checks(
        self,
        name: str,
        interval: float,
    ) -> None:
        """Periodically check connector health."""
        while True:
            await asyncio.sleep(interval)

            connector = self._connectors.get(name)
            if connector:
                healthy = await connector.is_healthy()
                if not healthy:
                    logger.warning(f"Connector {name} unhealthy, attempting reconnect")
                    await connector.reconnect()

    async def shutdown(self) -> None:
        """Shutdown all connectors."""
        for task in self._health_checks.values():
            task.cancel()

        for connector in self._connectors.values():
            await connector.close()


class TransportConnector(ABC):
    """Abstract base for transport connectors."""

    @abstractmethod
    async def connect(self) -> None:
        pass

    @abstractmethod
    async def close(self) -> None:
        pass

    @abstractmethod
    async def is_healthy(self) -> bool:
        pass

    @abstractmethod
    async def reconnect(self) -> None:
        pass


class HTTPTransportConnector(TransportConnector):
    """HTTP transport connector with pooling."""

    def __init__(
        self,
        base_url: str,
        api_key: Optional[str] = None,
        pool_size: int = 10,
    ):
        self._base_url = base_url
        self._api_key = api_key
        self._session: Optional[aiohttp.ClientSession] = None
        self._pool_size = pool_size

    async def connect(self) -> None:
        connector = aiohttp.TCPConnector(limit=self._pool_size)
        self._session = aiohttp.ClientSession(
            connector=connector,
            headers={"Authorization": f"Bearer {self._api_key}"} if self._api_key else {},
        )

    async def close(self) -> None:
        if self._session:
            await self._session.close()
            self._session = None

    async def is_healthy(self) -> bool:
        if not self._session:
            return False

        try:
            async with self._session.get(f"{self._base_url}/health", timeout=5):
                return True
        except Exception:
            return False

    async def reconnect(self) -> None:
        await self.close()
        await self.connect()

    async def request(
        self,
        method: str,
        path: str,
        **kwargs,
    ) -> aiohttp.ClientResponse:
        """Make HTTP request."""
        if not self._session:
            await self.connect()

        url = f"{self._base_url}{path}"
        return await self._session.request(method, url, **kwargs)


class WebSocketTransportConnector(TransportConnector):
    """WebSocket transport connector."""

    def __init__(self, url: str):
        self._url = url
        self._ws: Optional[aiohttp.ClientWebSocketResponse] = None

    async def connect(self) -> None:
        self._ws = await aiohttp.ClientSession().ws_connect(self._url)

    async def close(self) -> None:
        if self._ws:
            await self._ws.close()
            self._ws = None

    async def is_healthy(self) -> bool:
        return self._ws is not None and not self._ws.closed

    async def reconnect(self) -> None:
        await self.close()
        await self.connect()

    async def send(self, message: Dict) -> None:
        await self._ws.send_json(message)

    async def receive(self) -> Dict:
        return await self._ws.receive_json()
```

### Task Managers

```python
class TaskManager:
    """
    Manage long-running tasks across backends.

    Features:
    - Task queuing
    - Priority scheduling
    - Progress tracking
    - Cancellation
    """

    def __init__(self, max_concurrent: int = 10):
        self._queue: asyncio.PriorityQueue = asyncio.PriorityQueue()
        self._running: Dict[str, asyncio.Task] = {}
        self._semaphore = asyncio.Semaphore(max_concurrent)

    async def submit(
        self,
        task_id: str,
        backend: str,
        operation: Callable,
        priority: int = 0,
    ) -> str:
        """Submit a task for execution."""
        await self._queue.put((priority, task_id, backend, operation))
        return task_id

    async def run(self) -> None:
        """Process task queue."""
        while True:
            priority, task_id, backend, operation = await self._queue.get()

            async with self._semaphore:
                task = asyncio.create_task(self._execute(task_id, operation))
                self._running[task_id] = task

            try:
                await task
            finally:
                del self._running[task_id]

    async def _execute(self, task_id: str, operation: Callable) -> Any:
        """Execute a single task."""
        try:
            return await operation()
        except Exception as e:
            logger.error(f"Task {task_id} failed: {e}")
            raise

    async def cancel(self, task_id: str) -> bool:
        """Cancel a running task."""
        task = self._running.get(task_id)
        if task:
            task.cancel()
            return True
        return False

    def get_status(self, task_id: str) -> TaskStatus:
        """Get task status."""
        if task_id in self._running:
            task = self._running[task_id]
            if task.done():
                return TaskStatus.COMPLETED
            return TaskStatus.RUNNING
        return TaskStatus.PENDING


class TaskStatus(enum.Enum):
    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"
```

### Message Routing

```python
class MessageRouter:
    """
    Route messages between components.

    Handles:
    - Tool call routing to backends
    - Result routing back to LLM
    - Error propagation
    """

    def __init__(self):
        self._routes: Dict[str, List[RouteHandler]] = defaultdict(list)
        self._error_handlers: List[ErrorHandler] = []

    def register_route(
        self,
        pattern: str,
        handler: RouteHandler,
    ) -> None:
        """Register a route handler."""
        self._routes[pattern].append(handler)

    async def route(
        self,
        message: Message,
    ) -> Optional[Message]:
        """Route message to appropriate handler."""
        handlers = self._routes.get(message.type, [])

        for handler in handlers:
            if await handler.matches(message):
                return await handler.handle(message)

        return None

    def register_error_handler(self, handler: ErrorHandler) -> None:
        """Register error handler."""
        self._error_handlers.append(handler)

    async def handle_error(
        self,
        error: Exception,
        context: Dict,
    ) -> None:
        """Handle error with registered handlers."""
        for handler in self._error_handlers:
            await handler.handle(error, context)


class Message:
    """Message for routing."""

    def __init__(
        self,
        type: str,
        payload: Dict,
        correlation_id: Optional[str] = None,
    ):
        self.type = type
        self.payload = payload
        self.correlation_id = correlation_id or str(uuid.uuid4())


class RouteHandler(ABC):
    """Abstract route handler."""

    @abstractmethod
    async def matches(self, message: Message) -> bool:
        pass

    @abstractmethod
    async def handle(self, message: Message) -> Optional[Message]:
        pass
```

### Error Handling

```python
class ErrorHandling:
    """
    Centralized error handling with retry logic.
    """

    def __init__(self, max_retries: int = 3):
        self._max_retries = max_retries
        self._retry_delays = [1.0, 2.0, 4.0]  # Exponential backoff

    async def execute_with_retry(
        self,
        operation: Callable,
        operation_name: str,
        retryable_errors: List[type] = None,
    ) -> Any:
        """Execute operation with retry logic."""
        retryable = retryable_errors or [Exception]

        last_error = None
        for attempt in range(self._max_retries):
            try:
                return await operation()
            except Exception as e:
                last_error = e

                # Check if retryable
                if not any(isinstance(e, t) for t in retryable):
                    raise

                # Log retry
                logger.warning(
                    f"{operation_name} failed (attempt {attempt + 1}): {e}"
                )

                # Wait before retry
                if attempt < self._max_retries - 1:
                    await asyncio.sleep(self._retry_delays[attempt])

        raise last_error

    def categorize_error(self, error: Exception) -> ErrorCategory:
        """Categorize error for handling."""
        if isinstance(error, asyncio.TimeoutError):
            return ErrorCategory.TIMEOUT
        elif isinstance(error, aiohttp.ClientError):
            return ErrorCategory.NETWORK
        elif isinstance(error, json.JSONDecodeError):
            return ErrorCategory.PARSE
        elif isinstance(error, PermissionError):
            return ErrorCategory.PERMISSION
        else:
            return ErrorCategory.UNKNOWN


class ErrorCategory(enum.Enum):
    TIMEOUT = "timeout"
    NETWORK = "network"
    PARSE = "parse"
    PERMISSION = "permission"
    UNKNOWN = "unknown"


class ErrorHandler(ABC):
    """Abstract error handler."""

    @abstractmethod
    async def handle(
        self,
        error: Exception,
        context: Dict,
    ) -> None:
        pass


class LoggingErrorHandler(ErrorHandler):
    """Log errors."""

    async def handle(self, error: Exception, context: Dict) -> None:
        logger.error(f"Error in {context.get('operation', 'unknown')}: {error}")


class FallbackErrorHandler(ErrorHandler):
    """Provide fallback response on error."""

    def __init__(self, fallback_response: str):
        self._fallback_response = fallback_response

    async def handle(self, error: Exception, context: Dict) -> None:
        context["fallback_response"] = self._fallback_response
```

---

## 10. System Provider

### System-Level Tools

```python
class SystemProvider:
    """
    System-level abstraction and tools.

    Provides:
    - Platform detection
    - System info
    - Screenshot capabilities
    - Resource monitoring
    """

    def __init__(self):
        self._platform = sys.platform
        self._tools = self._create_tools()

    def _create_tools(self) -> List[LocalTool]:
        """Create system-level tools."""
        return [
            LocalTool(
                metadata=ToolMetadata(
                    name="get_system_info",
                    description="Get system information (OS, CPU, memory)",
                    category="system",
                    tags=["system", "info", "resources"],
                ),
                schema=ToolSchema(
                    properties={},
                ),
                handler=lambda _: self._get_system_info(),
            ),
            LocalTool(
                metadata=ToolMetadata(
                    name="take_screenshot",
                    description="Take a screenshot",
                    category="system",
                    tags=["screenshot", "visual"],
                ),
                schema=ToolSchema(
                    properties={
                        "region": {
                            "type": "object",
                            "properties": {
                                "x": {"type": "integer"},
                                "y": {"type": "integer"},
                                "width": {"type": "integer"},
                                "height": {"type": "integer"},
                            },
                        },
                    },
                ),
                handler=lambda args: self._take_screenshot(args),
            ),
            LocalTool(
                metadata=ToolMetadata(
                    name="get_resource_usage",
                    description="Get current resource usage",
                    category="system",
                    tags=["resources", "cpu", "memory"],
                ),
                schema=ToolSchema(properties={}),
                handler=lambda _: self._get_resource_usage(),
            ),
        ]

    def _get_system_info(self) -> str:
        """Get system information."""
        import platform
        import psutil

        info = {
            "os": platform.system(),
            "os_version": platform.version(),
            "machine": platform.machine(),
            "processor": platform.processor(),
            "cpu_count": psutil.cpu_count(),
            "memory_total_gb": psutil.virtual_memory().total / (1024**3),
        }

        return json.dumps(info, indent=2)

    def _take_screenshot(self, arguments: Dict) -> str:
        """Take screenshot."""
        import pyautogui
        import base64
        from io import BytesIO

        region = arguments.get("region")

        if region:
            screenshot = pyautogui.screenshot(
                region=(
                    region["x"],
                    region["y"],
                    region["width"],
                    region["height"],
                )
            )
        else:
            screenshot = pyautogui.screenshot()

        # Save and return path
        filepath = f"/tmp/screenshot_{datetime.now().strftime('%Y%m%d_%H%M%S')}.png"
        screenshot.save(filepath)

        return f"Screenshot saved to {filepath}"

    def _get_resource_usage(self) -> str:
        """Get current resource usage."""
        import psutil

        cpu_percent = psutil.cpu_percent(interval=1)
        memory = psutil.virtual_memory()
        disk = psutil.disk_usage("/")

        info = {
            "cpu_percent": cpu_percent,
            "memory_percent": memory.percent,
            "memory_used_gb": memory.used / (1024**3),
            "disk_percent": disk.percent,
        }

        return json.dumps(info, indent=2)

    def get_tools(self) -> List[LocalTool]:
        return self._tools


class PlatformAbstraction:
    """
    Cross-platform abstraction layer.
    """

    def __init__(self):
        self._platform = sys.platform

    def get_clipboard(self) -> str:
        """Get clipboard content."""
        if self._platform == "linux":
            return self._get_clipboard_linux()
        elif self._platform == "darwin":
            return self._get_clipboard_macos()
        else:
            return self._get_clipboard_windows()

    def set_clipboard(self, text: str) -> None:
        """Set clipboard content."""
        if self._platform == "linux":
            self._set_clipboard_linux(text)
        elif self._platform == "darwin":
            self._set_clipboard_macos(text)
        else:
            self._set_clipboard_windows(text)

    def open_file(self, path: str) -> None:
        """Open file with default application."""
        if self._platform == "linux":
            subprocess.run(["xdg-open", path])
        elif self._platform == "darwin":
            subprocess.run(["open", path])
        else:
            os.startfile(path)

    def _get_clipboard_linux(self) -> str:
        import subprocess
        result = subprocess.run(["xclip", "-selection", "clipboard", "-o"],
                                capture_output=True, text=True)
        return result.stdout

    def _set_clipboard_linux(self, text: str) -> None:
        import subprocess
        subprocess.run(["xclip", "-selection", "clipboard"], input=text, text=True)

    def _get_clipboard_macos(self) -> str:
        import subprocess
        result = subprocess.run(["pbpaste"], capture_output=True, text=True)
        return result.stdout

    def _set_clipboard_macos(self, text: str) -> None:
        import subprocess
        subprocess.run(["pbcopy"], input=text, text=True)

    def _get_clipboard_windows(self) -> str:
        import subprocess
        result = subprocess.run(["clip"], capture_output=True, text=True)
        return result.stdout

    def _set_clipboard_windows(self, text: str) -> None:
        import subprocess
        subprocess.run(["clip"], input=text, text=True)
```

### Screenshots

```python
class ScreenshotManager:
    """
    Screenshot capture and analysis.
    """

    def __init__(self, output_dir: str = "./screenshots"):
        self._output_dir = output_dir
        os.makedirs(output_dir, exist_ok=True)

    def capture(self, region: Optional[Dict] = None) -> ScreenshotResult:
        """Capture screenshot."""
        import pyautogui
        from datetime import datetime

        if region:
            screenshot = pyautogui.screenshot(
                region=(region["x"], region["y"], region["width"], region["height"])
            )
        else:
            screenshot = pyautogui.screenshot()

        # Generate filename
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filepath = os.path.join(self._output_dir, f"screenshot_{timestamp}.png")

        screenshot.save(filepath)

        return ScreenshotResult(
            filepath=filepath,
            width=screenshot.width,
            height=screenshot.height,
            base64=self._to_base64(screenshot),
        )

    def _to_base64(self, screenshot) -> str:
        """Convert to base64."""
        from io import BytesIO
        import base64

        buffered = BytesIO()
        screenshot.save(buffered, format="PNG")
        return base64.b64encode(buffered.getvalue()).decode()

    async def analyze(
        self,
        screenshot: ScreenshotResult,
        llm_client: LLMClient,
        prompt: str = "What is shown in this screenshot?",
    ) -> str:
        """Analyze screenshot with LLM."""
        return await llm_client.chat_with_image(
            image_base64=screenshot.base64,
            prompt=prompt,
        )


@dataclass
class ScreenshotResult:
    filepath: str
    width: int
    height: int
    base64: str
```

### System Info

```python
class SystemInfo:
    """
    System information provider.
    """

    @staticmethod
    def get_cpu_info() -> Dict:
        import psutil

        return {
            "physical_cores": psutil.cpu_count(logical=False),
            "logical_cores": psutil.cpu_count(logical=True),
            "current_freq_mhz": psutil.cpu_freq().current if psutil.cpu_freq() else 0,
        }

    @staticmethod
    def get_memory_info() -> Dict:
        import psutil

        mem = psutil.virtual_memory()
        return {
            "total_gb": mem.total / (1024**3),
            "available_gb": mem.available / (1024**3),
            "used_percent": mem.percent,
        }

    @staticmethod
    def get_disk_info() -> Dict:
        import psutil

        disk = psutil.disk_usage("/")
        return {
            "total_gb": disk.total / (1024**3),
            "used_gb": disk.used / (1024**3),
            "free_gb": disk.free / (1024**3),
            "used_percent": disk.percent,
        }

    @staticmethod
    def get_network_interfaces() -> List[Dict]:
        import psutil

        interfaces = []
        for name, addrs in psutil.net_if_addrs().items():
            for addr in addrs:
                if addr.family == psutil.AF_LINK:
                    interfaces.append({
                        "name": name,
                        "mac": addr.address,
                    })
        return interfaces

    @staticmethod
    def get_processes() -> List[Dict]:
        import psutil

        processes = []
        for proc in psutil.process_iter(["pid", "name", "cpu_percent", "memory_percent"]):
            try:
                processes.append(proc.info)
            except (psutil.NoSuchProcess, psutil.AccessDenied):
                pass

        return sorted(processes, key=lambda p: p["memory_percent"], reverse=True)[:20]
```

---

## Appendix: Backend Architecture Diagrams

### Complete Backend Flow

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Backend Architecture                             │
└─────────────────────────────────────────────────────────────────────────┘

                              ┌─────────────┐
                              │    LLM      │
                              │  (decides   │
                              │   actions)  │
                              └──────┬──────┘
                                     │
                          Tool Call  │
                          ┌──────────┘
                          │
                          ▼
                    ┌─────────────┐
                    │ Grounding   │
                    │   Client    │
                    └──────┬──────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
         ▼                 ▼                 ▼
   ┌───────────┐   ┌───────────┐   ┌───────────┐
   │  Policy   │   │   Tool    │   │  Quality  │
   │  Engine   │   │  Lookup   │   │  Manager  │
   └─────┬─────┘   └─────┬─────┘   └─────┬─────┘
         │               │               │
         └───────────────┼───────────────┘
                         │
                         ▼
                   ┌───────────┐
                   │  Backend  │
                   │   Router  │
                   └─────┬─────┘
                         │
         ┌───────────────┼───────────────┐
         │               │               │
         ▼               ▼               ▼
   ┌───────────┐ ┌───────────┐ ┌───────────┐
   │   Shell   │ │    GUI    │ │    MCP    │
   │  Backend  │ │  Backend  │ │  Backend  │
   └─────┬─────┘ └─────┬─────┘ └─────┬─────┘
         │             │             │
         │             │             │
         ▼             ▼             ▼
   ┌───────────┐ ┌───────────┐ ┌───────────┐
   │   bash    │ │ pyautogui │ │  stdio/   │
   │   cmds    │ │  screen   │ │   HTTP    │
   └───────────┘ └───────────┘ └───────────┘
```

### Tool Calling Sequence

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Tool Call Sequence                                │
└─────────────────────────────────────────────────────────────────────────┘

  Agent           GroundingClient        Backend           External
    │                    │                   │                 │
    │  Tool Call         │                   │                 │
    │───────────────────>│                   │                 │
    │                    │                   │                 │
    │                    │ Policy Check      │                 │
    │                    │────────┐          │                 │
    │                    │        │          │                 │
    │                    │<───────┘          │                 │
    │                    │                   │                 │
    │                    │ Tool Lookup       │                 │
    │                    │────────┐          │                 │
    │                    │        │          │                 │
    │                    │<───────┘          │                 │
    │                    │                   │                 │
    │                    │ Route to Backend  │                 │
    │                    │──────────────────>│                 │
    │                    │                   │                 │
    │                    │                   │ Execute         │
    │                    │                   │────────────────>│
    │                    │                   │                 │
    │                    │                   │                 │
    │                    │                   │ Result          │
    │                    │                   │<────────────────│
    │                    │                   │                 │
    │                    │ Record Quality    │                 │
    │                    │────────┐          │                 │
    │                    │        │          │                 │
    │                    │<───────┘          │                 │
    │                    │                   │                 │
    │  Result            │                   │                 │
    │<───────────────────│                   │                 │
    │                    │                   │                 │
```

---

## Summary

OpenSpace's grounding system provides:

1. **Unified Interface**: All backends implement `BaseBackend` with consistent `call()` API
2. **Multiple Backends**: Shell, GUI, MCP, and Web backends for diverse capabilities
3. **Tool Abstraction**: `BaseTool` with local and remote implementations
4. **Smart Selection**: Tool RAG with BM25 + embeddings + LLM ranking
5. **Security First**: Policy engine, sandboxing, and safety checks
6. **Quality Tracking**: Success rates, latency monitoring, degradation detection
7. **Self-Evolution**: Automatic triggers when tools degrade
8. **Robust Transport**: Connectors with health checks and reconnection
9. **System Integration**: Platform abstraction and system-level tools

This architecture enables OpenSpace to execute tasks across any environment while maintaining security, tracking quality, and continuously improving through automatic skill evolution.
