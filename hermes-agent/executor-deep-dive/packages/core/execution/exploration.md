# Executor Core Execution — Deep Dive Exploration

**Package:** `@executor/execution`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/packages/core/execution`  
**Total Files:** 7 TypeScript files  
**Total Lines:** 1,052 lines  

---

## 1. Module Overview

The Execution package is the **code execution engine** for the Executor system. It provides:

- **Sandboxed TypeScript execution** — Run AI-generated code safely
- **Elicitation handling** — Pause/resume for user approval flows
- **Tool discovery** — Search and describe available tools
- **Result formatting** — Structured output for hosts

### Key Responsibilities

1. **Code Execution** — Execute TypeScript code in a QuickJS sandbox
2. **Elicitation Interception** — Handle user approval requests during execution
3. **Tool Invocation Bridging** — Connect sandbox code to Executor SDK
4. **Dynamic Descriptions** — Generate contextual tool documentation

---

## 2. File Inventory

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `src/engine.ts` | 346 | Main execution engine with pause/resume |
| 2 | `src/tool-invoker.ts` | 334 | Tool discovery and invocation bridge |
| 3 | `src/description.ts` | 65 | Dynamic tool description generator |
| 4 | `src/errors.ts` | 6 | Execution-specific error types |
| 5 | `src/index.ts` | 19 | Public exports |
| 6 | `src/tool-invoker.test.ts` | 275 | Tool invoker tests |
| 7 | `vitest.config.ts` | 7 | Test configuration |

---

## 3. Key Exports

### Execution Engine

```typescript
// engine.ts
export type ExecutionEngine = {
  /**
   * Execute code with elicitation handled inline by the provided handler.
   * Use this when the host supports elicitation (e.g. MCP with elicitation capability).
   */
  readonly execute: (
    code: string,
    options: { readonly onElicitation: ElicitationHandler },
  ) => Promise<ExecuteResult>;

  /**
   * Execute code, intercepting the first elicitation as a pause point.
   * Use this when the host doesn't support inline elicitation.
   * Returns either a completed result or a paused execution that can be resumed.
   */
  readonly executeWithPause: (code: string) => Promise<ExecutionResult>;

  /**
   * Resume a paused execution.
   */
  readonly resume: (executionId: string, response: ResumeResponse) => Promise<ExecuteResult | null>;

  /**
   * Get the dynamic tool description (workflow + namespaces).
   */
  readonly getDescription: () => Promise<string>;
};
```

### Execution Result Types

```typescript
export type ExecutionResult =
  | { readonly status: "completed"; readonly result: ExecuteResult }
  | { readonly status: "paused"; readonly execution: PausedExecution };

export type PausedExecution = {
  readonly id: string;
  readonly elicitationContext: ElicitationContext;
  readonly resolve: (response: typeof ElicitationResponse.Type) => void;
  readonly completion: Promise<ExecuteResult>;
};

export type ResumeResponse = {
  readonly action: "accept" | "decline" | "cancel";
  readonly content?: Record<string, unknown>;
};
```

### Tool Invoker

```typescript
// tool-invoker.ts
export const makeExecutorToolInvoker: (
  executor: Executor,
  options: { readonly invokeOptions: InvokeOptions },
) => SandboxToolInvoker;

export const searchTools: (
  executor: Executor,
  query: string,
  limit?: number,
  options?: { readonly namespace?: string },
) => Effect.Effect<ReadonlyArray<ToolDiscoveryResult>>;

export const listExecutorSources: (
  executor: Executor,
  options?: { readonly query?: string; readonly limit?: number },
) => Effect.Effect<ReadonlyArray<ExecutorSourceListItem>>;

