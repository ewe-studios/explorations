# Protocol Deep Dive: Old vs New

## Overview

This document compares the legacy MCP-UI protocol with the new MCP Apps (SEP-1865) protocol, explaining the evolution and key differences.

## Legacy MCP-UI Protocol

### Message Structure

The original MCP-UI protocol used a custom message format with simple string types:

```typescript
// Lifecycle Messages
interface LifecycleReady {
  type: 'ui-lifecycle-iframe-ready';
}

interface LifecycleRenderData {
  type: 'ui-lifecycle-iframe-render-data';
  payload: {
    renderData: {
      toolInput?: Record<string, unknown>;
      toolOutput?: CallToolResult;
      theme?: 'light' | 'dark' | 'system';
      locale?: string;
      displayMode?: 'inline' | 'fullscreen' | 'pip';
      maxHeight?: number;
    };
  };
}

interface LifecycleTeardown {
  type: 'ui-lifecycle-teardown';
}

// Action Messages (View → Host)
interface ToolAction {
  type: 'tool';
  payload: {
    toolName: string;
    params: Record<string, unknown>;
  };
  messageId?: string; // Optional for async responses
}

interface PromptAction {
  type: 'prompt';
  payload: { prompt: string };
}

interface LinkAction {
  type: 'link';
  payload: { url: string };
}

interface NotifyAction {
  type: 'notify';
  payload: { message: string };
}

interface SizeChangeAction {
  type: 'ui-size-change';
  payload: { width: number; height: number };
}
```

### Response Messages (Async Pattern)

For actions with `messageId`, the host responds:

```typescript
// Acknowledgment
interface MessageReceived {
  type: 'ui-message-received';
  messageId: string;
}

// Final Response
interface MessageResponse {
  type: 'ui-message-response';
  messageId: string;
  payload: {
    response?: unknown;
    error?: unknown;
  };
}
```

### Resource Embedding Pattern

In legacy MCP-UI, resources were embedded directly in tool responses:

```typescript
// Tool result with embedded UI resource
{
  content: [
    { type: 'text', text: 'Processing complete' },
    {
      type: 'resource',
      resource: {
        uri: 'ui://my-server/widget/123',
        mimeType: 'text/html;profile=mcp-app',
        text: '<html>...</html>'
      }
    }
  ]
}
```

### Client Detection Pattern

```typescript
// Client-side detection of MCP-UI resources
if (
  mcpResource.type === 'resource' &&
  mcpResource.resource.uri?.startsWith('ui://')
) {
  return <UIResourceRenderer
    resource={mcpResource.resource}
    onUIAction={handleUIAction}
  />;
}
```

### Limitations of Legacy Protocol

1. **No formal capability negotiation** - Hosts couldn't advertise supported features
2. **Embedded resources** - HTML sent with every tool call (no caching)
3. **Custom message format** - Not compatible with standard MCP tooling
4. **Single iframe** - Less secure, no CSP enforcement layer
5. **No tool visibility control** - All tools visible to both model and app

## MCP Apps Protocol (SEP-1865)

### JSON-RPC 2.0 Message Format

All communication uses standard JSON-RPC 2.0:

```typescript
// Request format
{
  jsonrpc: "2.0";
  id: number | string;
  method: string;
  params?: object;
}

// Response format
{
  jsonrpc: "2.0";
  id: number | string;
  result?: object;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

// Notification format (no response expected)
{
  jsonrpc: "2.0";
  method: string;
  params?: object;
}
```

### Initialization Handshake

**View → Host:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "ui/initialize",
  "params": {
    "protocolVersion": "2026-01-26",
    "capabilities": {},
    "clientInfo": {
      "name": "my-view",
      "version": "1.0.0"
    },
    "appCapabilities": {
      "experimental": {},
      "tools": { "listChanged": true },
      "availableDisplayModes": ["inline", "fullscreen"]
    }
  }
}
```

**Host → View:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2026-01-26",
    "capabilities": {
      "openLinks": {},
      "serverTools": { "listChanged": true },
      "serverResources": {},
      "logging": {},
      "sandbox": {
        "permissions": { "clipboardWrite": {} },
        "csp": { "connectDomains": ["https://api.example.com"] }
      }
    },
    "hostInfo": {
      "name": "claude-desktop",
      "version": "1.0.0"
    },
    "hostContext": {
      "theme": "dark",
      "styles": {
        "variables": {
          "--color-background-primary": "light-dark(#ffffff, #171717)",
          "--color-text-primary": "light-dark(#171717, #fafafa)"
        }
      },
      "displayMode": "inline",
      "containerDimensions": { "width": 400, "maxHeight": 600 },
      "locale": "en-US",
      "timeZone": "America/New_York",
      "platform": "desktop"
    }
  }
}
```

