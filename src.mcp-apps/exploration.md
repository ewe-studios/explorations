# MCP Apps Exploration

## Overview

MCP Apps is an extension to the Model Context Protocol that enables MCP servers to deliver interactive user interfaces to hosts. This standardizes how servers declare UI resources, how hosts render them securely in sandboxed iframes, and how the two communicate bidirectionally.

## Source Directories Surveyed

### 1. ext-apps (Official MCP Apps Reference)
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MCP/ext-apps`

The official MCP Apps repository containing:
- **Specification** - SEP-1865: The official MCP Apps specification (2026-01-26 stable)
- **TypeScript SDK** - `@modelcontextprotocol/ext-apps` package
- **Documentation** - Guides, patterns, and quickstart materials
- **Examples** - 25+ example servers demonstrating various use cases

### 2. mcp-ui (Community Implementation)
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MCP/mcp-ui`

A community-driven implementation that pioneered UI-over-MCP patterns:
- **Client SDK** - `@mcp-ui/client` for building MCP Apps hosts
- **Server SDK** - `@mcp-ui/server`, Ruby (`mcp_ui_server`), Python (`mcp-ui-server`)
- **Documentation** - Comprehensive guides for client and server development
- **Examples** - Demo servers and client implementations

## Projects Structure

### ext-apps Sub-projects

| Category | Projects |
|----------|----------|
| **Documentation** | docs/ (overview, quickstart, patterns, authorization, CSP/CORS) |
| **Examples** | 25 example servers (map, threejs, sheet-music, wiki-explorer, etc.) |
| **SDKs** | TypeScript SDK with React hooks |
| **Specification** | 2026-01-26 (stable), draft |

### mcp-ui Sub-projects

| Category | Projects |
|----------|----------|
| **SDKs** | TypeScript client/server, Ruby server, Python server |
| **Documentation** | VitePress docs site |
| **Examples** | Server demos, external URL demo, WC demo, remote-dom demo |
| **Plugins** | Claude Code skills for scaffolding |

## Key Examples in ext-apps

### Complex Applications
- **map-server** - Interactive 3D globe viewer using CesiumJS
- **threejs-server** - Interactive 3D scene renderer
- **shadertoy-server** - Real-time GLSL shader renderer
- **sheet-music-server** - ABC notation to sheet music converter
- **wiki-explorer-server** - Wikipedia link graph visualization

### Data Visualization
- **cohort-heatmap-server** - Customer retention heatmap
- **scenario-modeler-server** - SaaS business projections
- **budget-allocator-server** - Interactive budget allocation
- **customer-segmentation-server** - Scatter chart with clustering
- **system-monitor-server** - Real-time OS metrics

