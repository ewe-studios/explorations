# Production Readiness Guide

## Overview

This guide covers what's needed to build production-ready MCP Apps experiences, covering server development, host implementation, and security considerations.

## Server Development Checklist

### 1. Resource Registration

```typescript
// ✅ Correct: Register resource separately
registerAppResource(
  server,
  'dashboard',
  'ui://dashboard/view',
  { description: 'Interactive dashboard' },
  async () => ({
    contents: [{
      uri: 'ui://dashboard/view',
      mimeType: RESOURCE_MIME_TYPE,
      text: dashboardHtml,
      _meta: {
        ui: {
          csp: {
            connectDomains: ['https://api.example.com'],
            resourceDomains: ['https://cdn.example.com']
          }
        }
      }
    }]
  })
);

// ✅ Correct: Tool references resource
registerAppTool(
  server,
  'show_dashboard',
  {
    description: 'Show dashboard',
    inputSchema: { query: z.string() },
    _meta: {
      ui: { resourceUri: 'ui://dashboard/view' }
    }
  },
  handler
);
```

### 2. CSP Configuration

**Required for any app making network requests:**

```typescript
_meta: {
  ui: {
    csp: {
      // APIs your app calls (fetch, XHR, WebSocket)
      connectDomains: [
        'https://api.example.com',
        'wss://realtime.example.com'
      ],
      // Static resources (CDN, fonts, images)
      resourceDomains: [
        'https://cdn.jsdelivr.net',
        'https://fonts.googleapis.com',
        'https://*.example.com'  // Wildcard subdomains
      ],
      // Nested iframes (YouTube, Vimeo, etc.)
      frameDomains: [
        'https://www.youtube.com',
        'https://player.vimeo.com'
      ],
      // Base URI for relative URLs
      baseUriDomains: [
        'https://cdn.example.com'
      ]
    }
  }
}
```

**Minimal CSP for localhost development:**

```typescript
csp: {
  connectDomains: ['http://localhost:3000'],
  resourceDomains: ['http://localhost:3000']
}
```

### 3. Stable Origins for OAuth/CORS

For apps needing OAuth callbacks or CORS allowlisting:

```typescript
import { createHash } from 'crypto';

function computeAppDomainForClaude(mcpServerUrl: string): string {
  const hash = createHash('sha256')
    .update(mcpServerUrl)
    .digest('hex')
    .slice(0, 32);
  return `${hash}.claudemcpcontent.com`;
}

const APP_DOMAIN = computeAppDomainForClaude(process.env.MCP_SERVER_URL!);

// In resource metadata
_meta: {
  ui: {
    domain: APP_DOMAIN,
    csp: {
      connectDomains: ['https://api.example.com']
    }
  }
}
```

### 4. Tool Visibility Control

```typescript
// Visible to both model and app (default)
visibility: ['model', 'app']

// Model-only tool (hidden from UI)
visibility: ['model']

// App-only tool (UI controls, hidden from model)
visibility: ['app']
```

**Use cases for app-only tools:**
- Refresh buttons
- Pagination controls
- Form submissions
- UI state changes

### 5. Error Handling

```typescript
try {
  const result = await app.callServerTool({
    name: 'fetch_data',
    arguments: { query }
  });
  setData(result);
} catch (error) {
  console.error('Tool call failed:', error);
  setError('Failed to load data');
}

// Handle tool cancellation
app.ontoolcancelled = (params) => {
  console.log('Tool cancelled:', params.reason);
  setCancelled(true);
};
```

### 6. Responsive Design

```css
/* Use host-provided CSS variables */
.container {
  background: var(--color-background-primary, #ffffff);
  color: var(--color-text-primary, #000000);
  font-family: var(--font-sans, system-ui);
}

/* Handle container dimensions */
:root {
  /* Fixed width from host */
  @media (width: 400px) {
    .container { width: 100%; }
  }

  /* Flexible width */
  @media (max-width: 600px) {
    .container { max-width: 600px; }
  }
}
```

```typescript
// Check host context in JavaScript
const containerDimensions = hostContext.containerDimensions;

if (containerDimensions?.height !== undefined) {
  // Fixed height - fill container
  document.documentElement.style.height = '100vh';
} else if (containerDimensions?.maxHeight) {
  // Flexible height
  document.documentElement.style.maxHeight =
    `${containerDimensions.maxHeight}px`;
}
```

## Host Development Checklist

### 1. Sandbox Proxy Implementation

