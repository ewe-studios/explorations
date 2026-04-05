# OpenSpace MCP Server Integration: Comprehensive Deep-Dive

A complete technical exploration of OpenSpace's Model Context Protocol (MCP) server implementation, covering architecture, tool definitions, transport mechanisms, host agent integration, and production-ready patterns.

---

## Table of Contents

1. [MCP Server Architecture](#1-mcp-server-architecture)
2. [MCP Protocol Overview](#2-mcp-protocol-overview)
3. [Transport Mechanisms](#3-transport-mechanisms)
4. [MCP Tools Reference](#4-mcp-tools-reference)
5. [Tool Implementation Details](#5-tool-implementation-details)
6. [Host Agent Integration](#6-host-agent-integration)
7. [Host Skills System](#7-host-skills-system)
8. [Credential Detection & Resolution](#8-credential-detection--resolution)
9. [MCP Safe Stdout](#9-mcp-safe-stdout)
10. [Configuration Reference](#10-configuration-reference)
11. [Production Integration Patterns](#11-production-integration-patterns)

---

## 1. MCP Server Architecture

### 1.1 System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Agent (Host)                              │
│  (Claude Code / OpenClaw / nanobot / Codex / Cursor)            │
│                          │                                        │
│                          │ MCP Protocol                          │
└──────────────────────────┼────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                    OpenSpace MCP Server                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  mcp_server.py                                            │   │
│  │  ┌────────────────────────────────────────────────────┐  │   │
│  │  │  MCP Server Instance (FastMCP)                     │  │   │
│  │  │  - stdio transport                                 │  │   │
│  │  │  - JSON-RPC 2.0 protocol                           │  │   │
│  │  │  - Tool registry (4 tools)                         │  │   │
│  │  └────────────────────────────────────────────────────┘  │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Tool Definitions                                         │   │
│  │  - execute_task: Main task delegation                    │   │
│  │  - search_skills: Skill discovery                        │   │
│  │  - fix_skill: Manual skill repair                        │   │
│  │  - upload_skill: Cloud upload                            │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                      OpenSpace Engine                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ SkillRegistry│  │GroundingAgent│  │SkillEvolver  │          │
│  │ (discovery)  │  │ (execution)  │  │ (evolution)  │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ SkillStore   │  │ Execution    │  │  Cloud       │          │
│  │ (SQLite DB)  │  │ Analyzer     │  │  Client      │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 mcp_server.py Structure

The MCP server is built using the `FastMCP` framework from the Anthropic MCP SDK:

```python
#!/usr/bin/env python3
"""
OpenSpace MCP Server

Exposes OpenSpace capabilities as MCP tools for agent clients.
"""

import asyncio
import json
import logging
import os
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional

from mcp.server.fastmcp import FastMCP

from openspace import OpenSpace, OpenSpaceConfig
from openspace.cloud.client import OpenSpaceClient
from openspace.skill_engine.registry import SkillRegistry
from openspace.skill_engine.store import SkillStore

# ============================================================================
# Configuration
# ============================================================================

# MCP Server instance
mcp = FastMCP(
    name="openspace",
    instructions="OpenSpace skill execution and evolution platform",
)

# Global OpenSpace instance (lazy-initialized)
_openspace: Optional[OpenSpace] = None
_cloud_client: Optional[OpenSpaceClient] = None

# ============================================================================
# Initialization
# ============================================================================

async def _get_openspace() -> OpenSpace:
    """Lazy-initialize and return the OpenSpace instance."""
    global _openspace
    
    if _openspace is None:
        config = OpenSpaceConfig(
            llm_model=os.getenv("OPENSPACE_MODEL", "openrouter/anthropic/claude-sonnet-4.5"),
            llm_kwargs={"api_key": os.getenv("OPENROUTER_API_KEY")},
            workspace_dir=os.getenv("OPENSPACE_WORKSPACE", Path.cwd()),
            grounding_max_iterations=int(os.getenv("OPENSPACE_MAX_ITERATIONS", "20")),
            enable_recording=os.getenv("OPENSPACE_ENABLE_RECORDING", "true").lower() == "true",
        )
        _openspace = OpenSpace(config=config)
        await _openspace.__aenter__()
    
    return _openspace


def _get_cloud_client() -> Optional[OpenSpaceClient]:
    """Get the cloud API client (if configured)."""
    global _cloud_client
    
    if _cloud_client is None:
        api_key = os.getenv("OPENSPACE_API_KEY")
        if api_key:
            _cloud_client = OpenSpaceClient(
                auth_headers={"Authorization": f"Bearer {api_key}"},
                api_base="https://api.open-space.cloud",
            )
    
    return _cloud_client


def has_api_key() -> bool:
    """Check if cloud API key is configured."""
    return bool(os.getenv("OPENSPACE_API_KEY"))


# ============================================================================
# Tool Definitions
# ============================================================================

@mcp.tool()
async def execute_task(
    task: str,
    search_scope: str = "all",
    max_iterations: int = 20,
) -> dict:
    """Execute a task with skill search, execution, and evolution."""
    # ... (see Section 5.1)


@mcp.tool()
async def search_skills(
    query: str,
    source: str = "all",
    limit: int = 20,
    auto_import: bool = True,
) -> list:
    """Search for skills matching the query."""
    # ... (see Section 5.2)


@mcp.tool()
async def fix_skill(
    skill_dir: str,
    direction: str,
) -> dict:
    """Fix a skill with explicit instructions."""
    # ... (see Section 5.3)


@mcp.tool()
async def upload_skill(
    skill_dir: str,
    visibility: str = "public",
    origin: Optional[str] = None,
    tags: Optional[List[str]] = None,
    change_summary: Optional[str] = None,
) -> dict:
    """Upload a skill to the cloud community."""
    # ... (see Section 5.4)


# ============================================================================
# Entry Point
# ============================================================================

if __name__ == "__main__":
    # Run MCP server with stdio transport
    mcp.run(transport="stdio")
```

### 1.3 Server Lifecycle

```
┌──────────────────────────────────────────────────────────────┐
│                      Server Startup                           │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  1. FastMCP instance created                                  │
│     - name: "openspace"                                       │
│     - transport: stdio                                        │
│     - tools registered via @mcp.tool() decorator             │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  2. Host agent spawns MCP server process                      │
│     - command: "openspace-mcp" or "python -m openspace.mcp"  │
│     - stdin/stdout connected to agent                        │
│     - stderr redirected to log file                          │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  3. mcp.run(transport="stdio") called                         │
│     - Reads JSON-RPC requests from stdin                     │
│     - Writes JSON-RPC responses to stdout                    │
│     - Handles initialize, tools/list, tools/call             │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────┐
│  4. Tool execution                                            │
│     - Lazy initialization of OpenSpace instance              │
│     - Tool handler executes task                             │
│     - Result returned as JSON-RPC response                   │
└──────────────────────────────────────────────────────────────┘
```

---

## 2. MCP Protocol Overview

### 2.1 Model Context Protocol (MCP)

MCP is an open protocol that standardizes how applications provide context to LLMs. It enables:

- **Tool Discovery**: Clients can list available tools
- **Tool Invocation**: Clients can call tools with structured arguments
- **Resource Access**: Servers can expose resources for reading
- **Prompt Templates**: Servers can provide reusable prompts

### 2.2 JSON-RPC 2.0 Message Format

All MCP messages follow JSON-RPC 2.0 format:

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "execute_task",
    "arguments": {
      "task": "Monitor Docker containers"
    }
  }
}
```

**Response (Success):**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"status\": \"success\", \"response\": \"...\"}"
      }
    ]
  }
}
```

**Response (Error):**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Invalid params: task is required"
  }
}
```

### 2.3 MCP Message Flow

```
┌─────────────┐                              ┌─────────────┐
│   Client    │                              │   Server    │
│  (Agent)    │                              │ (OpenSpace) │
└──────┬──────┘                              └──────┬──────┘
       │                                            │
       │  {"jsonrpc":"2.0","id":1,                  │
       │   "method":"initialize"}                   │
       │───────────────────────────────────────────>│
       │                                            │
       │  {"jsonrpc":"2.0","id":1,                  │
       │   "result":{"protocolVersion":"2024-11-05"}}│
       │<───────────────────────────────────────────│
       │                                            │
       │  {"jsonrpc":"2.0","id":2,                  │
       │   "method":"tools/list"}                   │
       │───────────────────────────────────────────>│
       │                                            │
       │  {"jsonrpc":"2.0","id":2,                  │
       │   "result":{"tools":[                      │
       │     {"name":"execute_task",...},           │
       │     {"name":"search_skills",...},          │
       │     {"name":"fix_skill",...},              │
       │     {"name":"upload_skill",...}            │
       │   ]}}                                      │
       │<───────────────────────────────────────────│
       │                                            │
       │  {"jsonrpc":"2.0","id":3,                  │
       │   "method":"tools/call",                   │
       │   "params":{"name":"execute_task",         │
       │            "arguments":{"task":"..."}}}    │
       │───────────────────────────────────────────>│
       │                                            │
       │  (Server executes task)                    │
       │                                            │
       │  {"jsonrpc":"2.0","id":3,                  │
       │   "result":{"content":[{"type":"text",     │
       │              "text":"{\"status\":\"ok\"}"}]}}│
       │<───────────────────────────────────────────│
       │                                            │
```

### 2.4 Standard MCP Methods

| Method | Description |
|--------|-------------|
| `initialize` | Establish protocol version and capabilities |
| `tools/list` | Return list of available tools |
| `tools/call` | Invoke a tool with arguments |
| `resources/list` | List available resources (optional) |
| `resources/read` | Read a resource (optional) |
| `prompts/list` | List available prompts (optional) |
| `prompts/get` | Get a prompt template (optional) |

### 2.5 Tool Definition Schema

```typescript
interface Tool {
  name: string;
  description?: string;
  inputSchema: {
    type: "object";
    properties: Record<string, {
      type: string;
      description?: string;
      required?: boolean;
    }>;
  };
}
```

---

## 3. Transport Mechanisms

### 3.1 Transport Overview

OpenSpace MCP supports multiple transport mechanisms:

| Transport | Use Case | Latency | Complexity |
|-----------|----------|---------|------------|
| stdio | Local processes | Low | Low |
| HTTP/HTTPS | Remote servers | Medium | Medium |
| WebSocket | Real-time bidirectional | Low | High |

### 3.2 stdio Transport (Primary)

The stdio transport is the primary mechanism for local agent integration:

```python
class StdioConnector(MCPConnector):
    """
    stdio transport connector for MCP servers.
    
    Communication happens via:
    - stdin: JSON-RPC requests written as newline-delimited JSON
    - stdout: JSON-RPC responses read as newline-delimited JSON
    - stderr: Logging and error output (redirected to file)
    """

    def __init__(self, config: Dict[str, Any]):
        self._command = config["command"]  # e.g., "openspace-mcp"
        self._args = config.get("args", [])  # e.g., ["--debug"]
        self._process: Optional[asyncio.subprocess.Process] = None
        self._message_id = 0
        self._pending: Dict[int, asyncio.Future] = {}

    async def connect(self) -> None:
        """Spawn the MCP server process."""
        self._process = await asyncio.create_subprocess_exec(
            self._command,
            *self._args,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        
        # Start reader task
        asyncio.create_task(self._read_loop())

    async def _read_loop(self) -> None:
        """Continuously read responses from stdout."""
        while True:
            line = await self._process.stdout.readline()
            if not line:
                break  # Process exited
            
            try:
                response = json.loads(line.decode())
                msg_id = response.get("id")
                if msg_id in self._pending:
                    self._pending[msg_id].set_result(response)
            except (json.JSONDecodeError, UnicodeDecodeError) as e:
                logger.error(f"Failed to parse response: {e}")

    async def list_tools(self) -> List[ToolDefinition]:
        """Request tool list from server."""
        return await self._send_request("tools/list", {})

    async def call_tool(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> MCPResult:
        """Call a tool on the server."""
        return await self._send_request("tools/call", {
            "name": tool_name,
            "arguments": arguments,
        })

    async def _send_request(self, method: str, params: Dict) -> Any:
        """Send JSON-RPC request and wait for response."""
        self._message_id += 1
        future = asyncio.Future()
        self._pending[self._message_id] = future

        request = {
            "jsonrpc": "2.0",
            "id": self._message_id,
            "method": method,
            "params": params,
        }

        # Write request (newline-delimited JSON)
        self._process.stdin.write((json.dumps(request) + "\n").encode())
        await self._process.stdin.drain()

        # Wait for response with timeout
        try:
            response = await asyncio.wait_for(future, timeout=60.0)
            if "error" in response:
                raise MCPError(response["error"]["message"])
            return response.get("result")
        finally:
            del self._pending[self._message_id]
```

### 3.3 HTTP Transport

For remote MCP servers:

```python
class HTTPConnector(MCPConnector):
    """
    HTTP transport connector for remote MCP servers.
    
    Endpoints:
    - GET  /tools       - List available tools
    - POST /tools/:name/call - Call a tool
    """

    def __init__(self, config: Dict[str, Any]):
        self._base_url = config["url"]
        self._api_key = config.get("api_key")
        self._headers = {}
        if self._api_key:
            self._headers["Authorization"] = f"Bearer {self._api_key}"
        self._session = aiohttp.ClientSession(headers=self._headers)

    async def list_tools(self) -> List[ToolDefinition]:
        """Fetch tool list from HTTP endpoint."""
        async with self._session.get(f"{self._base_url}/tools") as resp:
            data = await resp.json()
            return [ToolDefinition(**t) for t in data["tools"]]

    async def call_tool(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> MCPResult:
        """Call tool via HTTP POST."""
        async with self._session.post(
            f"{self._base_url}/tools/{tool_name}/call",
            json={"arguments": arguments},
        ) as resp:
            data = await resp.json()
            return MCPResult(**data)

    async def close(self) -> None:
        await self._session.close()
```

### 3.4 WebSocket Transport

For real-time bidirectional communication:

```python
class WebSocketConnector(MCPConnector):
    """
    WebSocket transport for real-time MCP communication.
    
    Supports:
    - Server-initiated notifications
    - Streaming tool results
    - Connection keepalive
    """

    def __init__(self, config: Dict[str, Any]):
        self._url = config["url"]
        self._ws: Optional[aiohttp.ClientWebSocketResponse] = None
        self._message_id = 0
        self._pending: Dict[int, asyncio.Future] = {}

    async def connect(self) -> None:
        """Establish WebSocket connection."""
        self._ws = await aiohttp.ClientSession().ws_connect(self._url)
        asyncio.create_task(self._message_handler())

    async def _message_handler(self) -> None:
        """Handle incoming WebSocket messages."""
        async for msg in self._ws:
            if msg.type == aiohttp.WSMsgType.TEXT:
                data = json.loads(msg.data)
                msg_id = data.get("id")
                if msg_id in self._pending:
                    self._pending[msg_id].set_result(data)

    async def call_tool(
        self,
        tool_name: str,
        arguments: Dict[str, Any],
    ) -> MCPResult:
        """Call tool via WebSocket."""
        self._message_id += 1
        future = asyncio.Future()
        self._pending[self._message_id] = future

        await self._ws.send_json({
            "jsonrpc": "2.0",
            "id": self._message_id,
            "method": "tools/call",
            "params": {"name": tool_name, "arguments": arguments},
        })

        response = await asyncio.wait_for(future, timeout=60.0)
        return response.get("result")
```

### 3.5 Transport Comparison

```
┌─────────────────────────────────────────────────────────────────┐
│                     Transport Comparison                         │
├─────────────────────────────────────────────────────────────────┤
│ Feature          │ stdio        │ HTTP         │ WebSocket      │
├─────────────────────────────────────────────────────────────────┤
│ Direction        │ Half-duplex  │ Request/Resp │ Full-duplex    │
│ Latency          │ ~1ms         │ ~10-100ms    │ ~1-5ms         │
│ Setup            │ Process spawn│ TCP connect  │ WS handshake   │
│ Auth             │ N/A (local)  │ Bearer token │ Bearer token   │
│ Streaming        │ No           │ SSE possible │ Native         │
│ Notifications    │ No           │ No           │ Yes            │
│ Firewall         │ N/A          │ Port needed  │ Port needed    │
│ Best For         │ Local agents │ Remote API   │ Real-time apps │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. MCP Tools Reference

OpenSpace exposes 4 MCP tools for agent integration:

### 4.1 Tool Summary

| Tool | Purpose | Input | Output |
|------|---------|-------|--------|
| `execute_task` | Delegate task execution | task, search_scope, max_iterations | status, response, evolved_skills |
| `search_skills` | Discover skills | query, source, limit, auto_import | List of skill matches |
| `fix_skill` | Manual skill repair | skill_dir, direction | status, skill_dir, upload_ready |
| `upload_skill` | Upload to cloud | skill_dir, visibility, tags | status, skill_id |

---

## 5. Tool Implementation Details

### 5.1 execute_task - Main Task Delegation Tool

**Purpose:** Delegate a natural language task to OpenSpace for execution with skill discovery, execution, and evolution.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "task": {
      "type": "string",
      "description": "Natural language task description"
    },
    "search_scope": {
      "type": "string",
      "enum": ["local", "cloud", "all"],
      "default": "all",
      "description": "Where to search for skills"
    },
    "max_iterations": {
      "type": "integer",
      "default": 20,
      "description": "Maximum tool-calling iterations"
    }
  },
  "required": ["task"]
}
```

**Handler Implementation:**
```python
@mcp.tool()
async def execute_task(
    task: str,
    search_scope: str = "all",
    max_iterations: int = 20,
) -> dict:
    """
    Execute a task with skill search, execution, and evolution.
    
    This is the primary tool for delegating work to OpenSpace.
    
    Flow:
    1. Search for relevant skills (BM25 + embedding + LLM)
    2. Inject skill context into agent prompt
    3. Run grounding agent with iterative tool calling
    4. Analyze execution for evolution opportunities
    5. Evolve skills if needed (FIX/DERIVED/CAPTURED)
    
    Args:
        task: Natural language task description
        search_scope: Where to search (local/cloud/all)
        max_iterations: Maximum tool-calling iterations
    
    Returns:
        dict with status, response, and evolved_skills
    """
    openspace = await _get_openspace()
    
    result = await openspace.execute(
        instruction=task,
        search_scope=search_scope,
        max_iterations=max_iterations,
    )
    
    # Format response for MCP
    return {
        "status": result["status"],
        "response": result["response"],
        "evolved_skills": result.get("evolved_skills", []),
        "metrics": result.get("metrics", {}),
    }
```

**Example Call Sequence:**
```
Agent: "Monitor my Docker containers and restart the highest memory one"

┌─────────────────────────────────────────────────────────────┐
│ execute_task Request                                        │
├─────────────────────────────────────────────────────────────┤
│ {                                                           │
│   "task": "Monitor Docker containers and restart highest",  │
│   "search_scope": "all",                                    │
│   "max_iterations": 20                                      │
│ }                                                           │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ OpenSpace Execution Flow                                    │
├─────────────────────────────────────────────────────────────┤
│ 1. SkillRegistry.select_relevant_skills(task)               │
│    - BM25 prefilter (threshold=0.3)                         │
│    - Embedding ranking                                    │
│    - LLM final selection                                  │
│    → ["docker-monitor", "container-restart"]               │
│                                                             │
│ 2. Build skills context                                     │
│    - Load SKILL.md content                                 │
│    - Format for system prompt                              │
│                                                             │
│ 3. GroundingAgent.process()                                 │
│    - Initialize messages with skill context                │
│    - Loop (max_iterations):                                │
│      a. LLM decides next action                            │
│      b. Parse tool calls                                   │
│      c. Execute tools via backends                         │
│      d. Append results to messages                         │
│                                                             │
│ 4. ExecutionAnalyzer.analyze()                              │
│    - Check for failures                                    │
│    - Identify evolution opportunities                      │
│                                                             │
│ 5. SkillEvolver.evolve() (if triggered)                     │
│    - FIX: Repair broken skill                              │
│    - DERIVED: Create enhanced version                      │
│    - CAPTURED: Extract new pattern                         │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ execute_task Response                                       │
├─────────────────────────────────────────────────────────────┤
│ {                                                           │
│   "status": "success",                                      │
│   "response": "Container 'api-server' restarted (was using │
│                2.1GB RAM). All containers now healthy.",    │
│   "evolved_skills": [                                       │
│     {                                                       │
│       "name": "docker-monitor",                             │
│       "origin": "FIX",                                      │
│       "change_summary": "Added memory-based sorting"        │
│     }                                                       │
│   ],                                                        │
│   "metrics": {                                              │
│     "iterations": 5,                                        │
│     "tools_called": ["shell.docker_ps", "shell.docker_inspect"],
│     "duration_ms": 3421                                      │
│   }                                                         │
│ }                                                           │
└─────────────────────────────────────────────────────────────┘
```

**Error Handling:**
```python
try:
    result = await openspace.execute(...)
except OpenSpaceError as e:
    return {
        "status": "error",
        "error_type": type(e).__name__,
        "message": str(e),
        "suggestion": e.suggestion,  # Optional recovery hint
    }
except Exception as e:
    logger.exception("Unexpected error in execute_task")
    return {
        "status": "error",
        "error_type": "InternalError",
        "message": f"Unexpected error: {str(e)}",
    }
```

---

### 5.2 search_skills - Skill Discovery Tool

**Purpose:** Search for skills matching a query across local and cloud sources.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "Search query"
    },
    "source": {
      "type": "string",
      "enum": ["local", "cloud", "all"],
      "default": "all",
      "description": "Where to search"
    },
    "limit": {
      "type": "integer",
      "default": 20,
      "description": "Maximum results to return"
    },
    "auto_import": {
      "type": "boolean",
      "default": true,
      "description": "Auto-import top cloud skill if found"
    }
  },
  "required": ["query"]
}
```

**Handler Implementation:**
```python
@mcp.tool()
async def search_skills(
    query: str,
    source: str = "all",
    limit: int = 20,
    auto_import: bool = True,
) -> list:
    """
    Search for skills matching the query.
    
    Uses hybrid search pipeline:
    1. BM25 keyword prefilter (fast, recall-oriented)
    2. Embedding similarity ranking (semantic matching)
    3. LLM final selection (precision-oriented)
    
    Args:
        query: Search query string
        source: Where to search (local/cloud/all)
        limit: Max results to return
        auto_import: Auto-import top cloud skill
    
    Returns:
        List of skill matches with metadata
    """
    openspace = await _get_openspace()
    registry = openspace._skill_registry
    store = openspace._skill_store
    
    results = []
    
    # Local search (always)
    if source in ("local", "all"):
        local_results = await registry.select_relevant_skills(query, limit=limit)
        for skill_id in local_results:
            meta = registry.get_skill_meta(skill_id)
            record = store.get_record(skill_id)
            results.append({
                "source": "local",
                "skill_id": skill_id,
                "name": meta.name,
                "description": meta.description,
                "skill_dir": str(meta.path),
                "applied_rate": record.applied_rate if record else 0,
                "completion_rate": record.completion_rate if record else 0,
            })
    
    # Cloud search (if configured)
    if source in ("cloud", "all") and has_api_key():
        client = _get_cloud_client()
        cloud_results = client.search_record_embeddings(query, limit=limit)
        
        for cloud_hit in cloud_results:
            results.append({
                "source": "cloud",
                "skill_id": cloud_hit["skill_id"],
                "record_id": cloud_hit["record_id"],
                "name": cloud_hit["name"],
                "description": cloud_hit["description"],
                "visibility": cloud_hit.get("visibility", "public"),
                "tags": cloud_hit.get("tags", []),
                # Pass-through metadata for potential import
                "_cloud_metadata": {
                    "artifact_id": cloud_hit.get("artifact_id"),
                    "level": cloud_hit.get("level"),
                }
            })
        
        # Auto-import top skill if enabled
        if auto_import and cloud_results:
            top_skill = cloud_results[0]
            await _auto_import_skill(top_skill, registry, store)
    
    # Sort by relevance (local first, then cloud)
    return results[:limit]
```

**Auto-Import Implementation:**
```python
async def _auto_import_skill(
    cloud_hit: dict,
    registry: SkillRegistry,
    store: SkillStore,
) -> None:
    """Auto-import the top cloud skill."""
    client = _get_cloud_client()
    
    # Get host workspace from env
    host_ws = os.getenv("OPENSPACE_HOST_SKILL_DIRS")
    if not host_ws:
        logger.warning("OPENSPACE_HOST_SKILL_DIRS not set, skipping auto-import")
        return
    
    base_dir = Path(host_ws) / "skills"
    base_dir.mkdir(parents=True, exist_ok=True)
    
    # Import skill files
    result = client.import_skill(
        skill_id=cloud_hit["skill_id"],
        record_id=cloud_hit["record_id"],
        base_dir=base_dir,
    )
    
    skill_dir = Path(result.get("local_path", ""))
    if skill_dir.exists():
        # Register with local registry
        meta = registry.register_skill_dir(skill_dir)
        if meta:
            await store.sync_from_registry([meta])
            logger.info(f"Auto-imported skill: {meta.name} -> {skill_dir}")
```

---

### 5.3 fix_skill - Manual Skill Repair Tool

**Purpose:** Manually fix a broken skill with explicit repair instructions.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "skill_dir": {
      "type": "string",
      "description": "Path to the skill directory"
    },
    "direction": {
      "type": "string",
      "description": "Explicit fix instructions"
    }
  },
  "required": ["skill_dir", "direction"]
}
```

**Handler Implementation:**
```python
@mcp.tool()
async def fix_skill(
    skill_dir: str,
    direction: str,
) -> dict:
    """
    Fix a skill with explicit instructions.
    
    Use this when a skill is broken or needs improvement.
    The evolver agent will read the current SKILL.md, analyze
    the issue, and apply edits based on the direction.
    
    Args:
        skill_dir: Path to the skill directory
        direction: Explicit fix instructions (e.g., "Add error handling for Docker API failures")
    
    Returns:
        dict with status, skill_dir, and upload_ready flag
    """
    openspace = await _get_openspace()
    
    result = await openspace.evolver.fix_skill_sync(
        skill_dir=Path(skill_dir),
        fix_direction=direction,
    )
    
    if result.success:
        # Write upload metadata for later cloud upload
        _write_upload_meta(result.skill_dir, {
            "origin": "fix",
            "change_summary": direction,
        })
        
        return {
            "status": "success",
            "skill_dir": str(result.skill_dir),
            "upload_ready": True,
            "validation_errors": result.validation_errors,
        }
    else:
        return {
            "status": "error",
            "error": result.error_message,
            "skill_dir": str(result.skill_dir),
            "upload_ready": False,
        }


def _write_upload_meta(skill_dir: Path, metadata: dict) -> None:
    """Write .upload_meta.json for later cloud upload."""
    meta_file = skill_dir / ".upload_meta.json"
    meta = {}
    if meta_file.exists():
        meta = json.loads(meta_file.read_text())
    meta.update(metadata)
    meta_file.write_text(json.dumps(meta, indent=2))
```

**Fix Flow:**
```
┌─────────────────────────────────────────────────────────────┐
│ fix_skill Flow                                              │
├─────────────────────────────────────────────────────────────┤
│ 1. Read current SKILL.md                                    │
│ 2. Build agent prompt with:                                 │
│    - Current skill content                                  │
│    - Fix direction                                          │
│    - Editing guidelines                                     │
│ 3. Agent loop:                                              │
│    - Analyze what's broken                                  │
│    - Generate edit plan                                     │
│    - Apply edits to SKILL.md                                │
│    - Validate new skill format                              │
│ 4. Write updated SKILL.md                                   │
│ 5. Write .upload_meta.json                                  │
│ 6. Return success/error                                     │
└─────────────────────────────────────────────────────────────┘
```

---

### 5.4 upload_skill - Cloud Upload Tool

**Purpose:** Upload a local skill to the OpenSpace cloud platform.

**Input Schema:**
```json
{
  "type": "object",
  "properties": {
    "skill_dir": {
      "type": "string",
      "description": "Path to the skill directory"
    },
    "visibility": {
      "type": "string",
      "enum": ["public", "private"],
      "default": "public",
      "description": "Cloud visibility"
    },
    "origin": {
      "type": "string",
      "enum": ["imported", "derived", "fixed", "captured"],
      "description": "Skill origin type"
    },
    "tags": {
      "type": "array",
      "items": {"type": "string"},
      "description": "Skill tags"
    },
    "change_summary": {
      "type": "string",
      "description": "Summary of changes (for derived/fixed)"
    }
  },
  "required": ["skill_dir"]
}
```

**Handler Implementation:**
```python
@mcp.tool()
async def upload_skill(
    skill_dir: str,
    visibility: str = "public",
    origin: Optional[str] = None,
    tags: Optional[List[str]] = None,
    change_summary: Optional[str] = None,
) -> dict:
    """
    Upload a skill to the cloud community.
    
    Full upload workflow:
    1. Read metadata from .upload_meta.json (if exists)
    2. Stage artifact (upload all files)
    3. Compute diff vs parent skill (if derived/fixed)
    4. Create cloud record
    5. Return skill_id
    
    Args:
        skill_dir: Path to skill directory
        visibility: "public" or "private"
        origin: Origin type (auto-detected if not provided)
        tags: Optional tags
        change_summary: Change description
    
    Returns:
        dict with status and skill_id
    """
    client = _get_cloud_client()
    if not client:
        return {
            "status": "error",
            "message": "OPENSPACE_API_KEY not configured",
        }
    
    skill_path = Path(skill_dir)
    if not skill_path.exists():
        return {
            "status": "error",
            "message": f"Skill directory not found: {skill_dir}",
        }
    
    # Read metadata from .upload_meta.json or use defaults
    metadata = _read_upload_meta(skill_path)
    
    # Merge with provided values
    if origin:
        metadata["origin"] = origin
    if tags:
        metadata["tags"] = tags
    if change_summary:
        metadata["change_summary"] = change_summary
    
    # Validate origin/parent relationships
    try:
        client.validate_origin_parents(
            origin=metadata.get("origin", "imported"),
            parents=metadata.get("parent_skill_ids", []),
        )
    except CloudError as e:
        return {
            "status": "error",
            "message": str(e),
        }
    
    # Execute upload
    try:
        skill_id = await client.upload_skill(
            skill_dir=skill_path,
            visibility=visibility,
            metadata=metadata,
        )
        
        return {
            "status": "success",
            "skill_id": skill_id,
            "cloud_url": f"https://open-space.cloud/skills/{skill_id}",
        }
    except CloudError as e:
        return {
            "status": "error",
            "message": str(e),
        }


def _read_upload_meta(skill_dir: Path) -> dict:
    """Read .upload_meta.json if it exists."""
    meta_file = skill_dir / ".upload_meta.json"
    if meta_file.exists():
        return json.loads(meta_file.read_text())
    return {}
```

**Upload Flow:**
```
┌─────────────────────────────────────────────────────────────┐
│ upload_skill Flow                                           │
├─────────────────────────────────────────────────────────────┤
│ 1. Read .upload_meta.json (if exists)                       │
│    - origin: "fix", "derived", "captured", or "imported"    │
│    - parent_skill_ids: [parent ids]                         │
│    - change_summary: "What changed"                         │
│                                                             │
│ 2. Stage artifact                                           │
│    - Upload all files to cloud storage                      │
│    - Get artifact_id                                        │
│                                                             │
│ 3. Compute diff (if derived/fixed)                          │
│    - Fetch parent skill content                             │
│    - Generate unified diff                                  │
│                                                             │
│ 4. Create record                                            │
│    - POST /records with:                                    │
│      - artifact_id                                          │
│      - skill_id                                             │
│      - metadata (visibility, origin, tags, etc.)            │
│      - content_diff (optional)                              │
│                                                             │
│ 5. Return record_id                                         │
└─────────────────────────────────────────────────────────────┘
```

---

## 6. Host Agent Integration

### 6.1 Integration Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      Host Agent                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  MCP Client                                              │   │
│  │  - Manages MCP server connections                        │   │
│  │  - Tool discovery                                        │   │
│  │  - Tool invocation                                       │   │
│  └──────────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Agent Skills                                            │   │
│  │  - delegate-task/SKILL.md                                │   │
│  │  - skill-discovery/SKILL.md                              │   │
│  │  (Teaches agent when/how to use OpenSpace)               │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                            │
                            │ MCP (stdio)
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                    OpenSpace MCP Server                          │
└─────────────────────────────────────────────────────────────────┘
```

### 6.2 Claude Code Integration

**CLAUDE.md Configuration:**
```markdown
# ~/.claude/projects/-home-user-MyProject/CLAUDE.md

## MCP Servers

{
  "mcpServers": {
    "openspace": {
      "command": "openspace-mcp",
      "toolTimeout": 600,
      "env": {
        "OPENSPACE_HOST_SKILL_DIRS": "/Users/user/.claude/projects/-home-user-MyProject/skills",
        "OPENSPACE_WORKSPACE": "/Users/user/Projects/OpenSpace",
        "OPENSPACE_MODEL": "openrouter/anthropic/claude-sonnet-4.5",
        "OPENSPACE_MAX_ITERATIONS": "20"
      }
    }
  }
}
```

**Host Skills Setup:**
```bash
# Copy OpenSpace host skills to agent skills directory
mkdir -p ~/.claude/projects/-home-user-MyProject/skills

cp -r OpenSpace/openspace/host_skills/delegate-task/ \
    ~/.claude/projects/-home-user-MyProject/skills/

cp -r OpenSpace/openspace/host_skills/skill-discovery/ \
    ~/.claude/projects/-home-user-MyProject/skills/
```

**delegate-task/SKILL.md:**
```markdown
---
name: delegate-task
description: When to delegate tasks to OpenSpace
---

# Delegate Task to OpenSpace

## When to Use

- You need to execute a complex task that requires multiple tool calls
- The task involves system administration, development workflows, or monitoring
- You want to leverage pre-existing skills for the task
- The task would benefit from OpenSpace's skill evolution

## Core Technique

Use `execute_task` to delegate the entire task to OpenSpace:

```
execute_task(task="Task description here")
```

## Examples

**Good:**
- "Monitor my Docker containers and restart the one using most memory"
- "Set up a monitoring dashboard for my PostgreSQL database"
- "Analyze the last 100 API requests and find slow endpoints"

**Bad (do yourself):**
- Simple file operations you can do directly
- Single shell commands
- Questions that don't require action

## Decision Tree

1. Does the task require multiple steps?
   - No → Do it yourself
   - Yes → Continue

2. Would skills help with this task?
   - No → Do it yourself
   - Yes → Use `execute_task`

3. Is the task clearly described?
   - No → Clarify with user first
   - Yes → Use `execute_task`
```

**skill-discovery/SKILL.md:**
```markdown
---
name: skill-discovery
description: How to discover and use skills via OpenSpace
---

# Skill Discovery

## When to Use

- Before delegating a task, check if relevant skills exist
- After a task fails, search for a skill that might help
- When the user asks about available capabilities

## Core Technique

Use `search_skills` to find relevant skills:

```
search_skills(query="docker monitoring", source="all", limit=10)
```

## Search Parameters

- `query`: What you're looking for
- `source`: "local", "cloud", or "all"
- `limit`: Max results (default 20)
- `auto_import`: Auto-import top cloud skill (default true)

## Response Format

```json
{
  "source": "local",
  "skill_id": "docker-monitor",
  "name": "Docker Monitor",
  "description": "Monitor container health and metrics",
  "skill_dir": "/path/to/skills/docker-monitor",
  "applied_rate": 0.85,
  "completion_rate": 0.92
}
```

## Workflow

1. User describes task
2. You search for skills: `search_skills(query=task_description)`
3. Review results:
   - If good skills found → `execute_task(task=...)`
   - If no skills → Do it yourself with available tools
4. After execution, check if skills evolved
```

---

### 6.3 OpenClaw Integration

OpenClaw uses the same MCP configuration pattern:

**~/.openclaw/config.json:**
```json
{
  "mcp": {
    "servers": {
      "openspace": {
        "command": "openspace-mcp",
        "args": [],
        "env": {
          "OPENSPACE_HOST_SKILL_DIRS": "~/.openclaw/skills",
          "OPENSPACE_WORKSPACE": "~/Projects/OpenSpace"
        },
        "timeout": 600
      }
    }
  }
}
```

**OpenClaw Skill (skills/openspace-delegate.md):**
```markdown
# OpenSpace Delegation

## Purpose

Delegate complex multi-step tasks to OpenSpace.

## Usage

```
mcp_call(
  server="openspace",
  tool="execute_task",
  args={"task": "Monitor Docker and restart highest memory container"}
)
```

## When to Delegate

1. Task requires 3+ tool calls
2. Task matches an existing skill pattern
3. User explicitly requests OpenSpace

## When NOT to Delegate

1. Simple operations (< 3 steps)
2. Tasks requiring user confirmation at each step
3. Tasks outside OpenSpace's backend scope
```

---

### 6.4 nanobot Integration

nanobot automatically detects and integrates with OpenSpace:

**~/.nanobot/mcp.json:**
```json
{
  "servers": [
    {
      "name": "openspace",
      "type": "stdio",
      "command": "openspace-mcp",
      "env": {
        "OPENSPACE_HOST_SKILL_DIRS": "~/.nanobot/skills",
        "OPENSPACE_WORKSPACE": "~/Projects/OpenSpace",
        "OPENSPACE_API_KEY": "$NANOBOT_OPENSPACE_KEY"
      }
    }
  ]
}
```

**nanobot Auto-Detection:**

nanobot can auto-detect OpenSpace if:
- `openspace-mcp` is in PATH
- `OPENSPACE_WORKSPACE` environment variable is set
- Skill directory exists

```python
# nanobot auto-detection logic
def detect_openspace() -> Optional[MCPConfig]:
    """Auto-detect OpenSpace installation."""
    import shutil
    
    # Check if openspace-mcp is available
    if not shutil.which("openspace-mcp"):
        return None
    
    # Check for workspace
    workspace = os.getenv("OPENSPACE_WORKSPACE")
    if not workspace:
        return None
    
    # Check for skills directory
    skills_dir = Path(workspace) / "openspace" / "skills"
    if not skills_dir.exists():
        return None
    
    return {
        "name": "openspace",
        "command": "openspace-mcp",
        "env": {
            "OPENSPACE_HOST_SKILL_DIRS": str(skills_dir),
            "OPENSPACE_WORKSPACE": workspace,
        }
    }
```

---

### 6.5 Codex Integration

Codex CLI configuration:

**~/.codex/config.toml:**
```toml
[mcp.servers.openspace]
command = "openspace-mcp"
timeout = 600

[mcp.servers.openspace.env]
OPENSPACE_HOST_SKILL_DIRS = "~/.codex/skills"
OPENSPACE_WORKSPACE = "~/Projects/OpenSpace"
OPENSPACE_MODEL = "openrouter/anthropic/claude-sonnet-4.5"
```

**Codex Skill (skills/openpace.md):**
```markdown
---
name: openspace-delegation
description: Delegate tasks to OpenSpace MCP server
---

# OpenSpace Delegation

## Tools Available

- `execute_task` - Main task execution
- `search_skills` - Find relevant skills
- `fix_skill` - Repair broken skills
- `upload_skill` - Upload to cloud

## Usage Pattern

1. First, search for skills: `search_skills(query="...")`
2. If skills found, delegate: `execute_task(task="...")`
3. Monitor execution results
4. Check for evolved skills

## Example Session

User: "Help me monitor my services"

You:
1. Search: `search_skills(query="service monitoring")`
2. Found: ["service-monitor", "health-check"]
3. Execute: `execute_task(task="Monitor all services and report unhealthy ones")`
4. Report results to user
```

---

### 6.6 Cursor Integration

Cursor MCP configuration:

**Settings → Features → MCP Servers:**
```json
{
  "mcpServers": {
    "openspace": {
      "command": "python",
      "args": ["-m", "openspace.mcp_server"],
      "env": {
        "OPENSPACE_HOST_SKILL_DIRS": "~/cursor-skills",
        "OPENSPACE_WORKSPACE": "~/Projects/OpenSpace"
      }
    }
  }
}
```

**Cursor Rules (.cursor/rules/openspace.mdc):**
```markdown
# OpenSpace Integration

## When to Use OpenSpace

Use OpenSpace's `execute_task` when:
- The task is complex (requires 3+ steps)
- You have relevant skills available
- The user wants automated execution

## Skill Discovery

Always check for relevant skills before tackling complex tasks:

1. `search_skills(query="<task_description>")`
2. Review results
3. If skills found → `execute_task(task="<task>")`
4. If no skills → handle with standard tools

## Error Handling

If `execute_task` fails:
1. Check error message
2. Consider `fix_skill` if it's a skill issue
3. Fall back to manual execution if needed
```

---

## 7. Host Skills System

### 7.1 Purpose

Host skills are meta-skills that teach agent clients **when** and **how** to use OpenSpace. They encode:

- Decision criteria for delegation
- Tool invocation patterns
- Error handling strategies
- Integration workflows

### 7.2 delegate-task/SKILL.md

**Location:** `openspace/host_skills/delegate-task/SKILL.md`

```markdown
---
name: delegate-task
description: Decision framework for delegating tasks to OpenSpace
---

# Delegate Task to OpenSpace

## When to Use

Use this skill when ALL of the following are true:

1. **Complexity**: Task requires 3+ distinct steps or tool calls
2. **Skill Match**: A relevant skill exists (check via search_skills)
3. **Autonomy**: User wants automated execution without step-by-step confirmation
4. **Scope**: Task is within OpenSpace's backend capabilities (shell, gui, mcp, web)

## Decision Tree

```
Task Requested
     │
     ▼
┌─────────────────┐
│ Is it complex?  │──No──→ Do it yourself
│ (3+ steps)      │
└────────┬────────┘
         │ Yes
         ▼
┌─────────────────┐
│ Search skills   │
│ search_skills() │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Skills found?   │──No──→ Do it yourself
└────────┬────────┘
         │ Yes
         ▼
┌─────────────────┐
│ Within scope?   │──No──→ Do it yourself
│ (shell/gui/mcp) │        or explain limitation
└────────┬────────┘
         │ Yes
         ▼
┌─────────────────┐
│ User wants      │──No──→ Explain plan, get confirmation
│ automation?     │
└────────┬────────┘
         │ Yes
         ▼
    execute_task()
```

## Tool Invocation

```python
# Pattern 1: Direct delegation
result = execute_task(
    task="Monitor Docker containers and restart unhealthy ones",
    search_scope="all",
    max_iterations=20,
)

# Pattern 2: Search first, then decide
skills = search_skills(query="docker monitoring", limit=5)
if skills:
    result = execute_task(task="...")
else:
    # Handle manually
    pass
```

## Response Handling

```python
if result["status"] == "success":
    # Report successful completion
    return result["response"]

elif result["status"] == "error":
    # Check if it's a skill issue (can be fixed)
    if "skill" in result.get("error_type", "").lower():
        # Offer to fix the skill
        return f"Skill issue: {result['message']}. Fix with fix_skill?"
    else:
        # Retry or fall back
        return f"Task failed: {result['message']}"

elif result["status"] == "max_iterations_reached":
    # Task may be incomplete
    return f"Task incomplete: {result['response']}"
```

## Anti-Patterns

**DON'T delegate when:**
- Task is a single shell command
- User wants to understand each step
- Task requires credentials you don't have
- Task is outside OpenSpace's scope (e.g., GUI on headless server)

**DO delegate when:**
- Task matches an existing skill pattern
- User wants "just get it done" automation
- Task requires iterative tool calling
- You want to benefit from skill evolution
```

---

### 7.3 skill-discovery/SKILL.md

**Location:** `openspace/host_skills/skill-discovery/SKILL.md`

```markdown
---
name: skill-discovery
description: How to discover and import skills for OpenSpace
---

# Skill Discovery

## Purpose

Find and import skills that match your current task needs.

## Search Pipeline

OpenSpace uses hybrid search:

```
Query ──┬──> BM25 Prefilter ──> Candidates ──┬──> Embedding Rank ──> Top 20 ──> LLM Select ──> Final
        │                                    │
        └──> (Fast, recall)                  └──> (Semantic matching)     └──> (Precision)
```

## Tool Usage

### search_skills

```python
results = search_skills(
    query="docker container monitoring memory",
    source="all",      # "local", "cloud", or "all"
    limit=20,          # Max results
    auto_import=True,  # Auto-import top cloud skill
)
```

### Response Format

```json
[
  {
    "source": "local",
    "skill_id": "docker-monitor-abc123",
    "name": "Docker Monitor",
    "description": "Monitor Docker container health and metrics",
    "skill_dir": "/path/to/skills/docker-monitor",
    "applied_rate": 0.85,
    "completion_rate": 0.92
  },
  {
    "source": "cloud",
    "skill_id": "k8s-health-xyz789",
    "record_id": "k8s-health__clo_abcd1234",
    "name": "Kubernetes Health Check",
    "description": "Check Kubernetes pod health and restart failed pods",
    "visibility": "public",
    "tags": ["kubernetes", "monitoring", "auto-heal"],
    "_cloud_metadata": {
      "artifact_id": "artifact_123",
      "level": "workflow"
    }
  }
]
```

## Auto-Import

When `auto_import=True` and a cloud skill is found:

1. Skill files downloaded to `OPENSPACE_HOST_SKILL_DIRS/skills/`
2. Skill registered with local `SkillRegistry`
3. Metrics initialized in `SkillStore`
4. Skill available for immediate use

## Manual Import

For manual import of cloud skills:

```python
client = _get_cloud_client()
result = client.import_skill(
    skill_id="skill-id-abc",
    record_id="record-id-xyz",
    base_dir=Path.home() / ".claude" / "skills",
)
```

## Best Practices

1. **Search before delegating**: Always check for relevant skills first
2. **Review top results**: Check applied_rate and completion_rate
3. **Cloud vs Local**: Prefer local skills (faster, no API needed), use cloud for gaps
4. **Iterative refinement**: If first search fails, try different query terms
```

---

### 7.4 Skill Format for Agents

Host skills follow the standard SKILL.md format:

```markdown
---
name: skill-name
description: One-line description
---

# Skill Name

## When to Use

- Condition 1
- Condition 2
- Condition 3

## Core Technique

Main approach explained.

## Step-by-Step Workflow

### Step 1: First Action

```
tool_call(param="value")
```

### Step 2: Second Action

```
another_tool(arg="value")
```

## Complete Example

Full working example with expected output.

## Troubleshooting

Common issues and solutions.

## Related Skills

- [Related Skill 1](link)
- [Related Skill 2](link)
```

---

### 7.5 Teaching Agents to Use OpenSpace

**Training Workflow:**

1. **Install Host Skills**: Copy `delegate-task/` and `skill-discovery/` to agent's skills directory

2. **Prime the Agent**: In first conversation, demonstrate:
   ```
   User: "Monitor my Docker containers"
   Agent: (uses search_skills, finds docker-monitor)
   Agent: (uses execute_task, task completes)
   ```

3. **Reinforce Pattern**: Agent learns from conversation history that:
   - Complex tasks → search_skills → execute_task
   - Simple tasks → direct action

4. **Evolution**: Over time, host skills themselves can evolve:
   ```
   fix_skill(
       skill_dir="/path/to/delegate-task",
       direction="Add guidance for handling rate-limited APIs"
   )
   ```

---

## 8. Host Detection

### 8.1 Auto-Detection Overview

OpenSpace can auto-detect host agent configurations to simplify setup:

```
┌─────────────────────────────────────────────────────────────┐
│                  Host Detection Flow                         │
├─────────────────────────────────────────────────────────────┤
│ 1. Detect agent type (Claude Code, OpenClaw, nanobot, etc.) │
│ 2. Locate credentials directory                             │
│ 3. Read API keys and configuration                          │
│ 4. Configure OpenSpace MCP accordingly                      │
└─────────────────────────────────────────────────────────────┘
```

### 8.2 nanobot Credential Detection

nanobot stores credentials in standard locations:

**Detection Logic:**
```python
def detect_nanobot_credentials() -> Optional[Dict[str, str]]:
    """Auto-detect nanobot API credentials."""
    
    # Common credential locations
    candidates = [
        Path.home() / ".nanobot" / "credentials.json",
        Path.home() / ".config" / "nanobot" / "credentials.json",
        Path.home() / ".nanobot" / "config.json",
    ]
    
    for cred_path in candidates:
        if cred_path.exists():
            try:
                config = json.loads(cred_path.read_text())
                
                # Extract API keys
                credentials = {}
                if "api_keys" in config:
                    credentials.update(config["api_keys"])
                
                # Extract OpenSpace config if present
                if "openspace" in config:
                    os_config = config["openspace"]
                    if "api_key" in os_config:
                        credentials["OPENSPACE_API_KEY"] = os_config["api_key"]
                    if "workspace" in os_config:
                        credentials["OPENSPACE_WORKSPACE"] = os_config["workspace"]
                
                return credentials
                
            except (json.JSONDecodeError, KeyError) as e:
                logger.warning(f"Failed to parse nanobot credentials: {e}")
    
    return None
```

**Credential Format (nanobot/credentials.json):**
```json
{
  "api_keys": {
    "openrouter": "sk-or-xxx",
    "anthropic": "sk-ant-xxx",
    "openai": "sk-xxx"
  },
  "openspace": {
    "api_key": "sk-xxx",
    "workspace": "/path/to/OpenSpace"
  }
}
```

---

### 8.3 OpenClaw Credential Detection

OpenClaw uses a similar pattern:

**Detection Logic:**
```python
def detect_openclaw_credentials() -> Optional[Dict[str, str]]:
    """Auto-detect OpenClaw API credentials."""
    
    candidates = [
        Path.home() / ".openclaw" / "credentials.json",
        Path.home() / ".openclaw" / "config.json",
        Path.home() / ".config" / "openclaw" / "credentials.json",
    ]
    
    for cred_path in candidates:
        if cred_path.exists():
            config = json.loads(cred_path.read_text())
            
            credentials = {}
            
            # API keys at root level
            for key in ["openrouter_api_key", "anthropic_api_key", "openai_api_key"]:
                if key in config:
                    credentials[key.upper()] = config[key]
            
            # OpenSpace-specific config
            if "mcp" in config and "openspace" in config["mcp"]:
                os_config = config["mcp"]["openspace"]
                if "env" in os_config:
                    for env_key, env_val in os_config["env"].items():
                        credentials[env_key] = env_val
            
            return credentials
    
    return None
```

**Credential Format (openclaw/config.json):**
```json
{
  "openrouter_api_key": "sk-or-xxx",
  "anthropic_api_key": "sk-ant-xxx",
  "mcp": {
    "openspace": {
      "command": "openspace-mcp",
      "env": {
        "OPENSPACE_API_KEY": "sk-xxx",
        "OPENSPACE_WORKSPACE": "/path/to/OpenSpace"
      }
    }
  }
}
```

---

### 8.4 Claude Code Credential Detection

Claude Code stores configuration in the `.claude` directory:

**Detection Logic:**
```python
def detect_claude_code_credentials() -> Optional[Dict[str, str]]:
    """Auto-detect Claude Code API credentials."""
    
    # Claude Code config is in ~/.claude/settings.json or project-specific
    candidates = [
        Path.home() / ".claude" / "settings.json",
        Path.home() / ".claude" / "config.json",
    ]
    
    # Also check for project-specific configs
    cwd = Path.cwd()
    for parent in [cwd] + list(cwd.parents):
        claude_dir = parent / ".claude"
        if claude_dir.exists():
            candidates.append(claude_dir / "settings.json")
            candidates.append(claude_dir / "config.json")
    
    for cred_path in candidates:
        if cred_path.exists():
            config = json.loads(cred_path.read_text())
            
            credentials = {}
            
            # MCP server configs
            if "mcpServers" in config:
                for server_name, server_config in config["mcpServers"].items():
                    if server_name == "openspace" and "env" in server_config:
                        for env_key, env_val in server_config["env"].items():
                            credentials[env_key] = env_val
            
            return credentials
    
    return None
```

---

### 8.5 Credential Resolution Order

When multiple credential sources exist, OpenSpace resolves in this order:

```
1. Environment variables (highest priority)
   - OPENSPACE_API_KEY
   - OPENROUTER_API_KEY
   - ANTHROPIC_API_KEY
   - etc.

2. Host agent credentials (if detected)
   - nanobot credentials
   - OpenClaw credentials
   - Claude Code credentials

3. OpenSpace .env file
   - ~/.openspace/.env
   - ./openspace/.env

4. System keyring (if available)
   - macOS Keychain
   - Windows Credential Manager
   - Linux Secret Service
```

**Resolution Implementation:**
```python
def resolve_credentials() -> Dict[str, str]:
    """Resolve credentials from all sources."""
    credentials = {}
    
    # 1. Environment variables (always highest priority)
    for key in ["OPENSPACE_API_KEY", "OPENROUTER_API_KEY", "ANTHROPIC_API_KEY"]:
        if os.getenv(key):
            credentials[key] = os.getenv(key)
    
    # 2. Host agent detection
    host_creds = detect_host_credentials()
    for key, val in host_creds.items():
        if key not in credentials:  # Don't override env vars
            credentials[key] = val
    
    # 3. .env file
    env_file = find_env_file()
    if env_file and env_file.exists():
        env_vars = parse_env_file(env_file)
        for key, val in env_vars.items():
            if key not in credentials:
                credentials[key] = val
    
    # 4. System keyring (fallback)
    if not credentials.get("OPENSPACE_API_KEY"):
        try:
            import keyring
            credentials["OPENSPACE_API_KEY"] = keyring.get_password("openspace", "api_key")
        except ImportError:
            pass
    
    return credentials


def detect_host_credentials() -> Dict[str, str]:
    """Try all host detection methods."""
    credentials = {}
    
    # Try each host type
    for detector in [
        detect_nanobot_credentials,
        detect_openclaw_credentials,
        detect_claude_code_credentials,
    ]:
        try:
            creds = detector()
            if creds:
                credentials.update(creds)
        except Exception as e:
            logger.debug(f"Host detection failed: {detector.__name__}: {e}")
    
    return credentials
```

---

### 8.6 Config Loading

**Configuration Hierarchy:**

```
┌─────────────────────────────────────────────────────────────┐
│              Configuration Loading Order                     │
├─────────────────────────────────────────────────────────────┤
│ 1. Command-line arguments (highest priority)                │
│ 2. Environment variables                                    │
│ 3. Host agent MCP config (detected)                         │
│ 4. OpenSpace config files                                   │
│    - ~/.openspace/config.json                               │
│    - ./openspace/config/config_grounding.json               │
│ 5. Built-in defaults (lowest priority)                      │
└─────────────────────────────────────────────────────────────┘
```

**Config Loading Implementation:**
```python
def load_config() -> OpenSpaceConfig:
    """Load configuration from all sources."""
    config = {}
    
    # 1. Command-line arguments
    args = parse_cli_args()
    config.update(vars(args))
    
    # 2. Environment variables
    env_mapping = {
        "OPENSPACE_MODEL": "llm_model",
        "OPENSPACE_WORKSPACE": "workspace_dir",
        "OPENSPACE_MAX_ITERATIONS": "grounding_max_iterations",
        "OPENSPACE_ENABLE_RECORDING": "enable_recording",
        "OPENSPACE_BACKEND_SCOPE": "backend_scope",
    }
    for env_key, config_key in env_mapping.items():
        if os.getenv(env_key):
            config[config_key] = os.getenv(env_key)
    
    # 3. Host agent config (if detected)
    host_config = detect_host_config()
    config.update(host_config)
    
    # 4. Config files
    file_config = load_config_files()
    config.update(file_config)
    
    # 5. Apply defaults for missing values
    defaults = {
        "llm_model": "openrouter/anthropic/claude-sonnet-4.5",
        "grounding_max_iterations": 20,
        "enable_recording": True,
        "backend_scope": ["shell", "gui", "mcp", "web"],
    }
    for key, val in defaults.items():
        if key not in config:
            config[key] = val
    
    return OpenSpaceConfig(**config)
```

---

## 9. MCP Safe Stdout

### 9.1 Windows Deadlock Prevention

On Windows, improper stdout handling can cause deadlocks when:
- MCP server writes to stdout
- Host agent is not reading
- Pipe buffer fills up
- Process blocks on write

**Symptoms:**
- MCP server hangs on startup
- Tool calls timeout without response
- Process must be killed manually

**Root Cause:**
Windows pipes have smaller default buffers (typically 4KB) compared to Unix (64KB). When stderr fills up and the process blocks, it can prevent stdout writes from completing.

---

### 9.2 Stdout/Stderr Handling

**Safe Pattern:**
```python
import sys
import os
from pathlib import Path

def setup_safe_stdio(debug: bool = False) -> None:
    """
    Set up safe stdio handling to prevent deadlocks.
    
    On Windows:
    - Redirect stderr to file to prevent pipe buffer overflow
    - Keep stdout for JSON-RPC only
    
    On Unix:
    - Standard stdio is typically safe
    - Still redirect stderr for cleaner output
    """
    
    if os.name == "nt":  # Windows
        # Create log directory
        log_dir = Path.home() / ".openspace" / "logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        
        # Generate log file name with timestamp
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        log_file = log_dir / f"mcp_server_{timestamp}.log"
        
        # Redirect stderr to file
        sys.stderr = open(log_file, "a", encoding="utf-8")
        
        # Set up logging
        logging.basicConfig(
            filename=log_file,
            level=logging.DEBUG if debug else logging.INFO,
            format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
        )
        
        # Ensure stdout is line-buffered for JSON-RPC
        sys.stdout = os.fdopen(sys.stdout.fileno(), "w", buffering=1)
        
    else:  # Unix-like
        # Still redirect stderr for cleaner JSON-RPC
        log_dir = Path.home() / ".openspace" / "logs"
        log_dir.mkdir(parents=True, exist_ok=True)
        
        log_file = log_dir / "mcp_server.log"
        sys.stderr = open(log_file, "a", encoding="utf-8")
        
        logging.basicConfig(
            stream=sys.stderr,
            level=logging.DEBUG if debug else logging.INFO,
            format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
        )
```

---

### 9.3 Log File Redirection

**Implementation:**
```python
class SafeStdoutManager:
    """Manage safe stdout/stderr for MCP servers."""
    
    def __init__(self, log_dir: Optional[Path] = None, debug: bool = False):
        self.log_dir = log_dir or (Path.home() / ".openspace" / "logs")
        self.debug = debug
        self._original_stderr = None
        self._log_file = None
    
    def __enter__(self):
        # Create log directory
        self.log_dir.mkdir(parents=True, exist_ok=True)
        
        # Save original stderr
        self._original_stderr = sys.stderr
        
        # Create log file
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        self._log_file = self.log_dir / f"mcp_{timestamp}.log"
        
        # Open file handle
        self._file_handle = open(self._log_file, "a", encoding="utf-8")
        
        # Redirect stderr
        sys.stderr = self._file_handle
        
        # Set up logging
        if self.debug:
            logging.basicConfig(
                stream=self._file_handle,
                level=logging.DEBUG,
                format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
            )
        else:
            logging.basicConfig(
                filename=self._log_file,
                level=logging.INFO,
                format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
            )
        
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        # Restore stderr
        sys.stderr = self._original_stderr
        
        # Close file handle
        if self._file_handle:
            self._file_handle.close()
    
    def log(self, message: str) -> None:
        """Log a message to stderr (redirected to file)."""
        print(message, file=sys.stderr, flush=True)
```

**Usage:**
```python
if __name__ == "__main__":
    with SafeStdoutManager(debug=os.getenv("OPENSPACE_DEBUG")):
        # MCP server runs here with safe stdio
        mcp.run(transport="stdio")
```

---

### 9.4 Binary Buffer Handling

When dealing with binary data (e.g., screenshots, recordings):

```python
def handle_binary_data(data: bytes) -> str:
    """
    Convert binary data to base64 for JSON-RPC transmission.
    
    MCP text content must be valid UTF-8 strings.
    Binary data (images, etc.) should be base64-encoded.
    """
    import base64
    return base64.b64encode(data).decode("ascii")


def decode_binary_data(encoded: str) -> bytes:
    """Decode base64-encoded binary data."""
    import base64
    return base64.b64decode(encoded)
```

**Example - Screenshot Result:**
```python
@mcp.tool()
async def capture_screen() -> dict:
    """Capture screenshot and return as base64."""
    import pyautogui
    
    # Capture screenshot
    screenshot = pyautogui.screenshot()
    
    # Convert to bytes
    from io import BytesIO
    buffer = BytesIO()
    screenshot.save(buffer, format="PNG")
    image_bytes = buffer.getvalue()
    
    # Encode for JSON-RPC
    image_base64 = handle_binary_data(image_bytes)
    
    return {
        "status": "success",
        "image_type": "image/png",
        "image_data": image_base64,
    }
```

---

## 10. Configuration Reference

### 10.1 MCP Config JSON

**Full Configuration Example:**
```json
{
  "mcpServers": {
    "openspace": {
      "command": "openspace-mcp",
      "args": ["--debug"],
      "toolTimeout": 600,
      "env": {
        "OPENSPACE_HOST_SKILL_DIRS": "/Users/user/.claude/projects/-MyProject/skills",
        "OPENSPACE_WORKSPACE": "/Users/user/Projects/OpenSpace",
        "OPENSPACE_MODEL": "openrouter/anthropic/claude-sonnet-4.5",
        "OPENSPACE_MAX_ITERATIONS": "20",
        "OPENSPACE_ENABLE_RECORDING": "true",
        "OPENSPACE_BACKEND_SCOPE": "all",
        "OPENSPACE_API_KEY": "sk_xxxxxxxxxxxxxxxx",
        "OPENROUTER_API_KEY": "sk-or_yyyyyyyyyyyyyyyy",
        "ANTHROPIC_API_KEY": "sk-ant_zzzzzzzzzzzzzzzzzz"
      }
    }
  }
}
```

### 10.2 Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `OPENSPACE_API_KEY` | OpenSpace Cloud API key | (none) | No |
| `OPENSPACE_HOST_SKILL_DIRS` | Local skill directories | (none) | No |
| `OPENSPACE_WORKSPACE` | Workspace root directory | (cwd) | No |
| `OPENSPACE_MODEL` | LLM model identifier | `openrouter/anthropic/claude-sonnet-4.5` | No |
| `OPENSPACE_MAX_ITERATIONS` | Max agent iterations | `20` | No |
| `OPENSPACE_ENABLE_RECORDING` | Enable screen recording | `true` | No |
| `OPENSPACE_BACKEND_SCOPE` | Backend scope | `all` | No |
| `OPENROUTER_API_KEY` | OpenRouter API key | (none) | Yes* |
| `ANTHROPIC_API_KEY` | Anthropic API key | (none) | Yes* |
| `OPENAI_API_KEY` | OpenAI API key | (none) | No |

*At least one LLM API key is required

---

### 10.3 Tool Timeouts

**Timeout Configuration:**
```json
{
  "mcpServers": {
    "openspace": {
      "toolTimeout": 600,
      "initializeTimeout": 30
    }
  }
}
```

**Timeout Values:**
| Setting | Recommended | Maximum | Description |
|---------|-------------|---------|-------------|
| `toolTimeout` | 600s (10 min) | 3600s | Per-tool execution timeout |
| `initializeTimeout` | 30s | 60s | Server initialization timeout |

---

### 10.4 API Key Management

**Best Practices:**

1. **Use environment variables for local development:**
   ```bash
   export OPENSPACE_API_KEY="sk_xxx"
   export OPENROUTER_API_KEY="sk-or_yyy"
   ```

2. **Use secrets manager for production:**
   ```python
   import keyring
   
   def get_api_key(service: str) -> str:
       return keyring.get_password("openspace", service)
   ```

3. **Never commit API keys to version control:**
   ```bash
   # Add to .gitignore
   .env
   **/credentials.json
   **/.claude/settings.json
   ```

4. **Rotate keys periodically:**
   ```bash
   # Check key age
   openspace auth status
   
   # Rotate key
   openspace auth rotate
   ```

---

## 11. Production Integration Patterns

### 11.1 High-Availability Setup

For production deployments:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Load Balancer                                 │
│                           │                                       │
│         ┌─────────────────┼─────────────────┐                    │
│         │                 │                 │                    │
│         ▼                 ▼                 ▼                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
│  │ MCP Server  │  │ MCP Server  │  │ MCP Server  │              │
│  │   (Node 1)  │  │   (Node 2)  │  │   (Node 3)  │              │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
│         │                 │                 │                    │
│         └─────────────────┼─────────────────┘                    │
│                           │                                       │
│                           ▼                                       │
│              ┌────────────────────────┐                          │
│              │   Shared Skill Store   │                          │
│              │   (PostgreSQL/Redis)   │                          │
│              └────────────────────────┘                          │
└─────────────────────────────────────────────────────────────────┘
```

**Configuration:**
```json
{
  "mcpServers": {
    "openspace": {
      "command": "openspace-mcp",
      "args": ["--cluster-mode"],
      "env": {
        "OPENSPACE_STORE_TYPE": "postgresql",
        "OPENSPACE_STORE_URL": "postgresql://user:pass@db-host:5432/openspace",
        "OPENSPACE_REDIS_URL": "redis://redis-host:6379",
        "OPENSPACE_NODE_ID": "node-1"
      }
    }
  }
}
```

---

### 11.2 Multi-Tenant Configuration

For serving multiple agents/tenants:

```python
class MultiTenantOpenSpace:
    """OpenSpace with tenant isolation."""
    
    def __init__(self):
        self._tenants: Dict[str, OpenSpace] = {}
    
    def get_tenant(self, tenant_id: str) -> OpenSpace:
        """Get or create tenant-specific OpenSpace instance."""
        if tenant_id not in self._tenants:
            config = self._load_tenant_config(tenant_id)
            self._tenants[tenant_id] = OpenSpace(config=config)
        return self._tenants[tenant_id]
    
    def _load_tenant_config(self, tenant_id: str) -> OpenSpaceConfig:
        """Load configuration for specific tenant."""
        # Load from database or config file
        tenant_config = load_tenant_config(tenant_id)
        
        return OpenSpaceConfig(
            llm_model=tenant_config["llm_model"],
            workspace_dir=Path(tenant_config["workspace"]),
            # ... other tenant-specific settings
        )


@mcp.tool()
async def execute_task(
    task: str,
    tenant_id: str,
    search_scope: str = "all",
    max_iterations: int = 20,
) -> dict:
    """Execute task for specific tenant."""
    openspace = multi_tenant.get_tenant(tenant_id)
    return await openspace.execute(task, search_scope, max_iterations)
```

---

### 11.3 Monitoring and Observability

**Metrics to Track:**
```python
from prometheus_client import Counter, Histogram, Gauge

# Tool call metrics
TOOL_CALLS = Counter(
    "openspace_tool_calls_total",
    "Total tool calls",
    ["tool_name", "status"]
)

TOOL_LATENCY = Histogram(
    "openspace_tool_latency_seconds",
    "Tool execution latency",
    ["tool_name"],
    buckets=[0.1, 0.5, 1.0, 5.0, 10.0, 30.0, 60.0]
)

# Skill metrics
SKILL_APPLICATIONS = Counter(
    "openspace_skill_applications_total",
    "Total skill applications",
    ["skill_id", "outcome"]
)

# System metrics
ACTIVE_EXECUTIONS = Gauge(
    "openspace_active_executions",
    "Number of active task executions"
)
```

**Health Check Endpoint:**
```python
@mcp.tool()
async def health_check() -> dict:
    """Return server health status."""
    import psutil
    
    return {
        "status": "healthy",
        "uptime_seconds": time.time() - START_TIME,
        "memory_usage_mb": psutil.Process().memory_info().rss / 1024 / 1024,
        "cpu_percent": psutil.Process().cpu_percent(),
        "active_executions": ACTIVE_EXECUTIONS._value.get(),
        "skill_count": len(_openspace._skill_registry._skills) if _openspace else 0,
    }
```

---

### 11.4 Security Hardening

**Security Checklist:**

1. **API Key Isolation:**
   - Never expose API keys in error messages
   - Use separate keys per tenant
   - Rotate keys on compromise

2. **Tool Input Validation:**
   ```python
   from jsonschema import validate, ValidationError
   
   def validate_tool_input(tool_name: str, args: dict) -> None:
       schema = get_tool_schema(tool_name)
       try:
           validate(instance=args, schema=schema)
       except ValidationError as e:
           raise MCPError(f"Invalid input for {tool_name}: {e.message}")
   ```

3. **Rate Limiting:**
   ```python
   from ratelimit import RateLimiter
   
   rate_limiter = RateLimiter(max_calls=100, period=60)
   
   @mcp.tool()
   async def execute_task(task: str, ...) -> dict:
       await rate_limiter.wait()
       # ... rest of implementation
   ```

4. **Audit Logging:**
   ```python
   def log_tool_call(tool_name: str, args: dict, result: dict, tenant_id: str) -> None:
       audit_logger.info({
           "timestamp": datetime.utcnow().isoformat(),
           "tenant_id": tenant_id,
           "tool": tool_name,
           "input_hash": hashlib.sha256(json.dumps(args).encode()).hexdigest(),
           "output_hash": hashlib.sha256(json.dumps(result).encode()).hexdigest(),
           "status": result.get("status", "unknown"),
       })
   ```

---

## Appendix A: Complete mcp_server.py

```python
#!/usr/bin/env python3
"""
OpenSpace MCP Server

Model Context Protocol server for OpenSpace skill execution platform.
Exposes 4 tools: execute_task, search_skills, fix_skill, upload_skill
"""

import asyncio
import json
import logging
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

from mcp.server.fastmcp import FastMCP

from openspace import OpenSpace, OpenSpaceConfig
from openspace.cloud.client import OpenSpaceClient
from openspace.cloud.errors import CloudError
from openspace.skill_engine.evolver import EvolutionContext, EvolutionSuggestion, EvolutionType

# ============================================================================
# Configuration
# ============================================================================

# MCP Server instance
mcp = FastMCP(
    name="openspace",
    instructions="OpenSpace skill execution and evolution platform. Use execute_task to delegate work, search_skills to find relevant skills, fix_skill to repair broken skills, and upload_skill to share skills with the community.",
)

# Global instances (lazy-initialized)
_openspace: Optional[OpenSpace] = None
_cloud_client: Optional[OpenSpaceClient] = None

# ============================================================================
# Initialization
# ============================================================================

async def _get_openspace() -> OpenSpace:
    """Lazy-initialize and return the OpenSpace instance."""
    global _openspace
    
    if _openspace is None:
        config = OpenSpaceConfig(
            llm_model=os.getenv("OPENSPACE_MODEL", "openrouter/anthropic/claude-sonnet-4.5"),
            llm_kwargs={
                "api_key": os.getenv("OPENROUTER_API_KEY") or os.getenv("ANTHROPIC_API_KEY")
            },
            workspace_dir=os.getenv("OPENSPACE_WORKSPACE", str(Path.cwd())),
            grounding_max_iterations=int(os.getenv("OPENSPACE_MAX_ITERATIONS", "20")),
            enable_recording=os.getenv("OPENSPACE_ENABLE_RECORDING", "true").lower() == "true",
            recording_log_dir=os.getenv("OPENSPACE_RECORDING_DIR", "./logs/recordings"),
        )
        _openspace = OpenSpace(config=config)
        await _openspace.__aenter__()
    
    return _openspace


def _get_cloud_client() -> Optional[OpenSpaceClient]:
    """Get the cloud API client (if configured)."""
    global _cloud_client
    
    if _cloud_client is None:
        api_key = os.getenv("OPENSPACE_API_KEY")
        if api_key:
            _cloud_client = OpenSpaceClient(
                auth_headers={"Authorization": f"Bearer {api_key}"},
                api_base=os.getenv("OPENSPACE_API_BASE", "https://api.open-space.cloud"),
            )
    
    return _cloud_client


def has_api_key() -> bool:
    """Check if cloud API key is configured."""
    return bool(os.getenv("OPENSPACE_API_KEY"))


# ============================================================================
# Tool: execute_task
# ============================================================================

@mcp.tool()
async def execute_task(
    task: str,
    search_scope: str = "all",
    max_iterations: int = 20,
) -> dict:
    """
    Execute a task with skill search, execution, and evolution.
    
    This is the primary tool for delegating work to OpenSpace.
    
    Flow:
    1. Search for relevant skills (BM25 + embedding + LLM selection)
    2. Inject skill context into agent prompt
    3. Run grounding agent with iterative tool calling
    4. Analyze execution for evolution opportunities
    5. Evolve skills if needed (FIX/DERIVED/CAPTURED)
    
    Args:
        task: Natural language task description
        search_scope: Where to search for skills ("local", "cloud", or "all")
        max_iterations: Maximum tool-calling iterations (default: 20)
    
    Returns:
        dict with keys:
        - status: "success", "error", or "max_iterations_reached"
        - response: Final response from agent
        - evolved_skills: List of skills that were evolved (if any)
        - metrics: Execution metrics (iterations, duration, tools_called)
    
    Examples:
        execute_task(task="Monitor Docker containers")
        execute_task(task="Deploy to production", search_scope="local")
        execute_task(task="Analyze logs", max_iterations=50)
    """
    start_time = time.time()
    
    try:
        openspace = await _get_openspace()
        
        result = await openspace.execute(
            instruction=task,
            search_scope=search_scope,
            max_iterations=max_iterations,
        )
        
        duration_ms = (time.time() - start_time) * 1000
        
        return {
            "status": result.get("status", "success"),
            "response": result.get("response", ""),
            "evolved_skills": result.get("evolved_skills", []),
            "metrics": {
                "iterations": result.get("iterations", 0),
                "duration_ms": int(duration_ms),
                "tools_called": result.get("tools_used", []),
            }
        }
        
    except Exception as e:
        logging.exception("execute_task failed")
        return {
            "status": "error",
            "error_type": type(e).__name__,
            "message": str(e),
        }


# ============================================================================
# Tool: search_skills
# ============================================================================

@mcp.tool()
async def search_skills(
    query: str,
    source: str = "all",
    limit: int = 20,
    auto_import: bool = True,
) -> list:
    """
    Search for skills matching the query.
    
    Uses hybrid search pipeline:
    1. BM25 keyword prefilter (fast, recall-oriented)
    2. Embedding similarity ranking (semantic matching)
    3. LLM final selection (precision-oriented)
    
    Args:
        query: Search query string
        source: Where to search ("local", "cloud", or "all")
        limit: Maximum results to return (default: 20)
        auto_import: Auto-import top cloud skill if found (default: true)
    
    Returns:
        List of skill matches with metadata:
        - source: "local" or "cloud"
        - skill_id: Unique skill identifier
        - name: Skill name
        - description: Skill description
        - skill_dir: Local path (for local skills)
        - applied_rate: Historical application rate (0.0-1.0)
        - completion_rate: Historical completion rate (0.0-1.0)
    
    Examples:
        search_skills(query="docker monitoring")
        search_skills(query="kubernetes", source="cloud")
        search_skills(query="deploy", limit=5, auto_import=False)
    """
    try:
        openspace = await _get_openspace()
        registry = openspace._skill_registry
        store = openspace._skill_store
        
        results = []
        
        # Local search
        if source in ("local", "all"):
            local_skill_ids = await registry.select_relevant_skills(query, limit=limit)
            for skill_id in local_skill_ids:
                meta = registry.get_skill_meta(skill_id)
                record = store.get_record(skill_id) if store else None
                
                results.append({
                    "source": "local",
                    "skill_id": skill_id,
                    "name": meta.name,
                    "description": meta.description,
                    "skill_dir": str(meta.path),
                    "applied_rate": record.applied_rate if record else 0.0,
                    "completion_rate": record.completion_rate if record else 0.0,
                })
        
        # Cloud search
        if source in ("cloud", "all") and has_api_key():
            client = _get_cloud_client()
            cloud_results = client.search_record_embeddings(query, limit=limit)
            
            for cloud_hit in cloud_results:
                results.append({
                    "source": "cloud",
                    "skill_id": cloud_hit.get("skill_id"),
                    "record_id": cloud_hit.get("record_id"),
                    "name": cloud_hit.get("name"),
                    "description": cloud_hit.get("description"),
                    "visibility": cloud_hit.get("visibility", "public"),
                    "tags": cloud_hit.get("tags", []),
                    "_cloud_metadata": {
                        "artifact_id": cloud_hit.get("artifact_id"),
                        "level": cloud_hit.get("level"),
                    }
                })
            
            # Auto-import top skill
            if auto_import and cloud_results:
                try:
                    await _auto_import_skill(cloud_results[0], registry, store)
                except Exception as e:
                    logging.warning(f"Auto-import failed: {e}")
        
        return results[:limit]
        
    except Exception as e:
        logging.exception("search_skills failed")
        return []


async def _auto_import_skill(
    cloud_hit: dict,
    registry: SkillRegistry,
    store: SkillStore,
) -> None:
    """Auto-import the top cloud skill."""
    client = _get_cloud_client()
    
    host_ws = os.getenv("OPENSPACE_HOST_SKILL_DIRS")
    if not host_ws:
        logging.warning("OPENSPACE_HOST_SKILL_DIRS not set, skipping auto-import")
        return
    
    base_dir = Path(host_ws) / "skills"
    base_dir.mkdir(parents=True, exist_ok=True)
    
    result = await asyncio.to_thread(
        client.import_skill,
        cloud_hit["skill_id"],
        base_dir
    )
    
    skill_dir = Path(result.get("local_path", ""))
    if skill_dir.exists():
        meta = registry.register_skill_dir(skill_dir)
        if meta and store:
            await store.sync_from_registry([meta])
            logging.info(f"Auto-imported skill: {meta.name}")


# ============================================================================
# Tool: fix_skill
# ============================================================================

@mcp.tool()
async def fix_skill(
    skill_dir: str,
    direction: str,
) -> dict:
    """
    Fix a skill with explicit instructions.
    
    Use this when a skill is broken or needs improvement.
    The evolver agent will read the current SKILL.md, analyze
    the issue, and apply edits based on the direction.
    
    Args:
        skill_dir: Path to the skill directory
        direction: Explicit fix instructions
    
    Returns:
        dict with keys:
        - status: "success" or "error"
        - skill_dir: Path to the (possibly updated) skill directory
        - upload_ready: Whether the skill is ready for upload
        - validation_errors: Any validation errors (if failed)
    
    Examples:
        fix_skill(
            skill_dir="/path/to/docker-monitor",
            direction="Add error handling for Docker API rate limits"
        )
    """
    try:
        openspace = await _get_openspace()
        
        result = await openspace.evolver.fix_skill_sync(
            skill_dir=Path(skill_dir),
            fix_direction=direction,
        )
        
        if result.success:
            _write_upload_meta(result.skill_dir, {
                "origin": "fix",
                "change_summary": direction,
            })
            
            return {
                "status": "success",
                "skill_dir": str(result.skill_dir),
                "upload_ready": True,
                "validation_errors": result.validation_errors,
            }
        else:
            return {
                "status": "error",
                "error": result.error_message,
                "skill_dir": str(result.skill_dir),
                "upload_ready": False,
                "validation_errors": result.validation_errors,
            }
            
    except Exception as e:
        logging.exception("fix_skill failed")
        return {
            "status": "error",
            "error_type": type(e).__name__,
            "message": str(e),
        }


def _write_upload_meta(skill_dir: Path, metadata: dict) -> None:
    """Write .upload_meta.json for later cloud upload."""
    meta_file = skill_dir / ".upload_meta.json"
    meta = {}
    if meta_file.exists():
        meta = json.loads(meta_file.read_text())
    meta.update(metadata)
    meta_file.write_text(json.dumps(meta, indent=2))


# ============================================================================
# Tool: upload_skill
# ============================================================================

@mcp.tool()
async def upload_skill(
    skill_dir: str,
    visibility: str = "public",
    origin: Optional[str] = None,
    tags: Optional[List[str]] = None,
    change_summary: Optional[str] = None,
) -> dict:
    """
    Upload a skill to the cloud community.
    
    Full upload workflow:
    1. Read metadata from .upload_meta.json (if exists)
    2. Stage artifact (upload all files)
    3. Compute diff vs parent skill (if derived/fixed)
    4. Create cloud record
    5. Return skill_id
    
    Args:
        skill_dir: Path to skill directory
        visibility: "public" or "private"
        origin: Origin type ("imported", "derived", "fixed", "captured")
        tags: Optional tags for the skill
        change_summary: Summary of changes (for derived/fixed)
    
    Returns:
        dict with keys:
        - status: "success" or "error"
        - skill_id: Cloud skill ID (if successful)
        - cloud_url: URL to view the skill (if successful)
    
    Examples:
        upload_skill(skill_dir="/path/to/skill")
        upload_skill(skill_dir="/path/to/skill", visibility="private")
        upload_skill(
            skill_dir="/path/to/derived-skill",
            origin="derived",
            tags=["enhanced", "v2"],
            change_summary="Added retry logic"
        )
    """
    client = _get_cloud_client()
    if not client:
        return {
            "status": "error",
            "message": "OPENSPACE_API_KEY not configured",
        }
    
    skill_path = Path(skill_dir)
    if not skill_path.exists():
        return {
            "status": "error",
            "message": f"Skill directory not found: {skill_dir}",
        }
    
    # Read metadata
    metadata = _read_upload_meta(skill_path)
    
    # Merge with provided values
    if origin:
        metadata["origin"] = origin
    if tags:
        metadata["tags"] = tags
    if change_summary:
        metadata["change_summary"] = change_summary
    
    # Validate origin/parent relationships
    try:
        client.validate_origin_parents(
            origin=metadata.get("origin", "imported"),
            parents=metadata.get("parent_skill_ids", []),
        )
    except CloudError as e:
        return {
            "status": "error",
            "message": str(e),
        }
    
    # Execute upload
    try:
        skill_id = client.upload_skill(
            skill_dir=skill_path,
            visibility=visibility,
            metadata=metadata,
        )
        
        return {
            "status": "success",
            "skill_id": skill_id,
            "cloud_url": f"https://open-space.cloud/skills/{skill_id}",
        }
    except CloudError as e:
        return {
            "status": "error",
            "message": str(e),
        }


def _read_upload_meta(skill_dir: Path) -> dict:
    """Read .upload_meta.json if it exists."""
    meta_file = skill_dir / ".upload_meta.json"
    if meta_file.exists():
        return json.loads(meta_file.read_text())
    return {}


# ============================================================================
# Entry Point
# ============================================================================

if __name__ == "__main__":
    # Set up safe stdio
    from openspace.utils.safe_stdio import setup_safe_stdio
    setup_safe_stdio(debug=os.getenv("OPENSPACE_DEBUG"))
    
    # Run MCP server
    logging.info("Starting OpenSpace MCP server...")
    mcp.run(transport="stdio")
```

---

## Appendix B: MCP Message Flow Diagrams

### B.1 Tool Discovery Flow

```
┌──────────┐                              ┌───────────┐
│  Agent   │                              │   MCP     │
│          │                              │  Server   │
└────┬─────┘                              └─────┬─────┘
     │                                          │
     │  {"jsonrpc":"2.0","id":1,                │
     │   "method":"initialize"}                 │
     │─────────────────────────────────────────>│
     │                                          │
     │  {"jsonrpc":"2.0","id":1,                │
     │   "result":{                             │
     │     "protocolVersion":"2024-11-05",      │
     │     "capabilities":{"tools":{}}}}        │
     │<─────────────────────────────────────────│
     │                                          │
     │  {"jsonrpc":"2.0","id":2,                │
     │   "method":"tools/list"}                 │
     │─────────────────────────────────────────>│
     │                                          │
     │  (Server builds tool list)               │
     │   - execute_task                         │
     │   - search_skills                        │
     │   - fix_skill                            │
     │   - upload_skill                         │
     │                                          │
     │  {"jsonrpc":"2.0","id":2,                │
     │   "result":{"tools":[                    │
     │     {"name":"execute_task",              │
     │      "description":"...",                │
     │      "inputSchema":{...}},               │
     │     {"name":"search_skills",...},        │
     │     {"name":"fix_skill",...},            │
     │     {"name":"upload_skill",...}          │
     │   ]}}                                    │
     │<─────────────────────────────────────────│
     │                                          │
     │  Tools registered, ready for invocation  │
```

### B.2 Tool Execution Flow

```
┌──────────┐                              ┌───────────┐
│  Agent   │                              │   MCP     │
│          │                              │  Server   │
└────┬─────┘                              └─────┬─────┘
     │                                          │
     │  {"jsonrpc":"2.0","id":3,                │
     │   "method":"tools/call",                 │
     │   "params":{                             │
     │     "name":"execute_task",               │
     │     "arguments":{"task":"..."}}}         │
     │─────────────────────────────────────────>│
     │                                          │
     │                          ┌──────────────┐│
     │                          │ _get_        ││
     │                          │ openspace()  ││
     │                          └──────────────┘│
     │                                          │
     │                          ┌──────────────┐│
     │                          │ openspace.   ││
     │                          │ execute()    ││
     │                          └──────────────┘│
     │                                          │
     │                          ┌──────────────┐│
     │                          │ Skill        ││
     │                          │ Registry     ││
     │                          │ select_      ││
     │                          │ relevant_    ││
     │                          │ skills()     ││
     │                          └──────────────┘│
     │                                          │
     │                          ┌──────────────┐│
     │                          │ Grounding    ││
     │                          │ Agent        ││
     │                          │ process()    ││
     │                          └──────────────┘│
     │                                          │
     │                          ┌──────────────┐│
     │                          │ Skill        ││
     │                          │ Evolver      ││
     │                          │ evolve()     ││
     │                          └──────────────┘│
     │                                          │
     │  {"jsonrpc":"2.0","id":3,                │
     │   "result":{"content":[{"type":"text",   │
     │              "text":"{\"status\":\"ok\"}"}]}}│
     │<─────────────────────────────────────────│
     │                                          │
```

---

## Appendix C: Host Agent Config Examples

### C.1 Claude Code (CLAUDE.md)

```markdown
# ~/.claude/projects/-home-user-MyProject/CLAUDE.md

## MCP Configuration

```json
{
  "mcpServers": {
    "openspace": {
      "command": "openspace-mcp",
      "toolTimeout": 600,
      "env": {
        "OPENSPACE_HOST_SKILL_DIRS": "~/.claude/projects/-home-user-MyProject/skills",
        "OPENSPACE_WORKSPACE": "~/Projects/OpenSpace",
        "OPENSPACE_MODEL": "openrouter/anthropic/claude-sonnet-4.5",
        "OPENSPACE_MAX_ITERATIONS": "20"
      }
    }
  }
}
```
```

### C.2 OpenClaw (config.json)

```json
{
  "mcp": {
    "servers": {
      "openspace": {
        "command": "openspace-mcp",
        "args": [],
        "env": {
          "OPENSPACE_HOST_SKILL_DIRS": "~/.openclaw/skills",
          "OPENSPACE_WORKSPACE": "~/Projects/OpenSpace"
        },
        "timeout": 600
      }
    }
  }
}
```

### C.3 nanobot (mcp.json)

```json
{
  "servers": [
    {
      "name": "openspace",
      "type": "stdio",
      "command": "openspace-mcp",
      "env": {
        "OPENSPACE_HOST_SKILL_DIRS": "~/.nanobot/skills",
        "OPENSPACE_WORKSPACE": "~/Projects/OpenSpace",
        "OPENSPACE_API_KEY": "$NANOBOT_OPENSPACE_KEY"
      }
    }
  ]
}
```

### C.4 Codex (config.toml)

```toml
[mcp.servers.openspace]
command = "openspace-mcp"
timeout = 600

[mcp.servers.openspace.env]
OPENSPACE_HOST_SKILL_DIRS = "~/.codex/skills"
OPENSPACE_WORKSPACE = "~/Projects/OpenSpace"
OPENSPACE_MODEL = "openrouter/anthropic/claude-sonnet-4.5"
```

### C.5 Cursor (Settings JSON)

```json
{
  "mcpServers": {
    "openspace": {
      "command": "python",
      "args": ["-m", "openspace.mcp_server"],
      "env": {
        "OPENSPACE_HOST_SKILL_DIRS": "~/cursor-skills",
        "OPENSPACE_WORKSPACE": "~/Projects/OpenSpace"
      }
    }
  }
}
```

---

## Appendix D: Troubleshooting

### D.1 Common Issues

| Issue | Symptoms | Solution |
|-------|----------|----------|
| Server won't start | MCP connection timeout | Check if `openspace-mcp` is in PATH |
| Tools not appearing | tools/list returns empty | Check skill directory permissions |
| Tool calls timeout | execute_task hangs | Increase toolTimeout, check API keys |
| Deadlock on Windows | Process hangs on startup | Redirect stderr to file (see Section 9) |
| Skills not found | search_skills returns [] | Verify OPENSPACE_HOST_SKILL_DIRS |

### D.2 Debug Mode

Enable debug logging:

```bash
export OPENSPACE_DEBUG=true
export OPENSPACE_LOG_LEVEL=DEBUG
```

Watch logs:

```bash
tail -f ~/.openspace/logs/mcp_*.log
```

### D.3 Health Check

```bash
# Test MCP connection
openspace-mcp --health

# Test skill registry
openspace skills list

# Test cloud connection
openspace cloud status
```

---

*Document generated for OpenSpace MCP Server Integration reference.*
