# Executor Tests — Deep Dive Exploration

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/executor/tests/`  
**Test Files:** 31 test files across the codebase  
**Test Framework:** Vitest + @effect/vitest  

---

## 1. Module Overview

The test suite provides **comprehensive testing** for the Executor system:

- **Unit tests** — Individual module and function tests
- **Integration tests** — Cross-module interaction tests
- **E2E tests** — Full system bootstrap and release tests
- **Preset tests** — External API reachability verification

### Key Responsibilities

1. **Release Bootstrap** — End-to-end installation and execution test
2. **Preset Reachability** — Verify external API endpoints are accessible
3. **Plugin Tests** — Test plugin initialization and functionality
4. **Core Tests** — SDK, execution, config, storage tests
5. **Kernel Tests** — Runtime execution tests

---

## 2. File Inventory

### Root Tests (2 files)

| # | File | Lines | Description |
|---|------|-------|-------------|
| 1 | `tests/release-bootstrap-smoke.test.ts` | 252 | Full release build and install test |
| 2 | `tests/presets-reachable.test.ts` | 235 | Preset endpoint reachability tests |

### Package Tests (29 files)

#### Core SDK (3 files)
| File | Description |
|------|-------------|
| `packages/core/sdk/src/index.test.ts` | SDK initialization tests |
| `packages/core/sdk/src/schema-types.test.ts` | Schema type tests |
| `packages/core/sdk/vitest.config.ts` | Test config |

#### Core Execution (2 files)
| File | Description |
|------|-------------|
| `packages/core/execution/src/tool-invoker.test.ts` | Tool search and invocation tests |
| `packages/core/execution/vitest.config.ts` | Test config |

#### Core Config (2 files)
| File | Description |
|------|-------------|
| `packages/core/config/src/config.test.ts` | Config loading/writing tests |
| `packages/core/config/vitest.config.ts` | Test config |

#### Core Storage (4 files)
| File | Description |
|------|-------------|
| `packages/core/storage-file/src/index.test.ts` | File KV tests |
| `packages/core/storage-file/vitest.config.ts` | Test config |
| `packages/core/storage-postgres/src/index.test.ts` | Postgres storage tests |
| `packages/core/storage-postgres/vitest.config.ts` | Test config |

#### Kernel (4 files)
| File | Description |
|------|-------------|
| `packages/kernel/runtime-quickjs/src/index.test.ts` | QuickJS runtime tests |
| `packages/kernel/runtime-quickjs/vitest.config.ts` | Test config |
| `packages/kernel/runtime-deno-subprocess/src/index.test.ts` | Deno subprocess tests |
| `packages/kernel/runtime-deno-subprocess/vitest.config.ts` | Test config |

#### Hosts (2 files)
| File | Description |
|------|-------------|
| `packages/hosts/mcp/src/server.test.ts` | MCP server tests |
| `packages/hosts/mcp/vitest.config.ts` | Test config |

#### Plugins (12 files)
| Plugin | Test Files |
|--------|------------|
| openapi | `index.test.ts`, `plugin.test.ts`, `real-specs.test.ts` |
| mcp | `plugin.test.ts`, `elicitation.test.ts` |
| graphql | `plugin.test.ts`, `extract.test.ts` |
| google-discovery | `plugin.test.ts`, `document.test.ts` |
| keychain | `index.test.ts` |

---

## 3. Key Exports

### Test Utilities

```typescript
// release-bootstrap-smoke.test.ts
type CommandResult = {
  readonly exitCode: number;
  readonly stdout: string;
  readonly stderr: string;
};

const runCommand = async (
  command: string,
  args: ReadonlyArray<string>,
  cwd: string,
  env: NodeJS.ProcessEnv = process.env,
): Promise<CommandResult>;

