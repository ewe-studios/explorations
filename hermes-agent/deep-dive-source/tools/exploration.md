# tools/ Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/tools/`

**Status:** complete

---

## Module Overview

The `tools/` module contains the tool implementations that form the action space of Hermes Agent. This ~38,602 line module provides 50+ tools across categories like terminal access, file operations, web interaction, browser automation, MCP integration, memory, skills management, and specialized utilities.

Tools follow a registration pattern where each tool file calls `registry.register()` at module level to declare its schema, handler, toolset membership, and availability check. The registry is queried by `model_tools.py` to provide tool definitions to the LLM.

Key features:
- **Toolsets** - Logical groupings of tools (terminal, file, web, browser, etc.)
- **Availability checks** - Tools only appear when their requirements are met
- **Async execution** - Thread pool for blocking tool calls
- **Approval system** - User approval for dangerous operations
- **MCP integration** - Dynamic tools from MCP servers
- **Environment abstraction** - Pluggable sandbox backends (local, Docker, Modal, etc.)

---

## Directory Structure

### Core Files

| File | Lines | Purpose |
|------|-------|---------|
| `__init__.py` | 25 | Package exports |
| `registry.py` | 275 | Tool registry |
| `approval.py` | 873 | Approval system |
| `interrupt.py` | 28 | Interrupt tool |
| `managed_tool_gateway.py` | 167 | Tool gateway wrapper |

### Core Tools

| File | Lines | Purpose |
|------|-------|---------|
| `terminal_tool.py` | 1,600 | Terminal command execution |
| `file_tools.py` | 828 | File read/write/patch |
| `file_operations.py` | 1,170 | Advanced file operations |
| `send_message_tool.py` | 952 | Send messages via gateway |
| `todo_tool.py` | 268 | Todo list management |
| `delegate_tool.py` | 881 | Subagent delegation |
| `memory_tool.py` | 560 | Memory operations |
| `mcp_tool.py` | 2,176 | MCP client tools |
| `clarify_tool.py` | 141 | Clarification requests |

### Web & Browser Tools

| File | Lines | Purpose |
|------|-------|---------|
| `web_tools.py` | 2,099 | Web search and extraction |
| `browser_tool.py` | 2,202 | Browser automation |
| `browser_camofox.py` | 571 | Camofox browser integration |
| `browser_camofox_state.py` | 47 | Browser state management |
| `website_policy.py` | 283 | Website access policies |
| `url_safety.py` | 106 | URL safety checking |
| `vision_tools.py` | 614 | Image analysis |
| `image_generation_tool.py` | 703 | Image generation |

### Code & Execution

| File | Lines | Purpose |
|------|-------|---------|
| `code_execution_tool.py` | 1,347 | Python code execution |
| `process_registry.py` | 889 | Process tracking |
| `checkpoint_manager.py` | 548 | Execution checkpoints |
| `patch_parser.py` | 455 | Patch/diff parsing |
| `debug_helpers.py` | 106 | Debugging utilities |

### Skills Management

| File | Lines | Purpose |
|------|-------|---------|
| `skills_hub.py` | 2,707 | Skills management |
| `skills_tool.py` | 1,378 | Skills operations |
| `skills_guard.py` | 1,105 | Skills safety |
| `skills_sync.py` | 295 | Skills synchronization |
| `skill_manager_tool.py` | 747 | Skill lifecycle |

### RL Training

| File | Lines | Purpose |
|------|-------|---------|
| `rl_training_tool.py` | 1,402 | RL training interface |
| `mixture_of_agents_tool.py` | 562 | Mixture of agents |
| `transcription_tools.py` | 622 | Audio transcription |
| `tts_tool.py` | 984 | Text-to-speech |
| `voice_mode.py` | 812 | Voice interaction |

### Environment Backends

| File | Lines | Purpose |
|------|-------|---------|
| `environments/base.py` | 112 | Environment base class |
| `environments/local.py` | 486 | Local environment |
| `environments/docker.py` | 601 | Docker environment |
| `environments/modal.py` | 445 | Modal environment |
| `environments/daytona.py` | 299 | Daytona environment |
| `environments/ssh.py` | 313 | SSH environment |
| `environments/singularity.py` | 395 | Singularity (HPC) |
| `environments/persistent_shell.py` | 290 | Persistent shell |
| `environments/managed_modal.py` | 282 | Managed Modal |
| `environments/modal_common.py` | 178 | Modal shared code |

### MCP & Integration

