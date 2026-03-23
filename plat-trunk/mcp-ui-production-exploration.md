---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/plat-trunk
repository: https://github.com/joeblew999/plat-trunk
explored_at: 2026-03-23T00:00:00Z
languages: Rust (WASM), TypeScript, Hono
---

# Plat-Trunk: MCP-UI Production Exploration

## Executive Summary

**plat-trunk** implements a **production-grade MCP (Model Context Protocol) integration** for a browser-based CAD platform, but does **NOT** use the official MCP Apps specification or @mcp-ui packages. Instead, it implements a **custom MCP bridge pattern** that achieves similar goals through a different architectural approach.

### Key Architectural Decisions

| Decision | plat-trunk Approach | Official MCP Apps Approach |
|----------|--------------------|---------------------------|
| UI Delivery | None (terminal-only MCP) | HTML over MCP Resources |
| MCP Transport | stdio bridge → HTTP | stdio or SSE |
| Tool Discovery | Schema polling + cache | `tools/list` with `_meta.ui` |
| Hot Reload | Schema version polling | `notifications/tools/list_changed` |
| Offline Support | Tool cache on disk | Embedded resources |
| Browser Integration | Playwright MCP (separate) | UI Actions over postMessage |

## Current MCP Implementation

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     AI Agent (Claude Code)                       │
│                          stdio MCP                               │
└────────────────────────────┬────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                    mcp-bridge.ts (stdio ↔ HTTP)                  │
│  - Instant stdio connection                                      │
│  - Lazy URL resolution (local → PR preview → fallback)           │
│  - Retry with exponential backoff (6 attempts)                   │
│  - Schema polling every 30s for hot-reload                       │
│  - Tool list caching to disk                                     │
│  - Bridge status tool (always available)                         │
└────────────────────────────┬────────────────────────────────────┘
                             │ HTTP POST /mcp
                             ▼
┌─────────────────────────────────────────────────────────────────┐
│                    plat-trunk Worker (Cloudflare)                │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  OpenAPIHono + MCP Server (@modelcontextprotocol/sdk)    │   │
│  │                                                           │   │
│  │  tools/list → 29 CAD tools + 4 model tools + 7 meta      │   │
│  │  tools/call → Queue command → SSE broadcast → Result     │   │
│  │                                                           │   │
│  │  Data-Plane Tools (server-direct):                        │   │
│  │    - cad_add_cube, cad_translate, cad_boolean_*           │   │
│  │    - cad_model_save, cad_model_load, cad_model_*          │   │
│  │                                                           │   │
│  │  Control-Plane Tools (browser-delegated):                 │   │
│  │    - cad_create_model, cad_delete_model                   │   │
│  │    - Returns "timeout" if browser not connected           │   │
│  └──────────────────────────────────────────────────────────┘   │
│                             │                                    │
│                             ▼                                    │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │  Headless WASM (truck-cad compiled for Worker)           │   │
│  │  - Executes geometry operations without rendering        │   │
│  │  - Returns Scene JSON result                             │   │
│  └──────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### MCP Bridge: Deep Dive

**Location:** `scripts/mcp-bridge.ts`

The bridge implements several sophisticated patterns:

#### 1. Instant stdio Connection (ADR-0020)

```typescript
async function start() {
  // Load cached tools BEFORE connecting stdio
  cachedTools = loadToolsCache();

  log(`Bridge starting (bun ${Bun.version})${cachedTools ? `, ${cachedTools.length} cached tools` : ', no cache'}`);

  // Connect stdio IMMEDIATELY — no blocking on HTTP
  await server.connect(transport);
  log('stdio transport connected');

  // URL resolution happens in background
  ensureUrl()
    .then(() => log(`Proxy → ${BASE_URL}/mcp`))
    .then(() => pollVersion())
    .then(() => prewarmToolsCache())
    .then(() => log(`Ready (${lastVersion || 'no schema yet'})`))
    .catch(() => log('Background URL resolution failed (will retry on request)'));

  setInterval(pollVersion, POLL_INTERVAL_MS);
}
```