const listen = async (server: ReturnType<typeof createServer>): Promise<number>;
const closeServer = async (server: ReturnType<typeof createServer>): Promise<void>;
```

### Preset Testing

```typescript
// presets-reachable.test.ts
const allPresets = [
  ...openApiPresets.map((p) => ({ ...p, plugin: "openapi" as const })),
  ...mcpPresets.map((p) => ({ ...p, plugin: "mcp" as const })),
  ...graphqlPresets.map((p) => ({ ...p, plugin: "graphql" as const })),
  ...googleDiscoveryPresets.map((p) => ({ ...p, plugin: "google-discovery" as const })),
];
```

---

## 4. Line-by-Line Analysis

### Release Bootstrap Test (`release-bootstrap-smoke.test.ts:86-163`)

```typescript
describe("release bootstrap smoke", () => {
  it(
    "fresh wrapper install bootstraps locally hosted release assets and stays runnable",
    async () => {
      if (!isSupportedPlatform) return;

      // 1. Build binary
      const build = await runCommand("bun", ["run", "src/build.ts", "binary", "--single"], cliRoot);
      expect(build.exitCode, build.stderr || build.stdout).toBe(0);

      // 2. Build release assets
      const assets = await runCommand("bun", ["run", "src/build.ts", "release-assets"], cliRoot);
      expect(assets.exitCode).toBe(0);

      // 3. Find platform-specific asset
      const assetNames = (await readdir(distDir))
        .filter((entry) => /^executor-.*\.(?:tar\.gz|zip)$/.test(entry))
        .sort();
      expect(assetNames).toHaveLength(1);

      // 4. Setup temp directory and mock server
      const tempRoot = await mkdtemp(join(tmpdir(), "executor-release-bootstrap-"));
      const installedPackageDir = join(tempRoot, "executor");

      // 5. Copy wrapper and patch package.json
      await cp(wrapperDir, installedPackageDir, { recursive: true });
      const assetRoute = `/releases/download/v${version}/${assetName}`;
      const server = createServer(async (request, response) => {
        if (request.url !== assetRoute) {
          response.statusCode = 404;
          response.end("not found");
          return;
        }
        const body = await readFile(assetPath);
        response.statusCode = 200;
        response.setHeader("content-length", String(body.byteLength));
        response.end(body);
      });

      // 6. Start mock server and run first install
      const port = await listen(server);
      const firstRun = await runCommand(
        process.execPath,
        [join(installedPackageDir, "bin", "executor"), "--help"],
        installedPackageDir,
      );
      expect(firstRun.exitCode).toBe(0);
      expect(firstRun.stdout).toContain("downloading release asset");
      expect(firstRun.stdout).toContain(`installed ${basename(assetName)}`);

      // 7. Verify web server works
      const webProcess = spawn(process.execPath, [executor, "web", "--port", String(webPort)]);
      const rootResponse = await fetch(`http://127.0.0.1:${webPort}/`);
      expect(rootResponse.status).toBe(200);

      // 8. Second run should use cached asset
      const secondRun = await runCommand(executor, "--help", installedPackageDir);
      expect(secondRun.stdout).not.toContain("downloading release asset");
    },
    180_000, // 3 minute timeout
  );
});
```

**Key patterns:**
1. **Mock HTTP server** — Simulates GitHub release hosting
2. **Temp directory** — Isolated test environment
3. **Two-run verification** — First install, second cached
4. **Full stack test** — Binary build, install, web server, API

### OpenAPI Preset Tests (`presets-reachable.test.ts:34-47`)

```typescript
describe("openapi presets parse as valid specs", () => {
  for (const preset of openApiPresets) {
    it.effect(
      preset.name,
      () =>
        Effect.gen(function* () {
          const doc = yield* parse(preset.url);
          expect(doc).toBeDefined();
          expect(doc.openapi).toBeDefined();
        }),
      { timeout: 30_000 },
    );
  }
});
```

**Key patterns:**
1. **it.effect** — Effect-based test with runtime
2. **parse function** — Fetch and validate OpenAPI spec
3. **Timeout** — 30 second timeout for network requests

### GraphQL Preset Tests (`presets-reachable.test.ts:53-85`)

```typescript
describe("graphql presets are reachable endpoints", () => {
  for (const preset of graphqlPresets) {
    it.effect(
      preset.name,
      () =>
        Effect.gen(function* () {
          const result = yield* introspect(preset.url).pipe(
            Effect.provide(FetchHttpClient.layer),
            Effect.map((r) => ({ ok: true as const, schema: r })),
            Effect.catchAll((err) =>
              Effect.succeed({
                ok: false as const,
                message: String(err),
              }),
            ),
          );

          if (result.ok) {
            expect(result.schema.__schema).toBeDefined();
            expect(result.schema.__schema.types.length).toBeGreaterThan(0);
          } else {
            // Auth-required — should fail with 401/403, not 404
            expect(result.message).toMatch(/401|403|Unauthorized|Forbidden|auth/i);
          }
        }),
      { timeout: 15_000 },
    );
  }
});
```

**Key patterns:**
1. **Graceful error handling** — 401/403 is acceptable (auth-required)
2. **Introspection query** — Verify GraphQL endpoint works
3. **Schema validation** — Check types array has content

### MCP Preset Tests (`presets-reachable.test.ts:95-121`)

```typescript
describe("mcp presets are reachable endpoints", () => {
  for (const preset of remoteMcpPresets) {
    it.effect(
      preset.name,
      () =>
        Effect.gen(function* () {
          const response = yield* Effect.tryPromise(() =>
            fetch(preset.url, {
              method: "POST",
              signal: AbortSignal.timeout(10_000),
              headers: { "Content-Type": "application/json" },
              body: "{}",
              redirect: "follow",
            }),
          );

          // Non-404/502/503 means endpoint is up
          expect(response.status !== 404 && response.status !== 502 && response.status !== 503).toBe(true);
        }),
      { timeout: 15_000 },
    );
  }
});
```

**Key patterns:**
1. **POST probe** — MCP endpoints respond to POST
2. **Status check** — Any status except 404/502/503 is valid
3. **Timeout** — 10 second fetch timeout

### Detection Tests (`presets-reachable.test.ts:167-205`)

```typescript
describe("public preset URLs are detected by the correct plugin", () => {
  const makeExecutor = () =>
    createExecutor(
      makeTestConfig({
        plugins: [
          openApiPlugin(),
          mcpPlugin(),
          graphqlPlugin(),
          googleDiscoveryPlugin(),
        ] as const,
      }),
    );

  for (const preset of publicPresets) {
    it.effect(
      `[${preset.plugin}] ${preset.name}`,
      () =>
        Effect.gen(function* () {
          const executor = yield* makeExecutor();
          const results = yield* executor.sources.detect(preset.url);

          expect(results.length).toBeGreaterThan(0);

          const expectedKinds: Record<string, string> = {
            openapi: "openapi",
            mcp: "mcp",
            graphql: "graphql",
            "google-discovery": "googleDiscovery",
          };
          const best = results[0]!;
          expect(best.kind).toBe(expectedKinds[preset.plugin]);
        }),
      { timeout: 30_000 },
    );
  }
});
```

**Key patterns:**
1. **Full executor** — Creates executor with all plugins
2. **Detection API** — `executor.sources.detect(url)`
3. **Plugin matching** — Verify correct plugin detects the URL

### Icon Reachability Tests (`presets-reachable.test.ts:211-234`)

```typescript
describe("preset icons are reachable", () => {
  const presetsWithIcons = allPresets.filter((p) => p.icon);
  for (const preset of presetsWithIcons) {
    it.effect(
      `[${preset.plugin}] ${preset.name} icon`,
      () =>
        Effect.gen(function* () {
          const response = yield* Effect.tryPromise(() =>
            fetch(preset.icon!, {
              method: "GET",
              signal: AbortSignal.timeout(10_000),
              headers: { "User-Agent": "executor-preset-test" },
              redirect: "follow",
            }),
          );
          expect(response.ok).toBe(true);
        }),
      { timeout: 15_000 },
    );
  }
});
```

**Key patterns:**
1. **User-Agent header** — Identify test traffic
2. **Simple GET** — Verify icon URL works
3. **OK check** — 200 status required

---

## 5. Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Test Architecture                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Root Tests (tests/)                               │   │
│  │                                                                       │   │
│  │  release-bootstrap-smoke.test.ts                                    │   │
│  │    ├── Builds CLI binary                                            │   │
│  │    ├── Builds release assets                                        │   │
│  │    ├── Mocks GitHub release server                                  │   │
│  │    ├── Tests wrapper install flow                                   │   │
│  │    ├── Verifies web server startup                                  │   │
│  │    └── Tests cached subsequent runs                                 │   │
│  │                                                                       │   │
│  │  presets-reachable.test.ts                                          │   │
│  │    ├── OpenAPI presets → parse specs                                │   │
│  │    ├── GraphQL presets → introspect endpoints                       │   │
│  │    ├── MCP presets → probe endpoints                                │   │
│  │    ├── Google Discovery → parse manifests                           │   │
│  │    ├── Detection tests → verify correct plugin                      │   │
│  │    └── Icon tests → verify icon URLs                                │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                    Package Tests                                     │   │
│  │                                                                       │   │
│  │  Core: SDK, Execution, Config, Storage                              │   │
│  │  Kernel: QuickJS, Deno Subprocess                                   │   │
│  │  Hosts: MCP Server                                                  │   │
│  │  Plugins: openapi, mcp, graphql, google-discovery, keychain         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Data Flow

### Release Bootstrap Test Flow

```
Test Start
    │
    ▼
