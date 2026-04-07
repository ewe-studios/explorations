# Executor Kernel — Deep Dive Exploration

**Package:** `@executor/kernel`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/packages/kernel`  
**Total Files:** 14 TypeScript files across 4 sub-packages  
**Total Lines:** ~2,000 lines  

---

## 1. Module Overview

The Kernel package provides the **code execution runtime** for the Executor system. It includes:

- **Core types** — Standard Schema-based tool definitions
- **Intermediate representation** — Tool catalog serialization
- **QuickJS runtime** — Sandboxed JavaScript execution via WASM
- **Deno subprocess runtime** — Full Node-compatible execution

### Key Responsibilities

1. **Tool Abstraction** — Standard Schema-based tool interface
2. **Sandboxed Execution** — Safe code execution in isolated runtimes
3. **Tool Invocation Bridging** — Connect sandbox code to external tools
4. **Resource Limits** — Timeout, memory, and stack size constraints

---

## 2. File Inventory

### kernel/core/ (5 files, ~200 lines)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/types.ts` | ~60 | Core type definitions |
| 2 | `src/validation.ts` | ~50 | Input validation |
| 3 | `src/json-schema.ts` | ~40 | JSON Schema utilities |
| 4 | `src/effect-errors.ts` | ~30 | Effect error types |
| 5 | `src/index.ts` | ~5 | Public exports |

### kernel/ir/ (3 files, ~150 lines)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/registry.ts` | ~60 | Tool registration types |
| 2 | `src/serialize.ts` | ~50 | Serialization logic |
| 3 | `src/index.ts` | ~10 | Public exports |

### kernel/runtime-quickjs/ (2 files, ~450 lines)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/index.ts` | ~430 | QuickJS executor implementation |
| 2 | `src/index.test.ts` | ~20 | QuickJS tests |

### kernel/runtime-deno-subprocess/ (4 files, ~600 lines)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/index.ts` | ~350 | Deno subprocess executor |
| 2 | `src/deno-worker-process.ts` | ~150 | Subprocess spawning |
| 3 | `src/index.test.ts` | ~80 | Deno tests |
| 4 | `vitest.config.ts` | ~7 | Test configuration |

---

## 3. Key Exports

### Core Types (`types.ts`)

```typescript
/** Branded tool path */
export type ToolPath = string & { readonly __toolPath: unique symbol };

export const asToolPath = (value: string): ToolPath => value as ToolPath;

/** A tool that can be invoked */
export interface Tool {
  readonly path: ToolPath;
  readonly description?: string;
  readonly inputSchema: StandardSchema;
  readonly outputSchema?: StandardSchema;
  readonly execute: (input: unknown) => unknown | Promise<unknown>;
}

/** Invoke a tool by path from inside a sandbox */
export interface SandboxToolInvoker {
  invoke(input: {
    path: string;
    args: unknown;
  }): Effect.Effect<unknown, unknown>;
}

/** Result of executing code in a sandbox */
export type ExecuteResult = {
  result: unknown;
  error?: string;
  logs?: string[];
};

/** Executes code in a sandboxed runtime with tool access */
export interface CodeExecutor {
  execute(
    code: string,
    toolInvoker: SandboxToolInvoker,
  ): Effect.Effect<ExecuteResult, unknown>;
}
```

### QuickJS Executor (`runtime-quickjs/src/index.ts`)

```typescript
export type QuickJsExecutorOptions = {
  readonly timeoutMs?: number;
  readonly memoryLimitBytes?: number;
  readonly maxStackSizeBytes?: number;
};

export const makeQuickJsExecutor = (
  options: QuickJsExecutorOptions = {},
): CodeExecutor => ({
  execute: (code: string, toolInvoker: SandboxToolInvoker) =>
    runInQuickJs(options, code, toolInvoker),
});
```

### Deno Subprocess Executor (`runtime-deno-subprocess/src/index.ts`)