**Why this matters:** MCP clients timeout if stdio doesn't connect quickly. By connecting stdio first and resolving the URL lazily, the bridge appears instantly available even when the backend is down.

#### 2. URL Resolution Strategy

```typescript
async function ensureUrl(): Promise<void> {
  if (urlResolved) return;

  // 1. Explicit override wins
  if (process.env.CAD_URL) {
    BASE_URL = process.env.CAD_URL;
    return;
  }

  // 2. Local dev server (fastest)
  if (await isReachable(LOCAL_URL)) {
    BASE_URL = LOCAL_URL;
    return;
  }

  // 3. PR preview URL
  const prUrl = detectPrPreviewUrl();
  if (prUrl && await isReachable(prUrl)) {
    BASE_URL = prUrl;
    return;
  }

  // 4. Fallback to local (retry handles it)
  BASE_URL = LOCAL_URL;
}
```

**Auto-detection flow:**
```
CAD_URL set? → Use it
    ↓ not set
Local dev running? → http://localhost:8788
    ↓ not running
PR branch with preview? → https://pr-{N}-truck-cad.gedw99.workers.dev
    ↓ no PR
Fallback → http://localhost:8788 (retry on request)
```

#### 3. Retry with Exponential Backoff

```typescript
async function proxy(body: any): Promise<any> {
  await ensureUrl();
  let lastError: Error | null = null;

  for (let attempt = 0; attempt < RETRY_ATTEMPTS; attempt++) {
    try {
      const res = await fetch(`${BASE_URL}/mcp`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(body),
        signal: AbortSignal.timeout(30_000),
      });
      if (res.status === 202) return { result: {} };
      return await res.json();
    } catch (err: any) {
      lastError = err;
      if (attempt < RETRY_ATTEMPTS - 1) {
        const delay = RETRY_BASE_MS * Math.pow(2, attempt);
        log(`retry ${attempt + 1}/${RETRY_ATTEMPTS} in ${delay}ms...`);
        await new Promise(r => setTimeout(r, delay));
      }
    }
  }
  throw new Error(`Worker unreachable after ${RETRY_ATTEMPTS} attempts`);
}
```

**Retry schedule:**
- Attempt 1: Immediate
- Attempt 2: 1s delay
- Attempt 3: 2s delay
- Attempt 4: 4s delay
- Attempt 5: 8s delay
- Attempt 6: 16s delay

Total max wait: ~31 seconds

#### 4. Schema Polling for Hot-Reload

```typescript
let lastVersion = '';

async function pollVersion() {
  try {
    const res = await fetch(`${BASE_URL}/api/cad/schema`, {
      signal: AbortSignal.timeout(3_000)
    });
    const schema = await res.json() as any;
    const changed = lastVersion && schema.version !== lastVersion;

    if (changed) {
      log(`Schema ${lastVersion} → ${schema.version}, notifying client...`);
      await server.notification({ method: 'notifications/tools/list_changed' });
    }
    lastVersion = schema.version;
  } catch {
    // Server might be down — retry later
  }
}

setInterval(pollVersion, POLL_INTERVAL_MS); // Every 30 seconds
```

**How hot-reload works:**
1. Bridge polls `/api/cad/schema` every 30s
2. Compares `schema.version` to cached version
3. On change, sends `notifications/tools/list_changed` to MCP client
4. AI client re-fetches tools with `tools/list`
5. New tools immediately available

#### 5. Tool List Caching

```typescript
const TOOLS_CACHE_FILE = join(LOG_DIR, 'tools-cache.json');

function loadToolsCache(): any[] | null {
  try {
    if (existsSync(TOOLS_CACHE_FILE)) {
      const data = JSON.parse(readFileSync(TOOLS_CACHE_FILE, 'utf8'));
      if (Array.isArray(data) && data.length > 1) {
        log(`Loaded ${data.length} tools from cache`);
        return data;
      }
    }
  } catch { /* cache corrupt — ignore */ }
  return null;
}

function saveToolsCache(tools: any[]) {
  try {
    mkdirSync(LOG_DIR, { recursive: true });
    writeFileSync(TOOLS_CACHE_FILE, JSON.stringify(tools));
    log(`Cached ${tools.length} tools to disk`);
  } catch { /* can't write — not fatal */ }
}
```