┌─────────────────────────────┐
│  Build CLI binary           │
│  bun run src/build.ts       │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  Build release assets       │
│  tar.gz / zip packages      │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  Start mock HTTP server     │
│  Serve asset at /releases/  │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  First run: --help          │
│  Downloads asset            │
│  Extracts to temp dir       │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  Start web server           │
│  Verify / responds 200      │
│  Verify /docs responds 200  │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  Second run: --help         │
│  Uses cached asset          │
│  No "downloading" message   │
└───────────┬─────────────────┘
            │
            ▼
    Cleanup: rm temp dir, close servers
```

### Preset Reachability Flow

```
Test Suite Start
    │
    ▼
┌─────────────────────────────┐
│  Load all presets           │
│  - openapiPresets           │
│  - mcpPresets               │
│  - graphqlPresets           │
│  - googleDiscoveryPresets   │
└───────────┬─────────────────┘
            │
            ▼
┌─────────────────────────────┐
│  OpenAPI: parse(url)        │
│  → Validate OpenAPI doc     │
└─────────────────────────────┘

┌─────────────────────────────┐
│  GraphQL: introspect(url)   │
│  → Schema or 401/403        │
└─────────────────────────────┘

┌─────────────────────────────┐
│  MCP: POST fetch(url)       │
│  → Non-404/502/503          │
└─────────────────────────────┘