### Host → View Notifications

```typescript
// Tool input (complete arguments)
{
  jsonrpc: "2.0",
  method: "ui/notifications/tool-input",
  params: {
    arguments: Record<string, unknown>
  }
}

// Tool input (streaming partial)
{
  jsonrpc: "2.0",
  method: "ui/notifications/tool-input-partial",
  params: {
    arguments: Record<string, unknown>
  }
}

// Tool execution result
{
  jsonrpc: "2.0",
  method: "ui/notifications/tool-result",
  params: CallToolResult
}

// Host context changed (theme, locale, etc.)
{
  jsonrpc: "2.0",
  method: "ui/notifications/host-context-changed",
  params: {
    theme?: "light" | "dark",
    displayMode?: "inline" | "fullscreen" | "pip",
    containerDimensions?: {...}
  }
}

// Size changed (from View resize request)
{
  jsonrpc: "2.0",
  method: "ui/notifications/size-changed",
  params: {
    width?: number,
    height?: number
  }
}

// Tool cancelled
{
  jsonrpc: "2.0",
  method: "ui/notifications/tool-cancelled",
  params: {}
}

// Resource teardown
{
  jsonrpc: "2.0",
  method: "ui/resource-teardown",
  params: {}
}
```

### View → Host Requests

```typescript
// Call another tool
{
  jsonrpc: "2.0",
  id: 2,
  method: "tools/call",
  params: {
    name: "other-tool",
    arguments: { key: "value" }
  }
}

// Send message to conversation
{
  jsonrpc: "2.0",
  id: 3,
  method: "ui/message",
  params: {
    role: "user",
    content: [{ type: "text", text: "Follow-up question" }]
  }
}

// Open external link
{
  jsonrpc: "2.0",
  id: 4,
  method: "ui/open-link",
  params: {
    url: "https://example.com"
  }
}

// Request display mode change
{
  jsonrpc: "2.0",
  id: 5,
  method: "ui/request-display-mode",
  params: {
    displayMode: "fullscreen"
  }
}

// Size change notification
{
  jsonrpc: "2.0",
  method: "ui/notifications/size-changed",
  params: {
    width: 500,
    height: 400
  }
}

// Log message
{
  jsonrpc: "2.0",
  method: "notifications/message",
  params: {
    level: "info",
    data: "Widget loaded"
  }
}
```

### Resource Discovery Pattern

```typescript
// Tool declaration with UI linkage
{
  name: "show_dashboard",
  description: "Show interactive dashboard",
  inputSchema: {
    type: "object",
    properties: {
      query: { type: "string" }
    }
  },
  _meta: {
    ui: {
      resourceUri: "ui://dashboard/view",
      visibility: ["model", "app"]
    }
  }
}
```

### Visibility Modes

| Visibility | Model Can See | App Can Call | Use Case |
|------------|---------------|--------------|----------|
| `["model", "app"]` (default) | ✅ | ✅ | Standard tools |
| `["model"]` | ✅ | ❌ | Model-only tools |
| `["app"]` | ❌ | ✅ | UI controls, refresh buttons |

## Protocol Translation (MCP-UI Adapter)

For migrating legacy MCP-UI widgets, the adapter translates:

### Outgoing (Widget → Host)

| MCP-UI Action | MCP Apps Method |
|---------------|-----------------|
| `tool` | `tools/call` |
| `prompt` | `ui/message` |
| `link` | `ui/open-link` |
| `notify` | `notifications/message` |
| `intent` | `ui/message` |
| `ui-size-change` | `ui/notifications/size-changed` |

### Incoming (Host → Widget)

| MCP Apps Notification | MCP-UI Message |
|----------------------|----------------|
| `ui/notifications/tool-input` | `ui-lifecycle-iframe-render-data` |
| `ui/notifications/tool-input-partial` | `ui-lifecycle-iframe-render-data` |
| `ui/notifications/tool-result` | `ui-lifecycle-iframe-render-data` |
| `ui/notifications/host-context-changed` | `ui-lifecycle-iframe-render-data` |
| `ui/notifications/size-changed` | `ui-lifecycle-iframe-render-data` |
| `ui/notifications/tool-cancelled` | `ui-lifecycle-tool-cancelled` |
| `ui/resource-teardown` | `ui-lifecycle-teardown` |

## Lifecycle Comparison

### Legacy MCP-UI Lifecycle

```
1. Host renders iframe with UI resource
2. View sends: { type: 'ui-lifecycle-iframe-ready' }
3. Host sends: { type: 'ui-lifecycle-iframe-render-data', payload: {...} }
4. User interacts, View sends actions
5. Host may send updated render data
6. Host removes iframe (no teardown notification)
```

### MCP Apps Lifecycle

