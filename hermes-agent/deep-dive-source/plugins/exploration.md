# Plugins Module - Comprehensive Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/plugins/`

**Output:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/hermes-agent/deep-dive-source/plugins/exploration.md`

---

## Table of Contents

1. [Directory Structure](#directory-structure)
2. [Architecture Overview](#architecture-overview)
3. [Memory Provider Plugin System](#memory-provider-plugin-system)
4. [Plugin Discovery & Loading](#plugin-discovery--loading)
5. [MemoryProvider ABC Interface](#memoryprovider-abc-interface)
6. [Individual Memory Providers](#individual-memory-providers)
7. [Plugin Lifecycle and Registration](#plugin-lifecycle-and-registration)
8. [Extension Points](#extension-points)
9. [Cross-References](#cross-references)

---

## Directory Structure

```
plugins/
├── __init__.py                          # Package marker ("Hermes plugins package")
└── memory/
    ├── __init__.py                      # Memory provider discovery module
    │
    ├── byterover/                       # ByteRover - Hierarchical knowledge tree
    │   ├── __init__.py                  # ByteRoverMemoryProvider implementation
    │   ├── README.md                    # Usage documentation
    │   └── plugin.yaml                  # Metadata (name, version, hooks)
    │
    ├── hindsight/                       # Hindsight - Knowledge graph + entity resolution
    │   ├── __init__.py                  # HindsightMemoryProvider implementation
    │   ├── README.md                    # Usage documentation
    │   └── plugin.yaml                  # Metadata
    │
    ├── holographic/                     # Holographic - SQLite + HRR vector symbolic
    │   ├── __init__.py                  # HolographicMemoryProvider implementation
    │   ├── holographic.py               # HRR (Holographic Reduced Representations) algebra
    │   ├── retrieval.py                 # Multi-strategy retrieval (FTS5 + Jaccard + HRR)
    │   ├── store.py                     # SQLite schema + MemoryStore class
    │   ├── README.md                    # Usage documentation
    │   └── plugin.yaml                  # Metadata
    │
    ├── honcho/                          # Honcho - AI-native cross-session modeling
    │   ├── __init__.py                  # HonchoMemoryProvider implementation
    │   ├── cli.py                       # CLI commands (12783 tokens)
    │   ├── client.py                    # HonchoClientConfig + SDK initialization
    │   ├── session.py                   # HonchoSessionManager for conversation history
    │   ├── README.md                    # Usage documentation
    │   └── plugin.yaml                  # Metadata
    │
    ├── mem0/                            # Mem0 - Server-side LLM fact extraction
    │   ├── __init__.py                  # Mem0MemoryProvider implementation
    │   ├── README.md                    # Usage documentation
    │   └── plugin.yaml                  # Metadata
    │
    ├── openviking/                      # OpenViking - Volcengine context database
    │   ├── __init__.py                  # OpenVikingMemoryProvider + HTTP client
    │   ├── README.md                    # Usage documentation
    │   └── plugin.yaml                  # Metadata
    │
    └── retaindb/                        # RetainDB - Cloud memory API
        ├── __init__.py                  # RetainDBMemoryProvider implementation
        ├── README.md                    # Usage documentation
        └── plugin.yaml                  # Metadata
```

---

## Architecture Overview

The plugins module implements a **Memory Provider** architecture where only ONE memory provider can be active at a time, selected via `memory.provider` in `config.yaml`. Unlike general plugins that users install, memory providers live in the repo and are always available.

### Key Design Decisions

1. **Single Active Provider**: Only one memory provider runs per session
2. **Discovery Without Import**: Scans `plugin.yaml` for metadata, does lightweight availability checks
3. **MemoryProvider ABC**: All providers implement the same interface defined in `agent/memory_provider.py`
4. **Hooks**: Providers can register for lifecycle hooks (`on_session_end`, `on_pre_compress`, etc.)
5. **CLI Registration**: Memory providers can expose CLI commands via `register_cli(subparser)`

### Plugin Configuration Chain

```
$HERMES_HOME/config.yaml  → memory.provider = "<name>"
         ↓
plugins/memory/<name>/    → Load provider from subdirectory
         ↓
plugin.yaml               → Read metadata (description, hooks, dependencies)
         ↓
__init__.py               → Import module, call register(ctx)
```

---

## Memory Provider Plugin System

### Core Discovery Module: `plugins/memory/__init__.py`

This module provides the discovery and loading infrastructure for memory providers.

#### Key Functions

**`discover_memory_providers() -> List[Tuple[str, str, bool]]`**

Scans `plugins/memory/` for available providers. Returns list of `(name, description, is_available)` tuples. Does NOT import providers — reads `plugin.yaml` for metadata and does lightweight availability checks.

```python
def discover_memory_providers() -> List[Tuple[str, str, bool]]:
    """Scan plugins/memory/ for available providers.

    Returns list of (name, description, is_available) tuples.
    Does NOT import the providers — just reads plugin.yaml for metadata
    and does a lightweight availability check.
    """
    results = []
    if not _MEMORY_PLUGINS_DIR.is_dir():
        return results

    for child in sorted(_MEMORY_PLUGINS_DIR.iterdir()):
        if not child.is_dir() or child.name.startswith(("_", ".")):
            continue
        init_file = child / "__init__.py"
        if not init_file.exists():
            continue

        # Read description from plugin.yaml if available
        desc = ""
        yaml_file = child / "plugin.yaml"
        if yaml_file.exists():
            try:
                import yaml
                with open(yaml_file) as f:
                    meta = yaml.safe_load(f) or {}
                desc = meta.get("description", "")
            except Exception:
                pass

        # Quick availability check — try loading and calling is_available()
        available = True
        try:
            provider = _load_provider_from_dir(child)
            if provider:
                available = provider.is_available()
            else:
                available = False
        except Exception:
            available = False

        results.append((child.name, desc, available))

    return results
```

**`load_memory_provider(name: str) -> Optional[MemoryProvider]`**

Load and return a MemoryProvider instance by name. Returns `None` if the provider is not found or fails to load.

```python
def load_memory_provider(name: str) -> Optional["MemoryProvider"]:
    """Load and return a MemoryProvider instance by name.

    Returns None if the provider is not found or fails to load.
    """
    provider_dir = _MEMORY_PLUGINS_DIR / name
    if not provider_dir.is_dir():
        logger.debug("Memory provider '%s' not found in %s", name, _MEMORY_PLUGINS_DIR)
        return None

    try:
        provider = _load_provider_from_dir(provider_dir)
        if provider:
            return provider
        logger.warning("Memory provider '%s' loaded but no provider instance found", name)
        return None
    except Exception as e:
        logger.warning("Failed to load memory provider '%s': %s", name, e)
        return None
```

**`_load_provider_from_dir(provider_dir: Path) -> Optional[MemoryProvider]`**

Internal function that imports a provider module and extracts the MemoryProvider instance. Supports two patterns:
1. `register(ctx)` function (plugin-style)
2. Top-level class extending `MemoryProvider`

```python
def _load_provider_from_dir(provider_dir: Path) -> Optional["MemoryProvider"]:
    """Import a provider module and extract the MemoryProvider instance.

    The module must have either:
    - A register(ctx) function (plugin-style) — we simulate a ctx
    - A top-level class that extends MemoryProvider — we instantiate it
    """
    name = provider_dir.name
    module_name = f"plugins.memory.{name}"
    init_file = provider_dir / "__init__.py"

    if not init_file.exists():
        return None

    # Check if already loaded
    if module_name in sys.modules:
        mod = sys.modules[module_name]
    else:
        # Handle relative imports within the plugin
        # First ensure the parent packages are registered
        for parent in ("plugins", "plugins.memory"):
            if parent not in sys.modules:
                parent_path = Path(__file__).parent
                if parent == "plugins":
                    parent_path = parent_path.parent
                parent_init = parent_path / "__init__.py"
                if parent_init.exists():
                    spec = importlib.util.spec_from_file_location(
                        parent, str(parent_init),
                        submodule_search_locations=[str(parent_path)]
                    )
                    if spec:
                        parent_mod = importlib.util.module_from_spec(spec)
                        sys.modules[parent] = parent_mod
                        try:
                            spec.loader.exec_module(parent_mod)
                        except Exception:
                            pass

        # Now load the provider module
        spec = importlib.util.spec_from_file_location(
            module_name, str(init_file),
            submodule_search_locations=[str(provider_dir)]
        )
        if not spec:
            return None

        mod = importlib.util.module_from_spec(spec)
        sys.modules[module_name] = mod

        # Register submodules so relative imports work
        # e.g., "from .store import MemoryStore" in holographic plugin
        for sub_file in provider_dir.glob("*.py"):
            if sub_file.name == "__init__.py":
                continue
            sub_name = sub_file.stem
            full_sub_name = f"{module_name}.{sub_name}"
            if full_sub_name not in sys.modules:
                sub_spec = importlib.util.spec_from_file_location(
                    full_sub_name, str(sub_file)
                )
                if sub_spec:
                    sub_mod = importlib.util.module_from_spec(sub_spec)
                    sys.modules[full_sub_name] = sub_mod
                    try:
                        sub_spec.loader.exec_module(sub_mod)
                    except Exception as e:
                        logger.debug("Failed to load submodule %s: %s", full_sub_name, e)

        try:
            spec.loader.exec_module(mod)
        except Exception as e:
            logger.debug("Failed to exec_module %s: %s", module_name, e)
            sys.modules.pop(module_name, None)
            return None

    # Try register(ctx) pattern first (how our plugins are written)
    if hasattr(mod, "register"):
        collector = _ProviderCollector()
        try:
            mod.register(collector)
            if collector.provider:
                return collector.provider
        except Exception as e:
            logger.debug("register() failed for %s: %s", name, e)

    # Fallback: find a MemoryProvider subclass and instantiate it
    from agent.memory_provider import MemoryProvider
    for attr_name in dir(mod):
        attr = getattr(mod, attr_name, None)
        if (isinstance(attr, type) and issubclass(attr, MemoryProvider)
                and attr is not MemoryProvider):
            try:
                return attr()
            except Exception:
                pass

    return None