| File | Lines | Purpose |
|------|---------|---------|
| `mcp_oauth.py` | 482 | MCP OAuth handling |
| `homeassistant_tool.py` | 490 | Home Assistant |
| `cronjob_tools.py` | 525 | Cron job management |
| `session_search_tool.py` | 504 | Session search |
| `osv_check.py` | 155 | OSV vulnerability check |
| `tirith_security.py` | 670 | Tirith security framework |

### Utilities

| File | Lines | Purpose |
|------|-------|---------|
| `fuzzy_match.py` | 482 | Fuzzy string matching |
| `env_passthrough.py` | 112 | Environment variable access |
| `credential_files.py` | 416 | Credential file access |
| `tool_backend_helpers.py` | 96 | Backend helpers |
| `neutts_synth.py` | 104 | Neutts synthesis |

### Browser Providers

| File | Lines | Purpose |
|------|-------|---------|
| `browser_providers/__init__.py` | 10 | Providers package |
| `browser_providers/base.py` | 59 | Provider base |
| `browser_providers/browser_use.py` | 107 | Browser-use provider |
| `browser_providers/firecrawl.py` | 107 | Firecrawl provider |
| `browser_providers/browserbase.py` | 281 | Browserbase provider |

### Neutts Samples

| File | Lines | Purpose |
|------|-------|---------|
| `neutts_samples/` | - | Sample Neutts outputs |

**Total:** ~38,602 lines across 55+ files

---

## Key Components

### 1. Tool Registry (`registry.py`)

Central registry for all tools.

**Key Classes:**
```python
class ToolEntry:
    """Metadata for a single registered tool."""
    __slots__ = ("name", "toolset", "schema", "handler", "check_fn",
                 "requires_env", "is_async", "description", "emoji")

class ToolRegistry:
    """Singleton registry that collects tool schemas + handlers."""
    
    def register(
        self,
        name: str,
        toolset: str,
        schema: dict,
        handler: Callable,
        check_fn: Callable = None,
        requires_env: list = None,
        is_async: bool = False,
        description: str = "",
        emoji: str = "",
    ):
        """Register a tool. Called at module-import time."""
    
    def deregister(self, name: str) -> None:
        """Remove a tool from the registry."""
    
    def get_tools_for_toolset(self, toolset: str) -> List[ToolEntry]:
        """Get all tools in a toolset."""
    
    def get_all_tools(self) -> List[ToolEntry]:
        """Get all registered tools."""
    
    def check_toolset(self, toolset: str) -> bool:
        """Check if a toolset is available."""
```

**Registration Pattern:**
```python
# In each tool file
from tools.registry import registry

@registry.register(
    name="read_file",
    toolset="file",
    schema={
        "name": "read_file",
        "description": "Read the contents of a file",
        "parameters": {
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "File path"}
            },
            "required": ["path"]
        }
    },
    handler=read_file_handler,
    requires_env=["terminal_backend"],
    is_async=True,
)
def read_file_handler(path: str) -> str:
    """Read file contents."""
    with open(path, 'r') as f:
        return f.read()
```

### 2. Terminal Tool (`terminal_tool.py`)

Terminal command execution with sandbox support.

**Key Features:**
- Multiple backends (local, Docker, Modal, Daytona, SSH, Singularity)
- Per-task sandbox isolation
- Timeout enforcement
- Output streaming
- Process tracking

**Key Functions:**
```python
def terminal_tool_handler(
    command: str,
    timeout: int = 120,
    background: bool = False,
) -> str:
    """Execute a terminal command.
    
    Args:
        command: Shell command to execute
        timeout: Timeout in seconds
        background: Run in background (non-blocking)
    
    Returns:
        Command output (stdout + stderr)
    """

def register_task_env_overrides(task_id: str, overrides: dict) -> None:
    """Register environment overrides for a task."""

def clear_task_env_overrides(task_id: str) -> None:
    """Clear environment overrides for a task."""
```

**Backend Selection:**
```python
TERMINAL_BACKENDS = {
    "local": LocalEnvironment,      # Subprocess
    "docker": DockerEnvironment,    # Docker containers
    "modal": ModalEnvironment,      # Modal.com cloud
    "daytona": DaytonaEnvironment,  # Daytona sandbox cloud
    "ssh": SSHEnvironment,          # Remote SSH
    "singularity": SingularityEnv,  # Singularity (HPC)
}
```

### 3. File Tools (`file_tools.py`, `file_operations.py`)

File manipulation operations.