**Cache flow:**
```
Bridge starts
    ↓
Load cache from disk → If valid, return immediately (worker offline OK)
    ↓
First tools/list call
    ↓
Proxy to worker → Get live tools → Save to cache
    ↓
Return [BRIDGE_STATUS_TOOL, ...workerTools]
```

#### 6. Bridge Status Tool

```typescript
const BRIDGE_STATUS_TOOL = {
  name: 'cad_bridge_status',
  description: 'Check bridge connectivity to the CAD Worker. Always available.',
  inputSchema: { type: 'object', properties: {} },
};

async function getBridgeStatus(): Promise<any> {
  let workerReachable = false;
  let toolsCount = 0;

  try {
    const res = await fetch(`${BASE_URL}/api/health`);
    if (res.ok) {
      workerReachable = true;
      const health = await res.json();
      toolsCount = cachedTools?.length || 0;
    }
  } catch (err: any) {
    lastError = err?.message;
  }

  return {
    content: [{
      type: 'text',
      text: JSON.stringify({
        bridge: 'connected',
        worker_url: BASE_URL,
        worker_reachable: workerReachable,
        tools_count: toolsCount,
        tools_source: workerReachable ? 'live' : (cachedTools ? 'cache' : 'none'),
        uptime_ms: Date.now() - startedAt,
      }, null, 2),
    }],
  };
}
```

**Output example:**
```json
{
  "bridge": "connected",
  "worker_url": "http://localhost:8788",
  "worker_reachable": true,
  "schema_version": "2026-03-22-truck",
  "tools_count": 40,
  "tools_source": "live",
  "uptime_ms": 3600000
}
```

### MCP Server in Worker: Deep Dive

**Location:** `systems/truck/worker/src/index.ts`

#### Tool Registration Pattern

The worker uses schema-driven tool registration:

```typescript
// From cad-schema.json (generated from Rust)
const schema = {
  commands: {
    add_cube: {
      description: "Add a cube to the scene",
      params: {
        type: "object",
        properties: {
          size: { type: "number", description: "Cube size" },
          modelId: { type: "string" }
        },
        required: ["size"]
      },
      ephemeral: false,
      readonly: false
    },
    select: {
      description: "Select objects (ephemeral UI state)",
      ephemeral: true,  // NOT exposed as MCP tool
      readonly: true
    }
  }
};

// Register non-ephemeral, non-readonly commands as MCP tools
for (const [cmd, def] of Object.entries(schema.commands)) {
  if (!def.ephemeral && !def.readonly) {
    registerMcpTool(`cad_${cmd}`, def);
  }
}
```

#### MCP Tool Categories

| Category | Tools | Description |
|----------|-------|-------------|
| **Geometry** | `cad_add_cube`, `cad_add_sphere`, `cad_add_cylinder` | Add primitives |
| **Transform** | `cad_translate`, `cad_rotate`, `cad_scale` | Transform objects |
| **Boolean** | `cad_boolean_union`, `cad_boolean_subtract`, `cad_boolean_intersect` | CSG operations |
| **Model** | `cad_model_save`, `cad_model_load`, `cad_model_list`, `cad_model_delete` | R2 persistence |
| **Meta** | `cad_health`, `cad_schema`, `cad_wasm_health` | System tools |
| **Docs** | `cad_docs_*` | Documentation tools |

#### Control-Plane vs Data-Plane

**Data-Plane (server-direct):**
```typescript
// Executed immediately in Worker via headless WASM
const result = await executeHeadless('add_cube', { size: 2, modelId });
// Returns: { id: 'uuid', status: 'done', result: {...} }
```

**Control-Plane (browser-delegated):**
```typescript
// Requires browser to be connected via SSE
// If no browser, returns timeout error
if (!model.sseClientCount) {
  return { status: 'timeout', error: 'Browser did not respond' };
}

// Broadcast to SSE clients
broadcast(modelId, {
  type: 'cad-command',
  data: { id: commandId, command }
});

// Wait for result (max 10s)
const result = await waitForCommandResult(modelId, commandId);
```

