# Executor Hosts — Deep Dive Exploration

**Package:** `@executor/hosts`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/packages/hosts`  
**Total Hosts:** 1 host (MCP server)  
**Total Files:** 4 files  

---

## 1. Module Overview

The Hosts package provides **runtime environments** for the Executor system. Currently includes:

- **mcp** — Model Context Protocol server host for AI assistant integration

### Key Responsibilities

1. **MCP Server** — Expose Executor tools via MCP protocol
2. **Elicitation Bridge** — Forward user interaction requests to MCP client
3. **Tool Registration** — Register `execute` and `resume` tools with MCP
4. **Capability Detection** — Adapt behavior based on client capabilities
5. **Result Formatting** — Convert execution results to MCP format

---

## 2. File Inventory

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/server.ts` | 208 | MCP server factory with elicitation handling |
| 2 | `src/index.ts` | 2 | Public exports |
| 3 | `src/server.test.ts` | — | Server tests |
| 4 | `vitest.config.ts` | — | Test configuration |

---

## 3. Key Exports

### Server Factory

```typescript
// server.ts
export const createExecutorMcpServer = async (
  config: ExecutorMcpServerConfig,
): Promise<McpServer> => {
  const engine = "engine" in config ? config.engine : createExecutionEngine(config);
  const description = await engine.getDescription();

  const server = new McpServer(
    { name: "executor", version: "1.0.0" },
    { capabilities: { tools: {} } },
  );

  // Register tools...
  return server;
};
```

### Elicitation Handler

```typescript
const makeMcpElicitationHandler = (server: McpServer): ElicitationHandler =>
  (ctx: ElicitationContext): Effect.Effect<ElicitationResponse> =>
    Effect.promise(async () => {
      const params = elicitationRequestToParams(ctx.request);
      const response = await server.server.elicitInput(params);
      return { action: response.action, content: response.content };
    });
```

### Capability Detection

```typescript
const supportsManagedElicitation = (server: McpServer): boolean => {
  const capabilities = server.server.getClientCapabilities();
  if (capabilities === undefined || !capabilities.elicitation) return false;
  const elicitation = capabilities.elicitation as Record<string, unknown>;
  return Boolean(elicitation.form) && Boolean(elicitation.url);
};
```

---

## 4. Line-by-Line Analysis

### MCP Server Creation (`server.ts:116-125`)

```typescript
export const createExecutorMcpServer = async (
  config: ExecutorMcpServerConfig,
): Promise<McpServer> => {
  const engine = "engine" in config ? config.engine : createExecutionEngine(config);
  const description = await engine.getDescription();

  const server = new McpServer(
    { name: "executor", version: "1.0.0" },
    { capabilities: { tools: {} } },
  );
```

**Key patterns:**
1. **Flexible config** — Accepts engine instance or config object
2. **Server metadata** — Name and version for MCP protocol
3. **Tool capability** — Declares tools support to clients

### Execute Tool Registration (`server.ts:156-163`)

```typescript
const executeTool = server.registerTool(
  "execute",
  {
    description,
    inputSchema: { code: z.string().trim().min(1) },
  },
  async ({ code }) => executeCode(code),
);
```

**Key patterns:**
1. **Zod validation** — Input schema with trim and min length
2. **Simple interface** — Single `code` string parameter
3. **Async handler** — Returns Promise for MCP result format

### Resume Tool Registration (`server.ts:165-191`)

```typescript
const resumeTool = server.registerTool(
  "resume",
  {
    description: [
      "Resume a paused execution using the executionId returned by execute.",
      "Never call this without user approval unless they explicitly state otherwise.",
    ].join("\n"),
    inputSchema: {
      executionId: z.string().describe("The execution ID from the paused result"),
      action: z.enum(["accept", "decline", "cancel"]).describe("How to respond to the interaction"),
      content: z.string().describe("Optional JSON-encoded response content for form elicitations").default("{}"),
    },
  },
  async ({ executionId, action, content: rawContent }) => {
    const content = parseJsonContent(rawContent);
    const result = await engine.resume(executionId, { action, content });
    // ...
  },
);
```

