# MCP-UI Deep Dive

## Overview

MCP-UI (`@mcp-ui/*` packages) is a community-driven implementation that pioneered UI-over-MCP patterns. The patterns developed here directly influenced the official MCP Apps specification (SEP-1865).

**Repository:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MCP/mcp-ui`

## Package Structure

### SDKs

| Package | Language | Purpose |
|---------|----------|---------|
| `@mcp-ui/server` | TypeScript | Create UI resources on MCP servers |
| `@mcp-ui/client` | TypeScript | Render tool UIs in MCP Apps hosts |
| `mcp_ui_server` | Ruby | Server-side UI resource creation |
| `mcp-ui-server` | Python | Server-side UI resource creation |

### Examples

| Example | Purpose |
|---------|---------|
| `server` | Full-featured TypeScript server (Cloudflare deployment) |
| `typescript-server-demo` | Simple TypeScript demo |
| `python-server-demo` | Python implementation example |
| `ruby-server-demo` | Ruby implementation example |
| `external-url-demo` | External URL content type demo |
| `mcp-apps-demo` | MCP Apps compliance demo |

## Core Concepts

### UIResource Wire Format

```typescript
interface UIResource {
  type: 'resource';
  resource: {
    uri: string;       // e.g., 'ui://component/id'
    mimeType: 'text/html;profile=mcp-app';
    text?: string;     // HTML content (text encoding)
    blob?: string;     // Base64-encoded HTML (blob encoding)
  };
  annotations?: Record<string, unknown>;
  _meta?: Record<string, unknown>;
}
```

### Content Types

#### 1. Raw HTML

```typescript
const htmlResource = await createUIResource({
  uri: 'ui://greeting/1',
  encoding: 'text',
  content: {
    type: 'rawHtml',
    htmlString: '<h1>Hello from MCP-UI!</h1>'
  }
});
```

#### 2. External URL

```typescript
const externalResource = await createUIResource({
  uri: 'ui://external/1',
  encoding: 'text',
  content: {
    type: 'externalUrl',
    iframeUrl: 'https://example.com'
  }
});
```

**TypeScript SDK behavior:**
- Fetches URL server-side
- Injects `<base>` tag for relative path resolution
- Validates URL (http/https only, blocks private IPs)
- Enforces timeout and response size limit
- Auto-populates `_meta.csp.baseUriDomains`

**Python/Ruby SDK behavior:**
- Stores URL directly (no fetching)
- Host client responsible for fetching

#### 3. Remote DOM

```typescript
const remoteDomResource = await createUIResource({
  uri: 'ui://remote/button',
  encoding: 'text',
  content: {
    type: 'remoteDom',
    script: `
      const button = document.createElement('ui-button');
      button.setAttribute('label', 'Click me!');
      button.addEventListener('press', () => {
        window.parent.postMessage({
          type: 'tool',
          payload: { toolName: 'buttonClicked' }
        }, '*');
      });
      root.appendChild(button);
    `,
    framework: 'react'
  }
});
```

## Server-Side Implementation

### createUIResource Function

```typescript
import { createUIResource } from '@mcp-ui/server';

// Basic HTML resource
const widget = await createUIResource({
  uri: 'ui://widget/1',
  encoding: 'text',
  content: {
    type: 'rawHtml',
    htmlString: '<div>Widget content</div>'
  }
});

// With CSP configuration
const dashboard = await createUIResource({
  uri: 'ui://dashboard/1',
  encoding: 'text',
  content: {
    type: 'rawHtml',
    htmlString: dashboardHtml
  },
  embeddedResourceProps: {
    _meta: {
      ui: {
        csp: {
          connectDomains: ['https://api.example.com'],
          resourceDomains: ['https://cdn.example.com']
        }
      }
    }
  }
});
```

### Integration with MCP Apps SDK

```typescript
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { registerAppTool, registerAppResource } from '@modelcontextprotocol/ext-apps/server';
import { createUIResource } from '@mcp-ui/server';
import { z } from 'zod';

const server = new McpServer({ name: 'my-server', version: '1.0.0' });

// 1. Create UI resource
const widgetUI = await createUIResource({
  uri: 'ui://my-server/widget',
  content: { type: 'rawHtml', htmlString: '<h1>Widget</h1>' },
  encoding: 'text',
});