### Playwright MCP Integration

**Location:** `scripts/playwright-mcp-claude.config.json`

plat-trunk also integrates Playwright MCP for browser automation:

```json
{
  "mcpServers": {
    "truck-cad": {
      "type": "stdio",
      "command": "bun",
      "args": ["./scripts/mcp-bridge.ts"]
    },
    "playwright": {
      "type": "stdio",
      "command": "bunx",
      "args": [
        "@playwright/mcp@latest",
        "--config",
        "./scripts/playwright-mcp-claude.config.json"
      ]
    }
  }
}
```

**Playwright capabilities:**
- Navigate to CAD app
- Click buttons, fill forms
- Take screenshots
- Extract page content

**Combined usage pattern:**
```
1. Claude uses truck-cad MCP to create 3D model
2. Claude uses Playwright MCP to:
   - Navigate to model viewer
   - Take screenshot
   - Verify rendering
```

## Production Usage Patterns

### Current Usage Flow

```bash
# 1. Start dev server
mise run dev

# 2. Claude Code automatically uses MCP bridge
claude

# In Claude session:
/health              # Check bridge status
cad_add_cube size=2  # Add cube to scene
cad_model_save       # Persist to R2
```

### Tool Call Flow

```
User (Claude Code)
    ↓ "cad_add_cube size=2"
stdio MCP
    ↓
mcp-bridge.ts
    ↓ POST /mcp (tools/call)
plat-trunk Worker
    ↓ executeHeadless('add_cube', { size: 2 })
truck-wasm (headless)
    ↓ Scene JSON
Worker
    ↓ Write to Automerge CRDT in R2
    ↓ Broadcast via SSE
    ↓ Return result
mcp-bridge.ts
    ↓
stdio MCP
    ↓
User sees: "Cube added successfully"
```

### Hot Development Flow

```
1. Developer changes Rust command definition
       ↓
2. `bun run build:truck` regenerates cad-schema.json
       ↓
3. Worker reloads schema (auto-reload in dev)
       ↓
4. Bridge polls /api/cad/schema (every 30s)
       ↓
5. Bridge detects version change
       ↓
6. Bridge sends tools/list_changed notification
       ↓
7. Claude re-fetches tools
       ↓
8. New tool available without restart
```

## What's Missing

### 1. No UI-over-MCP Implementation

**Current state:** MCP tools are terminal-only (text in/text out).

**What's missing:** Interactive UI panels in AI clients that show:
- 3D scene viewer
- Object properties editor
- Model gallery with thumbnails
- Real-time operation progress

**Comparison to MCP Apps:**

| Feature | MCP Apps Spec | plat-trunk Current |
|---------|--------------|-------------------|
| UI Resource | `ui://` URI + HTML | None |
| Rendering | Sandboxed iframe | N/A |
| UI Actions | postMessage tool calls | N/A |
| CSP | Declarative via `_meta.ui.csp` | N/A |

**Recommendation:** Implement MCP Apps pattern for 3D viewer:

```typescript
// In worker/index.ts MCP handler
const viewerResource = {
  uri: 'ui://cad/viewer',
  mimeType: 'text/html;profile=mcp-app',
  text: `
    <html>
    <body>
      <div id="canvas"></div>
      <script type="module">
        import { App } from '@modelcontextprotocol/ext-apps';
        const app = new App({ name: 'cad-viewer', version: '1.0.0' });

        // Subscribe to scene updates
        app.ontoolinput = async (input) => {
          renderScene(input.arguments.sceneJson);
        };

        // Send tool calls back
        document.getElementById('addCube').onclick = () => {
          app.callServerTool({ name: 'cad_add_cube', arguments: { size: 1 } });
        };

        await app.connect(transport);
      </script>
    </body>
    </html>
  `,
  _meta: {
    ui: {
      csp: {
        connectDomains: ['https://cad.ubuntusoftware.net'],
        resourceDomains: ['https://cdn.jsdelivr.net']
      }
    }
  }
};

// Register resource
registerAppResource(server, 'cad_viewer', viewerResource.uri, {}, async () => ({
  contents: [viewerResource]
}));

// Reference from tool
registerAppTool(server, 'cad_view_scene', {
  _meta: { ui: { resourceUri: 'ui://cad/viewer' } }
}, handler);
```