**Key patterns:**
1. **Instruction in description** — Guides AI assistant behavior
2. **Action enum** — Restricted to accept/decline/cancel
3. **Content parsing** — JSON string to object for form responses

### Elicitation Mode Detection (`server.ts:127-139`)

```typescript
const executeCode = async (code: string): Promise<McpToolResult> => {
  if (supportsManagedElicitation(server)) {
    // Managed mode: handle elicitation within MCP
    const result = await engine.execute(code, {
      onElicitation: makeMcpElicitationHandler(server),
    });
    return toMcpResult(formatExecuteResult(result));
  }

  // Fallback mode: return paused state for manual resume
  const outcome = await engine.executeWithPause(code);
  return outcome.status === "completed"
    ? toMcpResult(formatExecuteResult(outcome.result))
    : toMcpPausedResult(formatPausedExecution(outcome.execution));
};
```

**Key patterns:**
1. **Capability check** — Detect client elicitation support
2. **Managed mode** — Handle elicitation inline via MCP protocol
3. **Fallback mode** — Return paused state requiring resume tool

### MCP Result Formatting (`server.ts:91-110`)

```typescript
type McpToolResult = {
  content: Array<{ type: "text"; text: string }>;
  structuredContent?: Record<string, unknown>;
  isError?: boolean;
};

const toMcpResult = (
  formatted: ReturnType<typeof formatExecuteResult>,
): McpToolResult => ({
  content: [{ type: "text", text: formatted.text }],
  structuredContent: formatted.structured,
  isError: formatted.isError || undefined,
});

const toMcpPausedResult = (
  formatted: ReturnType<typeof formatPausedExecution>,
): McpToolResult => ({
  content: [{ type: "text", text: formatted.text }],
  structuredContent: formatted.structured,
});
```

**Key patterns:**
1. **Text content** — Human-readable output
2. **Structured content** — Machine-readable result
3. **Error flag** — Indicate tool failure

### Tool Visibility Sync (`server.ts:195-205`)

```typescript
const syncToolAvailability = () => {
  executeTool.enable();
  if (supportsManagedElicitation(server)) {
    resumeTool.disable();
  } else {
    resumeTool.enable();
  }
};

syncToolAvailability();
server.server.oninitialized = syncToolAvailability;
```

**Key patterns:**
1. **Capability-based visibility** — Hide resume if managed elicitation supported
2. **Dynamic enabling** — Tools can be enabled/disabled at runtime
3. **Re-initialization handler** — Re-check on client reconnect

### Elicitation Request to Params (`server.ts:42-61`)

```typescript
const elicitationRequestToParams: (request: ElicitationRequest) => ElicitInputParams =
  Match.type<ElicitationRequest>().pipe(
    Match.tag("UrlElicitation", (req) => ({
      mode: "url" as const,
      message: req.message,
      url: req.url,
      elicitationId: req.elicitationId,
    })),
    Match.tag("FormElicitation", (req) => ({
      message: req.message,
      requestedSchema:
        Object.keys(req.requestedSchema).length === 0
          ? { type: "object" as const, properties: {} }
          : req.requestedSchema,
    })),
    Match.exhaustive,
  );
```