// 2. Register resource handler
registerAppResource(
  server,
  'widget_ui',
  widgetUI.resource.uri,
  {},
  async () => ({ contents: [widgetUI.resource] })
);

// 3. Register tool with _meta linking
registerAppTool(
  server,
  'show_widget',
  {
    description: 'Show widget',
    inputSchema: { query: z.string() },
    _meta: {
      ui: { resourceUri: widgetUI.resource.uri }
    }
  },
  async ({ query }) => ({
    content: [{ type: 'text', text: `Query: ${query}` }]
  })
);
```

### Legacy Pattern (Embedded Resources)

For hosts that expect embedded resources:

```typescript
registerAppTool(server, 'show_widget', {...}, async ({ query }) => {
  const embeddedResource = await createUIResource({
    uri: `ui://my-server/widget/${query}`,
    encoding: 'text',
    content: {
      type: 'rawHtml',
      htmlString: renderWidget(query)
    }
  });

  return {
    content: [
      { type: 'text', text: `Result: ${query}` },
      embeddedResource  // For legacy MCP-UI hosts
    ]
  };
});
```

## Client-Side Implementation

### AppRenderer Component

High-level component for rendering tool UIs:

```tsx
import { AppRenderer, type AppRendererHandle } from '@mcp-ui/client';

function ToolUI({ client, toolName, toolInput, toolResult }) {
  const appRef = useRef<AppRendererHandle>(null);

  return (
    <AppRenderer
      ref={appRef}
      client={client}
      toolName={toolName}
      sandbox={{ url: new URL('http://localhost:8765/sandbox_proxy.html') }}
      toolInput={toolInput}
      toolResult={toolResult}
      hostContext={{ theme: 'dark' }}
      onOpenLink={async ({ url }) => {
        window.open(url, '_blank');
        return { isError: false };
      }}
      onMessage={async (params) => {
        console.log('Message from UI:', params);
        return { isError: false };
      }}
      onError={(error) => console.error('UI Error:', error)}
    />
  );
}
```

### AppFrame Component

Lower-level component when you have HTML and AppBridge:

```tsx
import { AppFrame, AppBridge } from '@mcp-ui/client';

function LowLevelToolUI({ html, client }) {
  const bridge = useMemo(
    () => new AppBridge(client, hostInfo, capabilities),
    [client]
  );

  return (
    <AppFrame
      html={html}
      sandbox={{ url: sandboxUrl }}
      appBridge={bridge}
      toolInput={{ query: 'test' }}
      onSizeChanged={(size) => console.log('Size:', size)}
    />
  );
}
```

### Without MCP Client

```tsx
<AppRenderer
  toolName="my-tool"
  toolResourceUri="ui://my-server/my-tool"
  sandbox={{ url: sandboxUrl }}
  onReadResource={async ({ uri }) => {
    return myMcpProxy.readResource({ uri });
  }}
  onCallTool={async (params) => {
    return myMcpProxy.callTool(params);
  }}
  toolInput={{ query: 'hello' }}
/>
```

## Sandbox Proxy Setup

### Basic Sandbox Proxy

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>Sandbox Proxy</title>
  <style>html, body { margin: 0; padding: 0; width: 100%; height: 100%; }</style>
</head>
<body>
  <script>
    // Listen for messages from host
    window.addEventListener('message', (event) => {
      const data = event.data;
      if (!data || typeof data !== 'object') return;

      // Handle resource ready notification
      if (data.method === 'ui/notifications/sandbox-resource-ready') {
        const { html } = data.params || {};
        if (html) {
          document.open();
          document.write(html);
          document.close();
        }
      }
    });

    // Signal ready
    window.parent.postMessage({
      method: 'ui/notifications/sandbox-proxy-ready',
      params: {}
    }, '*');
  </script>
</body>
</html>
```

### Production Sandbox (ext-apps reference)

The reference sandbox proxy (`@modelcontextprotocol/ext-apps/app-bridge`) includes:

1. **Origin validation** - Validates referrer and message origins
2. **CSP enforcement** - Constructs CSP from metadata
3. **Permission Policy** - Sets iframe `allow` attribute
4. **Security self-test** - Verifies sandbox isolation
5. **Bidirectional relay** - Forwards messages between host and view

