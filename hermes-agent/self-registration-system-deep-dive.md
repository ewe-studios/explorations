---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.NousResearch/hermes-agent
repository: git@github.com:NousResearch/hermes-agent.git
explored_at: 2026-03-25
---

# Self-Registration System Deep Dive

Hermes Agent uses a **self-registration system** for tools that provides a clean separation of concerns, eliminates circular dependencies, and significantly reduces context bloat. This document explores how the system works and what makes it unique.

## Architecture Overview

```
tools/registry.py  (no deps — imported by all tool files)
       ^
tools/*.py  (each calls registry.register() at module level)
       ^
model_tools.py  (imports tools/registry + triggers tool discovery)
       ^
run_agent.py, cli.py, batch_runner.py, environments/
```

### Key Design Principle

**Each tool file is self-contained and self-registering.** The tool file:
1. Imports the registry
2. Defines its handler function
3. Defines its schema
4. Calls `registry.register()` at module level

This eliminates the need for a central "tool manifest" that would require constant updates.

## The Registry Module

### ToolEntry Class

```python
# tools/registry.py

class ToolEntry:
    """Metadata for a single registered tool."""

    __slots__ = (
        "name", "toolset", "schema", "handler", "check_fn",
        "requires_env", "is_async", "description", "emoji",
    )

    def __init__(self, name, toolset, schema, handler, check_fn,
                 requires_env, is_async, description, emoji):
        self.name = name
        self.toolset = toolset
        self.schema = schema
        self.handler = handler
        self.check_fn = check_fn
        self.requires_env = requires_env
        self.is_async = is_async
        self.description = description
        self.emoji = emoji
```

Using `__slots__` reduces memory footprint — important when registering 40+ tools.

### ToolRegistry Class

```python
class ToolRegistry:
    """Singleton registry that collects tool schemas + handlers."""

    def __init__(self):
        self._tools: Dict[str, ToolEntry] = {}
        self._toolset_checks: Dict[str, Callable] = {}

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
        self._tools[name] = ToolEntry(
            name=name,
            toolset=toolset,
            schema=schema,
            handler=handler,
            check_fn=check_fn,
            requires_env=requires_env or [],
            is_async=is_async,
            description=description or schema.get("description", ""),
            emoji=emoji,
        )
        if check_fn and toolset not in self._toolset_checks:
            self._toolset_checks[toolset] = check_fn
```

### Module-Level Singleton

```python
# At bottom of tools/registry.py
registry = ToolRegistry()
```

This singleton is imported by all tool files and by `model_tools.py`.

## Example Tool File

```python
# tools/web_tools.py

import json
import os
from typing import List
from tools.registry import registry

# Parallel and Firecrawl imports
from parallel_web import ParallelWeb
from firecrawl import FirecrawlApp


def check_requirements() -> bool:
    """Check if required API keys are present."""
    return bool(
        os.getenv("PARALLEL_API_KEY") or
        os.getenv("FIRECRAWL_API_KEY")
    )


def web_search(query: str, task_id: str = None) -> str:
    """Search the web and return results."""
    # Implementation...
    results = parallel.search(query)
    return json.dumps({"results": results})


def web_extract(urls: List[str], task_id: str = None) -> str:
    """Extract content from URLs."""
    # Implementation...
    content = firecrawl.scrape_urls(urls)
    return json.dumps({"content": content})


# Register web_search
registry.register(
    name="web_search",
    toolset="web",
    schema={
        "name": "web_search",
        "description": "Search the web for current information",
        "parameters": {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        }
    },
    handler=lambda args, **kw: web_search(
        param=args.get("query", ""),
        task_id=kw.get("task_id")
    ),
    check_fn=check_requirements,
    requires_env=["PARALLEL_API_KEY"],
)

# Register web_extract
registry.register(
    name="web_extract",
    toolset="web",
    schema={...},
    handler=lambda args, **kw: web_extract(...),
    check_fn=check_requirements,
    requires_env=["FIRECRAWL_API_KEY"],
)
```

## Discovery Mechanism

### _discover_tools() Function

```python
# model_tools.py

def _discover_tools():
    """Trigger tool discovery by importing all tool modules.

    Each tool module calls registry.register() at module level.
    This function ensures all modules are imported.
    """
    from tools import (
        # File tools
        file_tools,
        # Web tools
        web_tools,
        # Terminal
        terminal_tool,
        # Browser
        browser_tool,
        # Skills
        skills_tool,
        # ... all other tool modules
    )
```