**Key patterns:**
1. **Effect Match** — Type-safe pattern matching
2. **URL elicitation** — Opens browser for OAuth-style flows
3. **Form elicitation** — Schema-based form rendering
4. **Empty schema handling** — Minimal schema for approval-only requests

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         MCP Host Server                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    createExecutorMcpServer()                         │   │
│  │                                                                       │   │
│  │  1. Create ExecutionEngine                                           │   │
│  │  2. Initialize McpServer                                             │   │
│  │  3. Register tools:                                                  │   │
│  │     - execute(code: string) → result                                 │   │
│  │     - resume(executionId, action, content) → result                  │   │
│  │  4. Setup elicitation handler                                        │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Capability Detection                               │   │
│  │                                                                       │   │
│  │  supportsManagedElicitation(server)                                  │   │
│  │    │                                                                 │   │
│  │    ├──> true:  Use onElicitation handler (inline)                   │   │
│  │    └──> false: Use executeWithPause + resume (two-step)            │   │
│  │                                                                       │   │
│  │  Tool visibility:                                                    │   │
│  │    - Managed elicitation → disable resume tool                       │   │
│  │    - Fallback mode → enable resume tool                              │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Result Formatting                                  │   │
│  │                                                                       │   │
│  │  formatExecuteResult(result) → { text, structured, isError }        │   │
│  │    │                                                                 │   │
│  │    └──> toMcpResult() → { content, structuredContent, isError }     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### Execute Tool Flow (Managed Elicitation)

```
MCP Client: call execute(code)
    │
    ▼
┌─────────────────────────┐
│  executeTool handler    │
│  executeCode(code)      │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Check client caps      │
│  supportsManagedElicit? │
└───────────┬─────────────┘
            │ YES
            ▼
┌─────────────────────────┐
│  engine.execute(code,   │
│    onElicitation: ...)  │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Elicitation needed?    │
└───────────┬─────────────┘
      ┌─────┴─────┐
      │           │
     NO          YES
      │           │
      ▼           ▼
┌─────────┐  ┌─────────────────────────┐
│ Return  │  │ makeMcpElicitationHandler│
│ result  │  │   │                      │
└────┬────┘  │   └──> server.elicitInput()
     │       │        │                   │
     │       │        ▼                   │
     │       │   MCP Client shows UI     │
     │       │        │                   │
     │       └────────┴───────────────────┘
     │                  │
     ▼                  ▼
┌─────────────────────────────────┐
│  toMcpResult(formatResult)      │
│  → { content, structuredContent }│
└───────────┬─────────────────────┘
            │
            ▼
MCP Response to Client
```

### Execute Tool Flow (Fallback Mode)

```
MCP Client: call execute(code)
    │
    ▼
┌─────────────────────────┐
│  executeCode(code)      │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Check client caps      │
│  supportsManagedElicit? │
└───────────┬─────────────┘
            │ NO
            ▼
┌─────────────────────────┐
│  engine.executeWithPause│
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  Status: completed?     │
└───────────┬─────────────┘
      ┌─────┴─────┐
      │           │
    YES          NO (paused)
      │           │
      ▼           ▼
┌─────────┐  ┌─────────────────────────┐
│ Return  │  │ Return paused state     │
│ result  │  │ → { executionId, ... }  │
└─────────┘  └─────────────────────────┘
              │
              │ User must call resume
              ▼
MCP Client: call resume(executionId, action, content)
    │
    ▼
┌─────────────────────────┐
│  engine.resume()        │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│  toMcpResult(result)    │
└───────────┬─────────────┘
            │
            ▼
MCP Response to Client
```

---

## 7. Key Patterns

### Capability Detection

```typescript
const supportsManagedElicitation = (server: McpServer): boolean => {
  const capabilities = server.server.getClientCapabilities();
  if (capabilities === undefined || !capabilities.elicitation) return false;
  return Boolean(capabilities.elicitation.form) && 
         Boolean(capabilities.elicitation.url);
};
```

**Benefits:**
1. **Graceful degradation** — Works with any MCP client
2. **Optimal UX** — Uses advanced features when available
3. **Future-proof** — Adapts to client capabilities

### Dynamic Tool Visibility

```typescript
const syncToolAvailability = () => {
  executeTool.enable();
  resumeTool.disable(); // Or enable based on caps
};

server.server.oninitialized = syncToolAvailability;
```

**Benefits:**
1. **Clean interface** — Only show relevant tools
2. **Runtime adaptation** — Re-check on reconnect
3. **Prevent errors** — Hide unavailable operations

