# ext-apps Deep Dive

## Overview

The `ext-apps` repository is the official reference implementation for MCP Apps (SEP-1865). It contains the specification, TypeScript SDK, documentation, and example applications.

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MCP/ext-apps`

## Repository Structure

```
ext-apps/
├── specification/
│   ├── 2026-01-26/          # Stable specification
│   │   └── apps.mdx
│   └── draft/               # Working draft
│       └── apps.mdx
├── docs/                    # Documentation
│   ├── overview.md
│   ├── quickstart.md
│   ├── patterns.md
│   ├── authorization.md
│   ├── csp-cors.md
│   ├── agent-skills.md
│   └── testing-mcp-apps.md
├── examples/                # Example applications
│   ├── basic-host/          # Reference host implementation
│   ├── basic-server-react/  # React server template
│   ├── basic-server-vue/    # Vue server template
│   ├── basic-server-svelte/ # Svelte server template
│   ├── ...                  # 20+ example servers
│   └── run-all.ts           # Run all examples
├── src/                     # SDK source (if applicable)
├── plugins/                 # Agent Skills plugins
└── package.json
```

## Specification (2026-01-26)

### Key Sections

1. **UI Resource Format** - Structure of `ui://` resources
2. **Resource Discovery** - How tools reference UI resources
3. **Communication Protocol** - JSON-RPC over postMessage
4. **Sandbox Proxy** - Double-iframe architecture
5. **Host Context** - Theme, locale, viewport info
6. **Theming** - CSS custom properties
7. **Display Modes** - inline, fullscreen, pip
8. **Security Implications** - Threat model and mitigations

### Extension Identifier

```
io.modelcontextprotocol/ui
```

### UI Resource Interface

```typescript
interface UIResource {
  uri: string;  // MUST start with 'ui://'
  name: string;
  description?: string;
  mimeType: 'text/html;profile=mcp-app';
  _meta?: {
    ui?: {
      csp?: McpUiResourceCsp;
      permissions?: {
        camera?: {};
        microphone?: {};
        geolocation?: {};
        clipboardWrite?: {};
      };
      domain?: string;
      prefersBorder?: boolean;
    }
  }
}
```

### CSP Interface

```typescript
interface McpUiResourceCsp {
  connectDomains?: string[];    // fetch/XHR/WebSocket
  resourceDomains?: string[];   // scripts, styles, images
  frameDomains?: string[];      // nested iframes
  baseUriDomains?: string[];    // document base URI
}
```

## TypeScript SDK

### Package Structure

```
@modelcontextprotocol/ext-apps
├── app/               # App class for Views
├── app-bridge/        # Host-side bridge
├── server/            # Server helpers
└── react/             # React hooks
```

### App Class (View-side)

```typescript
import { App } from '@modelcontextprotocol/ext-apps';

const app = new App({
  name: 'my-view',
  version: '1.0.0'
}, {
  onAppCreated: (app) => {
    app.ontoolresult = async (result) => { /* handle result */ };
    app.onhostcontextchanged = (params) => { /* handle theme change */ };
    app.onteardown = async () => { /* cleanup */ };
  }
});

await app.connect(transport);
```

### React Hooks

```typescript
import { useApp } from '@modelcontextprotocol/ext-apps/react';

function MyView() {
  const { app, error } = useApp({
    appInfo: { name: 'My View', version: '1.0.0' },
    capabilities: {},
    onAppCreated: (app) => {
      app.ontoolresult = async (result) => setData(result);
      app.onhostcontextchanged = (params) => setTheme(params.theme);
    }
  });

  if (!app) return <div>Connecting...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return <ViewContent app={app} />;
}
```

### Server Helpers

```typescript
import { registerAppTool, registerAppResource } from '@modelcontextprotocol/ext-apps/server';

// Register tool with UI
registerAppTool(server, 'show_widget', {
  description: 'Show widget',
  inputSchema: { query: z.string() },
  _meta: {
    ui: { resourceUri: 'ui://widget' }
  }
}, handler);

// Register resource
registerAppResource(server, 'widget', 'ui://widget', {}, async () => ({
  contents: [{
    uri: 'ui://widget',
    mimeType: RESOURCE_MIME_TYPE,
    text: widgetHtml
  }]
}));
```

### AppBridge (Host-side)

```typescript
import { AppBridge, PostMessageTransport } from '@modelcontextprotocol/ext-apps/app-bridge';

const bridge = new AppBridge(
  client,
  { name: 'host', version: '1.0.0' },
  {
    openLinks: {},
    serverTools: { listChanged: true },
    serverResources: {}
  }
);

await bridge.connect(
  new PostMessageTransport(iframe.contentWindow, iframe.contentWindow)
);
```

## Example Applications

### basic-host

**Purpose:** Reference host implementation

**Key files:**
- `src/index.tsx` - Host entry point
- `src/sandbox.ts` - Sandbox proxy implementation
- `src/implementation.ts` - MCP client setup