### Import Order Matters

```python
# model_tools.py — top of file

# 1. Import registry first (no deps)
from tools.registry import registry

# 2. Import toolsets for resolution
from toolsets import resolve_toolset, validate_toolset

# 3. Trigger discovery (imports all tool modules)
_discover_tools()

# 4. Now registry._tools is populated
```

## Schema Retrieval

### get_definitions() Method

```python
def get_definitions(self, tool_names: Set[str],
                    quiet: bool = False) -> List[dict]:
    """Return OpenAI-format tool schemas.

    Only includes tools whose check_fn() returns True.
    """
    result = []
    for name in sorted(tool_names):
        entry = self._tools.get(name)
        if not entry:
            continue

        # Run availability check
        if entry.check_fn:
            try:
                if not entry.check_fn():
                    if not quiet:
                        logger.debug("Tool %s unavailable", name)
                    continue
            except Exception:
                if not quiet:
                    logger.debug("Tool %s check raised; skipping", name)
                continue

        result.append({
            "type": "function",
            "function": entry.schema
        })

    return result
```

## Dispatch Mechanism

### dispatch() Method

```python
def dispatch(self, name: str, args: dict, **kwargs) -> str:
    """Execute a tool handler by name.

    - Async handlers bridged via _run_async()
    - All exceptions caught and returned as {"error": "..."}
    """
    entry = self._tools.get(name)
    if not entry:
        return json.dumps({"error": f"Unknown tool: {name}"})

    try:
        if entry.is_async:
            from model_tools import _run_async
            return _run_async(entry.handler(args, **kwargs))
        return entry.handler(args, **kwargs)
    except Exception as e:
        logger.exception("Tool %s dispatch error: %s", name, e)
        return json.dumps(
            {"error": f"Tool execution failed: {type(e).__name__}: {e}"}
        )
```

## Why This Reduces Context Bloat

### Traditional Approach (High Bloat)

```python
# In a central "tools.py" file:

TOOLS = {
    "web_search": {
        "schema": {...},
        "handler": web_search_handler,
        "requires": ["PARALLEL_API_KEY"],
    },
    "web_extract": {
        "schema": {...},
        "handler": web_extract_handler,
        "requires": ["FIRECRAWL_API_KEY"],
    },
    # ... 40+ more tools
    # Every time you add a tool, this file grows
    # Every tool's code is visible in one place
}

def get_tool_schemas():
    return [t["schema"] for t in TOOLS.values()]
```

**Problems:**
1. Single file grows unbounded (~5000+ lines)
2. Hard to find specific tool implementation
3. Circular import risks (tool A imports tool B)
4. Every tool loaded even if not needed

### Self-Registration Approach (Low Bloat)

```python
# Each tool file is独立 (standalone):
# tools/web_tools.py — only web logic
# tools/file_tools.py — only file logic
# tools/terminal_tool.py — only terminal logic

# model_tools.py — thin orchestration layer
from tools.registry import registry
_discover_tools()  # Just imports modules

# Registry stays small (~200 lines)
# No tool implementation code in central files
```

**Benefits:**
1. Each tool file is focused and findable
2. Central files stay small (~200 lines for registry)
3. No circular imports (registry has no deps)
4. Tools can be lazily loaded in future optimization

## What Makes It Unique

### 1. Circular-Import Safe

```python
# tools/registry.py
# NO imports from model_tools or tool files
# This is the key insight!

class ToolRegistry:
    pass

registry = ToolRegistry()

# tools/web_tools.py
from tools.registry import registry  # Safe - registry has no deps
registry.register(...)

# model_tools.py
from tools.registry import registry  # Safe
from tools import web_tools  # Safe - web_tools only imports registry
```

### 2. Plugin-Ready Architecture

```python
# Plugin tool registration (external package)

# my_plugin/tools.py
from tools.registry import registry

registry.register(
    name="my_custom_tool",
    toolset="my_plugin",
    schema={...},
    handler=my_handler,
    check_fn=my_check,
)

# hermes-agent automatically picks up plugin tools
# because registry is a singleton shared across imports
```

### 3. Toolset Checks at Registration Time

```python
def register(self, ..., check_fn=None, ...):
    # ...
    if check_fn and toolset not in self._toolset_checks:
        self._toolset_checks[toolset] = check_fn
```

This automatically builds toolset availability checks without a central manifest.

### 4. Async Bridging Built-In

```python
def dispatch(self, name, args, **kwargs):
    entry = self._tools.get(name)
    if entry.is_async:
        return _run_async(entry.handler(args, **kwargs))
    return entry.handler(args, **kwargs)
```