### Elicitation Pattern Matching

```typescript
const elicitationRequestToParams = Match.type<ElicitationRequest>().pipe(
  Match.tag("UrlElicitation", (req) => ({ /* ... */ })),
  Match.tag("FormElicitation", (req) => ({ /* ... */ })),
  Match.exhaustive,
);
```

**Benefits:**
1. **Type safety** — All cases handled
2. **Exhaustiveness** — Compile-time guarantee
3. **Clear separation** — Different handling per type

### JSON Content Parsing

```typescript
const parseJsonContent = (raw: string): Record<string, unknown> | undefined => {
  if (raw === "{}") return undefined;
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return undefined;
  }
  return typeof parsed === "object" && parsed !== null && !Array.isArray(parsed)
    ? parsed
    : undefined;
};
```

**Benefits:**
1. **Defensive parsing** — Handle invalid JSON gracefully
2. **Type guard** — Ensure object type
3. **Empty optimization** — Skip empty object parsing

---

## 8. Integration Points

### Dependencies

| Package | Purpose |
|---------|---------|
| `@modelcontextprotocol/sdk` | MCP server implementation |
| `@executor/execution` | Execution engine |
| `@executor/sdk` | Elicitation types |
| `effect` | Effect, Match utilities |
| `zod` | Input validation |

### Dependents

| Package | Relationship |
|---------|-------------|
| `@executor/apps/server` | Creates MCP server for remote access |
| `@executor/hosts/mcp` | Re-exports for CLI usage |

---

## 9. Error Handling

### Elicitation Fallback

```typescript
try {
  const response = await server.server.elicitInput(params);
  return { action: response.action, content: response.content };
} catch (err) {
  console.error("[executor] elicitInput failed — falling back to cancel.");
  return { action: "cancel" };
}
```

**Strategy:** Fail-safe default to prevent hanging executions.

### Resume Error

```typescript
const result = await engine.resume(executionId, { action, content });
if (!result) {
  return {
    content: [{ type: "text", text: `No paused execution: ${executionId}` }],
    isError: true,
  };
}
```

**Strategy:** Return error message for invalid execution IDs.

---

## 10. Testing Strategy

### Server Tests (`server.test.ts`)

Tests cover:
1. Server creation with config object
2. Server creation with engine instance
3. Execute tool registration
4. Resume tool registration
5. Capability detection
6. Elicitation handler

---

## 11. Design Decisions

### Why Separate Execute and Resume Tools?

1. **MCP model** — Each tool call is independent
2. **User control** — Explicit approval before resuming
3. **Stateless server** — Execution state in engine, not tool

### Why Capability Detection?

1. **Client diversity** — Different MCP clients have different capabilities
2. **Optimal UX** — Use managed elicitation when available
3. **Backward compatible** — Fallback for older clients

### Why Dynamic Tool Visibility?

1. **Reduce confusion** — Hide irrelevant tools
2. **Prevent errors** — Can't call disabled tools
3. **Adaptive** — Responds to client capabilities

---

## 12. MCP Session Management (`server/mcp.ts`)

The MCP session management is implemented in the server package's `mcp.ts` file, which handles HTTP-based MCP sessions.

### Session Handler Factory (`mcp.ts:22-90`)