```typescript
export type DenoPermissions = {
  readonly net?: "read" | "write" | "none";
  readonly env?: "read" | "none";
  readonly sys?: "read" | "none";
  readonly ffi?: "read" | "none";
};

export type DenoSubprocessExecutorOptions = {
  readonly denoExecutable?: string;
  readonly timeoutMs?: number;
  readonly permissions?: DenoPermissions;
};

export const makeDenoSubprocessExecutor = (
  options: DenoSubprocessExecutorOptions = {},
): CodeExecutor => ({
  execute: (code: string, toolInvoker: SandboxToolInvoker) =>
    executeInDeno(code, toolInvoker, options),
});
```

---

## 4. Line-by-Line Analysis

### QuickJS Execution Source Builder (`runtime-quickjs/src/index.ts:97-156`)

```typescript
const buildExecutionSource = (code: string): string => {
  const trimmed = code.trim();
  const looksLikeArrowFunction =
    (trimmed.startsWith("async") || trimmed.startsWith("("))
    && trimmed.includes("=>");

  const body = looksLikeArrowFunction
    ? [
        `const __fn = (${trimmed});`,
        "if (typeof __fn !== 'function') throw new Error('Code must evaluate to a function');",
        "return await __fn();",
      ].join("\n")
    : code;

  return [
    '"use strict";',
    "const __invokeTool = __executor_invokeTool;",
    "const __log = __executor_log;",
    "try { delete globalThis.__executor_invokeTool; } catch {}",
    "try { delete globalThis.__executor_log; } catch {}",
    // Console bridge
    "const __formatLogArg = (value) => { ... };",
    "const __formatLogLine = (args) => args.map(__formatLogArg).join(' ');",
    // Tools proxy
    "const __makeToolsProxy = (path = []) => new Proxy(() => undefined, {",
    "  get(_target, prop) {",
    "    if (prop === 'then' || typeof prop === 'symbol') return undefined;",
    "    return __makeToolsProxy([...path, String(prop)]);",
    "  },",
    "  apply(_target, _thisArg, args) {",
    "    const toolPath = path.join('.');",
    "    if (!toolPath) throw new Error('Tool path missing');",
    "    return Promise.resolve(__invokeTool(toolPath, args[0]))",
    "      .then((raw) => raw === undefined ? undefined : JSON.parse(raw));",
    "  },",
    "});",
    "const tools = __makeToolsProxy();",
    // Disabled fetch
    "const fetch = (..._args) => { throw new Error('fetch is disabled'); };",
    // Execute user code
    "(async () => {", body, "})()",
  ].join("\n");
};
```

**Key patterns:**

1. **Arrow function detection** — Wraps arrow functions to enforce function requirement
2. **Tools proxy** — Lazy proxy for `tools.github.issues.list()` style calls
3. **Console bridge** — Captures `console.log` to logs array
4. **Fetch disabled** — Prevents network access outside tool invocations
5. **Strict mode** — Enforces strict JavaScript semantics

### QuickJS Tool Bridge (`runtime-quickjs/src/index.ts:195-240`)

```typescript
const createToolBridge = (
  context: QuickJSContext,
  toolInvoker: SandboxToolInvoker,
  pendingDeferreds: Set<QuickJSDeferredPromise>,
): QuickJSHandle =>
  context.newFunction("__executor_invokeTool", (pathHandle, argsHandle) => {
    const path = context.getString(pathHandle);
    const args = argsHandle === undefined || context.typeof(argsHandle) === "undefined"
      ? undefined
      : context.dump(argsHandle);
    
    const deferred = context.newPromise();
    pendingDeferreds.add(deferred);
    deferred.settled.finally(() => pendingDeferreds.delete(deferred));

    // Bridge Effect → Promise
    void Effect.runPromise(toolInvoker.invoke({ path, args })).then(
      (value) => {
        if (!deferred.alive) return;
        const serialized = serializeJson(value, `Tool result for ${path}`);
        if (typeof serialized === "undefined") {
          deferred.resolve();
          return;
        }
        const valueHandle = context.newString(serialized);
        deferred.resolve(valueHandle);
        valueHandle.dispose();
      },
      (cause) => {
        if (!deferred.alive) return;
        const errorHandle = context.newError(toErrorMessage(cause));
        deferred.reject(errorHandle);
        errorHandle.dispose();
      },
    );

    return deferred.handle;
  });
```