export const describeTool: (
  executor: Executor,
  path: string,
) => Effect.Effect<{
  path: string;
  name: string;
  description?: string;
  inputTypeScript?: string;
  outputTypeScript?: string;
  typeScriptDefinitions?: Record<string, string>;
}>;
```

---

## 4. Line-by-Line Analysis

### Execution Engine Factory (`engine.ts:283-346`)

```typescript
export const createExecutionEngine = (config: ExecutionEngineConfig): ExecutionEngine => {
  const { executor } = config;
  const codeExecutor = config.codeExecutor ?? makeQuickJsExecutor();
  const pausedExecutions = new Map<string, PausedExecution>();
  let nextId = 0;

  return {
    execute: async (code, options) => {
      const invoker = makeFullInvoker(executor, {
        onElicitation: options.onElicitation,
      });
      return runEffect(codeExecutor.execute(code, invoker));
    },

    executeWithPause: async (code) => {
      // Signal from the elicitation handler to the race below.
      let signalPause: ((paused: PausedExecution) => void) | null = null;
      const pausePromise = new Promise<PausedExecution>((resolve) => {
        signalPause = resolve;
      });

      const elicitationHandler: ElicitationHandler = (ctx: ElicitationContext) =>
        Effect.async<typeof ElicitationResponse.Type>((resume) => {
          const id = `exec_${++nextId}`;
          const paused: PausedExecution = {
            id,
            elicitationContext: ctx,
            resolve: (response) => resume(Effect.succeed(response)),
            completion: undefined as unknown as Promise<ExecuteResult>,
          };
          pausedExecutions.set(id, paused);
          signalPause!(paused);
        });

      const invoker = makeFullInvoker(executor, { onElicitation: elicitationHandler });
      const completionPromise = runEffect(codeExecutor.execute(code, invoker));

      // Race: either the execution completes, or it pauses for elicitation.
      const result = await Promise.race([
        completionPromise.then((r) => ({ kind: "completed" as const, result: r })),
        pausePromise.then((p) => ({ kind: "paused" as const, execution: p })),
      ]);

      if (result.kind === "completed") {
        return { status: "completed", result: result.result };
      }

      // Execution paused — attach the completion promise and return
      (result.execution as { completion: Promise<ExecuteResult> }).completion = completionPromise;
      return { status: "paused", execution: result.execution };
    },

    resume: async (executionId, response) => {
      const paused = pausedExecutions.get(executionId);
      if (!paused) return null;

      pausedExecutions.delete(executionId);
      paused.resolve({ action: response.action, content: response.content });
      return paused.completion;
    },

    getDescription: () => runEffect(buildExecuteDescription(executor)),
  };
};
```

**Key patterns:**

1. **Promise.race for pause detection** — Uses `Promise.race` to detect whether execution completes or pauses first
2. **Async Effect bridge** — `Effect.async` wraps the elicitation handler to suspend execution
3. **Execution tracking** — Map stores paused executions by ID for later resume
4. **Completion attachment** — The completion promise is attached to paused execution for seamless resume

### Full Tool Invoker (`tool-invoker.ts:152-246`)

```typescript
const makeFullInvoker = (
  executor: Executor,
  invokeOptions: InvokeOptions,
): SandboxToolInvoker => {
  const base = makeExecutorToolInvoker(executor, { invokeOptions });
  return {
    invoke: ({ path, args }) => {
      // Handle special discovery tools
      if (path === "search") {
        // Validate args
        if (!isRecord(args)) {
          return Effect.fail(
            new ExecutionToolError({
              message: "tools.search expects an object: { query?: string; namespace?: string; limit?: number }",
            }),
          );
        }
        // Validate query
        if (args.query !== undefined && typeof args.query !== "string") {
          return Effect.fail(
            new ExecutionToolError({
              message: "tools.search query must be a string when provided",
            }),
          );
        }
        // Validate limit
        const limit = readOptionalLimit(args.limit, "tools.search");
        if (limit instanceof ExecutionToolError) {
          return Effect.fail(limit);
        }
        return searchTools(executor, args.query ?? "", limit, {
          namespace: args.namespace,
        });
      }
      
      if (path === "executor.sources.list") {
        // ... similar validation and delegation
        return listExecutorSources(executor, {
          query: isRecord(args) && typeof args.query === "string" ? args.query : undefined,
          limit,
        });
      }
      
      if (path === "describe.tool") {
        // ... validation
        return describeTool(executor, args.path);
      }
      
      // Default: delegate to base invoker
      return base.invoke({ path, args });
    },
  };
};
```

**Key patterns:**

1. **Decorator pattern** — Wraps base invoker to add discovery tools
2. **Runtime validation** — Validates args before delegating to handlers
3. **Special tool paths** — `search`, `executor.sources.list`, `describe.tool` are handled specially

### Tool Search Algorithm (`tool-invoker.ts:168-235`)

```typescript
const scoreToolMatch = (
  tool: SearchableTool,
  query: string,
): ToolDiscoveryResult | null => {
  const normalizedQuery = normalizeSearchText(query);
  const queryTokens = tokenizeSearchText(query);

  if (normalizedQuery.length === 0 || queryTokens.length === 0) {
    return null;
  }

  const path = prepareField(tool.id);
  const sourceId = prepareField(tool.sourceId);
  const name = prepareField(tool.name);
  const description = prepareField(tool.description);

  const fieldScores = [
    scorePreparedField(normalizedQuery, queryTokens, path, SEARCH_FIELD_WEIGHTS.path),
    scorePreparedField(normalizedQuery, queryTokens, sourceId, SEARCH_FIELD_WEIGHTS.sourceId),
    scorePreparedField(normalizedQuery, queryTokens, name, SEARCH_FIELD_WEIGHTS.name),
    scorePreparedField(normalizedQuery, queryTokens, description, SEARCH_FIELD_WEIGHTS.description),
  ];

  const matchedTokens = new Set<string>();
  let score = 0;
  let exactPhraseMatch = false;

  for (const fieldScore of fieldScores) {
    score += fieldScore.score;
    exactPhraseMatch ||= fieldScore.exactPhraseMatch;
    for (const token of fieldScore.matchedTokens) {
      matchedTokens.add(token);
    }
  }

  if (matchedTokens.size === 0) {
    return null;
  }

  const coverage = matchedTokens.size / queryTokens.length;
  const minimumCoverage = queryTokens.length <= 2 ? 1 : 0.6;

  if (coverage < minimumCoverage && !exactPhraseMatch) {
    return null;
  }

  if (coverage === 1) {
    score += 25;
  } else {
    score += Math.round(coverage * 10);
  }

  if (path.tokens[0] === queryTokens[0] || name.tokens[0] === queryTokens[0]) {
    score += 8;
  }

  if (normalizeSearchText(tool.id) === normalizedQuery || normalizeSearchText(tool.name) === normalizedQuery) {
    score += 20;
  }

  return {
    path: tool.id,
    name: tool.name,
    description: tool.description,
    sourceId: tool.sourceId,
    score,
  };
};
```

**Search scoring weights:**

```typescript
const SEARCH_FIELD_WEIGHTS = {
  path: 12,        // Full tool path (e.g., "github.issues.list")
  sourceId: 8,     // Source namespace (e.g., "github")
  name: 10,        // Tool name (e.g., "issues")
  description: 5,  // Tool description
} as const;
```

**Key patterns:**

1. **Token-based search** — Query and fields are tokenized for matching
2. **Weighted scoring** — Different fields have different importance
3. **Fuzzy matching** — Supports prefix and substring matches
4. **Coverage threshold** — Requires minimum token coverage (100% for short queries, 60% for longer)
5. **Bonus scoring** — Extra points for full coverage and exact matches

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        Execution Engine                                      │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    createExecutionEngine()                           │   │
│  │                                                                       │   │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │   │
│  │  │    execute()    │  │executeWithPause()│  │    resume()     │     │   │
│  │  │                 │  │                 │  │                 │     │   │
│  │  │ Inline handler  │  │ Pause on first  │  │ Resume paused   │     │   │
│  │  │ for elicitation │  │ elicitation     │  │ execution by ID │     │   │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────┘     │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    makeFullInvoker()                                 │   │
│  │                                                                       │   │
│  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │  SandboxToolInvoker                                          │   │   │
│  │  │                                                               │   │   │
│  │  │  - search({ query, namespace, limit })                       │   │   │
│  │  │  - executor.sources.list({ query?, limit? })                 │   │   │
│  │  │  - describe.tool({ path })                                   │   │   │
│  │  │  - tools.<path>(args)  (all other paths)                     │   │   │
│  │  └──────────────────────────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                              │                                              │
│                              ▼                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Executor SDK                                      │   │
│  │  - executor.tools.list()                                            │   │
│  │  - executor.tools.invoke()                                          │   │
│  │  - executor.tools.schema()                                          │   │
│  │  - executor.sources.list()                                          │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Code Executor (QuickJS)                           │   │
│  │  - makeQuickJsExecutor() from @executor/runtime-quickjs             │   │
│  │  - Sandboxed TypeScript execution                                   │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### Execute with Pause Flow

```
executeWithPause(code)
    │
    ├──> 1. Create pause signal promise
    │    └──> pausePromise: Promise<PausedExecution>
    │
    ├──> 2. Build elicitation handler
    │    └──> On elicitation request:
    │         ├──> Create PausedExecution with unique ID
    │         ├──> Store in pausedExecutions map
    │         └──> Signal pausePromise
    │
    ├──> 3. Create full invoker with pause handler
    │    └──> makeFullInvoker(executor, { onElicitation: handler })
    │
    ├──> 4. Start code execution
    │    └──> codeExecutor.execute(code, invoker)
    │
    ├──> 5. Race: completion vs pause
    │    ├──> If completes first: return { status: "completed", result }
    │    └──> If pauses first: return { status: "paused", execution }
    │
    └──> 6. Attach completion promise to paused execution
         └──> execution.completion = completionPromise