```

**`discover_plugin_cli_commands() -> List[dict]`**

Returns CLI commands for the **active** memory plugin only. Only one memory provider can be active at a time.

```python
def discover_plugin_cli_commands() -> List[dict]:
    """Return CLI commands for the **active** memory plugin only.

    Only one memory provider can be active at a time (set via
    ``memory.provider`` in config.yaml).  This function reads that
    value and only loads CLI registration for the matching plugin.

    Looks for a ``register_cli(subparser)`` function in the active
    plugin's ``cli.py``.
    """
    results: List[dict] = []
    if not _MEMORY_PLUGINS_DIR.is_dir():
        return results

    active_provider = _get_active_memory_provider()
    if not active_provider:
        return results

    # Only look at the active provider's directory
    plugin_dir = _MEMORY_PLUGINS_DIR / active_provider
    if not plugin_dir.is_dir():
        return results

    cli_file = plugin_dir / "cli.py"
    if not cli_file.exists():
        return results

    module_name = f"plugins.memory.{active_provider}.cli"
    try:
        # Import the CLI module (lightweight — no SDK needed)
        if module_name in sys.modules:
            cli_mod = sys.modules[module_name]
        else:
            spec = importlib.util.spec_from_file_location(
                module_name, str(cli_file)
            )
            if not spec or not spec.loader:
                return results
            cli_mod = importlib.util.module_from_spec(spec)
            sys.modules[module_name] = cli_mod
            spec.loader.exec_module(cli_mod)

        register_cli = getattr(cli_mod, "register_cli", None)
        if not callable(register_cli):
            return results

        # Read metadata from plugin.yaml if available
        help_text = f"Manage {active_provider} memory plugin"
        description = ""
        yaml_file = plugin_dir / "plugin.yaml"
        if yaml_file.exists():
            try:
                import yaml
                with open(yaml_file) as f:
                    meta = yaml.safe_load(f) or {}
                desc = meta.get("description", "")
                if desc:
                    help_text = desc
                    description = desc
            except Exception:
                pass

        handler_fn = getattr(cli_mod, f"{active_provider}_command", None) or \
                     getattr(cli_mod, "honcho_command", None)

        results.append({
            "name": active_provider,
            "help": help_text,
            "description": description,
            "setup_fn": register_cli,
            "handler_fn": handler_fn,
            "plugin": active_provider,
        })
    except Exception as e:
        logger.debug("Failed to scan CLI for memory plugin '%s': %s", active_provider, e)

    return results
```

#### The `_ProviderCollector` Class

A fake plugin context that captures `register_memory_provider` calls:

```python
class _ProviderCollector:
    """Fake plugin context that captures register_memory_provider calls."""

    def __init__(self):
        self.provider = None

    def register_memory_provider(self, provider):
        self.provider = provider

    # No-op for other registration methods
    def register_tool(self, *args, **kwargs):
        pass

    def register_hook(self, *args, **kwargs):
        pass

    def register_cli_command(self, *args, **kwargs):
        pass  # CLI registration happens via discover_plugin_cli_commands()
```

---

## Plugin Discovery & Loading

### Discovery Flow

```
1. Scan plugins/memory/ directories
   ↓
2. For each subdirectory:
   - Check for __init__.py
   - Read plugin.yaml for description
   - Attempt lightweight import + is_available() check
   ↓
3. Return [(name, description, available), ...]
```

### plugin.yaml Format

Each plugin has a `plugin.yaml` file with metadata:

```yaml
name: <provider_name>
version: <semver>
description: "<Human-readable description>"
pip_dependencies:     # Optional
  - package1
  - package2
requires_env:         # Optional
  - ENV_VAR_1
  - ENV_VAR_2
hooks:                # Optional lifecycle hooks
  - on_session_end
  - on_pre_compress
```

### Example plugin.yaml Files

**byterover/plugin.yaml:**
```yaml
name: byterover
version: 1.0.0
description: "ByteRover — persistent knowledge tree with tiered retrieval via the brv CLI."
external_dependencies:
  - name: brv
    install: "curl -fsSL https://byterover.dev/install.sh | sh"
    check: "brv --version"
hooks:
  - on_pre_compress
```

**hindsight/plugin.yaml:**
```yaml
name: hindsight
version: 1.0.0
description: "Hindsight — long-term memory with knowledge graph, entity resolution, and multi-strategy retrieval."
pip_dependencies:
  - hindsight-client
  - hindsight-all
requires_env:
  - HINDSIGHT_API_KEY
hooks:
  - on_session_end
```

**holographic/plugin.yaml:**
```yaml
name: holographic
version: 0.1.0
description: "Holographic memory — local SQLite fact store with FTS5 search, trust scoring, and HRR-based compositional retrieval."
hooks:
  - on_session_end
```

**honcho/plugin.yaml:**
```yaml
name: honcho
version: 1.0.0
description: "Honcho AI-native memory — cross-session user modeling with dialectic Q&A, semantic search, and persistent conclusions."
pip_dependencies:
  - honcho-ai
hooks:
  - on_session_end
```

**mem0/plugin.yaml:**
```yaml
name: mem0
version: 1.0.0
description: "Mem0 — server-side LLM fact extraction with semantic search, reranking, and automatic deduplication."
pip_dependencies:
  - mem0ai
```

**openviking/plugin.yaml:**
```yaml
name: openviking
version: 2.0.0
description: "OpenViking context database — session-managed memory with automatic extraction, tiered retrieval, and filesystem-style knowledge browsing."
pip_dependencies:
  - httpx
requires_env:
  - OPENVIKING_ENDPOINT
hooks:
  - on_session_end
```

**retaindb/plugin.yaml:**
```yaml
name: retaindb
version: 1.0.0
description: "RetainDB — cloud memory API with hybrid search and 7 memory types."
pip_dependencies:
  - requests
requires_env:
  - RETAINDB_API_KEY
```

---

## MemoryProvider ABC Interface

All memory providers implement the `MemoryProvider` abstract base class from `agent/memory_provider.py`. Here's the complete interface:

### Required Properties & Methods

```python
class MemoryProvider(ABC):
    """Abstract base class for memory providers."""

    @property
    @abstractmethod
    def name(self) -> str:
        """Return the provider name (e.g., 'honcho', 'hindsight')."""
        pass

    @abstractmethod
    def is_available(self) -> bool:
        """Check if the provider can run (dependencies installed, configured)."""
        pass

    @abstractmethod
    def get_config_schema(self):
        """Return configuration schema for the provider."""
        pass

    @abstractmethod
    def initialize(self, session_id: str, **kwargs) -> None:
        """Initialize the provider for a session."""
        pass

    @abstractmethod
    def system_prompt_block(self) -> str:
        """Return text to inject into the system prompt."""
        pass

    @abstractmethod
    def prefetch(self, query: str, *, session_id: str = "") -> str:
        """Return prefetched context (consumed at turn start)."""
        pass

    @abstractmethod
    def queue_prefetch(self, query: str, *, session_id: str = "") -> None:
        """Queue an async prefetch for the next turn."""
        pass

    @abstractmethod
    def sync_turn(self, user_content: str, assistant_content: str, *, session_id: str = "") -> None:
        """Record a conversation turn to memory (async)."""
        pass

    @abstractmethod
    def get_tool_schemas(self) -> List[Dict[str, Any]]:
        """Return LLM tool schemas provided by this provider."""
        pass

    @abstractmethod
    def handle_tool_call(self, tool_name: str, args: dict, **kwargs) -> str:
        """Execute a tool call. Return JSON string response."""
        pass

    @abstractmethod
    def shutdown(self) -> None:
        """Clean up resources on session end."""
        pass