**Key patterns:**

1. **Promise bridging** — Effect.Effect → QuickJS Promise
2. **Deferred tracking** — Tracks pending promises for async completion
3. **JSON serialization** — Tool results must be JSON-serializable
4. **Resource cleanup** — Disposes handles to prevent memory leaks

### QuickJS Async Draining (`runtime-quickjs/src/index.ts:287-302`)

```typescript
const drainAsync = async (
  context: QuickJSContext,
  runtime: QuickJSRuntime,
  pendingDeferreds: ReadonlySet<QuickJSDeferredPromise>,
  deadlineMs: number,
  timeoutMs: number,
): Promise<void> => {
  drainJobs(context, runtime, deadlineMs, timeoutMs);

  while (pendingDeferreds.size > 0) {
    await waitForDeferreds(pendingDeferreds, deadlineMs, timeoutMs);
    drainJobs(context, runtime, deadlineMs, timeoutMs);
  }

  drainJobs(context, runtime, deadlineMs, timeoutMs);
};
```

**Purpose:** Processes async tool invocations until all promises settle or timeout.

### Deno Subprocess Execution (`runtime-deno-subprocess/src/index.ts:166-328`)

```typescript
const executeInDeno = (
  code: string,
  toolInvoker: SandboxToolInvoker,
  options: DenoSubprocessExecutorOptions,
): Effect.Effect<ExecuteResult, never> =>
  Effect.gen(function* () {
    const rt = yield* Effect.runtime<never>();
    const runSync = Runtime.runSync(rt);

    // Queue bridges Node callbacks → Effect fibers
    const messages = yield* Queue.unbounded<WorkerToHostMessage>();
    const result = yield* Deferred.make<ExecuteResult>();

    const completeWith = (value: ExecuteResult): Effect.Effect<boolean> =>
      Deferred.complete(result, Effect.succeed(value));

    // Spawn subprocess
    const worker = yield* Effect.try({
      try: () => spawnDenoWorkerProcess(
        {
          executable: denoExecutable,
          scriptPath: workerScriptPath(),
          permissions: options.permissions,
        },
        {
          onStdoutLine: (rawLine) => {
            const line = rawLine.trim();
            if (!line.startsWith(IPC_PREFIX)) return;

            const decoded = Schema.decodeUnknownOption(WorkerMessage)(
              JSON.parse(line.slice(IPC_PREFIX.length)),
            );
            if (decoded._tag === "Some") {
              runSync(Queue.offer(messages, decoded.value));
            }
          },
          onError: (cause) => {
            runSync(completeWith({
              result: null,
              error: new DenoSpawnError({ executable: denoExecutable, reason: cause }).message,
            }));
          },
          onExit: (exitCode, signal) => {
            runSync(completeWith({
              result: null,
              error: `Deno subprocess exited unexpectedly (code=${exitCode} signal=${signal})`,
            }));
          },
        },
      ),
      catch: (cause) => new DenoSpawnError({ executable: denoExecutable, reason: cause }),
    });

    // Send code to subprocess
    writeMessage(worker.stdin, { type: "start", code });

    // Timeout handling
    const timer = setTimeout(() => {
      worker.dispose();
      runSync(completeWith({
        result: null,
        error: `Deno subprocess execution timed out after ${timeoutMs}ms`,
      }));
    }, timeoutMs);

    // Process messages fiber
    const processFiber = yield* Effect.fork(
      Effect.gen(function* () {
        while (true) {
          const msg = yield* Queue.take(messages);

          switch (msg.type) {
            case "tool_call": {
              const toolResult = yield* toolInvoker
                .invoke({ path: msg.toolPath, args: msg.args })
                .pipe(
                  Effect.map((value): HostToWorkerMessage => ({
                    type: "tool_result",
                    requestId: msg.requestId,
                    ok: true,
                    value,
                  })),
                  Effect.catchAllCause((cause) =>
                    Effect.succeed<HostToWorkerMessage>({
                      type: "tool_result",
                      requestId: msg.requestId,
                      ok: false,
                      error: causeMessage(cause),
                    }),
                  ),
                );
              writeMessage(worker.stdin, toolResult);
              break;
            }
            case "completed":
              yield* completeWith({ result: msg.result, logs: msg.logs });
              return;
            case "failed":
              yield* completeWith({ result: null, error: msg.error, logs: msg.logs });
              return;
          }
        }
      }),
    );

    // Await result and clean up
    const output = yield* Deferred.await(result).pipe(
      Effect.ensuring(
        Effect.gen(function* () {
          clearTimeout(timer);
          yield* Fiber.interrupt(processFiber);
          worker.dispose();
        }),
      ),
    );

    return output;
  }).pipe(
    Effect.catchTag("DenoSpawnError", (e) =>
      Effect.succeed<ExecuteResult>({ result: null, error: e.message }),
    ),
  );
```