**Minimal sandbox proxy:**

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>Sandbox Proxy</title>
</head>
<body>
  <script>
    const EXPECTED_HOST_ORIGIN = window.location.origin;

    window.addEventListener('message', (event) => {
      // Validate origin
      if (event.origin !== EXPECTED_HOST_ORIGIN) {
        console.error('Invalid origin:', event.origin);
        return;
      }

      // Handle resource ready
      if (event.data.method === 'ui/notifications/sandbox-resource-ready') {
        const { html, csp } = event.data.params;
        if (html) {
          // Set CSP via meta tag (fallback)
          if (csp) {
            const meta = document.createElement('meta');
            meta.httpEquiv = 'Content-Security-Policy';
            meta.content = buildCspString(csp);
            document.head.appendChild(meta);
          }

          document.open();
          document.write(html);
          document.close();
        }
      }
    });

    function buildCspString(csp) {
      const directives = [
        "default-src 'none'",
        "script-src 'self' 'unsafe-inline'",
        "style-src 'self' 'unsafe-inline'",
        "img-src 'self' data:",
        `connect-src ${csp.connectDomains?.join(' ') || "'none'"}`
      ];
      return directives.join('; ');
    }

    // Signal ready
    window.parent.postMessage({
      method: 'ui/notifications/sandbox-proxy-ready',
      params: {}
    }, '*');
  </script>
</body>
</html>
```

### 2. Capability Declaration

```typescript
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import {
  type ClientCapabilitiesWithExtensions,
  UI_EXTENSION_CAPABILITIES,
} from '@mcp-ui/client';

const capabilities: ClientCapabilitiesWithExtensions = {
  roots: { listChanged: true },
  extensions: UI_EXTENSION_CAPABILITIES,
  // Optional: Declare specific UI capabilities
};