## UI Actions Pattern

### Sending Actions from View

```javascript
// Call another tool
window.parent.postMessage({
  type: 'tool',
  payload: {
    toolName: 'get_weather',
    params: { city: 'San Francisco' }
  }
}, '*');

// Send follow-up prompt
window.parent.postMessage({
  type: 'prompt',
  payload: { prompt: 'What is the weather like?' }
}, '*');

// Open external link
window.parent.postMessage({
  type: 'link',
  payload: { url: 'https://example.com' }
}, '*');

// Send notification
window.parent.postMessage({
  type: 'notify',
  payload: { message: 'Widget loaded' }
}, '*');

// Request resize
window.parent.postMessage({
  type: 'ui-size-change',
  payload: { width: 500, height: 400 }
}, '*');
```

### Receiving Data in View

```javascript
window.addEventListener('message', (event) => {
  if (event.data.type === 'ui-lifecycle-iframe-render-data') {
    const { renderData } = event.data.payload;

    // Tool input arguments
    const toolInput = renderData.toolInput;

    // Tool execution result
    const toolOutput = renderData.toolOutput;

    // Host context
    const theme = renderData.theme;
    const locale = renderData.locale;
    const displayMode = renderData.displayMode;

    updateWidget(renderData);
  }
});
```

### Async Communication with messageId

```javascript
// Send message with messageId for response
const messageId = `msg-${Date.now()}-${counter++}`;
window.parent.postMessage({
  type: 'tool',
  messageId: messageId,
  payload: { toolName: 'asyncTool', params: { data: 'test' } }
}, '*');

// Listen for responses
window.addEventListener('message', (event) => {
  if (event.data.messageId === messageId) {
    switch (event.data.type) {
      case 'ui-message-received':
        console.log('Request acknowledged');
        break;
      case 'ui-message-response':
        if (event.data.payload.error) {
          console.error('Error:', event.data.payload.error);
        } else {
          console.log('Response:', event.data.payload.response);
        }
        break;
    }
  }
});
```

## MCP-UI Adapter for MCP Apps

The adapter enables legacy MCP-UI widgets to work in MCP Apps hosts:

```typescript
const widgetUI = await createUIResource({
  uri: 'ui://widget',
  encoding: 'text',
  content: {
    type: 'rawHtml',
    htmlString: `
      <html>
      <body>
        <div id="app">Loading...</div>
        <script>
          // Legacy MCP-UI messages still work
          window.addEventListener('message', (e) => {
            if (e.data.type === 'ui-lifecycle-iframe-render-data') {
              const { toolInput, toolOutput } = e.data.payload.renderData;
              document.getElementById('app').textContent =
                JSON.stringify({ toolInput, toolOutput }, null, 2);
            }
          });

          window.parent.postMessage({ type: 'ui-lifecycle-iframe-ready' }, '*');
        </script>
      </body>
      </html>
    `
  }
});
```

## Python SDK

```python
from mcp_ui_server import create_ui_resource

# Inline HTML
html_resource = create_ui_resource({
    "uri": "ui://greeting/1",
    "content": {
        "type": "rawHtml",
        "htmlString": "<p>Hello from Python!</p>"
    },
    "encoding": "text",
})

# External URL
external_resource = create_ui_resource({
    "uri": "ui://external/2",
    "content": {
        "type": "externalUrl",
        "iframeUrl": "https://example.com"
    },
    "encoding": "text",
})
```

## Ruby SDK

```ruby
require 'mcp_ui_server'

# Inline HTML
html_resource = McpUiServer.create_ui_resource(
  uri: 'ui://greeting/1',
  content: {
    type: :raw_html,
    htmlString: '<p>Hello from Ruby!</p>'
  },
  encoding: :text
)

# Remote DOM
remote_dom_resource = McpUiServer.create_ui_resource(
  uri: 'ui://remote/button',
  content: {
    type: :remote_dom,
    script: <<-JS
      const button = document.createElement('ui-button');
      button.setAttribute('label', 'Click me!');
      root.appendChild(button);
    JS,
    framework: :react
  },
  encoding: :text
)
```