```

### Optional Hook Methods

```python
def on_session_end(self, messages: List[Dict[str, Any]]) -> None:
    """Called when the session ends."""
    pass

def on_pre_compress(self, messages: List[Dict[str, Any]]) -> str:
    """Called before context compression. Return insights to preserve."""
    return ""

def on_memory_write(self, action: str, target: str, content: str) -> None:
    """Called when built-in memory is written. Mirror if desired."""
    pass

def on_turn_start(self, turn_number: int, message: str, **kwargs) -> None:
    """Called at the start of each turn."""
    pass
```

---

## Individual Memory Providers

### 1. ByteRover Memory Provider

**Path:** `plugins/memory/byterover/`

**Description:** Persistent memory via the ByteRover CLI (`brv`). Organizes knowledge into a hierarchical context tree with tiered retrieval (fuzzy text → LLM-driven search). Local-first with optional cloud sync.

#### Key Components

**Binary Resolution (cached, thread-safe):**

```python
_brv_path_lock = threading.Lock()
_cached_brv_path: Optional[str] = None

def _resolve_brv_path() -> Optional[str]:
    """Find the brv binary on PATH or well-known install locations."""
    global _cached_brv_path
    with _brv_path_lock:
        if _cached_brv_path is not None:
            return _cached_brv_path if _cached_brv_path != "" else None

    found = shutil.which("brv")
    if not found:
        home = Path.home()
        candidates = [
            home / ".brv-cli" / "bin" / "brv",
            Path("/usr/local/bin/brv"),
            home / ".npm-global" / "bin" / "brv",
        ]
        for c in candidates:
            if c.exists():
                found = str(c)
                break

    with _brv_path_lock:
        if _cached_brv_path is not None:
            return _cached_brv_path if _cached_brv_path != "" else None
        _cached_brv_path = found or ""
    return found
```

**CLI Command Execution:**

```python
def _run_brv(args: List[str], timeout: int = _QUERY_TIMEOUT,
             cwd: str = None) -> dict:
    """Run a brv CLI command. Returns {success, output, error}."""
    brv_path = _resolve_brv_path()
    if not brv_path:
        return {"success": False, "error": "brv CLI not found..."}

    cmd = [brv_path] + args
    effective_cwd = cwd or str(_get_brv_cwd())
    Path(effective_cwd).mkdir(parents=True, exist_ok=True)

    env = os.environ.copy()
    brv_bin_dir = str(Path(brv_path).parent)
    env["PATH"] = brv_bin_dir + os.pathsep + env.get("PATH", "")

    try:
        result = subprocess.run(
            cmd, capture_output=True, text=True,
            timeout=timeout, cwd=effective_cwd, env=env,
        )
        stdout = result.stdout.strip()
        stderr = result.stderr.strip()

        if result.returncode == 0:
            return {"success": True, "output": stdout}
        return {"success": False, "error": stderr or stdout}
    except subprocess.TimeoutExpired:
        return {"success": False, "error": f"brv timed out after {timeout}s"}
```

#### Tool Schemas

```python
QUERY_SCHEMA = {
    "name": "brv_query",
    "description": (
        "Search ByteRover's persistent knowledge tree for relevant context. "
        "Returns memories, project knowledge, architectural decisions, and "
        "patterns from previous sessions."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "What to search for."},
        },
        "required": ["query"],
    },
}

CURATE_SCHEMA = {
    "name": "brv_curate",
    "description": (
        "Store important information in ByteRover's persistent knowledge tree. "
        "Use for architectural decisions, bug fixes, user preferences, project "
        "patterns — anything worth remembering across sessions."
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string", "description": "The information to remember."},
        },
        "required": ["content"],
    },
}

STATUS_SCHEMA = {
    "name": "brv_status",
    "description": "Check ByteRover status — CLI version, context tree stats, cloud sync state.",
    "parameters": {"type": "object", "properties": {}, "required": []},
}
```

#### ByteRoverMemoryProvider Class

```python
class ByteRoverMemoryProvider(MemoryProvider):
    """ByteRover persistent memory via the brv CLI."""

    def __init__(self):
        self._cwd = ""
        self._session_id = ""
        self._turn_count = 0
        self._sync_thread: Optional[threading.Thread] = None

    @property
    def name(self) -> str:
        return "byterover"

    def is_available(self) -> bool:
        return _resolve_brv_path() is not None

    def get_config_schema(self):
        return [
            {
                "key": "api_key",
                "description": "ByteRover API key (optional, for cloud sync)",
                "secret": True,
                "env_var": "BRV_API_KEY",
                "url": "https://app.byterover.dev",
            },
        ]

    def initialize(self, session_id: str, **kwargs) -> None:
        self._cwd = str(_get_brv_cwd())
        self._session_id = session_id
        self._turn_count = 0
        Path(self._cwd).mkdir(parents=True, exist_ok=True)

    def system_prompt_block(self) -> str:
        if not _resolve_brv_path():
            return ""
        return (
            "# ByteRover Memory\n"
            "Active. Persistent knowledge tree with hierarchical context.\n"
            "Use brv_query to search past knowledge, brv_curate to store "
            "important facts, brv_status to check state."
        )

    def prefetch(self, query: str, *, session_id: str = "") -> str:
        """Run brv query synchronously before the agent's first LLM call."""
        if not query or len(query.strip()) < _MIN_QUERY_LEN:
            return ""
        result = _run_brv(
            ["query", "--", query.strip()[:5000]],
            timeout=_QUERY_TIMEOUT, cwd=self._cwd,
        )
        if result["success"] and result.get("output"):
            output = result["output"].strip()
            if len(output) > _MIN_OUTPUT_LEN:
                return f"## ByteRover Context\n{output}"
        return ""

    def queue_prefetch(self, query: str, *, session_id: str = "") -> None:
        """No-op: prefetch() now runs synchronously at turn start."""
        pass

    def sync_turn(self, user_content: str, assistant_content: str, *, session_id: str = "") -> None:
        """Curate the conversation turn in background (non-blocking)."""
        self._turn_count += 1

        if len(user_content.strip()) < _MIN_QUERY_LEN:
            return

        def _sync():
            try:
                combined = f"User: {user_content[:2000]}\nAssistant: {assistant_content[:2000]}"
                _run_brv(
                    ["curate", "--", combined],
                    timeout=_CURATE_TIMEOUT, cwd=self._cwd,
                )
            except Exception as e:
                logger.debug("ByteRover sync failed: %s", e)

        if self._sync_thread and self._sync_thread.is_alive():
            self._sync_thread.join(timeout=5.0)

        self._sync_thread = threading.Thread(
            target=_sync, daemon=True, name="brv-sync"
        )
        self._sync_thread.start()

    def on_pre_compress(self, messages: List[Dict[str, Any]]) -> str:
        """Extract insights before context compression discards turns."""
        if not messages:
            return ""

        parts = []
        for msg in messages[-10:]:
            role = msg.get("role", "")
            content = msg.get("content", "")
            if isinstance(content, str) and content.strip() and role in ("user", "assistant"):
                parts.append(f"{role}: {content[:500]}")

        if not parts:
            return ""

        combined = "\n".join(parts)

        def _flush():
            try:
                _run_brv(
                    ["curate", "--", f"[Pre-compression context]\n{combined}"],
                    timeout=_CURATE_TIMEOUT, cwd=self._cwd,
                )
            except Exception as e:
                logger.debug("ByteRover pre-compression flush failed: %s", e)

        t = threading.Thread(target=_flush, daemon=True, name="brv-flush")
        t.start()
        return ""

    def get_tool_schemas(self) -> List[Dict[str, Any]]:
        return [QUERY_SCHEMA, CURATE_SCHEMA, STATUS_SCHEMA]

    def handle_tool_call(self, tool_name: str, args: dict, **kwargs) -> str:
        if tool_name == "brv_query":
            return self._tool_query(args)
        elif tool_name == "brv_curate":
            return self._tool_curate(args)
        elif tool_name == "brv_status":
            return self._tool_status()
        return json.dumps({"error": f"Unknown tool: {tool_name}"})

    def shutdown(self) -> None:
        if self._sync_thread and self._sync_thread.is_alive():
            self._sync_thread.join(timeout=10.0)