```
1. Host renders sandbox proxy iframe
2. Sandbox sends: { method: 'ui/notifications/sandbox-proxy-ready' }
3. Host sends: { method: 'ui/notifications/sandbox-resource-ready', params: { html } }
4. View loads, sends: { method: 'ui/initialize' }
5. Host responds with hostContext, capabilities
6. View sends: { method: 'ui/notifications/initialized' }
7. Host sends tool input/result via notifications
8. User interacts, View sends requests
9. Host sends: { method: 'ui/resource-teardown' }
10. View acknowledges, cleans up
```

## Key Improvements in MCP Apps

### 1. Standardization
- JSON-RPC 2.0 aligns with core MCP protocol
- Reuses MCP SDK types and patterns
- Compatible with existing MCP tooling

### 2. Security
- Double-iframe sandbox architecture
- CSP enforcement via HTTP headers
- Permission Policy via `allow` attribute
- Origin validation at each layer

### 3. Capability Negotiation
- Views declare supported features
- Hosts advertise available capabilities
- Graceful degradation when features unavailable

### 4. Resource Efficiency
- Templates declared upfront
- Prefetching and caching possible
- Separation of template (static) and data (dynamic)

### 5. Tool Visibility
- Model-visible vs app-only tools
- Cleaner agent context
- UI controls don't clutter conversation

### 6. Theming
- Standardized CSS custom properties
- 80+ standardized variable names
- Light/dark mode via `light-dark()` function

## Migration Guide

### From MCP-UI to MCP Apps

**Before (MCP-UI):**
```typescript
// Embedded resource in tool response
const resource = createUIResource({
  uri: 'ui://widget/1',
  content: { type: 'rawHtml', htmlString: html },
  encoding: 'text'
});

return {
  content: [
    { type: 'text', text: 'Result' },
    resource
  ]
};
```

**After (MCP Apps):**
```typescript
// Register resource separately
registerAppResource(server, 'widget', 'ui://widget', {}, async () => ({
  contents: [{
    uri: 'ui://widget',
    mimeType: RESOURCE_MIME_TYPE,
    text: html
  }]
}));

// Tool references resource
registerAppTool(server, 'show_widget', {
  description: 'Show widget',
  inputSchema: {...},
  _meta: {
    ui: { resourceUri: 'ui://widget' }
  }
}, async (args) => ({
  content: [{ type: 'text', text: 'Result' }]
}));
```

### Adapting Legacy Widgets

Use the MCP Apps adapter in `@mcp-ui/server`:

```typescript
const widgetUI = await createUIResource({
  uri: 'ui://widget',
  encoding: 'text',
  content: {
    type: 'rawHtml',
    htmlString: `
      <script>
        // Legacy MCP-UI messages still work
        window.parent.postMessage({
          type: 'tool',
          payload: { toolName: 'myTool', params: {} }
        }, '*');
      </script>
    `
  }
});
```

## Message Flow Diagrams

### Tool Call Flow

```
┌──────┐         ┌──────┐         ┌───────┐         ┌────────┐
│ View │         │ Host │         │ MCP   │         │ Server │
│      │         │      │         │ Server│         │        │
└──┬───┘         └──┬───┘         └───┬───┘         └───┬────┘
   │                │                 │                 │
   │ ui/initialize  │                 │                 │
   │───────────────>│                 │                 │
   │                │                 │                 │
   │ hostContext    │                 │                 │
   │<───────────────│                 │                 │
   │                │                 │                 │
   │ ui/initialized │                 │                 │
   │───────────────>│                 │                 │
   │                │                 │                 │
   │                │ ui/notifications/tool-input       │
   │                │────────────────>│                 │
   │                │                 │                 │
   │ tools/call     │                 │                 │
   │───────────────>│                 │                 │
   │                │ tools/call     │                 │
   │                │────────────────>│                 │
   │                │                 │ tools/call     │
   │                │                 │────────────────>│
   │                │                 │                 │
   │                │                 │ CallToolResult │
   │                │                 │<────────────────│
   │                │ tool result    │                 │
   │                │<────────────────│                 │
   │ tool result    │                 │                 │
   │<───────────────│                 │                 │
```

### Context Change Flow

```
┌──────┐         ┌──────┐
│ View │         │ Host │
└──┬───┘         └──┬───┘
   │                │
   │                │ User toggles dark mode
   │                │
   │                │ ui/notifications/host-context-changed
   │                │────────────────>
   │                │
   │ View adapts    │
   │ to new theme   │
   │                │
```

### Size Change Flow

```
┌──────┐         ┌──────┐
│ View │         │ Host │
└──┬───┘         └──┬───┘
   │                │
   │ ResizeObserver │
   │ detects change │
   │                │
   │ ui/notifications/size-changed
   │────────────────>
   │                │
   │                │ Updates iframe dimensions
   │                │
```