```typescript
export const createMcpRequestHandler = (
  config: ExecutorMcpServerConfig,
): McpRequestHandler => {
  const transports = new Map<string, WebStandardStreamableHTTPServerTransport>();
  const servers = new Map<string, McpServer>();

  const dispose = async (
    id: string,
    opts: { transport?: boolean; server?: boolean } = {},
  ) => {
    const t = transports.get(id);
    const s = servers.get(id);
    transports.delete(id);
    servers.delete(id);
    if (opts.transport) await t?.close().catch(() => undefined);
    if (opts.server) await s?.close().catch(() => undefined);
  };

  return {
    handleRequest: async (request) => {
      const sessionId = request.headers.get("mcp-session-id");

      if (sessionId) {
        // Existing session - route to existing transport
        const transport = transports.get(sessionId);
        if (!transport) return jsonError(404, -32001, "Session not found");
        return transport.handleRequest(request);
      }

      // New session - create transport and server
      let created: McpServer | undefined;
      const transport = new WebStandardStreamableHTTPServerTransport({
        sessionIdGenerator: () => crypto.randomUUID(),
        enableJsonResponse: true,
        onsessioninitialized: (sid) => {
          transports.set(sid, transport);
          if (created) servers.set(sid, created);
        },
        onsessionclosed: (sid) => void dispose(sid, { server: true }),
      });

      transport.onclose = () => {
        const sid = transport.sessionId;
        if (sid) void dispose(sid, { server: true });
      };

      try {
        created = await createExecutorMcpServer(config);
        await created.connect(transport);
        const response = await transport.handleRequest(request);

        if (!transport.sessionId) {
          await transport.close().catch(() => undefined);
          await created.close().catch(() => undefined);
        }
        return response;
      } catch (error) {
        if (!transport.sessionId) {
          await transport.close().catch(() => undefined);
          await created?.close().catch(() => undefined);
        }
        return jsonError(500, -32603, 
          error instanceof Error ? error.message : "Internal server error");
      }
    },

    close: async () => {
      const ids = new Set([...transports.keys(), ...servers.keys()]);
      await Promise.all([...ids].map((id) => 
        dispose(id, { transport: true, server: true })
      ));
    },
  };
};
```

**Key patterns:**
1. **Session maps** — Separate maps for transports and servers by session ID
2. **Session lifecycle** — `onsessioninitialized` and `onsessionclosed` hooks
3. **Cleanup on close** — Both transport and server closed when session ends
4. **Error responses** — JSON-RPC error format with standard error codes

### JSON-RPC Error Helper (`mcp.ts:16-20`)

```typescript
const jsonError = (status: number, code: number, message: string): Response =>
  new Response(
    JSON.stringify({ 
      jsonrpc: "2.0", 
      error: { code, message }, 
      id: null 
    }),
    { 
      status, 
      headers: { "content-type": "application/json" } 
    },
  );
```

**Standard error codes:**
- `-32001` — Session not found
- `-32603` — Internal server error

### Stdio Transport (`mcp.ts:96-122`)

```typescript
export const runMcpStdioServer = async (
  config: ExecutorMcpServerConfig
): Promise<void> => {
  const server = await createExecutorMcpServer(config);
  const transport = new StdioServerTransport();

  const waitForExit = () =>
    new Promise<void>((resolve) => {
      const finish = () => {
        process.off("SIGINT", finish);
        process.off("SIGTERM", finish);
        process.stdin.off("end", finish);
        process.stdin.off("close", finish);
        resolve();
      };
      process.once("SIGINT", finish);
      process.once("SIGTERM", finish);
      process.stdin.once("end", finish);
      process.stdin.once("close", finish);
    });

  try {
    await server.connect(transport);
    await waitForExit();
  } finally {
    await transport.close().catch(() => undefined);
    await server.close().catch(() => undefined);
  }
};
```

**Key patterns:**
1. **Stdio transport** — For MCP over stdin/stdout (Claude Desktop integration)
2. **Signal handling** — SIGINT, SIGTERM, stdin end/close
3. **Cleanup** — Transport and server closed on exit

---

## 13. Summary

The MCP Host package provides **MCP server integration** for the Executor:

1. **Two tools** — `execute` for running code, `resume` for paused executions
2. **Capability detection** — Adapts to client elicitation support
3. **Elicitation bridge** — Forward user interaction via MCP protocol
4. **Dynamic visibility** — Show/hide tools based on capabilities
5. **Result formatting** — Convert execution results to MCP format

The MCP host enables **AI assistant integration** while maintaining **flexibility** for different client capabilities.