┌─────────────────────────────┐
│  Google: fetch + parse()    │
│  → Manifest with methods    │
└─────────────────────────────┘

┌─────────────────────────────┐
│  Detection: executor.detect │
│  → Correct plugin kind      │
└─────────────────────────────┘

┌─────────────────────────────┐
│  Icons: GET fetch(icon)     │
│  → 200 OK                   │
└─────────────────────────────┘
```

---

## 7. Key Patterns

### Effect-Based Tests

```typescript
it.effect(
  "test name",
  () =>
    Effect.gen(function* () {
      const result = yield* someEffect();
      expect(result).toBeDefined();
    }),
  { timeout: 15_000 },
);
```

**Benefits:**
1. **Effect runtime** — Automatic cleanup on failure
2. **Timeout support** — Per-test timeout configuration
3. **Composable** — Can use Effect operators

### Mock HTTP Server

```typescript
const server = createServer(async (request, response) => {
  if (request.url !== expectedRoute) {
    response.statusCode = 404;
    response.end("not found");
    return;
  }
  const body = await readFile(assetPath);
  response.statusCode = 200;
  response.end(body);
});
const port = await listen(server);
```

**Benefits:**
1. **Isolated testing** — No external dependencies
2. **Deterministic** — Controlled responses
3. **Fast** — Local server only

### Platform Detection

```typescript
const isSupportedPlatform = ["darwin", "linux", "win32"].includes(process.platform) &&
  ["x64", "arm64"].includes(process.arch);

if (!isSupportedPlatform) {
  return; // Skip test
}
```

**Benefits:**
1. **Skip unsupported** — Don't fail on ARM Windows etc.
2. **Clear intent** — Document supported platforms
3. **CI efficiency** — Don't waste time on known failures

### Graceful Error Handling

```typescript
const result = yield* introspect(url).pipe(
  Effect.map((r) => ({ ok: true, schema: r })),
  Effect.catchAll((err) =>
    Effect.succeed({ ok: false, message: String(err) }),
  ),
);