### Utilities
- **pdf-server** - Interactive PDF viewer with chunked loading
- **qr-server** - QR code generator (Python)
- **transcript-server** - Live speech transcription
- **video-resource-server** - Binary video via MCP resources

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                        MCP Server                               │
│  ┌─────────────┐  ┌──────────────┐                             │
│  │   Tools     │  │ UI Resources │                             │
│  │  (with      │  │  (ui:// URI) │                             │
│  │  _meta.ui)  │  │              │                             │
│  └──────┬──────┘  └──────┬───────┘                             │
└─────────┼────────────────┼──────────────────────────────────────┘
          │                │
          │  MCP Protocol  │
          │                │
          ▼                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Host (Chat Client)                           │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    AppBridge                            │   │
│  │  (JSON-RPC proxy, postMessage transport)                │   │
│  └────────────────────────┬────────────────────────────────┘   │
│                           │                                     │
│                           │ postMessage                         │
│                           ▼                                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              Sandbox Proxy (outer iframe)               │   │
│  │  - Different origin from host                           │   │
│  │  - CSP enforcement                                      │   │
│  │  - Message relay                                        │   │
│  └────────────────────────┬────────────────────────────────┘   │
│                           │                                     │
│                           │ document.write                      │
│                           ▼                                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │         View / App (inner iframe)                       │   │
│  │  - Sandboxed (allow-scripts, allow-same-origin)         │   │
│  │  - Your HTML/JS/CSS                                     │   │
│  │  - MCP client over postMessage                          │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Protocol Comparison: Old vs New

### Legacy MCP-UI Protocol (mcp-ui)

**Message Format:**
```javascript
// Lifecycle messages
{ type: 'ui-lifecycle-iframe-ready' }
{ type: 'ui-lifecycle-iframe-render-data', payload: { renderData } }
{ type: 'ui-lifecycle-teardown' }

// Actions
{ type: 'tool', payload: { toolName, params } }
{ type: 'prompt', payload: { prompt } }
{ type: 'link', payload: { url } }
{ type: 'notify', payload: { message } }
```

**Characteristics:**
- Custom message format (non-JSON-RPC)
- Embedded resources in tool responses
- No formal capability negotiation
- Single iframe architecture

### MCP Apps Protocol (ext-apps)

**Message Format (JSON-RPC 2.0):**
```javascript
// Initialization
{ jsonrpc: "2.0", id: 1, method: "ui/initialize", params: {...} }
{ jsonrpc: "2.0", id: 1, result: { protocolVersion, hostContext, ...} }

// Host → View Notifications
{ jsonrpc: "2.0", method: "ui/notifications/tool-input", params: {...} }
{ jsonrpc: "2.0", method: "ui/notifications/tool-result", params: {...} }
{ jsonrpc: "2.0", method: "ui/notifications/host-context-changed", params: {...} }

// View → Host Requests
{ jsonrpc: "2.0", id: 2, method: "tools/call", params: {...} }
{ jsonrpc: "2.0", id: 3, method: "ui/message", params: {...} }
{ jsonrpc: "2.0", id: 4, method: "ui/open-link", params: {...} }
```

**Characteristics:**
- Standard JSON-RPC 2.0 over postMessage
- Resource discovery via `_meta.ui.resourceUri`
- Formal capability negotiation via extensions
- Double-iframe sandbox architecture
- CSP enforcement via metadata

## Supported Hosts

### MCP Apps Hosts
| Host | Status | Notes |
|------|--------|-------|
| Claude | ✅ Full | Native support via MCP Apps |
| VSCode | ✅ Full | Built-in extension support |
| Postman | ✅ Full | MCP playground |
| Goose | ✅ Full | Open source AI agent |
| MCPJam | ✅ Full | Community client |
| LibreChat | ✅ Full | Enhanced ChatGPT clone |

### Legacy MCP-UI Hosts
| Host | Rendering | UI Actions |
|------|-----------|------------|
| Nanobot | ✅ | ✅ |
| MCPJam | ✅ | ✅ |
| Postman | ✅ | ⚠️ Partial |
| Smithery | ✅ | ❌ |

## Security Model

### Sandboxing Architecture

1. **Double-Iframe Isolation**
   - Host (outer) → Sandbox Proxy (middle) → View (inner)
   - Each layer has different origin
   - Sandbox enforces CSP via HTTP headers

2. **Iframe Sandbox Attributes**
   ```html
   <iframe sandbox="allow-scripts allow-same-origin allow-forms">
   ```

3. **Content Security Policy**
   - Servers declare domains via `_meta.ui.csp`
   - `connectDomains` - fetch/XHR/WebSocket origins
   - `resourceDomains` - scripts, styles, images origins
   - `frameDomains` - nested iframe origins
   - `baseUriDomains` - base URI origins

4. **Permission Policy**
   - Camera, microphone, geolocation, clipboard-write
   - Declared in `_meta.ui.permissions`
   - Enforced via iframe `allow` attribute

## Production Readiness Requirements

### For Server Developers

1. **CSP Configuration**
   ```typescript
   _meta: {
     ui: {
       csp: {
         connectDomains: ['https://api.example.com'],
         resourceDomains: ['https://cdn.jsdelivr.net']
       }
     }
   }
   ```

2. **Stable Origins for OAuth/CORS**
   ```typescript
   _meta: {
     ui: {
       domain: 'a904794854a047f6.claudemcpcontent.com'
     }
   }
   ```

3. **Tool Visibility Control**
   ```typescript
   _meta: {
     ui: {
       visibility: ['model', 'app'] // or ['app'] for UI-only tools
     }
   }
   ```

### For Host Developers

1. **Sandbox Proxy Implementation**
   - Separate origin from host
   - CSP enforcement via HTTP headers
   - Message relay between host and view

2. **Capability Declaration**
   ```typescript
   const capabilities: ClientCapabilitiesWithExtensions = {
     roots: { listChanged: true },
     extensions: UI_EXTENSION_CAPABILITIES
   };
   ```

3. **Security Enforcement**
   - Validate referrer origins
   - Enforce declared CSP domains
   - Handle tool visibility correctly

## Key Design Patterns

### 1. Progressive Enhancement
UI is optional - tools work as text-only on hosts without MCP Apps support.

### 2. Template/Data Separation
- UI templates declared upfront (static HTML)
- Tool results provide dynamic data
- Enables prefetching and caching

### 3. Bidirectional Communication
- Host → View: tool input/result, context changes
- View → Host: tool calls, messages, link requests

### 4. Graceful Degradation
- Hosts decide which capabilities to support
- Views use feature detection for permissions
- Fallback to text-only when UI unavailable

## Related Documentation

- [Protocol Details](./protocol-details.md) - Wire format reference
- [CSP and CORS](./csp-cors.md) - Security configuration guide
- [Agent Skills](./agent-skills.md) - Claude Code plugin skills
- [Authorization](./authorization.md) - Auth patterns for MCP Apps
- [Patterns](./patterns.md) - Common design patterns

## Resources

- [Official Specification](https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/2026-01-26/apps.mdx)
- [MCP-UI Documentation](https://mcpui.dev/)
- [SEP-1865 Discussion](https://github.com/modelcontextprotocol/modelcontextprotocol/pull/1865)