**Features:**
- Double-iframe sandbox
- CSP enforcement
- Origin validation
- Message relay

### basic-server-react

**Purpose:** Minimal React server template

**Structure:**
```
basic-server-react/
├── src/
│   ├── mcp-app.tsx    # React View component
│   └── vite-env.d.ts
├── main.ts            # App entry
├── server.ts          # MCP server
├── package.json
└── vite.config.ts
```

### Complex Examples

| Server | Description | Key Features |
|--------|-------------|--------------|
| **map-server** | 3D globe viewer | CesiumJS, connectDomains, resourceDomains |
| **threejs-server** | 3D scene renderer | Three.js, WebGL |
| **shadertoy-server** | GLSL shaders | Real-time rendering |
| **sheet-music-server** | ABC notation | Music rendering, CSP config |
| **wiki-explorer-server** | Link graph | D3.js visualization |
| **pdf-server** | PDF viewer | Chunked loading, binary resources |
| **cohort-heatmap-server** | Analytics | Chart.js, data visualization |
| **scenario-modeler-server** | Business projections | Financial modeling |

### Running Examples

```bash
# Clone repo
git clone https://github.com/modelcontextprotocol/ext-apps.git
cd ext-apps

# Install dependencies
npm install

# Run all examples
npm start

# Open browser
open http://localhost:8080
```

### Running Individual Examples

```bash
# Run map server
cd examples/map-server
npm install
npm start

# Add to MCP client config
{
  "mcpServers": {
    "map": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-map", "--stdio"]
    }
  }
}
```

## Agent Skills

### Available Skills

| Skill | Purpose |
|-------|---------|
| `create-mcp-app` | Scaffold new MCP App |
| `migrate-oai-app` | Convert OpenAI Apps to MCP |
| `add-app-to-server` | Add UI to existing MCP server |
| `convert-web-app` | Convert web app to MCP App |

### Installation (Claude Code)

```
/plugin marketplace add modelcontextprotocol/ext-apps
/plugin install mcp-apps@modelcontextprotocol-ext-apps
```

### Usage

```
/create-mcp-app
"Migrate from OpenAI Apps SDK"
/add-app-to-server
/convert-web-app
```

## Documentation

### overview.md

- Why MCP Apps
- Architecture diagram
- Lifecycle sequence
- Use cases

### quickstart.md

- Step-by-step guide
- Basic server setup
- Basic host setup
- Testing

### patterns.md

- Tool visibility patterns
- CSP configuration
- Theming patterns
- Error handling
- State management

### authorization.md

- OAuth flows
- Token management
- CORS configuration
- Stable origins

### csp-cors.md

- CSP declaration
- CORS requirements
- Domain allowlisting
- Development vs production

### testing-mcp-apps.md

- Unit testing
- Integration testing
- Debugging tips
- Common issues

## Key Implementation Details

### Sandbox Proxy (sandbox.ts)

```typescript
// Origin validation
const ALLOWED_REFERRER_PATTERN = /^http:\/\/(localhost|127\.0\.0\.1)(:|\/|$)/;

// Security self-test
try {
  window.top!.alert("If you see this, sandbox is broken");
  throw "FAIL";
} catch (e) {
  // Expected: SecurityError
}

// Message relay
window.addEventListener("message", (event) => {
  if (event.source === window.parent) {
    if (event.origin !== EXPECTED_HOST_ORIGIN) return;
    inner.contentWindow.postMessage(event.data, "*");
  } else if (event.source === inner.contentWindow) {
    if (event.origin !== OWN_ORIGIN) return;
    window.parent.postMessage(event.data, EXPECTED_HOST_ORIGIN);
  }
});
```

### Resource MIME Type

```typescript
const RESOURCE_MIME_TYPE = 'text/html;profile=mcp-app';
```

### Tool-UI Linkage

```typescript
// Deprecated (flat structure)
_meta: {
  "ui/resourceUri": "ui://widget"
}

// Current (nested structure)
_meta: {
  ui: {
    resourceUri: "ui://widget"
  }
}
```

## Package.json Scripts

```json
{
  "scripts": {
    "start": "ts-node examples/run-all.ts",
    "build": "tsc",
    "test": "vitest",
    "lint": "eslint src/"
  }
}
```

## Dependencies

```json
{
  "dependencies": {
    "@modelcontextprotocol/sdk": "^1.0.0",
    "react": "^18.0.0",
    "zod": "^3.0.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "vitest": "^1.0.0",
    "eslint": "^8.0.0"
  }
}
```

## Related Resources

- [Specification](https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/2026-01-26/apps.mdx)
- [API Documentation](https://apps.extensions.modelcontextprotocol.io/api/)
- [SEP-1865 Discussion](https://github.com/modelcontextprotocol/modelcontextprotocol/pull/1865)
