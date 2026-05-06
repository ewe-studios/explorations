# OpenUI -- OpenClaw Plugin Integration

The OpenClaw UI integration is a two-part system: a server-side plugin that extends the OpenClaw agent's capabilities with generative UI tools, and a Next.js web client that communicates via WebSocket RPC.

**Aha:** The plugin detection is elegantly simple: it checks if the `sessionKey` ends with `:openclaw-ui`. If so, it prepends the CLAW_PREAMBLE to the system prompt via the `before_prompt_build` hook. No configuration file, no separate setup — the session key suffix triggers the entire generative UI mode. The agent automatically gets access to UI-building tools without any explicit opt-in.

Source: `openclaw-ui/packages/claw-plugin/src/index.ts` — server-side plugin
Source: `openclaw-ui/packages/claw-client/src/` — Next.js web client

## Server-Side Plugin

### Plugin Detection

```typescript
// Defined via definePluginEntry({ id, name, description, configSchema, register })
export default definePluginEntry({
  id: "openclaw-ui-plugin",
  name: "Claw — OpenUI for OpenClaw",
  configSchema: emptyPluginConfigSchema,

  register(api) {
    // Only activate for sessions from the Claw client
    api.on("before_prompt_build", (_event, ctx) => {
      if (!ctx.sessionKey?.endsWith(":openclaw-ui")) return;
      return { prependSystemContext: CLAW_PREAMBLE };
    });

    // Register tools (artifacts, apps, SQLite, notifications, uploads)
    api.registerTool('create_markdown_artifact', ...);
    api.registerTool('db_query', ...);
    api.registerTool('app_create', ...);
  }
});
```

### Tool Registration

| Tool | Purpose | Storage |
|------|---------|---------|
| `create_markdown_artifact` | Create a markdown artifact with version history | ArtifactStore (JSON files) |
| `update_markdown_artifact` | Update existing artifact with version | ArtifactStore |
| `get_artifact` | Retrieve artifact by ID | ArtifactStore |
| `list_artifacts` | List artifacts with kind filter | ArtifactStore |
| `db_query` | Read-only SQLite query | Per-namespace SQLite DB |
| `db_execute` | Write SQLite operation | Per-namespace SQLite DB |
| `app_create` | Create OpenUI app with linting | AppStore (JSON files) |
| `get_app` | Retrieve app by ID | AppStore |
| `app_update` | Update app with version history | AppStore |

### Gateway RPC Methods

The plugin registers Gateway RPC methods:

| Method | Purpose |
|--------|---------|
| `artifacts.*` | Create, get, list, update artifacts |
| `apps.*` | Create, get, update apps |
| `uploads.*` | Upload, get, list files |
| `notifications.*` | Create, get, list notifications |
| `tools.invoke` | Invoke a registered tool |

### exec/read Tool Proxying

Apps can call `exec`, `read`, `db_query`, and `db_execute` through a proxy:

```typescript
// App running in the browser calls:
gatewaySocket.invoke('tools.invoke', {
  tool: 'db_query',
  args: { sql: 'SELECT * FROM users' }
});
```

The server validates the SQL (read-only for `db_query`) and executes against the per-namespace SQLite database.

## Client-Side Web Client

### GatewaySocket

Source: `openclaw-ui/packages/claw-client/src/lib/gateway/socket.ts`

WebSocket client with:

```typescript
class GatewaySocket {
  // Connect with challenge handshake
  async connect() {
    const ws = new WebSocket(this.url);
    ws.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      if (msg.type === 'challenge') {
        // Respond with device token
        this.send({ type: 'auth', token: this.deviceToken });
      } else if (msg.type === 'response') {
        // Resolve pending RPC request
        const pending = this.pendingRequests.get(msg.id);
        pending.resolve(msg.result);
      }
    };
  }

  // RPC with pending map
  async invoke(method: string, params: any): Promise<any> {
    const id = generateId();
    const promise = new Promise((resolve, reject) => {
      this.pendingRequests.set(id, { resolve, reject });
    });
    this.send({ type: 'request', id, method, params });
    return promise;
  }

  // Exponential backoff reconnect
  // 1s → 2s → 4s → 8s → 16s → 30s (max, 6 attempts)
}
```

### Auth Flow

1. Connect to WebSocket
2. Receive `challenge` message with nonce
3. Respond with device token
4. If auth fails, close with code 4001, 4003, or 4401 (fatal — don't reconnect)

### OpenClawEngine

Source: `openclaw-ui/packages/claw-client/src/lib/engines/openclaw/OpenClawEngine.ts`

High-level engine over GatewaySocket:

| Feature | Method |
|---------|--------|
| Session management | `createSession()`, `deleteSession()`, `renameSession()`, `compact()` |
| Thread history | `loadHistory()` |
| Model selection | `setModel()` |
| Messaging | `sendMessage()` |
| Cron management | `listCrons()`, `updateCron()`, `runCron()`, `removeCron()` |
| Notification sync | `syncNotifications()` |

### Auto-Title Derivation

The engine auto-generates session titles from the first user message:

```typescript
function deriveTitle(messages: Message[]): string {
  const firstUser = messages.find(m => m.role === 'user');
  if (firstUser) {
    return firstUser.content.slice(0, 50);  // First 50 chars
  }
  return 'New session';
}
```

### Cron Polling

```typescript
// Poll every 30s, only when tab is visible
const interval = setInterval(() => {
  if (document.visibilityState === 'visible') {
    engine.syncNotifications();
  }
}, 30000);
```

**Aha:** Cron polling respects tab visibility — it doesn't poll when the tab is hidden. This saves server resources and battery life. The 30-second interval is a balance between responsiveness and server load.

See [Gateway Socket](09-gateway-socket.md) for the WebSocket protocol.
See [Storage Patterns](10-storage-patterns.md) for the JSON file and SQLite storage.
See [React Renderer](06-react-renderer.md) for the UI rendering.