```

### Resume Flow

```
resume(executionId, response)
    │
    ├──> 1. Look up paused execution
    │    └──> paused = pausedExecutions.get(executionId)
    │
    ├──> 2. Remove from map
    │    └──> pausedExecutions.delete(executionId)
    │
    ├──> 3. Resolve elicitation handler
    │    └──> paused.resolve({ action, content })
    │
    └──> 4. Return completion promise
         └──> return paused.completion
```

### Search Flow

```
tools.search({ query: "github issues", namespace: "github", limit: 12 })
    │
    ├──> 1. Normalize and tokenize query
    │    └──> "github issues" → ["github", "issues"]
    │
    ├──> 2. Fetch all tools
    │    └──> executor.tools.list()
    │
    ├──> 3. Filter by namespace (if provided)
    │    └──> matchesNamespace(tool, namespace)
    │
    ├──> 4. Score each tool
    │    └──> scoreToolMatch(tool, query)
    │         ├──> Tokenize fields (id, name, description, sourceId)
    │         ├──> Score each field with weights
    │         ├──> Apply coverage threshold
    │         └──> Apply bonuses (full coverage, exact match)
    │
    ├──> 5. Filter low-scoring tools
    │    └──> Must meet minimum coverage OR have exact phrase match
    │
    ├──> 6. Sort by score
    │    └──> Higher scores first, then alphabetically
    │
    └──> 7. Return top N results
         └──> slice(0, limit)