### 2. No Resource Discovery

**Current state:** Tools are discovered via `tools/list`, but no resources.

**What's missing:**
- Model thumbnails as resources
- Scene previews
- Documentation resources

**Recommendation:**

```typescript
// Model thumbnail resource
const thumbnailResource = {
  uri: `ui://model/${modelId}/thumbnail`,
  mimeType: 'image/png',
  blob: await generateThumbnail(modelId)  // Base64 PNG
};

// Register per-model resources
registerAppResource(server, `thumbnail_${modelId}`, thumbnailResource.uri, {}, async () => ({
  contents: [thumbnailResource]
}));
```

### 3. No Tool Visibility Control

**Current state:** All tools visible to both model and UI.

**What's missing:** `_meta.ui.visibility` to control tool visibility:

```typescript
// Model-only tool (hidden from UI)
registerAppTool(server, 'cad_internal_validate', {
  _meta: { ui: { visibility: ['model'] } }
}, handler);

// UI-only tool (refresh button, hidden from model)
registerAppTool(server, 'cad_refresh_view', {
  _meta: { ui: { visibility: ['app'] } }
}, handler);
```

### 4. No Structured Error Recovery

**Current state:** Errors returned as text strings.

**What's missing:** Structured error types with recovery suggestions:

```typescript
// Current
return { error: 'WASM initialization failed' };

// Recommended
return {
  isError: true,
  content: [{
    type: 'text',
    text: JSON.stringify({
      error_type: 'WASM_INIT_FAILED',
      error_message: 'WASM module failed to initialize',
      recovery: {
        suggestion: 'Reload the page or run cad_wasm_health',
        retryable: true,
        related_tools: ['cad_wasm_health']
      }
    })
  }]
};
```

### 5. No Conversation Context Persistence

**Current state:** Each tool call is stateless.

**What's missing:**
- Conversation history for undo/redo
- Session-aware operations
- Context-aware suggestions

**Recommendation:**

```typescript
interface ConversationContext {
  modelId: string;
  recentOperations: Array<{ type: string; params: any; result: any }>;
  userPreferences: { defaultSize: number; defaultMaterial: string };
}

const contextStore = new Map<string, ConversationContext>();

// In tool handler
const context = contextStore.get(sessionId) || createContext();
context.recentOperations.push({ type, params, result });
contextStore.set(sessionId, context);
```

### 6. No Streaming Progress

**Current state:** Operations return when complete (no progress updates).

**What's missing:** Streaming progress for long operations:

```typescript
// Current: Single response after completion
return { result: { status: 'done', scene } };

// Recommended: Progress updates via SSE
async function* executeWithProgress(modelId: string, command: any) {
  yield { type: 'progress', percent: 0, message: 'Starting...' };

  const result = await executeLongOperation(command);

  yield { type: 'progress', percent: 50, message: 'Processing...' };
  yield { type: 'progress', percent: 100, message: 'Complete' };
  yield { type: 'result', ...result };
}
```

### 7. No Multi-Model Context

**Current state:** Single `modelId` per session (lastActiveModelId).

**What's missing:**
- Multi-model operations
- Model references across calls
- Batch operations

**Recommendation:**

```typescript
// Batch tool for multi-step operations
registerAppTool(server, 'cad_batch_execute', {
  description: 'Execute multiple commands atomically',
  inputSchema: {
    modelId: z.string(),
    commands: z.array(z.object({
      type: z.string(),
      params: z.any()
    }))
  }
}, async ({ modelId, commands }) => {
  const results = [];
  for (const cmd of commands) {
    results.push(await execute(modelId, cmd));
  }
  return { results };
});
```

### 8. No Authorization on MCP Endpoint

**Current state:** MCP endpoint is open (no auth required).

```typescript
// In worker/index.ts
MCP_AUTH_ENABLED: string; // Env var, but not enforced