```

---

### 2. Hindsight Memory Provider

**Path:** `plugins/memory/hindsight/`

**Description:** Long-term memory with knowledge graph, entity resolution, and multi-strategy retrieval. Supports cloud (API key) and local (embedded) modes.

#### Dedicated Event Loop

Hindsight uses a dedicated event loop for async calls to avoid leaking aiohttp sessions:

```python
_loop: asyncio.AbstractEventLoop | None = None
_loop_thread: threading.Thread | None = None
_loop_lock = threading.Lock()

def _get_loop() -> asyncio.AbstractEventLoop:
    """Return a long-lived event loop running on a background thread."""
    global _loop, _loop_thread
    with _loop_lock:
        if _loop is not None and _loop.is_running():
            return _loop
        _loop = asyncio.new_event_loop()

        def _run():
            asyncio.set_event_loop(_loop)
            _loop.run_forever()

        _loop_thread = threading.Thread(target=_run, daemon=True, name="hindsight-loop")
        _loop_thread.start()
        return _loop

def _run_sync(coro, timeout: float = 120.0):
    """Schedule *coro* on the shared loop and block until done."""
    loop = _get_loop()
    future = asyncio.run_coroutine_threadsafe(coro, loop)
    return future.result(timeout=timeout)
```

#### Config Loading

```python
def _load_config() -> dict:
    """Load config from profile-scoped path, legacy path, or env vars.

    Resolution order:
      1. $HERMES_HOME/hindsight/config.json  (profile-scoped)
      2. ~/.hindsight/config.json             (legacy, shared)
      3. Environment variables
    """
    from pathlib import Path
    from hermes_constants import get_hermes_home

    profile_path = get_hermes_home() / "hindsight" / "config.json"
    if profile_path.exists():
        try:
            return json.loads(profile_path.read_text(encoding="utf-8"))
        except Exception:
            pass

    legacy_path = Path.home() / ".hindsight" / "config.json"
    if legacy_path.exists():
        try:
            return json.loads(legacy_path.read_text(encoding="utf-8"))
        except Exception:
            pass

    return {
        "mode": os.environ.get("HINDSIGHT_MODE", "cloud"),
        "apiKey": os.environ.get("HINDSIGHT_API_KEY", ""),
        "banks": {
            "hermes": {
                "bankId": os.environ.get("HINDSIGHT_BANK_ID", "hermes"),
                "budget": os.environ.get("HINDSIGHT_BUDGET", "mid"),
                "enabled": True,
            }
        },
    }
```

#### Tool Schemas

```python
RETAIN_SCHEMA = {
    "name": "hindsight_retain",
    "description": "Store information to long-term memory. Hindsight automatically extracts structured facts, resolves entities, and indexes for retrieval.",
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string", "description": "The information to store."},
            "context": {"type": "string", "description": "Short label (e.g. 'user preference', 'project decision')."},
        },
        "required": ["content"],
    },
}

RECALL_SCHEMA = {
    "name": "hindsight_recall",
    "description": "Search long-term memory. Returns memories ranked by relevance using semantic search, keyword matching, entity graph traversal, and reranking.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "What to search for."},
        },
        "required": ["query"],
    },
}

REFLECT_SCHEMA = {
    "name": "hindsight_reflect",
    "description": "Synthesize a reasoned answer from long-term memories. Unlike recall, this reasons across all stored memories to produce a coherent response.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "The question to reflect on."},
        },
        "required": ["query"],
    },
}
```

#### HindsightMemoryProvider - Key Methods

```python
class HindsightMemoryProvider(MemoryProvider):
    """Hindsight long-term memory with knowledge graph and multi-strategy retrieval."""

    def __init__(self):
        self._config = None
        self._api_key = None
        self._api_url = _DEFAULT_API_URL
        self._bank_id = "hermes"
        self._budget = "mid"
        self._mode = "cloud"
        self._memory_mode = "hybrid"  # "context", "tools", or "hybrid"
        self._prefetch_method = "recall"  # "recall" or "reflect"
        self._client = None
        self._prefetch_result = ""
        self._prefetch_lock = threading.Lock()
        self._prefetch_thread = None
        self._sync_thread = None

    @property
    def name(self) -> str:
        return "hindsight"

    def is_available(self) -> bool:
        cfg = _load_config()
        mode = cfg.get("mode", "cloud")
        if mode == "local":
            return True
        has_key = bool(cfg.get("apiKey") or os.environ.get("HINDSIGHT_API_KEY", ""))
        has_url = bool(cfg.get("api_url") or os.environ.get("HINDSIGHT_API_URL", ""))
        return has_key or has_url

    def _get_client(self):
        """Return the cached Hindsight client (created once, reused)."""
        if self._client is None:
            if self._mode == "local":
                from hindsight import HindsightEmbedded
                HindsightEmbedded.__del__ = lambda self: None  # Prevent loop errors
                self._client = HindsightEmbedded(
                    profile=self._config.get("profile", "hermes"),
                    llm_provider=self._config.get("llm_provider", ""),
                    llm_api_key=self._config.get("llmApiKey") or os.environ.get("HINDSIGHT_LLM_API_KEY", ""),
                    llm_model=self._config.get("llm_model", ""),
                )
            else:
                from hindsight_client import Hindsight
                kwargs = {"base_url": self._api_url, "timeout": 30.0}
                if self._api_key:
                    kwargs["api_key"] = self._api_key
                self._client = Hindsight(**kwargs)
        return self._client

    def initialize(self, session_id: str, **kwargs) -> None:
        self._config = _load_config()
        self._mode = self._config.get("mode", "cloud")
        self._bank_id = self._config.get("bank_id") or "hermes"
        self._budget = self._config.get("budget", "mid")
        self._memory_mode = self._config.get("memory_mode", "hybrid")
        self._prefetch_method = self._config.get("prefetch_method", "recall")

        # For local mode, start embedded daemon in background
        if self._mode == "local":
            def _start_daemon():
                client = self._get_client()
                client._ensure_started()
            t = threading.Thread(target=_start_daemon, daemon=True)
            t.start()

    def system_prompt_block(self) -> str:
        if self._memory_mode == "context":
            return "# Hindsight Memory\nActive (context mode). Relevant memories auto-injected."
        if self._memory_mode == "tools":
            return "# Hindsight Memory\nActive (tools mode). Use hindsight_recall, hindsight_reflect, hindsight_retain."
        return "# Hindsight Memory\nActive. Memories auto-injected + tools available."

    def prefetch(self, query: str, *, session_id: str = "") -> str:
        if self._prefetch_thread and self._prefetch_thread.is_alive():
            self._prefetch_thread.join(timeout=3.0)
        with self._prefetch_lock:
            result = self._prefetch_result
            self._prefetch_result = ""
        if not result:
            return ""
        return f"## Hindsight Memory\n{result}"

    def queue_prefetch(self, query: str, *, session_id: str = "") -> None:
        if self._memory_mode == "tools":
            return
        def _run():
            client = self._get_client()
            if self._prefetch_method == "reflect":
                resp = _run_sync(client.areflect(bank_id=self._bank_id, query=query, budget=self._budget))
                text = resp.text or ""
            else:
                resp = _run_sync(client.arecall(bank_id=self._bank_id, query=query, budget=self._budget))
                text = "\n".join(r.text for r in resp.results if r.text) if resp.results else ""
            if text:
                with self._prefetch_lock:
                    self._prefetch_result = text
        self._prefetch_thread = threading.Thread(target=_run, daemon=True)
        self._prefetch_thread.start()

    def sync_turn(self, user_content: str, assistant_content: str, *, session_id: str = "") -> None:
        combined = f"User: {user_content}\nAssistant: {assistant_content}"
        def _sync():
            client = self._get_client()
            _run_sync(client.aretain(bank_id=self._bank_id, content=combined, context="conversation"))
        if self._sync_thread and self._sync_thread.is_alive():
            self._sync_thread.join(timeout=5.0)
        self._sync_thread = threading.Thread(target=_sync, daemon=True)
        self._sync_thread.start()

    def get_tool_schemas(self) -> List[Dict[str, Any]]:
        if self._memory_mode == "context":
            return []
        return [RETAIN_SCHEMA, RECALL_SCHEMA, REFLECT_SCHEMA]

    def handle_tool_call(self, tool_name: str, args: dict, **kwargs) -> str:
        client = self._get_client()
        if tool_name == "hindsight_retain":
            _run_sync(client.aretain(bank_id=self._bank_id, content=args["content"], context=args.get("context")))
            return json.dumps({"result": "Memory stored successfully."})
        elif tool_name == "hindsight_recall":
            resp = _run_sync(client.arecall(bank_id=self._bank_id, query=args["query"], budget=self._budget))
            if not resp.results:
                return json.dumps({"result": "No relevant memories found."})
            lines = [f"{i}. {r.text}" for i, r in enumerate(resp.results, 1)]
            return json.dumps({"result": "\n".join(lines)})
        elif tool_name == "hindsight_reflect":
            resp = _run_sync(client.areflect(bank_id=self._bank_id, query=args["query"], budget=self._budget))
            return json.dumps({"result": resp.text or "No relevant memories found."})
        return json.dumps({"error": f"Unknown tool: {tool_name}"})

    def shutdown(self) -> None:
        global _loop, _loop_thread
        for t in (self._prefetch_thread, self._sync_thread):
            if t and t.is_alive():
                t.join(timeout=5.0)
        if self._client:
            if self._mode == "local":
                try:
                    self._client.close()
                except RuntimeError:
                    pass
            else:
                _run_sync(self._client.aclose())
        if _loop and _loop.is_running():
            _loop.call_soon_threadsafe(_loop.stop)
            _loop = None
            _loop_thread = None
