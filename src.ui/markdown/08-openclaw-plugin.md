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

WebSocket client with challenge-response auth and exponential backoff:

```typescript
class GatewaySocket {
  private pendingRpcs = new Map<string, {
    resolve: (v: unknown) => void;
    reject: (e: Error) => void;
    method: string;
  }>();
  private rpcCounter = 0;

  async request<T>(method: string, params?: unknown): Promise<T> {
    const id = `rpc-${++this.rpcCounter}`;
    const frame: GatewayFrame = { type: "req", id, method, params };
    return new Promise<T>((resolve, reject) => {
      this.pendingRpcs.set(id, { resolve, reject, method });
      this.ws!.send(JSON.stringify(frame));
    });
  }

  handleMessage(raw: string): void {
    const frame = JSON.parse(raw) as GatewayFrame;
    if (frame.type === "event" && frame.event === "connect.challenge") {
      // Respond with nonce via RPC connect call
    } else if (frame.type === "res") {
      const pending = this.pendingRpcs.get(frame.id);
      if (frame.ok) pending.resolve(frame.payload);
      else pending.reject(new Error(frame.error));
    }
  }
}
```

### Auth Flow

1. Connect to WebSocket
2. Server sends `{ event: "connect.challenge", nonce: "..." }`
3. Client responds with RPC `connect` call including device token
4. Server replies with `hello-ok` handshake response
5. If auth fails, server closes with code 4001, 4003, or 4401 (fatal — don't reconnect)

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