**Tools:**
```python
# read_file - Read file contents
def read_file_handler(path: str, start_line: int = None, end_line: int = None) -> str:
    """Read file contents with optional line range."""

# write_file - Write file contents
def write_file_handler(path: str, content: str, append: bool = False) -> str:
    """Write content to file."""

# patch_file - Apply a patch to a file
def patch_file_handler(path: str, patch: str) -> str:
    """Apply a unified diff patch to a file."""

# list_directory - List directory contents
def list_directory_handler(path: str, recursive: bool = False) -> str:
    """List directory contents."""

# search_files - Search for files by pattern
def search_files_handler(pattern: str, path: str = ".") -> str:
    """Search for files matching a glob pattern."""

# grep - Search file contents
def grep_handler(pattern: str, path: str = ".", include: str = "*") -> str:
    """Search file contents with regex."""
```

### 4. Web Tools (`web_tools.py`)

Web search and content extraction.

**Tools:**
```python
# web_search - Search the web
def web_search_handler(query: str, num_results: int = 10) -> str:
    """Search the web using configured search engine."""

# web_extract - Extract content from URLs
def web_extract_handler(urls: List[str], instructions: str = None) -> str:
    """Extract and summarize content from URLs."""

# web_scrape - Scrape webpage content
def web_scrape_handler(url: str) -> str:
    """Scrape full webpage content."""
```

**Search Providers:**
- Tavily API
- SerpAPI
- Google Custom Search
- DuckDuckGo (unofficial)

### 5. Browser Tool (`browser_tool.py`)

Browser automation for interactive web tasks.

**Key Features:**
- Playwright-based browser control
- Navigation, clicking, typing
- Screenshot capture
- Content extraction
- Form filling

**Tools:**
```python
# browser_navigate - Navigate to URL
def browser_navigate_handler(url: str) -> str:
    """Navigate browser to URL."""

# browser_click - Click an element
def browser_click_handler(ref: str) -> str:
    """Click an element by reference."""

# browser_type - Type text into an element
def browser_type_handler(ref: str, text: str) -> str:
    """Type text into an element."""

# browser_screenshot - Capture screenshot
def browser_screenshot_handler() -> str:
    """Capture and return browser screenshot."""

# browser_content - Get page content
def browser_content_handler() -> str:
    """Get current page content."""
```

**Browser Providers:**
- Local Playwright
- Browser-use
- Firecrawl
- Browserbase (cloud)

### 6. MCP Tool (`mcp_tool.py`)

Model Context Protocol integration for dynamic tools.

**Key Features:**
- Connect to MCP servers
- Dynamic tool discovery
- Server lifecycle management
- Tool call forwarding

**Key Functions:**
```python
def mcp_connect_handler(server_name: str, config: dict) -> str:
    """Connect to an MCP server."""

def mcp_disconnect_handler(server_name: str) -> str:
    """Disconnect from an MCP server."""

def mcp_list_tools_handler(server_name: str) -> str:
    """List tools from connected MCP server."""

def mcp_call_tool_handler(
    server_name: str,
    tool_name: str,
    arguments: dict
) -> str:
    """Call a tool on an MCP server."""
```

**MCP Server Config:**
```yaml
mcp:
  servers:
    filesystem:
      command: npx
      args: ["-y", "@modelcontextprotocol/server-filesystem"]
      root: /workspace
    github:
      command: npx
      args: ["-y", "@modelcontextprotocol/server-github"]
      env:
        GITHUB_TOKEN: ${GITHUB_TOKEN}
```

### 7. Approval System (`approval.py`)

User approval for dangerous operations.

**Key Classes:**
```python
class ApprovalManager:
    """Manages tool call approvals."""
    
    def __init__(self, config: dict):
        self.config = config
        self.auto_approve = set(config.get("auto_approve", []))
        self.require_approval = set(config.get("require_approval", []))
    
    def should_approve(self, tool_name: str, args: dict) -> bool:
        """Check if a tool call requires approval."""
    
    async def request_approval(
        self, 
        tool_name: str, 
        args: dict,
        preview: str
    ) -> bool:
        """Request user approval for a tool call."""
```

**Approval Modes:**
- `auto` - Auto-approve listed tools
- `manual` - Require approval for listed tools
- `interactive` - Prompt for all dangerous ops

### 8. Delegate Tool (`delegate_tool.py`)

Subagent delegation for parallel task execution.

**Key Function:**
```python
def delegate_tool_handler(
    task: str,
    context: dict = None,
    model: str = None,
    tools: List[str] = None,
) -> str:
    """Delegate a subtask to a subagent.
    
    Args:
        task: Task description for subagent
        context: Context to pass to subagent
        model: Model to use (defaults to parent model)
        tools: Tools available to subagent
    
    Returns:
        Subagent's response
    """
```