Sync code calls async tools without knowing they're async.

## Toolset Integration

### Checking Toolset Availability

```python
def is_toolset_available(self, toolset: str) -> bool:
    """Check if a toolset's requirements are met."""
    check = self._toolset_checks.get(toolset)
    if not check:
        return True  # No check = always available
    try:
        return bool(check())
    except Exception:
        logger.debug("Toolset %s check raised; marking unavailable", toolset)
        return False
```

### Getting All Toolset Requirements

```python
def check_toolset_requirements(self) -> Dict[str, bool]:
    """Return {toolset: available_bool} for every toolset."""
    toolsets = set(e.toolset for e in self._tools.values())
    return {ts: self.is_toolset_available(ts) for ts in sorted(toolsets)}
```

### Building UI Display Data

```python
def get_available_toolsets(self) -> Dict[str, dict]:
    """Return toolset metadata for UI display."""
    toolsets: Dict[str, dict] = {}

    for entry in self._tools.values():
        ts = entry.toolset
        if ts not in toolsets:
            toolsets[ts] = {
                "available": self.is_toolset_available(ts),
                "tools": [],
                "description": "",
                "requirements": [],
            }
        toolsets[ts]["tools"].append(entry.name)
        if entry.requires_env:
            for env in entry.requires_env:
                if env not in toolsets[ts]["requirements"]:
                    toolsets[ts]["requirements"].append(env)

    return toolsets
```

## Comparison with Other Systems

| Feature | Traditional Manifest | Self-Registration |
|---------|---------------------|-------------------|
| Central file size | Grows unbounded | Fixed (~200 lines) |
| Circular imports | Risk increases with tools | Impossible by design |
| Adding new tool | Edit manifest + add file | Just add file |
| Plugin support | Requires extension points | Automatic |
| Lazy loading | Difficult | Possible |
| Tool removal | Edit manifest + delete file | Just delete file |

## Extension: Plugin Toolsets

```python
def _get_plugin_toolset_names() -> Set[str]:
    """Return toolset names registered by plugins.

    These are toolsets that exist in the registry but not in
    the static TOOLSETS dict — added by plugins at load time.
    """
    try:
        from tools.registry import registry
        return {
            entry.toolset
            for entry in registry._tools.values()
            if entry.toolset not in TOOLSETS
        }
    except Exception:
        return set()
```

This allows plugins to introduce entirely new toolsets without modifying `toolsets.py`.

## Query Helpers

The registry provides helpers that replace redundant dicts in `model_tools.py`:

```python
def get_all_tool_names(self) -> List[str]:
    """Return sorted list of all registered tool names."""
    return sorted(self._tools.keys())

def get_toolset_for_tool(self, name: str) -> Optional[str]:
    """Return the toolset a tool belongs to."""
    entry = self._tools.get(name)
    return entry.toolset if entry else None

def get_emoji(self, name: str, default: str = "⚡") -> str:
    """Return the emoji for a tool."""
    entry = self._tools.get(name)
    return entry.emoji if entry and entry.emoji else default

def get_tool_to_toolset_map(self) -> Dict[str, str]:
    """Return {tool_name: toolset_name} for every tool."""
    return {name: e.toolset for name, e in self._tools.items()}
```

## Backward Compatibility

The registry maintains compatibility with the old system:

```python
def get_available_toolsets(self, quiet: bool = False):
    """Return (available_toolsets, unavailable_info) like old function."""
    available = []
    unavailable = []
    seen = set()

    for entry in self._tools.values():
        ts = entry.toolset
        if ts in seen:
            continue
        seen.add(ts)

        if self.is_toolset_available(ts):
            available.append(ts)
        else:
            unavailable.append({
                "name": ts,
                "env_vars": entry.requires_env,
                "tools": [
                    e.name for e in self._tools.values()
                    if e.toolset == ts
                ],
            })

    return available, unavailable
```

## Summary

The self-registration system provides:

1. **Separation of Concerns** — Each tool file is independent
2. **Bloat Reduction** — Central files stay small
3. **Circular Import Safety** — Registry has no dependencies
4. **Plugin Readiness** — External packages can register tools
5. **Automatic Discovery** — No manifest to maintain
6. **Clean Extension** — New toolsets via plugins
7. **Async Transparency** — Sync/async unified at dispatch

This architecture scales to 100+ tools without central file bloat and enables a plugin ecosystem without explicit extension points.