// Current check (if enabled):
if (env.MCP_AUTH_ENABLED === 'true') {
  // TODO: Verify auth header
}
```

**What's missing:**
- Session token verification via AUTH service binding
- Rate limiting per session
- Audit logging for tool calls

**Recommendation:**

```typescript
// MCP auth middleware
async function verifyMCPAuth(request: Request, env: Bindings) {
  const authHeader = request.headers.get('Authorization');
  if (!authHeader?.startsWith('Bearer ')) {
    return { valid: false, error: 'Missing auth header' };
  }

  const token = authHeader.slice(7);
  const authResponse = await env.AUTH.fetch(
    new Request(`https://auth/auth/verify`, {
      headers: { Authorization: `Bearer ${token}` }
    })
  );

  if (!authResponse.ok) {
    return { valid: false, error: 'Invalid token' };
  }

  const session = await authResponse.json();
  return { valid: true, userId: session.user_id };
}
```

### 9. No Metrics/Observability

**Current state:** Basic console logging.

**What's missing:**
- Tool call metrics (latency, error rates)
- Usage analytics per tool
- Distributed tracing

**Recommendation:**

```typescript
// Add to mcp-bridge.ts
const metrics = {
  toolCalls: new Map<string, { count: number; avgLatency: number }>(),

  recordToolCall(toolName: string, latencyMs: number, success: boolean) {
    const metric = this.toolCalls.get(toolName) || { count: 0, avgLatency: 0 };
    metric.count++;
    metric.avgLatency = (metric.avgLatency * (metric.count - 1) + latencyMs) / metric.count;
    this.toolCalls.set(toolName, metric);

    // Send to observability backend
    fetch('https://metrics.internal/record', {
      method: 'POST',
      body: JSON.stringify({ toolName, latencyMs, success })
    });
  }
};
```

### 10. No MCP Inspector Integration

**Current state:** Manual debugging via log files.

**What's missing:**
- MCP Inspector compatibility
- Debug mode with verbose logging
- Tool call replay

## What We Can Add

### 1. MCP Apps UI Integration

Implement UI-over-MCP for 3D scene viewer:

**File:** `systems/truck/web/mcp-viewer.ts` (new)

```typescript
import { App } from '@modelcontextprotocol/ext-apps';

export class MCPViewer {
  private app: App;
  private canvas: HTMLCanvasElement;

  constructor() {
    this.app = new App({ name: 'cad-viewer', version: '1.0.0' });
    this.canvas = document.getElementById('canvas') as HTMLCanvasElement;
  }

  async init() {
    // Handle scene updates
    this.app.ontoolinput = async (input) => {
      const { sceneJson } = input.arguments;
      await this.renderScene(sceneJson);
    };

    // Handle tool result
    this.app.ontoolresult = async (result) => {
      if (result.content?.[0]?.text) {
        const parsed = JSON.parse(result.content[0].text);
        await this.updateScene(parsed.scene);
      }
    };

    // Handle host context (theme changes)
    this.app.onhostcontextchanged = (ctx) => {
      this.setTheme(ctx.theme);
    };

    await this.app.connect(transport);
  }

  private async renderScene(sceneJson: string) {
    // Use existing Three.js/WebGPU renderer
    const scene = JSON.parse(sceneJson);
    renderToCanvas(scene, this.canvas);
  }

  private setTheme(theme: 'light' | 'dark') {
    document.documentElement.setAttribute('data-theme', theme);
  }
}
```

**HTML Resource:** `systems/truck/web/mcp-viewer.html`

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <title>CAD Viewer</title>
  <style>
    html, body { margin: 0; padding: 0; width: 100%; height: 100%; }
    #canvas { width: 100%; height: 100%; }
    .controls {
      position: absolute;
      bottom: 20px;
      left: 20px;
      display: flex;
      gap: 10px;
    }
    button {
      padding: 8px 16px;
      border: none;
      border-radius: 4px;
      background: var(--color-background-primary);
      color: var(--color-text-primary);
      cursor: pointer;
    }
  </style>
</head>
<body>
  <canvas id="canvas"></canvas>
  <div class="controls">
    <button id="addCube">Add Cube</button>
    <button id="addSphere">Add Sphere</button>
    <button id="save">Save Model</button>
  </div>
  <script type="module" src="/mcp-viewer.ts"></script>
</body>
</html>
```