```

---

## 7. Key Patterns

### Pause/Resume Pattern

The execution engine implements a **suspension pattern** for elicitation:

```typescript
// Async Effect creates a suspend point
Effect.async<typeof ElicitationResponse.Type>((resume) => {
  // Store resume callback for later
  const paused: PausedExecution = {
    id,
    elicitationContext: ctx,
    resolve: (response) => resume(Effect.succeed(response)),
    completion: undefined as unknown as Promise<ExecuteResult>,
  };
  pausedExecutions.set(id, paused);
  signalPause!(paused);
});

// Promise.race detects the pause
const result = await Promise.race([
  completionPromise.then((r) => ({ kind: "completed", result: r })),
  pausePromise.then((p) => ({ kind: "paused", execution: p })),
]);
```

### Decorator Pattern

The full invoker decorates the base invoker:

```typescript
const makeFullInvoker = (executor: Executor, invokeOptions: InvokeOptions): SandboxToolInvoker => {
  const base = makeExecutorToolInvoker(executor, { invokeOptions });
  return {
    invoke: ({ path, args }) => {
      // Handle special discovery tools
      if (path === "search") { /* ... */ }
      if (path === "executor.sources.list") { /* ... */ }
      if (path === "describe.tool") { /* ... */ }
      // Default: delegate to base
      return base.invoke({ path, args });
    },
  };
};
```

### Field Scoring Pattern

Search uses a **field-based scoring** approach:

```typescript
const scorePreparedField = (
  query: string,
  queryTokens: readonly string[],
  field: PreparedField,
  weight: number,
): {
  readonly score: number;
  readonly matchedTokens: ReadonlySet<string>;
  readonly exactPhraseMatch: boolean;
} => {
  // Exact match bonuses
  if (field.raw === query) score += weight * 14;
  else if (field.raw.startsWith(query)) score += weight * 9;
  else if (field.raw.includes(query)) score += weight * 6;

  // Token matching
  for (const token of queryTokens) {
    if (field.tokens.includes(token)) score += weight * 4;
    else if (field.tokens.some(t => t.startsWith(token) || token.startsWith(t))) score += weight * 2;
    else if (field.raw.includes(token)) score += weight;
  }

  return { score, matchedTokens, exactPhraseMatch };
};
```

---

## 8. Integration Points

### Dependencies

| Package | Purpose |
|---------|---------|
| `@executor/sdk` | Executor interface for tool invocation |
| `@executor/codemode-core` | SandboxToolInvoker type |
| `@executor/runtime-quickjs` | Default QuickJS code executor |
| `effect` | Effect.ts runtime |

### Dependents

| Package | Relationship |
|---------|--------------|
| `@executor/hosts/mcp` | MCP server uses execution engine for code execution |
| `@executor/apps/cli` | CLI uses execution engine |
| `@executor/apps/server` | Server uses execution engine |
| `@executor/apps/desktop` | Desktop app uses execution engine |

---

## 9. Error Handling

### ExecutionToolError

```typescript
// errors.ts
export class ExecutionToolError extends Data.TaggedError("ExecutionToolError")<{
  readonly message: string;
  readonly cause?: unknown;
}> {}
```

### Validation Errors

Tool argument validation produces `ExecutionToolError`:

```typescript
if (!isRecord(args)) {
  return Effect.fail(
    new ExecutionToolError({
      message: "tools.search expects an object: { query?: string; namespace?: string; limit?: number }",
    }),
  );
}
```

### Elicitation Error Handling

```typescript
// engine.ts:21-29
invoke: ({ path, args }) =>
  Effect.gen(function* () {
    const result = yield* executor.tools.invoke(path, args, options).pipe(
      Effect.catchTag("ElicitationDeclinedError", (err) =>
        Effect.fail(
          new ExecutionToolError({
            message: `Tool "${err.toolId}" requires approval but the request was ${err.action === "cancel" ? "cancelled" : "declined"} by the user.`,
            cause: err,
          }),
        ),
      ),
    );
    // ...
  }),