```

---

### 3. Holographic Memory Provider

**Path:** `plugins/memory/holographic/`

**Description:** Local SQLite fact store with FTS5 search, trust scoring, entity resolution, and HRR-based compositional retrieval. The most sophisticated retrieval system using Vector Symbolic Architecture.

#### Module Structure

| File | Purpose |
|------|---------|
| `__init__.py` | HolographicMemoryProvider (MemoryProvider ABC implementation) |
| `holographic.py` | HRR (Holographic Reduced Representations) algebra |
| `retrieval.py` | FactRetriever with multi-strategy search |
| `store.py` | MemoryStore (SQLite + FTS5 + entity resolution) |

#### holographic.py - HRR Algebra

HRRs encode compositional structure into fixed-width distributed representations using phase vectors.

```python
"""Holographic Reduced Representations (HRR) with phase encoding.

Phase vectors: each concept is a vector of angles in [0, 2π).

Operations:
  bind   — circular convolution (phase addition)  — associates two concepts
  unbind — circular correlation (phase subtraction) — retrieves a bound value
  bundle — superposition (circular mean)           — merges multiple concepts
"""

import hashlib
import math
import struct

try:
    import numpy as np
    _HAS_NUMPY = True
except ImportError:
    _HAS_NUMPY = False

_TWO_PI = 2.0 * math.pi

def encode_atom(word: str, dim: int = 1024) -> "np.ndarray":
    """Deterministic phase vector via SHA-256 counter blocks.

    Uses hashlib for cross-platform reproducibility.
    """
    values_per_block = 16
    blocks_needed = math.ceil(dim / values_per_block)

    uint16_values: list[int] = []
    for i in range(blocks_needed):
        digest = hashlib.sha256(f"{word}:{i}".encode()).digest()
        uint16_values.extend(struct.unpack("<16H", digest))

    phases = np.array(uint16_values[:dim], dtype=np.float64) * (_TWO_PI / 65536.0)
    return phases


def bind(a: "np.ndarray", b: "np.ndarray") -> "np.ndarray":
    """Circular convolution = element-wise phase addition."""
    return (a + b) % _TWO_PI


def unbind(memory: "np.ndarray", key: "np.ndarray") -> "np.ndarray":
    """Circular correlation = element-wise phase subtraction.
    
    unbind(bind(a, b), a) ≈ b  (up to superposition noise)
    """
    return (memory - key) % _TWO_PI


def bundle(*vectors: "np.ndarray") -> "np.ndarray":
    """Superposition via circular mean of complex exponentials.
    
    The result can hold O(sqrt(dim)) items before similarity degrades.
    """
    complex_sum = np.sum([np.exp(1j * v) for v in vectors], axis=0)
    return np.angle(complex_sum) % _TWO_PI


def similarity(a: "np.ndarray", b: "np.ndarray") -> float:
    """Phase cosine similarity. Range [-1, 1]."""
    return float(np.mean(np.cos(a - b)))


def encode_text(text: str, dim: int = 1024) -> "np.ndarray":
    """Bag-of-words: bundle of atom vectors for each token."""
    tokens = [token.strip(".,!?;:\"'()[]{}") for token in text.lower().split()]
    tokens = [t for t in tokens if t]
    if not tokens:
        return encode_atom("__hrr_empty__", dim)
    atom_vectors = [encode_atom(token, dim) for token in tokens]
    return bundle(*atom_vectors)


def encode_fact(content: str, entities: list[str], dim: int = 1024) -> "np.ndarray":
    """Structured encoding with role vectors.

    Components:
    1. bind(encode_text(content), ROLE_CONTENT)
    2. For each entity: bind(encode_atom(entity), ROLE_ENTITY)
    3. bundle all components

    Enables algebraic extraction: unbind(fact, bind(entity, ROLE_ENTITY)) ≈ content
    """
    role_content = encode_atom("__hrr_role_content__", dim)
    role_entity = encode_atom("__hrr_role_entity__", dim)

    components = [bind(encode_text(content, dim), role_content)]
    for entity in entities:
        components.append(bind(encode_atom(entity.lower(), dim), role_entity))

    return bundle(*components)


def phases_to_bytes(phases: "np.ndarray") -> bytes:
    """Serialize phase vector to bytes (8 KB at dim=1024)."""
    return phases.tobytes()


def bytes_to_phases(data: bytes) -> "np.ndarray":
    """Deserialize bytes to phase vector."""
    return np.frombuffer(data, dtype=np.float64).copy()


def snr_estimate(dim: int, n_items: int) -> float:
    """Signal-to-noise ratio estimate.
    
    SNR = sqrt(dim / n_items). Falls below 2.0 when n_items > dim / 4.
    """
    if n_items <= 0:
        return float("inf")
    return math.sqrt(dim / n_items)
```

#### store.py - SQLite Schema

```python
_SCHEMA = """
CREATE TABLE IF NOT EXISTS facts (
    fact_id         INTEGER PRIMARY KEY AUTOINCREMENT,
    content         TEXT NOT NULL UNIQUE,
    category        TEXT DEFAULT 'general',
    tags            TEXT DEFAULT '',
    trust_score     REAL DEFAULT 0.5,
    retrieval_count INTEGER DEFAULT 0,
    helpful_count   INTEGER DEFAULT 0,
    created_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    hrr_vector      BLOB
);

CREATE TABLE IF NOT EXISTS entities (
    entity_id   INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    entity_type TEXT DEFAULT 'unknown',
    aliases     TEXT DEFAULT '',
    created_at  TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS fact_entities (
    fact_id   INTEGER REFERENCES facts(fact_id),
    entity_id INTEGER REFERENCES entities(entity_id),
    PRIMARY KEY (fact_id, entity_id)
);

CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(content, tags);

CREATE TABLE IF NOT EXISTS memory_banks (
    bank_id    INTEGER PRIMARY KEY AUTOINCREMENT,
    bank_name  TEXT NOT NULL UNIQUE,
    vector     BLOB NOT NULL,
    dim        INTEGER NOT NULL,
    fact_count INTEGER DEFAULT 0,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
"""
```

#### retrieval.py - Multi-Strategy Search

```python
class FactRetriever:
    """Multi-strategy fact retrieval with trust-weighted scoring."""

    def __init__(self, store: MemoryStore, hrr_weight: float = 0.3, hrr_dim: int = 1024):
        self.store = store
        self.hrr_dim = hrr_dim
        self.hrr_weight = hrr_weight if hrr._HAS_NUMPY else 0.0

    def search(self, query: str, category: str = None, min_trust: float = 0.3, limit: int = 10) -> list[dict]:
        """Hybrid search: FTS5 → Jaccard rerank → HRR → trust weighting."""
        candidates = self._fts_candidates(query, category, min_trust, limit * 3)
        if not candidates:
            return []

        query_tokens = self._tokenize(query)
        scored = []

        for fact in candidates:
            content_tokens = self._tokenize(fact["content"])
            tag_tokens = self._tokenize(fact.get("tags", ""))
            all_tokens = content_tokens | tag_tokens

            jaccard = self._jaccard_similarity(query_tokens, all_tokens)

            # HRR similarity
            if self.hrr_weight > 0 and fact.get("hrr_vector"):
                fact_vec = hrr.bytes_to_phases(fact["hrr_vector"])
                query_vec = hrr.encode_text(query, self.hrr_dim)
                hrr_sim = (hrr.similarity(query_vec, fact_vec) + 1.0) / 2.0
            else:
                hrr_sim = 0.5

            relevance = 0.4 * fact.get("fts_rank", 0) + 0.3 * jaccard + self.hrr_weight * hrr_sim
            score = relevance * fact["trust_score"]
            fact["score"] = score
            scored.append(fact)

        scored.sort(key=lambda x: x["score"], reverse=True)
        return scored[:limit]

    def probe(self, entity: str, category: str = None, limit: int = 10) -> list[dict]:
        """Compositional entity query using HRR algebra.

        Unbinds entity from memory bank to extract associated content.
        """
        if not hrr._HAS_NUMPY:
            return self.search(entity, category=category, limit=limit)

        role_entity = hrr.encode_atom("__hrr_role_entity__", self.hrr_dim)
        entity_vec = hrr.encode_atom(entity.lower(), self.hrr_dim)
        probe_key = hrr.bind(entity_vec, role_entity)

        # Score facts by how much entity structurally appears
        # ... (full implementation)

    def contradict(self, category: str = None, threshold: float = 0.3, limit: int = 10) -> list[dict]:
        """Find contradictory facts via entity overlap + content divergence.

        Two facts contradict when they share entities but have low content similarity.
        """
        if not hrr._HAS_NUMPY:
            return []

        # O(n²) comparison with guards against explosion
        # High entity overlap + low content similarity = contradiction
        # ... (full implementation)
