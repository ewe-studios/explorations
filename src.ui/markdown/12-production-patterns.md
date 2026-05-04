# OpenUI -- Production Patterns

This document covers production considerations for running the OpenUI ecosystem: streaming reliability, LLM error handling, WebSocket stability, storage durability, and scaling.

**Aha:** The most critical production consideration for generative UI is handling LLM hallucination. The LLM may emit unknown component names, missing required props, or malformed syntax. OpenUI handles this through three layers: (1) the streaming parser produces placeholder nodes for incomplete input rather than failing, (2) the materializer drops unknown components with error hints for the LLM, and (3) the ElementErrorBoundary shows the last good state when rendering fails. This three-layer defense means a bad LLM output degrades gracefully rather than crashing the UI.

## Streaming Reliability

### Chunk Handling

LLM responses arrive as SSE chunks. Each chunk may contain partial OpenUI Lang:

```
Chunk 1: "Stack\n  <Button label=\"S"
Chunk 2: "ave\" />\n  <Text>"
Chunk 3: "Hello</Text>\n"
```

The streaming parser handles this correctly:
1. Chunk 1 arrives → buffer = `Stack\n  <Button label="S` → no complete boundary → pending state
2. Chunk 2 arrives → buffer += `\n  <Text>` → found newline after Button → complete Button, pending Text
3. Chunk 3 arrives → buffer += `Hello</Text>\n` → found newline → complete Text

### Backpressure

When the LLM streams faster than the renderer can process, the browser's event queue acts as a backpressure buffer. However, for very fast streams, the parser may accumulate many incomplete states:

```typescript
// Debounce rapid pushes
let rafId: number | null = null;
function scheduleParse() {
  if (rafId === null) {
    rafId = requestAnimationFrame(() => {
      rafId = null;
      parse();
    });
  }
}
```

**Aha:** Using `requestAnimationFrame` for parse scheduling ensures parsing happens at most once per frame (~60fps). This prevents the parser from running hundreds of times per second during a fast stream, which would block the main thread and freeze the UI.

## LLM Error Handling

### Parser Errors

Parser errors are structured for LLM correction:

```typescript
{
  source: 'parser',
  code: 'UNCLOSED_PAREN',
  message: 'Expected `)` at position 15',
  hint: 'Close the parenthesis in the Button component props'
}
```

The hint is included in the LLM's next system prompt, allowing automatic correction.

### Runtime Errors

Runtime errors (unknown component, missing required prop) include the available options:

```typescript
{
  source: 'runtime',
  code: 'UNKNOWN_COMPONENT',
  message: 'Component "FooBar" not found',
  hint: 'Available components: Button, Stack, Text, Table, Card, ...'
}
```

### Mutation Errors

Action mutations halt on failure:

```typescript
for (const step of plan.steps) {
  try {
    await execute(step);
  } catch (e) {
    if (step.type === 'Mutation') {
      break;  // Halt remaining steps
    }
  }
}
```

**Aha:** Halting on mutation failure is a safety mechanism. If a database write fails, you don't want subsequent actions (like sending a notification) to execute based on the assumption that the write succeeded. The remaining steps are silently dropped — no error is shown to the user.

## WebSocket Stability

### Connection Lifecycle

| Event | Behavior |
|-------|----------|
| Connect | Challenge-response auth |
| Auth success | Ready for RPC |
| Auth failure | Fatal close (codes 4001, 4003, 4401) |
| Network error | Exponential backoff reconnect |
| Server restart | Reconnect with saved credentials |

### Message Ordering

WebSocket guarantees ordered delivery — messages arrive in the order sent. This is critical for chat: messages are displayed in the order they were sent by the LLM.

### Session Recovery

When the client reconnects after a disconnect:

1. Auth with stored device token
2. Request session history
3. Merge with locally buffered messages
4. Resume real-time streaming

## Storage Durability

### JSON File Atomicity

Only the NotificationStore uses atomic writes. The other stores (AppStore, ArtifactStore, UploadStore) write directly. This means:
- A crash during AppStore write → corrupted JSON file → app data lost for that write
- A crash during NotificationStore write → either old or new content visible, never partial

**Production recommendation:** Apply atomic writes to all stores. The temp + rename pattern is cheap and prevents data corruption.

### SQLite WAL Mode

Enable WAL mode for the per-namespace SQLite databases:

```sql
PRAGMA journal_mode=WAL;
```

WAL mode provides:
- Concurrent readers with a single writer
- Crash recovery via WAL file
- Better performance for write-heavy workloads

### localStorage Limits

Browser localStorage has a ~5-10MB limit per origin. The settings object is small (~200 bytes), so this is not a concern. However, if the client caches chat history in localStorage, the limit becomes relevant.

## Scaling

### Client-Side Scaling

The streaming parser scales with the number of LLM tokens:
- Parse cost per token: O(1) amortized (watermark mechanism)
- Re-render cost: O(n) where n = number of components rendered
- Store update cost: O(m) where m = number of subscribers

### Server-Side Scaling

The GatewaySocket server scales horizontally:
- Each WebSocket connection is tied to a single server instance
- Sticky sessions ensure the client reconnects to the same server
- Cross-server communication is handled through the shared SQLite database (for multi-instance setups, use a shared database backend like PostgreSQL)

## Security

### Input Validation

OpenUI Lang from the LLM is not trusted — it is parsed, validated, and materialized before rendering. Unknown components are dropped, and malformed syntax produces errors.

### Tool Sandboxing

Tools registered via the OpenClaw plugin run with limited permissions:
- `db_query` is read-only
- `db_execute` validates SQL (no `DROP`, `ALTER`)
- `exec` commands are sandboxed (timeout, restricted paths)

### WebSocket Auth

The challenge-response auth flow prevents unauthorized connections. The device token is per-device, allowing device management (list, revoke, rename).

See [Storage Patterns](10-storage-patterns.md) for storage details.
See [Gateway Socket](09-gateway-socket.md) for WebSocket protocol.
See [React Renderer](06-react-renderer.md) for error handling.