```

---

## 10. Testing Strategy

### Tool Invoker Tests

**File:** `src/tool-invoker.test.ts` (275 lines)

Tests cover:
- `searchTools` — Various query scenarios
- `listExecutorSources` — Source listing with filters
- `describeTool` — Tool description generation
- Argument validation — Invalid inputs produce errors
- Namespace filtering — Prefix matching

### Test Examples

```typescript
// Search with namespace filter
it("filters by namespace", async () => {
  const results = await searchTools(executor, "issues", 10, { namespace: "github" });
  expect(results.every(r => r.sourceId.startsWith("github"))).toBe(true);
});

// Search scoring
it("scores exact matches higher", async () => {
  const results = await searchTools(executor, "listIssues", 10);
  expect(results[0].path).toContain("listIssues");
  expect(results[0].score).toBeGreaterThan(results[1].score);
});
```

---

## 11. Result Formatting

### Execute Result Formatting

```typescript
// engine.ts:58-91
export const formatExecuteResult = (result: ExecuteResult): {
  text: string;
  structured: Record<string, unknown>;
  isError: boolean;
} => {
  const resultText = result.result != null
    ? typeof result.result === "string"
      ? result.result
      : JSON.stringify(result.result, null, 2)
    : null;

  const logText = result.logs?.length ? result.logs.join("\n") : null;

  if (result.error) {
    const parts = [`Error: ${result.error}`, ...(logText ? [`\nLogs:\n${logText}`] : [])];
    return {
      text: truncate(parts.join("\n"), MAX_PREVIEW_CHARS),
      structured: { status: "error", error: result.error, logs: result.logs ?? [] },
      isError: true,
    };
  }

  const parts = [
    ...(resultText ? [truncate(resultText, MAX_PREVIEW_CHARS)] : ["(no result)"]),
    ...(logText ? [`\nLogs:\n${logText}`] : []),
  ];
  return {
    text: parts.join("\n"),
    structured: { status: "completed", result: result.result ?? null, logs: result.logs ?? [] },
    isError: false,
  };
};
```

### Paused Execution Formatting

```typescript
// engine.ts:93-126
export const formatPausedExecution = (paused: PausedExecution): {
  text: string;
  structured: Record<string, unknown>;
} => {
  const req = paused.elicitationContext.request;
  const lines: string[] = [`Execution paused: ${(req as any).message}`];

  if (req._tag === "UrlElicitation") {
    lines.push(`\nOpen this URL in a browser:\n${(req as any).url}`);
    lines.push("\nAfter the browser flow, resume with the executionId below:");
  } else {
    lines.push("\nResume with the executionId below and a response matching the requested schema:");
    const schema = (req as any).requestedSchema;
    if (schema && Object.keys(schema).length > 0) {
      lines.push(`\nRequested schema:\n${JSON.stringify(schema, null, 2)}`);
    }
  }

  lines.push(`\nexecutionId: ${paused.id}`);

  return {
    text: lines.join("\n"),
    structured: {
      status: "waiting_for_interaction",
      executionId: paused.id,
      interaction: {
        kind: req._tag === "UrlElicitation" ? "url" : "form",
        message: (req as any).message,
        ...(req._tag === "UrlElicitation" ? { url: (req as any).url } : {}),
        ...(req._tag === "FormElicitation" ? { requestedSchema: (req as any).requestedSchema } : {}),
      },
    },
  };
};
```

---

## 12. Dynamic Description Generation

**File:** `description.ts` (65 lines)

### Description Structure

```typescript
export const buildExecuteDescription = (
  executor: Executor,
): Effect.Effect<string> =>
  Effect.gen(function* () {
    const sources = yield* executor.sources.list();
    const tools = yield* executor.tools.list();

    const namespaces = new Set<string>();
    for (const tool of tools) namespaces.add(tool.sourceId);

    return formatDescription([...namespaces], sources);
  });