**Features:**
- Independent subagent sessions
- Context passing
- Result integration
- Parallel execution support

### 9. Memory Tool (`memory_tool.py`)

Long-term memory operations.

**Tools:**
```python
# memory_read - Read from memory
def memory_read_handler(query: str) -> str:
    """Query long-term memory."""

# memory_write - Write to memory
def memory_write_handler(content: str, category: str = None) -> str:
    """Write content to long-term memory."""

# memory_delete - Delete memory entries
def memory_delete_handler(ids: List[str]) -> str:
    """Delete memory entries by ID."""

# memory_list - List memory entries
def memory_list_handler(category: str = None, limit: int = 100) -> str:
    """List memory entries."""
```

**Memory Providers:**
- Built-in (MEMORY.md/USER.md files)
- Honcho
- Holographic
- OpenViking
- RetainDB
- Mem0

### 10. Skills Tools

Skills management and execution.

**Key Tools:**
```python
# skills_list - List available skills
# skills_install - Install a skill
# skills_enable - Enable a skill
# skills_disable - Disable a skill
# skills_run - Run a skill
```

### 11. Environment Backends

Sandbox abstraction for tool execution.

**Base Interface:**
```python
class Environment(ABC):
    """Abstract base for tool execution environments."""
    
    @abstractmethod
    async def run_command(
        self, 
        command: str, 
        timeout: int = 120
    ) -> Tuple[int, str, str]:
        """Run a command and return (exit_code, stdout, stderr)."""
    
    @abstractmethod
    async def read_file(self, path: str) -> str:
        """Read a file from the environment."""
    
    @abstractmethod
    async def write_file(self, path: str, content: str) -> None:
        """Write a file to the environment."""
    
    @abstractmethod
    async def cleanup(self) -> None:
        """Clean up environment resources."""
```

**Backend Comparison:**
| Backend | Isolation | Speed | Cost | Best For |
|---------|-----------|-------|------|----------|
| Local | None | Fast | Free | Development |
| Docker | Container | Medium | Free | Local testing |
| Modal | Cloud VM | Medium | Pay-per-use | Production RL |
| Daytona | Cloud sandbox | Medium | Subscription | Production RL |
| SSH | Remote server | Fast | Fixed | Dedicated hardware |
| Singularity | HPC container | Medium | Free | Academic clusters |

---

## Toolsets

Tools are organized into logical toolsets:

| Toolset | Tools | Description |
|---------|-------|-------------|
| `terminal` | terminal, process operations | Command execution |
| `file` | read_file, write_file, patch, search | File operations |
| `web` | web_search, web_extract, web_scrape | Web interaction |
| `browser` | browser_*, vision | Browser automation |
| `code` | code_execution, patch_parser | Code operations |
| `memory` | memory_*, session_search | Memory operations |
| `skills` | skills_*, skill_manager | Skills management |
| `mcp` | mcp_*, mcp_oauth | MCP integration |
| `delegation` | delegate | Subagent delegation |
| `communication` | send_message | Platform messaging |
| `approval` | approval_*, clarify | User interaction |
| `rl_training` | rl_training_*, mixture_of_agents | RL training |
| `voice` | tts, transcription, voice_mode | Voice features |
| `homeassistant` | homeassistant_*, webhook | Home automation |
| `security` | tirith_security, osv_check | Security scanning |

---

## Integration Points

### With Model Tools (`model_tools.py`)
- Tool definitions provided to LLM
- Tool call parsing and dispatch

### With Agent (`agent/`)
- Tool execution via AIAgent
- Approval callback integration

### With CLI (`hermes_cli/`)
- Tools configuration commands
- Toolset enable/disable

### With Gateway (`gateway/`)
- send_message_tool routes via gateway
- Platform-specific tool filtering

---

## Related Files

**Individual File Explorations:**
- [tools-core.md](./tools/tools-core.md) - Comprehensive tools analysis
- [registry.md](./tools/registry.md)
- [approval.md](./tools/approval.md)
- [terminal_tool.md](./tools/terminal_tool.md)
- [file_tools.md](./tools/file_tools.md)
- [web_tools.md](./tools/web_tools.md)
- [browser_tool.md](./tools/browser_tool.md)
- [mcp_tool.md](./tools/mcp_tool.md)

**Related Modules:**
- [model_tools.md](../root/model_tools.md) - Tool integration
- [toolsets.md](../root/toolsets.md) - Toolset definitions
- [hermes_cli/tools_config.md](../hermes_cli/tools_config.md) - Tools CLI

---

*Deep dive created: 2026-04-07*