```

#### Tool Schemas

```python
FACT_STORE_SCHEMA = {
    "name": "fact_store",
    "description": (
        "Deep structured memory with algebraic reasoning. "
        "ACTIONS:\n"
        "• add — Store a fact\n"
        "• search — Keyword lookup\n"
        "• probe — Entity recall\n"
        "• related — Structural adjacency\n"
        "• reason — Multi-entity compositional query\n"
        "• contradict — Find conflicting facts\n"
        "• update/remove/list — CRUD"
    ),
    "parameters": {
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["add", "search", "probe", "related", "reason", "contradict", "update", "remove", "list"],
            },
            "content": {"type": "string"},
            "query": {"type": "string"},
            "entity": {"type": "string"},
            "entities": {"type": "array", "items": {"type": "string"}},
            "fact_id": {"type": "integer"},
            "category": {"type": "string", "enum": ["user_pref", "project", "tool", "general"]},
            "trust_delta": {"type": "number"},
            "min_trust": {"type": "number"},
            "limit": {"type": "integer"},
        },
        "required": ["action"],
    },
}

FACT_FEEDBACK_SCHEMA = {
    "name": "fact_feedback",
    "description": "Rate a fact as helpful/unhelpful (trains trust scores).",
    "parameters": {
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["helpful", "unhelpful"]},
            "fact_id": {"type": "integer"},
        },
        "required": ["action", "fact_id"],
    },
}
```

---

### 4. Honcho Memory Provider

**Path:** `plugins/memory/honcho/`

**Description:** AI-native cross-session user modeling with dialectic Q&A, semantic search, peer cards, and persistent conclusions.

#### Module Structure

| File | Purpose |
|------|---------|
| `__init__.py` | HonchoMemoryProvider (1100+ lines) |
| `client.py` | HonchoClientConfig + SDK initialization |
| `session.py` | HonchoSessionManager for conversation history |
| `cli.py` | CLI commands (setup, status, mode, peer, tokens, map, sync) |

#### Config Resolution

```python
def resolve_active_host() -> str:
    """Derive Honcho host key from active Hermes profile.

    Resolution order:
      1. HERMES_HONCHO_HOST env var
      2. Active profile name -> "hermes.<profile>"
      3. Fallback: "hermes"
    """
    explicit = os.environ.get("HERMES_HONCHO_HOST", "").strip()
    if explicit:
        return explicit

    try:
        from hermes_cli.profiles import get_active_profile_name
        profile = get_active_profile_name()
        if profile and profile not in ("default", "custom"):
            return f"hermes.{profile}"
    except Exception:
        pass
    return "hermes"


def resolve_config_path() -> Path:
    """Resolution order:
      1. $HERMES_HOME/honcho.json (profile-local)
      2. ~/.hermes/honcho.json (default profile)
      3. ~/.honcho/config.json (global, cross-app)
    """
    local_path = get_hermes_home() / "honcho.json"
    if local_path.exists():
        return local_path

    default_path = Path.home() / ".hermes" / "honcho.json"
    if default_path.exists():
        return default_path

    return Path.home() / ".honcho" / "config.json"
```

#### HonchoClientConfig Dataclass

```python
@dataclass
class HonchoClientConfig:
    """Configuration for Honcho client, resolved for a specific host."""

    host: str = "hermes"
    workspace_id: str = "hermes"
    api_key: str | None = None
    base_url: str | None = None
    peer_name: str | None = None
    ai_peer: str = "hermes"
    enabled: bool = False
    save_messages: bool = True
    write_frequency: str | int = "async"  # "async", "turn", "session", or int N
    context_tokens: int | None = None
    dialectic_reasoning_level: str = "low"
    dialectic_dynamic: bool = True
    dialectic_max_chars: int = 600
    message_max_chars: int = 25000
    dialectic_max_input_chars: int = 10000
    recall_mode: str = "hybrid"  # "hybrid", "context", "tools"
    observation_mode: str = "directional"
    user_observe_me: bool = True
    user_observe_others: bool = True
    ai_observe_me: bool = True
    ai_observe_others: bool = True
    session_strategy: str = "per-directory"
    session_peer_prefix: bool = False
    sessions: dict[str, str] = field(default_factory=dict)
    raw: dict[str, Any] = field(default_factory=dict)
    explicitly_configured: bool = False

    def resolve_session_name(
        self,
        cwd: str = None,
        session_title: str = None,
        session_id: str = None,
    ) -> str | None:
        """Resolve Honcho session name.

        Resolution order:
          1. Manual directory override from sessions map
          2. Hermes session title (/title command)
          3. per-session strategy
          4. per-repo strategy (git root name)
          5. per-directory strategy (basename)
          6. global strategy (workspace name)
        """
        # Manual override always wins
        manual = self.sessions.get(cwd or os.getcwd())
        if manual:
            return manual

        # /title mid-session remap
        if session_title:
            sanitized = re.sub(r'[^a-zA-Z0-9_-]', '-', session_title).strip('-')
            if sanitized:
                prefix = f"{self.peer_name}-" if self.session_peer_prefix and self.peer_name else ""
                return f"{prefix}{sanitized}"

        # per-session
        if self.session_strategy == "per-session" and session_id:
            prefix = f"{self.peer_name}-" if self.session_peer_prefix and self.peer_name else ""
            return f"{prefix}{session_id}"

        # per-repo
        if self.session_strategy == "per-repo":
            base = self._git_repo_name(cwd) or Path(cwd).name
            prefix = f"{self.peer_name}-" if self.session_peer_prefix and self.peer_name else ""
            return f"{prefix}{base}"

        # per-directory (default)
        base = Path(cwd or os.getcwd()).name
        prefix = f"{self.peer_name}-" if self.session_peer_prefix and self.peer_name else ""
        return f"{prefix}{base}"
```

#### Tool Schemas

```python
PROFILE_SCHEMA = {
    "name": "honcho_profile",
    "description": "Retrieve user's peer card — key facts snapshot. Fast, no LLM.",
    "parameters": {"type": "object", "properties": {}, "required": []},
}

SEARCH_SCHEMA = {
    "name": "honcho_search",
    "description": "Semantic search over stored context. Raw excerpts, no synthesis.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string"},
            "max_tokens": {"type": "integer", "description": "Default 800, max 2000"},
        },
        "required": ["query"],
    },
}

CONTEXT_SCHEMA = {
    "name": "honcho_context",
    "description": "Ask Honcho a question, get LLM-synthesized answer (dialectic reasoning).",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string"},
            "peer": {"type": "string", "enum": ["user", "ai"], "default": "user"},
        },
        "required": ["query"],
    },
}