```

### Description Format

```
Execute TypeScript in a sandboxed runtime with access to configured API tools.

## Workflow

1. `const matches = await tools.search({ query: "<intent + key nouns>", limit: 12 });`
2. `const path = matches[0]?.path; if (!path) return "No matching tools found.";`
3. `const details = await tools.describe.tool({ path });`
4. Use `details.inputTypeScript` / `details.outputTypeScript` and `details.typeScriptDefinitions` for compact shapes.
5. Use `tools.executor.sources.list()` when you need configured source inventory.
6. Call the tool: `const result = await tools.<path>(input);`

## Rules

- `tools.search()` returns ranked matches, best-first. Use short intent phrases like `github issues`, `repo details`, or `create calendar event`.
- When you already know the namespace, narrow with `tools.search({ namespace: "github", query: "issues" })`.
- Use `tools.executor.sources.list()` to inspect configured sources and their tool counts.
- The `tools` object is a lazy proxy — `Object.keys(tools)` won't work. Use `tools.search()` or `tools.executor.sources.list()` instead.
- Pass an object to system tools, e.g. `tools.search({ query: "..." })`, `tools.executor.sources.list()`, and `tools.describe.tool({ path })`.
- `tools.describe.tool()` returns compact TypeScript shapes. Use `inputTypeScript`, `outputTypeScript`, and `typeScriptDefinitions`.
- Do not use `fetch` — all API calls go through `tools.*`.
- If execution pauses for interaction, resume it with the returned `resumePayload`.

## Available namespaces

- `github` — GitHub REST API
- `google` — Google APIs
- `mcp` — MCP Server Tools
```

---

## 13. Design Decisions

### Why Promise.race for Pause Detection?

1. **Non-blocking** — Execution continues without polling
2. **Efficient** — No timers or intervals needed
3. **Clean semantics** — Either completes or pauses, never both

### Why Token-based Search?

1. **Fuzzy matching** — Supports partial matches and typos
2. **Field weighting** — More important fields get higher scores
3. **Coverage threshold** — Ensures results are relevant to the query

### Why Separate execute and executeWithPause?

1. **Host capability detection** — Some hosts support inline elicitation, others don't
2. **Flexibility** — Hosts can choose the right pattern for their environment
3. **MCP compatibility** — MCP hosts with elicitation capability use inline, others use pause/resume

---

## 14. Summary

The Execution package provides a **sandboxed code execution environment** with sophisticated features:

1. **Pause/Resume Support** — Handle elicitation requests in hosts without inline support
2. **Intelligent Tool Search** — Token-based scoring with field weights and coverage thresholds
3. **Dynamic Documentation** — Generate contextual tool descriptions at runtime
4. **Result Formatting** — Human-readable and structured output for hosts
5. **Error Handling** — Validation and propagation through Effect pipeline

The execution engine is the **runtime heart** of Executor, enabling AI agents to safely execute code that interacts with configured APIs.