**Key patterns:**

1. **Queue-based IPC** — Bridges Node callbacks to Effect fibers
2. **Deferred result** — Single completion point for execution
3. **Message protocol** — Structured IPC with tagged union messages
4. **Fork/join** — Message processing runs in separate fiber
5. **Ensuring cleanup** — Always cleans up resources even on error

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Kernel Package                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Core Types (@executor/codemode-core)              │   │
│  │                                                                       │   │
│  │  Tool               — Standard Schema-based tool definition         │   │
│  │  SandboxToolInvoker — Tool invocation from sandbox                  │   │
│  │  CodeExecutor       — Code execution interface                       │   │
│  │  ExecuteResult      — Execution output structure                     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│              ┌───────────────┴───────────────┐                             │
│              ▼                               ▼                             │
│  ┌─────────────────────────┐     ┌─────────────────────────┐             │
│  │  QuickJS Runtime        │     │  Deno Subprocess        │             │
│  │  (@executor/runtime-    │     │  (@executor/runtime-    │             │
│  │   quickjs)              │     │   deno-subprocess)      │             │
│  │                         │     │                         │             │
│  │  - WASM sandbox         │     │  - Full Deno runtime    │             │
│  │  - Memory limits        │     │  - Node compatibility   │             │
│  │  - Timeout enforcement  │     │  - Permission control   │             │
│  │  - Tool bridge (Promise)│     │  - IPC message protocol│             │
│  └─────────────────────────┘     └─────────────────────────┘             │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Tool Invoker (from SDK)                           │   │
│  │                                                                       │   │
│  │  invoke({ path, args }) → Effect.Effect<unknown, unknown>           │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### QuickJS Execution Flow

```
execute(code, toolInvoker)
    │
    ├──> 1. Initialize QuickJS runtime
    │    ├──> Set memory limit
    │    ├──> Set stack size limit
    │    └──> Set interrupt handler (timeout)
    │
    ├──> 2. Create context and bridges
    │    ├──> createLogBridge() → __executor_log
    │    └──> createToolBridge() → __executor_invokeTool
    │
    ├──> 3. Build execution source
    │    ├──> Wrap user code with runtime
    │    ├──> Inject tools proxy
    │    └──> Disable fetch
    │
    ├──> 4. Evaluate code
    │    └──> context.evalCode(source, filename)
    │
    ├──> 5. Drain async operations
    │    ├──> Process pending jobs
    │    ├──> Wait for deferred promises (tool calls)
    │    └──> Repeat until all settled or timeout
    │
    ├──> 6. Extract result
    │    ├──> Read promise state
    │    └──> Return { result, error?, logs? }
    │
    └──> 7. Cleanup
         ├──> Dispose deferreds
         ├──> Dispose context
         └──> Dispose runtime
```