CONCLUDE_SCHEMA = {
    "name": "honcho_conclude",
    "description": "Write a persistent fact about the user.",
    "parameters": {
        "type": "object",
        "properties": {
            "conclusion": {"type": "string"},
        },
        "required": ["conclusion"],
    },
}
```

---

### 5. Mem0 Memory Provider

**Path:** `plugins/memory/mem0/`

**Description:** Server-side LLM fact extraction with semantic search, reranking, and automatic deduplication.

#### Circuit Breaker Pattern

```python
_BREAKER_THRESHOLD = 5
_BREAKER_COOLDOWN_SECS = 120

class Mem0MemoryProvider(MemoryProvider):
    def __init__(self):
        self._consecutive_failures = 0
        self._breaker_open_until = 0.0

    def _is_breaker_open(self) -> bool:
        if self._consecutive_failures < _BREAKER_THRESHOLD:
            return False
        if time.monotonic() >= self._breaker_open_until:
            self._consecutive_failures = 0  # Reset after cooldown
            return False
        return True

    def _record_failure(self):
        self._consecutive_failures += 1
        if self._consecutive_failures >= _BREAKER_THRESHOLD:
            self._breaker_open_until = time.monotonic() + _BREAKER_COOLDOWN_SECS
            logger.warning("Mem0 circuit breaker tripped. Pausing for %ds", _BREAKER_COOLDOWN_SECS)
```

#### Tool Schemas

```python
PROFILE_SCHEMA = {
    "name": "mem0_profile",
    "description": "All stored memories about the user. Fast, no reranking.",
    "parameters": {"type": "object", "properties": {}, "required": []},
}

SEARCH_SCHEMA = {
    "name": "mem0_search",
    "description": "Semantic search with optional reranking.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string"},
            "rerank": {"type": "boolean", "default": False},
            "top_k": {"type": "integer", "default": 10, "max": 50},
        },
        "required": ["query"],
    },
}

CONCLUDE_SCHEMA = {
    "name": "mem0_conclude",
    "description": "Store a fact verbatim (no LLM extraction).",
    "parameters": {
        "type": "object",
        "properties": {
            "conclusion": {"type": "string"},
        },
        "required": ["conclusion"],
    },
}
```

---

### 6. OpenViking Memory Provider

**Path:** `plugins/memory/openviking/`

**Description:** Context database by Volcengine (ByteDance) with filesystem-style knowledge hierarchy (viking:// URIs), tiered retrieval, and automatic memory extraction.

#### Custom HTTP Client (no SDK required)

```python
class _VikingClient:
    """Thin HTTP client for OpenViking REST API."""

    def __init__(self, endpoint: str, api_key: str = "", account: str = "", user: str = ""):
        self._endpoint = endpoint.rstrip("/")
        self._api_key = api_key
        self._account = account or os.environ.get("OPENVIKING_ACCOUNT", "root")
        self._user = user or os.environ.get("OPENVIKING_USER", "default")
        self._httpx = _get_httpx()  # Lazy import

    def _headers(self) -> dict:
        h = {
            "Content-Type": "application/json",
            "X-OpenViking-Account": self._account,
            "X-OpenViking-User": self._user,
        }
        if self._api_key:
            h["X-API-Key"] = self._api_key
        return h

    def get(self, path: str, **kwargs) -> dict:
        resp = self._httpx.get(self._url(path), headers=self._headers(), timeout=30, **kwargs)
        resp.raise_for_status()
        return resp.json()

    def post(self, path: str, payload: dict = None, **kwargs) -> dict:
        resp = self._httpx.post(self._url(path), json=payload, headers=self._headers(), timeout=30, **kwargs)
        resp.raise_for_status()
        return resp.json()

    def health(self) -> bool:
        try:
            return self._httpx.get(self._url("/health"), timeout=3).status_code == 200
        except Exception:
            return False
```

#### Atexit Safety Net

```python
_last_active_provider: Optional["OpenVikingMemoryProvider"] = None

def _atexit_commit_sessions():
    """Fire on_session_end on process exit for pending sessions."""
    global _last_active_provider
    if _last_active_provider:
        try:
            _last_active_provider.on_session_end([])
        except Exception:
            pass

atexit.register(_atexit_commit_sessions)
```

#### Tool Schemas

```python
SEARCH_SCHEMA = {
    "name": "viking_search",
    "description": "Semantic search with viking:// URIs. mode: auto/fast/deep.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string"},
            "mode": {"type": "string", "enum": ["auto", "fast", "deep"]},
            "scope": {"type": "string", "description": "viking:// URI prefix"},
            "limit": {"type": "integer", "default": 10},
        },
        "required": ["query"],
    },
}

READ_SCHEMA = {
    "name": "viking_read",
    "description": "Read viking:// URI at abstract/overview/full level.",
    "parameters": {
        "type": "object",
        "properties": {
            "uri": {"type": "string"},
            "level": {"type": "string", "enum": ["abstract", "overview", "full"]},
        },
        "required": ["uri"],
    },
}

BROWSE_SCHEMA = {
    "name": "viking_browse",
    "description": "Browse like filesystem: list/tree/stat actions.",
    "parameters": {
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["tree", "list", "stat"]},
            "path": {"type": "string", "default": "viking://"},
        },
        "required": ["action"],
    },
}

REMEMBER_SCHEMA = {
    "name": "viking_remember",
    "description": "Store fact for extraction on session commit.",
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string"},
            "category": {"type": "string", "enum": ["preference", "entity", "event", "case", "pattern"]},
        },
        "required": ["content"],
    },
}

ADD_RESOURCE_SCHEMA = {
    "name": "viking_add_resource",
    "description": "Ingest URL/doc into knowledge base.",
    "parameters": {
        "type": "object",
        "properties": {
            "url": {"type": "string"},
            "reason": {"type": "string", "description": "Why relevant"},
        },
        "required": ["url"],
    },
}
```

---

### 7. RetainDB Memory Provider

**Path:** `plugins/memory/retaindb/`

**Description:** Cloud memory API with hybrid search (Vector + BM25 + Reranking) and 7 memory types.

#### Durable Write-Behind Queue

```python
class _WriteQueue:
    """SQLite-backed async write queue. Survives crashes — replays pending rows."""

    def __init__(self, client: _Client, db_path: Path):
        self._client = client
        self._db_path = db_path
        self._q: queue.Queue = queue.Queue()
        self._thread = threading.Thread(target=self._loop, daemon=True)
        self._local = threading.local()  # Per-thread connections
        self._init_db()
        self._thread.start()
        # Replay pending rows from crash
        for row_id, user_id, session_id, msgs_json in self._pending_rows():
            self._q.put((row_id, user_id, session_id, json.loads(msgs_json)))

    def enqueue(self, user_id: str, session_id: str, messages: list) -> None:
        conn = self._get_conn()
        cur = conn.execute(
            "INSERT INTO pending (user_id, session_id, messages_json, created_at) VALUES (?,?,?,?)",
            (user_id, session_id, json.dumps(messages), datetime.now(timezone.utc).isoformat()),
        )
        conn.commit()
        self._q.put((cur.lastrowid, user_id, session_id, messages))

    def _loop(self) -> None:
        while True:
            try:
                item = self._q.get(timeout=5)
                if item is _ASYNC_SHUTDOWN:
                    break
                self._flush_row(*item)
            except queue.Empty:
                continue

    def _flush_row(self, row_id: int, user_id: str, session_id: str, messages: list) -> None:
        try:
            self._client.ingest_session(user_id, session_id, messages)
            conn = self._get_conn()
            conn.execute("DELETE FROM pending WHERE id = ?", (row_id,))
            conn.commit()
        except Exception as exc:
            logger.warning("RetainDB ingest failed (retrying): %s", exc)
            conn = self._get_conn()
            conn.execute("UPDATE pending SET last_error = ? WHERE id = ?", (str(exc), row_id))
            conn.commit()
```

#### Overlay Context Builder

```python
def _build_overlay(profile: dict, query_result: dict, local_entries: list[str] = None) -> str:
    """Build deduplicated context overlay from profile + query results."""
    def _norm(s: str) -> str:
        return re.sub(r"[^a-z0-9 ]", "", re.sub(r"\s+", " ", s).strip()[:320].lower())

    seen = [_norm(e) for e in (local_entries or [])]
    items = []

    for m in list((profile or {}).get("memories") or [])[:5]:
        c = (m or {}).get("content", "")
        n = _norm(c)
        if c and n not in seen:
            seen.append(n)
            items.append(f"- {c}")

    for r in list((query_result or {}).get("results") or [])[:5]:
        c = (r or {}).get("content", "")
        n = _norm(c)
        if c and n not in seen:
            seen.append(n)
            items.append(f"- {c}")

    if not items:
        return ""
    return "\n".join(["[RetainDB Context]", "Profile:"] + items)