const client = new Client(
  { name: 'my-host', version: '1.0.0' },
  { capabilities }
);
```

### 3. Handler Implementation

```typescript
<AppRenderer
  client={client}
  toolName={toolName}
  sandbox={{ url: sandboxUrl }}
  toolInput={toolInput}
  toolResult={toolResult}

  // Required handlers
  onOpenLink={async ({ url }) => {
    if (url.startsWith('https://') || url.startsWith('http://')) {
      window.open(url, '_blank');
      return { isError: false };
    }
    return { isError: true, error: 'Invalid URL' };
  }}

  onMessage={async (params) => {
    // Handle follow-up prompts from UI
    console.log('UI message:', params);
    return { isError: false };
  }}

  // Optional handlers
  onLoggingMessage={(params) => {
    console.log('[UI Log]', params.level, params.data);
  }}

  onSizeChanged={(params) => {
    // Resize iframe based on content
    if (params.width) iframe.style.width = `${params.width}px`;
    if (params.height) iframe.style.height = `${params.height}px`;
  }}

  onError={(error) => {
    console.error('UI Error:', error);
    // Show error UI to user
  }}

  // Custom MCP handlers (optional)
  onCallTool={async (params, extra) => {
    // Intercept/modify tool calls from UI
    return client.callTool(params);
  }}

  onReadResource={async (params, extra) => {
    // Proxy resource reads
    return client.readResource(params);
  }}

  // Handle experimental methods
  onFallbackRequest={async (request, extra) => {
    switch (request.method) {
      case 'x/clipboard/write':
        await navigator.clipboard.writeText(request.params.text);
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

### 4. Security Validation

```typescript
// Validate sandbox isolation
function validateSandbox(iframe: HTMLIFrameElement) {
  try {
    // Should throw SecurityError if properly sandboxed
    (iframe.contentWindow as any).top.alert('test');
    throw new Error('Sandbox not secure - top is accessible');
  } catch (e) {
    if (e.message.includes('secure')) {
      throw e;
    }
    // Expected: SecurityError
  }
}

// Validate message origins
window.addEventListener('message', (event) => {
  if (!ALLOWED_ORIGINS.includes(event.origin)) {
    console.error('Rejected message from:', event.origin);
    return;
  }
  // Process message
});
```

## View Development Checklist

### 1. Initialization

```typescript
import { App } from '@modelcontextprotocol/ext-apps';

const app = new App({
  name: 'my-view',
  version: '1.0.0'
});

app.ontoolresult = async (result) => {
  setData(result);
};

app.onhostcontextchanged = (params) => {
  setTheme(params.theme);
  setContainerDimensions(params.containerDimensions);
};

app.onerror = console.error;

await app.connect(transport);
```

### 2. Feature Detection

```typescript
// Check for geolocation
if ('geolocation' in navigator) {
  navigator.geolocation.getCurrentPosition(success, error);
} else {
  // Fallback
}

// Check for camera
if (navigator.mediaDevices?.getUserMedia) {
  try {
    const stream = await navigator.mediaDevices.getUserMedia({ video: true });
    // Use camera
  } catch (err) {
    // Permission denied or unavailable
  }
}
```

### 3. Theme Support

```css
:root {
  /* Provide fallbacks for all theme variables */
  --color-background-primary: light-dark(#ffffff, #171717);
  --color-background-secondary: light-dark(#f5f5f5, #1a1a1a);
  --color-text-primary: light-dark(#171717, #fafafa);
  --color-text-secondary: light-dark(#666666, #a0a0a0);
  --color-border-primary: light-dark(#e0e0e0, #333333);
  --font-sans: system-ui, -apple-system, sans-serif;
  --border-radius-md: 8px;
}

.container {
  background: var(--color-background-primary);
  color: var(--color-text-primary);
  border: 1px solid var(--color-border-primary);
  border-radius: var(--border-radius-md);
}
```

### 4. Size Management

```typescript
// Auto-resize with ResizeObserver
const resizeObserver = new ResizeObserver((entries) => {
  const { width, height } = entries[0].contentRect;
  app.sendSizeChanged({ width, height });
});
resizeObserver.observe(document.body);

// Or manual resize request
function requestResize(width: number, height: number) {
  app.sendSizeChanged({ width, height });
}
```

### 5. Tool Calling

```typescript
// Call server tool
const result = await app.callServerTool({
  name: 'fetch_data',
  arguments: { query: 'test' }
});

// Call app-only tool
await app.callServerTool({
  name: 'refresh_data',
  arguments: {}
});
```

### 6. Sending Messages

```typescript
// Send follow-up prompt
await app.sendMessage({
  role: 'user',
  content: [{ type: 'text', text: 'Tell me more' }]
});

// Send log message
await app.sendLog({
  level: 'info',
  data: 'Widget loaded successfully'
});

// Open external link
await app.openLink({
  url: 'https://example.com'
});
```

## Common Patterns

### 1. Data Fetching on Mount

```typescript
useEffect(() => {
  const fetchData = async () => {
    try {
      const result = await app.callServerTool({
        name: 'get_data',
        arguments: { query }
      });
      setData(result);
    } catch (error) {
      setError(error.message);
    }
  };
  fetchData();
}, [app, query]);
```

### 2. Streaming Updates

```typescript
// Host sends partial updates
app.ontoolinputpartial = async (input) => {
  setPartialData(input.arguments);
};

// Complete update
app.ontoolinput = async (input) => {
  setCompleteData(input.arguments);
};
```

### 3. Cancellation Handling

```typescript
app.ontoolcancelled = (params) => {
  console.log('Cancelled:', params.reason);
  setIsCancelled(true);
};

// In component
if (isCancelled) {
  return <div>Operation cancelled</div>;
}
```

### 4. Experimental Requests

```typescript
// In View
import { sendExperimentalRequest } from '@mcp-ui/server';

const result = await sendExperimentalRequest('x/clipboard/write', {
  text: 'Copy this!'
});

// In Host
onFallbackRequest={async (request) => {
  if (request.method === 'x/clipboard/write') {
    await navigator.clipboard.writeText(request.params.text);
    return { success: true };
  }
}}
```

## Debugging

### Server-Side

```typescript
// Log resource registration
console.log('Registered resource:', resourceUri);

// Log tool calls
server.tool('my_tool', async (args) => {
  console.log('Tool called:', args);
  const result = await handler(args);
  console.log('Tool result:', result);
  return result;
});
```

### Client-Side

```typescript
// Enable debug logging
const app = new App({...}, { debug: true });

// Log all messages
window.addEventListener('message', (event) => {
  console.log('[UI Message]', event.data);
});
```

### Browser DevTools

```javascript
// In View console
console.log('Host context:', app.getHostContext());
console.log('Capabilities:', app.getServerCapabilities());
```

## Performance Optimization

### 1. Prefetching

```typescript
// Prefetch UI resource before tool execution
const resourceUri = tool._meta?.ui?.resourceUri;
if (resourceUri) {
  const html = await readToolUiResourceHtml(client, { uri: resourceUri });
  // Cache for later
}
```

### 2. Caching

```typescript
// Cache HTML resources
const cache = new Map<string, string>();

async function getCachedResource(uri: string) {
  if (!cache.has(uri)) {
    const resource = await client.readResource({ uri });
    cache.set(uri, resource.contents[0].text);
  }
  return cache.get(uri);
}
```

### 3. Lazy Loading

```typescript
// Load heavy libraries only when needed
async function loadChartLibrary() {
  if (!chartLib) {
    chartLib = await import('chart.js');
  }
  return chartLib;
}
```

## Testing

### Unit Tests

```typescript
describe('MCP App', () => {
  it('should initialize correctly', async () => {
    const app = new App({ name: 'test', version: '1.0.0' });
    expect(app).toBeDefined();
  });

  it('should handle tool calls', async () => {
    const result = await app.callServerTool({
      name: 'test_tool',
      arguments: {}
    });
    expect(result).toBeDefined();
  });
});
```

### Integration Tests

```typescript
describe('Integration', () => {
  it('should render UI correctly', async () => {
    // Set up test host
    const host = createTestHost();

    // Call tool with UI
    await host.callTool({
      name: 'show_widget',
      arguments: { query: 'test' }
    });

    // Verify UI rendered
    const iframe = document.querySelector('iframe');
    expect(iframe).toBeDefined();
  });
});
```

## Related Resources

- [CSP and CORS Guide](https://apps.extensions.modelcontextprotocol.io/api/documents/CSPandCORS.html)
- [Specification](https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/2026-01-26/apps.mdx)
- [Example Servers](https://github.com/modelcontextprotocol/ext-apps/tree/main/examples)