if (result.ok) {
  expect(result.schema.__schema).toBeDefined();
} else {
  // Auth-required is acceptable
  expect(result.message).toMatch(/401|403|Unauthorized|Forbidden|auth/i);
}
```

**Benefits:**
1. **Realistic expectations** — Auth-required ≠ broken
2. **Clear validation** — Document acceptable failures
3. **No false negatives** — Test intent is reachability

---

## 8. Integration Points

### Test Dependencies

| Package | Purpose |
|---------|---------|
| `@effect/vitest` | Effect integration with Vitest |
| `vitest` | Test framework |
| `effect` | Effect runtime for tests |

### External Dependencies

| Service | Test Purpose |
|---------|-------------|
| GitHub Releases | Release asset hosting (mocked) |
| OpenAPI specs | Validate spec parsing |
| GraphQL endpoints | Verify introspection |
| MCP servers | Confirm endpoint availability |
| Google Discovery | Test manifest parsing |

---

## 9. Error Handling

### Command Execution

```typescript
const runCommand = async (command, args, cwd, env): Promise<CommandResult> => {
  const child = spawn(command, args, { cwd, env, stdio: ["ignore", "pipe", "pipe"] });
  
  let stdout = "";
  let stderr = "";
  
  child.stdout.on("data", (chunk) => { stdout += chunk; });
  child.stderr.on("data", (chunk) => { stderr += chunk; });
  
  const exitCode = await new Promise<number>((resolve, reject) => {
    child.once("error", reject);
    child.once("close", (code) => resolve(code ?? -1));
  });
  
  return { exitCode, stdout, stderr };
};
```

**Strategy:** Capture all output, include in assertion messages.

### Server Cleanup

```typescript
const closeServer = async (server): Promise<void> =>
  new Promise((resolve, reject) => {
    server.close((error) => {
      if (error) { reject(error); return; }
      resolve();
    });
  });
```

**Strategy:** Graceful close with error propagation.

### Process Cleanup

```typescript
try {
  // Test code
} finally {
  webProcess.kill("SIGTERM");
  await Promise.race([
    new Promise((resolve) => webProcess.once("close", resolve)),
    new Promise((resolve) => setTimeout(resolve, 5_000)),
  ]);
  if (webProcess.exitCode === null) {
    webProcess.kill("SIGKILL");
  }
  await closeServer(server);
  await rm(tempRoot, { recursive: true, force: true });
}
```

**Strategy:** SIGTERM, wait, SIGKILL fallback, cleanup temp files.

---

## 10. Testing Strategy

### Unit Tests

- **SDK tests** — Schema types, initialization
- **Config tests** — Loading, writing, validation
- **Storage tests** — KV operations, migrations
- **Kernel tests** — Runtime execution, timeouts

### Integration Tests

- **Tool invoker** — Tool search scoring
- **MCP server** — Session handling
- **Plugin tests** — Plugin initialization and extension API

### E2E Tests

- **Release bootstrap** — Full build, install, run cycle
- **Preset reachability** — External API verification
- **Detection tests** — URL → correct plugin

### Test Categories

| Category | Count | Purpose |
|----------|-------|---------|
| Unit | ~20 | Individual function/module tests |
| Integration | ~5 | Cross-module tests |
| E2E | 2 | Full system tests |
| Preset | ~50 | External API tests |

---

## 11. Design Decisions

### Why @effect/vitest?

1. **Effect integration** — Native Effect.Effect support
2. **Resource safety** — Automatic fiber cleanup
3. **Timeout support** — Per-test timeout configuration

### Why Mock Server for Release Tests?

1. **Isolation** — No external GitHub dependency
2. **Speed** — Local server is faster
3. **Determinism** — Controlled test environment

### Why Separate Preset Tests?

1. **External dependencies** — Different failure modes
2. **Longer timeouts** — Network requests take time
3. **Auth handling** — Different expectations for external APIs

### Why Platform Detection?

1. **Binary availability** — Not all platforms have binaries
2. **CI efficiency** — Skip known failures
3. **Clear documentation** — Explicit supported platforms

---

## 12. Summary

The test suite provides **comprehensive coverage**:

1. **E2E Tests** — Release bootstrap validates full installation flow
2. **Preset Tests** — External API reachability verification
3. **Unit Tests** — Core functionality testing
4. **Integration Tests** — Cross-module interaction
5. **Effect-Based** — Leverages Effect runtime for safety

Key patterns include:
- **Mock HTTP servers** — Isolated testing without external deps
- **Graceful error handling** — Auth-required ≠ broken
- **Platform detection** — Skip unsupported platforms
- **Process cleanup** — SIGTERM, wait, SIGKILL fallback

The test strategy ensures **reliable releases** through automated verification of the full installation and execution flow.