### Deno Subprocess Execution Flow

```
execute(code, toolInvoker)
    │
    ├──> 1. Spawn Deno subprocess
    │    ├──> Resolve deno executable
    │    ├──> Resolve worker script path
    │    └──> Spawn with permissions
    │
    ├──> 2. Set up IPC
    │    ├──> Queue for messages
    │    ├──> Deferred for result
    │    └──> Fork message processing fiber
    │
    ├──> 3. Send code to subprocess
    │    └──> writeMessage(stdin, { type: "start", code })
    │
    ├──> 4. Process messages
    │    ├──> Parse stdout lines (IPC_PREFIX prefix)
    │    ├──> tool_call → invoke tool → send result
    │    ├──> completed → resolve with result
    │    └──> failed → resolve with error
    │
    ├──> 5. Handle timeout/exit
    │    └──> Kill process and resolve error
    │
    └──> 6. Cleanup
         ├──> Clear timer
         ├──> Interrupt fiber
         └──> Dispose subprocess
```

### Tool Invocation from Sandbox

```
// User code in sandbox
const result = await tools.github.issues.list({ owner: "foo", repo: "bar" });
    │
    ├──> 1. Proxy get handler
    │    └──> Accumulate path: ["github", "issues", "list"]
    │
    ├──> 2. Proxy apply handler
    │    └──> Call __invokeTool("github.issues.list", args)
    │
    ├──> 3. QuickJS bridge
    │    ├──> Create QuickJS deferred promise
    │    └──> Call toolInvoker.invoke({ path, args })
    │
    ├──> 4. Effect runtime (outside sandbox)
    │    └──> Run Effect → Promise → QuickJS promise
    │
    ├──> 5. Promise resolution
    │    ├──> Serialize result to JSON
    │    └──> Resolve QuickJS promise
    │
    └──> 6. Sandbox receives result
         └──> Parse JSON and return to user code
```

---

## 7. Key Patterns

### Standard Schema Interface

```typescript
export const unknownInputSchema: StandardSchema = {
  "~standard": {
    version: 1,
    vendor: "@executor/codemode-core",
    validate: (value: unknown) => ({ value }),
  },
};
```

**Purpose:** Accept-anything schema for tools without input validation.

### Tools Proxy Pattern

```typescript
const __makeToolsProxy = (path = []) => new Proxy(() => undefined, {
  get(_target, prop) {
    if (prop === 'then' || typeof prop === 'symbol') return undefined;
    return __makeToolsProxy([...path, String(prop)]);
  },
  apply(_target, _thisArg, args) {
    const toolPath = path.join('.');
    return Promise.resolve(__invokeTool(toolPath, args[0]))
      .then((raw) => raw === undefined ? undefined : JSON.parse(raw));
  },
});
const tools = __makeToolsProxy();
```

**Benefits:**
1. **Lazy path building** — `tools.github.issues.list` builds path dynamically
2. **Natural syntax** — Looks like normal object property access
3. **Promise-based** — Returns promises for async tool calls

### Deferred Tracking Pattern

```typescript
const pendingDeferreds = new Set<QuickJSDeferredPromise>();

const deferred = context.newPromise();
pendingDeferreds.add(deferred);
deferred.settled.finally(() => pendingDeferreds.delete(deferred));

// Later: wait for all deferreds
while (pendingDeferreds.size > 0) {
  await waitForDeferreds(pendingDeferreds, deadlineMs, timeoutMs);
  drainJobs(context, runtime, deadlineMs, timeoutMs);
}
```

**Purpose:** Track async operations and wait for completion before returning result.

### IPC Message Protocol

```typescript
// Host → Worker
type HostToWorkerMessage =
  | { type: "start"; code: string }
  | { type: "tool_result"; requestId: string; ok: boolean; value?: unknown; error?: string };

// Worker → Host
type WorkerToHostMessage =
  | { type: "tool_call"; requestId: string; toolPath: string; args: unknown }
  | { type: "completed"; result: unknown; logs?: string[] }
  | { type: "failed"; error: string; logs?: string[] };
```