### 2. Model Thumbnail Resources

Generate thumbnails as MCP resources:

**File:** `systems/truck/worker/src/thumbnail-resource.ts` (new)

```typescript
import { generateThumbnailFromScene } from './thumbnail';

export async function createThumbnailResource(
  modelId: string,
  sceneJson: string
) {
  const thumbnail = await generateThumbnailFromScene(sceneJson);

  return {
    uri: `ui://model/${modelId}/thumbnail`,
    mimeType: 'image/png',
    blob: thumbnail,  // Base64 encoded PNG
    _meta: {
      ui: {
        csp: {
          resourceDomains: ['https://cad.ubuntusoftware.net']
        }
      }
    }
  };
}

// Register in MCP handler
registerAppResource(
  server,
  `thumbnail_${modelId}`,
  `ui://model/${modelId}/thumbnail`,
  { description: `Thumbnail for model ${modelId}` },
  async () => ({
    contents: [await createThumbnailResource(modelId, sceneJson)]
  })
);
```

### 3. Enhanced Bridge Status Tool

Add detailed diagnostics:

```typescript
const ENHANCED_STATUS_TOOL = {
  name: 'cad_bridge_diagnostics',
  description: 'Comprehensive bridge and worker diagnostics',
  inputSchema: {
    type: 'object',
    properties: {
      includeMetrics: { type: 'boolean', default: false },
      includeLogs: { type: 'boolean', default: false }
    }
  }
};

async function getDiagnostics(params: any) {
  const diagnostics = {
    bridge: {
      status: 'connected',
      url: BASE_URL,
      uptime_ms: Date.now() - startedAt,
      retry_count: retryCount,
      last_error: lastError
    },
    worker: {
      reachable: workerReachable,
      schema_version: lastVersion,
      tools_count: cachedTools?.length || 0,
      health: await fetchHealth()
    },
    cache: {
      tools_cached: cachedTools?.length || 0,
      cache_file_exists: existsSync(TOOLS_CACHE_FILE),
      cache_age_ms: getCacheAge()
    }
  };

  if (params.includeMetrics) {
    diagnostics.metrics = await fetchMetrics();
  }

  if (params.includeLogs) {
    diagnostics.recent_logs = getRecentLogs(50);
  }

  return {
    content: [{
      type: 'text',
      text: JSON.stringify(diagnostics, null, 2)
    }]
  };
}
```

### 4. Structured Error Handling

```typescript
class MCPToolError extends Error {
  constructor(
    public errorType: string,
    public message: string,
    public recovery?: {
      suggestion: string;
      retryable: boolean;
      related_tools: string[];
    }
  ) {
    super(message);
  }

  toMCPResponse() {
    return {
      isError: true,
      content: [{
        type: 'text',
        text: JSON.stringify({
          error_type: this.errorType,
          error_message: this.message,
          recovery: this.recovery
        })
      }]
    };
  }
}

// Usage in tool handler
try {
  const result = await executeCommand(command);
  return { content: [{ type: 'text', text: JSON.stringify(result) }] };
} catch (err) {
  if (err instanceof WASMError) {
    throw new MCPToolError(
      'WASM_EXECUTION_FAILED',
      err.message,
      {
        suggestion: 'Run cad_wasm_health to diagnose',
        retryable: false,
        related_tools: ['cad_wasm_health']
      }
    );
  }
  throw err;
}
```

### 5. Authorization Middleware

```typescript
// systems/truck/worker/src/mcp-auth.ts (new)