```

#### Tool Schemas

```python
PROFILE_SCHEMA = {"name": "retaindb_profile", "description": "User's stable profile", "parameters": {"type": "object", "properties": {}}}

SEARCH_SCHEMA = {
    "name": "retaindb_search",
    "description": "Semantic search with relevance scores.",
    "parameters": {
        "type": "object",
        "properties": {
            "query": {"type": "string"},
            "top_k": {"type": "integer", "default": 8, "max": 20},
        },
        "required": ["query"],
    },
}

CONTEXT_SCHEMA = {
    "name": "retaindb_context",
    "description": "Synthesized context for current task.",
    "parameters": {
        "type": "object",
        "properties": {"query": {"type": "string"}},
        "required": ["query"],
    },
}

REMEMBER_SCHEMA = {
    "name": "retaindb_remember",
    "description": "Persist fact with type + importance.",
    "parameters": {
        "type": "object",
        "properties": {
            "content": {"type": "string"},
            "memory_type": {"type": "string", "enum": ["factual", "preference", "goal", "instruction", "event", "opinion"]},
            "importance": {"type": "number", "default": 0.7},
        },
        "required": ["content"],
    },
}

FORGET_SCHEMA = {
    "name": "retaindb_forget",
    "description": "Delete memory by ID.",
    "parameters": {
        "type": "object",
        "properties": {"memory_id": {"type": "string"}},
        "required": ["memory_id"],
    },
}

# File tools
FILE_UPLOAD_SCHEMA = {"name": "retaindb_upload_file", "description": "Upload file, get rdb:// URI", "..."}
FILE_LIST_SCHEMA = {"name": "retaindb_list_files", "description": "List files by prefix", "..."}
FILE_READ_SCHEMA = {"name": "retaindb_read_file", "description": "Read file content by ID", "..."}
FILE_INGEST_SCHEMA = {"name": "retaindb_ingest_file", "description": "Extract memories from file", "..."}
FILE_DELETE_SCHEMA = {"name": "retaindb_delete_file", "description": "Delete file by ID", "..."}
```

---

## Plugin Lifecycle and Registration

### Registration Pattern

All memory providers use the `register(ctx)` pattern:

```python
def register(ctx) -> None:
    """Register <Provider> as a memory provider plugin."""
    ctx.register_memory_provider(<Provider>MemoryProvider())
```

The `_ProviderCollector` class simulates the plugin context:

```python
class _ProviderCollector:
    def __init__(self):
        self.provider = None

    def register_memory_provider(self, provider):
        self.provider = provider

    def register_tool(self, *args, **kwargs): pass
    def register_hook(self, *args, **kwargs): pass
    def register_cli_command(self, *args, **kwargs): pass
```

### Plugin Loading Flow

```
1. User selects provider via `hermes memory setup` or config
   ↓
2. Config: memory.provider = "<name>"
   ↓
3. Agent startup:
   - discover_memory_providers() scans plugins/memory/
   - Returns [(name, description, available), ...]
   ↓
4. Load active provider:
   - load_memory_provider("<name>") called
   - _load_provider_from_dir() imports module
   - Calls module.register(_ProviderCollector())
   ↓
5. Provider.initialize(session_id, **kwargs)
   ↓
6. During session:
   - system_prompt_block() → system prompt
   - queue_prefetch() → async prefetch
   - prefetch() → consume results
   - sync_turn() → record turns
   - handle_tool_call() → execute tools
   ↓
7. Session end:
   - on_session_end(messages)
   - shutdown()
```

---

## Extension Points

### Adding a New Memory Provider

Create subdirectory in `plugins/memory/`:

```
plugins/memory/myprovider/
├── __init__.py      # MyProviderMemoryProvider + register(ctx)
├── README.md        # Usage docs
└── plugin.yaml      # Metadata
```

**`__init__.py` template:**

```python
from agent.memory_provider import MemoryProvider

class MyProviderMemoryProvider(MemoryProvider):
    @property
    def name(self) -> str: return "myprovider"

    def is_available(self) -> bool: return True

    def get_config_schema(self):
        return [{"key": "api_key", "description": "API key", "secret": True}]

    def initialize(self, session_id: str, **kwargs) -> None: pass

    def system_prompt_block(self) -> str: return "# MyProvider\nActive."

    def prefetch(self, query: str, *, session_id: str = "") -> str: return ""

    def queue_prefetch(self, query: str, *, session_id: str = "") -> None: pass

    def sync_turn(self, user_content: str, assistant_content: str, *, session_id: str = "") -> None: pass

    def get_tool_schemas(self) -> List[Dict[str, Any]]: return []

    def handle_tool_call(self, tool_name: str, args: dict, **kwargs) -> str:
        return json.dumps({"error": f"Unknown tool: {tool_name}"})

    def shutdown(self) -> None: pass


def register(ctx) -> None:
    ctx.register_memory_provider(MyProviderMemoryProvider())
```

**`plugin.yaml` template:**

```yaml
name: myprovider
version: 1.0.0
description: "MyProvider — description."
pip_dependencies:
  - package1
requires_env:
  - MY_API_KEY
hooks:
  - on_session_end
```

### Adding CLI Commands

Create `cli.py`:

```python
def register_cli(subparser) -> None:
    parser = subparser.add_parser("myprovider", help="Manage myprovider")
    subparsers = parser.add_subparsers(dest="command")

    setup = subparsers.add_parser("setup", help="Set up")
    setup.set_defaults(func=myprovider_command)

    status = subparsers.add_parser("status", help="Show status")
    status.set_defaults(func=myprovider_status_command)


def myprovider_command(args):
    # Implementation
    pass
```

---

## Cross-References

### Related Modules

| Module | Relationship |
|--------|--------------|
| `agent/memory_provider.py` | ABC all providers implement |
| `agent/run_agent.py` | Initializes provider via `load_memory_provider()` |
| `hermes_cli/` | `hermes memory setup` for provider selection |
| `hermes_constants.py` | `get_hermes_home()` for paths |

### Config Files

| Path | Purpose |
|------|---------|
| `$HERMES_HOME/config.yaml` | `memory.provider` selection |
| `$HERMES_HOME/<provider>.json` | Provider-specific config |
| `$HERMES_HOME/.env` | Environment variables (API keys) |

### Database Files

| Provider | Database |
|----------|----------|
| holographic | `$HERMES_HOME/memory_store.db` |
| retaindb | `$HERMES_HOME/retaindb_queue.db` |
| byterover | `$HERMES_HOME/byterover/` (context tree) |

### External Dependencies

| Provider | Dependencies |
|----------|-------------|
| byterover | `brv` CLI |
| hindsight | `hindsight-client`, `hindsight-all` |
| holographic | `numpy` (optional) |
| honcho | `honcho-ai` |
| mem0 | `mem0ai` |
| openviking | `httpx` |
| retaindb | `requests` |

---

## Summary Table

| Provider | Storage | Key Features | Tools | Hooks |
|----------|---------|--------------|-------|-------|
| **byterover** | Hierarchical tree | CLI-based, local-first | `brv_query`, `brv_curate`, `brv_status` | `on_pre_compress` |
| **hindsight** | Cloud/local | Knowledge graph, entity resolution | `hindsight_retain`, `hindsight_recall`, `hindsight_reflect` | `on_session_end` |
| **holographic** | SQLite + HRR | Trust scoring, compositional retrieval | `fact_store` (9 actions), `fact_feedback` | `on_session_end` |
| **honcho** | Honcho Cloud | Cross-session, dialectic Q&A | `honcho_profile`, `honcho_search`, `honcho_context`, `honcho_conclude` | `on_session_end` |
| **mem0** | Mem0 Cloud | Server extraction, deduplication | `mem0_profile`, `mem0_search`, `mem0_conclude` | None |
| **openviking** | OpenViking | Filesystem browsing, tiered retrieval | `viking_search`, `viking_read`, `viking_browse`, `viking_remember`, `viking_add_resource` | `on_session_end` |
| **retaindb** | RetainDB Cloud | Hybrid search, durable queue, files | `retaindb_profile`, `retaindb_search`, `retaindb_context`, `retaindb_remember`, file tools | None |

---

**Document generated from source:**
- `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/plugins/`
