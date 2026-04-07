# acp_registry/ Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/hermes-agent/acp_registry/`

**Status:** complete

---

## Module Overview

The `acp_registry/` module is a minimal registry directory containing ACP (Agent Communication Protocol) registration metadata for the Hermes Agent. This module allows Hermes to be discovered and launched by ACP-compatible hosts like VS Code (Cline, Claude Code extensions) and Zed.

The module contains only configuration/metadata files - no Python code. The actual ACP implementation lives in the `acp_adapter/` module.

---

## Directory Structure

| File | Lines | Purpose |
|------|-------|---------|
| `agent.json` | 309 | ACP agent registration manifest |
| `icon.svg` | 1402 | Hermes agent icon for editor display |

**Total:** ~1.7KB across 2 files (metadata only)

---

## Key Components

### 1. Agent Registration (`agent.json`)

ACP-compatible agent manifest that describes:
- Agent identity (name, version, description)
- Supported transports (stdio, SSE, HTTP)
- Capabilities and features
- Icon and branding assets

**Typical Structure:**
```json
{
  "name": "hermes-agent",
  "version": "0.7.0",
  "description": "Hermes Agent - AI coding assistant by Nous Research",
  "icon": "icon.svg",
  "transports": {
    "stdio": {
      "command": ["hermes", "acp-serve"],
      "cwd": "${workspaceFolder}"
    },
    "sse": {
      "endpoint": "/sse"
    }
  },
  "capabilities": {
    "sessions": true,
    "tools": true,
    "prompts": true
  }
}
```

### 2. Icon Asset (`icon.svg`)

SVG icon displayed in editor UI when Hermes is connected via ACP.

---

## ACP Transport Modes

### Stdio Transport
- Primary mode for local editor integration
- JSON-RPC frames over stdin/stdout
- Launched by editor via command

### SSE Transport (Server-Sent Events)
- HTTP-based streaming
- Used for remote connections
- Complements WebSocket for bidirectional

### HTTP Transport
- Full HTTP POST/GET for requests
- Used for non-streaming operations

---

## Integration Points

### With acp_adapter/
- `agent.json` references `hermes acp-serve` command
- Command launches the ACP server from `acp_adapter.server`

### With Editor Extensions
- VS Code Cline reads `agent.json` for available agents
- Zed uses similar discovery mechanism

---

## Related Files

**Implementation:**
- [acp_adapter/exploration.md](../acp_adapter/exploration.md) - ACP server implementation
- [acp_adapter/server.md](../acp_adapter/server.md) - Server details

**Related:**
- [copilot_acp_client.md](../agent/copilot_acp_client.md) - GitHub Copilot ACP client

---

*Deep dive created: 2026-04-07*