## Host Capabilities Declaration

```typescript
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import {
  type ClientCapabilitiesWithExtensions,
  UI_EXTENSION_CAPABILITIES,
} from '@mcp-ui/client';

const capabilities: ClientCapabilitiesWithExtensions = {
  roots: { listChanged: true },
  extensions: UI_EXTENSION_CAPABILITIES,
};

const client = new Client(
  { name: 'my-app', version: '1.0.0' },
  { capabilities }
);
```

## Experimental Requests

The `sendExperimentalRequest` helper enables custom JSON-RPC requests:

### View-Side (Guest UI)

```typescript
import { sendExperimentalRequest } from '@mcp-ui/server';

// Write to clipboard
const result = await sendExperimentalRequest('x/clipboard/write', {
  text: 'Hello!'
});

// Analytics tracking
await sendExperimentalRequest('x/analytics/track', {
  event: 'button_click',
  properties: { button: 'submit' }
});
```

### Host-Side Handler

```tsx
<AppRenderer
  client={client}
  toolName="my-tool"
  sandbox={sandboxConfig}
  onFallbackRequest={async (request, extra) => {
    switch (request.method) {
      case 'x/clipboard/write':
        await navigator.clipboard.writeText(request.params?.text);
        return { success: true };
      case 'sampling/createMessage':
        return client.createMessage(request.params);
      default:
        throw new McpError(
          ErrorCode.MethodNotFound,
          `Unknown method: ${request.method}`
        );
    }
  }}
/>
```

## Implementation Techniques

### 1. Double-Iframe Isolation

```
Host (origin A)
  └─> Sandbox Proxy (origin B)
       └─> View (same origin as B, via document.write)
```

### 2. CSP via Query Parameter

```
GET /sandbox.html?csp={"connectDomains":["https://api.example.com"]}

Server parses CSP from query param and sets HTTP header:
Content-Security-Policy: connect-src https://api.example.com; ...
```

### 3. Bridge Connection Pattern

```typescript
// AppBridge handles JSON-RPC over postMessage
const bridge = new AppBridge(
  client,
  { name: 'host', version: '1.0.0' },
  {
    openLinks: {},
    serverTools: { listChanged: true },
    serverResources: {}
  }
);

// Connect transport
await bridge.connect(
  new PostMessageTransport(
    iframe.contentWindow!,
    iframe.contentWindow!
  )
);
```

### 4. Size Observation

```typescript
// View-side: Auto-resize with ResizeObserver
const resizeObserver = new ResizeObserver((entries) => {
  const { width, height } = entries[0].contentRect;
  bridge.sendSizeChanged({ width, height });
});
resizeObserver.observe(document.body);

// Host-side: Update iframe dimensions
bridge.onsizechange = async (params) => {
  if (params.width !== undefined) {
    iframe.style.width = `${params.width}px`;
  }
  if (params.height !== undefined) {
    iframe.style.height = `${params.height}px`;
  }
};
```

## Supported Hosts

### MCP Apps Hosts (via @mcp-ui/client)
- Claude (native)
- VSCode (built-in)
- Postman (MCP playground)
- Goose (open source agent)
- MCPJam
- LibreChat

### Legacy MCP-UI Hosts
- Nanobot (full support)
- MCPJam (full support)
- Postman (partial UI actions)
- Smithery (rendering only)

## Migration Path

### Phase 1: Embed Legacy Resource
Keep existing MCP-UI resource in tool response for legacy hosts.

### Phase 2: Register MCP Apps Resource
Register resource separately for MCP Apps hosts.

### Phase 3: Dual Support
```typescript
registerAppTool(server, 'widget', {
  _meta: {
    ui: { resourceUri: 'ui://widget' }  // For MCP Apps
  }
}, async (args) => ({
  content: [
    { type: 'text', text: 'Result' },
    legacyResource  // For legacy MCP-UI
  ]
}));
```

## Related Resources

- [MCP-UI Documentation](https://mcpui.dev/)
- [GitMCP Integration](https://gitmcp.io/idosal/mcp-ui)
- [UI Inspector](https://github.com/idosal/ui-inspector)
- [MCP-UI Chat Demo](https://github.com/idosal/scira-mcp-ui-chat)