export async function verifyMCPAuth(
  request: Request,
  env: Bindings
): Promise<{ valid: boolean; userId?: string; error?: string }> {
  const authHeader = request.headers.get('Authorization');

  if (!authHeader?.startsWith('Bearer ')) {
    return { valid: false, error: 'Missing or invalid Authorization header' };
  }

  const token = authHeader.slice(7);

  try {
    // Call auth-worker via service binding
    const authResponse = await env.AUTH.fetch(
      new Request('https://auth/api/auth/verify', {
        headers: { Authorization: `Bearer ${token}` }
      })
    );

    if (!authResponse.ok) {
      return { valid: false, error: 'Token verification failed' };
    }

    const session = await authResponse.json();
    return { valid: true, userId: session.user_id };
  } catch (err) {
    console.error('Auth error:', err);
    return { valid: false, error: 'Auth service unavailable' };
  }
}

// Apply in MCP handler
if (env.MCP_AUTH_ENABLED === 'true') {
  const auth = await verifyMCPAuth(request, env);
  if (!auth.valid) {
    return new Response(JSON.stringify({
      jsonrpc: '2.0',
      error: { code: -32600, message: auth.error },
      id: null
    }), { status: 401 });
  }
}
```

## Production Readiness Assessment

### What Works Well (Production Strengths)

1. **Resilient Connection Handling**
   - Instant stdio init prevents MCP client timeouts
   - Retry with exponential backoff survives server restarts
   - URL auto-detection (local → PR preview → fallback)

2. **Hot-Reload Support**
   - Schema polling every 30s
   - `tools/list_changed` notification
   - No AI client restart needed

3. **Offline Tolerance**
   - Tool list cached to disk
   - Bridge status tool always available
   - Graceful degradation when worker is down

4. **Developer Experience**
   - Simple `mise run dev` startup
   - Automatic WASM rebuild on Rust changes
   - Comprehensive logging

### What Needs Work (Production Gaps)

1. **No UI Integration** - Terminal-only MCP in 2026 is a significant limitation. AI agents benefit greatly from visual feedback, especially for 3D/CAD applications.

2. **No Authorization** - MCP endpoint is open. For production, this is a critical security gap.

3. **No Metrics** - No visibility into tool call patterns, latency, or error rates in production.

4. **No Structured Errors** - Text errors are hard to parse programmatically or provide actionable guidance.

5. **No Streaming Progress** - Long operations (boolean ops on complex meshes) appear to hang.

### Security Assessment

| Concern | Current State | Risk Level |
|---------|---------------|------------|
| MCP Auth | Optional env var, not enforced | **HIGH** |
| Rate Limiting | None | **MEDIUM** |
| Audit Logging | Basic console logs | **MEDIUM** |
| Input Validation | Schema-based | **LOW** |
| CORS | Hono CORS on HTTP endpoints | **LOW** |

## Recommendations

### Immediate (P0 - Security)

1. **Enforce MCP authorization** when `MCP_AUTH_ENABLED=true`
2. **Add rate limiting** per session/IP
3. **Enable audit logging** for all tool calls

### Short Term (P1 - UX)

1. **Implement MCP Apps UI** for 3D viewer
2. **Add model thumbnail resources**
3. **Implement structured error handling**
4. **Add streaming progress for long operations**

### Medium Term (P2 - Observability)

1. **Integrate metrics collection** (latency, error rates)
2. **Add MCP Inspector compatibility**
3. **Implement conversation context persistence**
4. **Add multi-model batch operations**

## Conclusion

plat-trunk's MCP implementation is **architecturally sound** with sophisticated patterns for:
- Resilient stdio ↔ HTTP bridging
- Hot-reload via schema polling
- Offline tolerance via disk caching

However, it **lags behind MCP Apps specification** in:
- UI delivery (no visual feedback for AI agents)
- Resource discovery (no thumbnails, previews)
- Tool visibility control (no model vs UI separation)

For production deployment, the **critical gaps** are:
1. Authorization/enforcement on MCP endpoint
2. UI integration for 3D visualization
3. Observability/metrics for production monitoring

The foundation is excellent, and implementing the recommended additions would make this a **best-in-class MCP integration** for CAD/3D applications.