**Benefits:**
1. **Structured communication** — Clear message types
2. **Request/response correlation** — requestId for tool calls
3. **Type safety** — Schema-validated messages

---

## 8. Integration Points

### Dependencies

| Package | Purpose |
|---------|---------|
| `effect` | Effect runtime |
| `quickjs-emscripten` | QuickJS WASM binding |
| `@standard-schema/spec` | Standard Schema type |

### Dependents

| Package | Relationship |
|---------|--------------|
| `@executor/execution` | Uses CodeExecutor for code execution |
| `@executor/apps/*` | Applications use execution engine |

---

## 9. Error Handling

### QuickJS Errors

```typescript
class QuickJsExecutionError extends Data.TaggedError("QuickJsExecutionError")<{
  readonly message: string;
}> {}

const normalizeExecutionError = (
  cause: unknown,
  deadlineMs: number,
  timeoutMs: number,
): string => {
  const message = toErrorMessage(cause);
  return Date.now() >= deadlineMs && looksLikeInterruptedError(message)
    ? timeoutMessage(timeoutMs)
    : message;
};
```

### Deno Errors

```typescript
class DenoSpawnError extends Data.TaggedError("DenoSpawnError")<{
  readonly executable: string;
  readonly reason: unknown;
}> {
  override get message() {
    const code = /* extract error code */;
    return code === "ENOENT"
      ? `Failed to spawn Deno subprocess: Deno executable "${this.executable}" was not found.`
      : `Failed to spawn Deno subprocess: ${this.reason instanceof Error ? this.reason.message : String(this.reason)}`;
  }
}
```

### Timeout Handling

```typescript
// QuickJS
runtime.setInterruptHandler(shouldInterruptAfterDeadline(deadlineMs));

// Deno
const timer = setTimeout(() => {
  worker.dispose();
  runSync(completeWith({
    result: null,
    error: `Deno subprocess execution timed out after ${timeoutMs}ms`,
  }));
}, timeoutMs);
```

---

## 10. Runtime Comparison

| Feature | QuickJS | Deno Subprocess |
|---------|---------|-----------------|
| **Isolation** | WASM sandbox | OS subprocess |
| **Memory limit** | Configurable | OS-level |
| **Timeout** | Interrupt handler | Timer + kill |
| **Node APIs** | None | Full Deno APIs |
| **Permissions** | None (sandboxed) | Configurable |
| **Startup time** | Fast (~50ms) | Slow (~200ms) |
| **Compatibility** | ES5+ | Full TypeScript |
| **Use case** | Simple scripts | Complex code |

---

## 11. Design Decisions

### Why Two Runtimes?

1. **QuickJS** — Fast, lightweight, safe for untrusted code
2. **Deno** — Full-featured for complex operations requiring Node APIs

### Why Standard Schema?

1. **Framework agnostic** — Works with Zod, Effect Schema, etc.
2. **Type inference** — TypeScript infers input/output types
3. **Validation** — Runtime validation of inputs

### Why Proxy-based Tools?

1. **Natural syntax** — `tools.github.issues.list()` feels natural
2. **Lazy evaluation** — No need to pre-register tool paths
3. **Composable** — Easy to add new tools/sources

### Why JSON Serialization?

1. **Sandbox boundary** — Clear serialization at sandbox edge
2. **Safety** — Prevents leaking non-serializable values
3. **Debugging** — Easy to log and inspect values

---

## 12. Summary

The Kernel package provides **sandboxed code execution**:

1. **Core Types** — Standard Schema-based tool interface
2. **QuickJS Runtime** — Fast WASM sandbox with resource limits
3. **Deno Runtime** — Full-featured subprocess execution
4. **Tool Bridging** — Proxy-based tool invocation from sandbox
5. **Error Handling** — Structured errors with timeout detection

The kernel enables **safe AI code execution** while providing **flexible runtime options** for different use cases.
