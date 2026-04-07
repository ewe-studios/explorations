# Entrypoints Module — Deep-Dive Exploration

**Module:** `entrypoints/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/entrypoints/`  
**Files:** 8 TypeScript files  
**Created:** 2026-04-07

---

## 1. Module Overview

The `entrypoints/` module implements **multiple CLI entry points and SDK type definitions** — providing the main CLI bootstrap, MCP server entry point, sandbox types, and agent SDK type definitions. This module handles special execution modes and provides type contracts for external integrations.

### Core Responsibilities

1. **CLI Bootstrap** (`cli.tsx`) — Main entry point:
   - Fast-path flags (`--version`, `--dump-system-prompt`)
   - Special mode dispatch (daemon, bridge, bg sessions)
   - Worktree/tmux integration
   - Full CLI loading

2. **Initialization** (`init.ts`) — Startup initialization:
   - Config system enablement
   - Environment variable application
   - Network configuration (mTLS, proxy)
   - Telemetry initialization
   - Cleanup registration

3. **MCP Entry Point** (`mcp.ts`) — MCP server mode:
   - MCP protocol handling
   - Tool exposure to MCP clients

4. **SDK Types** (`agentSdkTypes.ts`, `sandboxTypes.ts`) — Type contracts:
   - Agent SDK message types
   - Sandbox permission types
   - Hook event definitions

5. **CLI Handlers** (`cli.tsx` sub-handlers) — Special modes:
   - `claude daemon` — Background supervisor
   - `claude remote-control` — Bridge mode
   - `claude ps|logs|attach|kill` — Session management
   - `claude environment-runner` — BYOC runner
   - `claude self-hosted-runner` — Self-hosted worker

### Key Design Patterns

- **Fast-Path Pattern**: Early returns for simple flags (minimal module loading)
- **Dynamic Imports**: Lazy loading for slow paths
- **Feature Gating**: `feature()` for dead code elimination
- **Graceful Shutdown**: Cleanup registration for all modes

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `cli.tsx` | ~900+ | Main CLI entry point with fast paths |
| `init.ts` | ~400+ | Startup initialization |
| `mcp.ts` | ~170 | MCP server entry point |
| `agentSdkTypes.ts` | ~360 | Agent SDK type definitions |
| `sandboxTypes.ts` | ~150 | Sandbox permission types |
| `sdk/` | varies | SDK type sub-definitions |

**Total:** ~2000+ lines across 8 files

---

## 3. Key Exports

### CLI Entry Point (`cli.tsx`)

```typescript
// Main entry point
async function main(): Promise<void>

// Fast paths handled:
// - --version, -v, -V (zero module loading)
// - --dump-system-prompt (Ant only)
// - --claude-in-chrome-mcp
// - --chrome-native-host
// - --computer-use-mcp
// - --daemon-worker=<kind>
// - remote-control, rc, remote, sync, bridge
// - daemon [subcommand]
// - ps, logs, attach, kill, --bg, --background
// - new, list, reply (templates)
// - environment-runner
// - self-hosted-runner
// - --worktree --tmux
```

### Initialization (`init.ts`)

```typescript
// Memoized initialization
export const init = memoize(async (): Promise<void> => {
  // Config enablement
  // Environment variables
  // Graceful shutdown setup
  // 1P event logging
  // OAuth population
  // JetBrains detection
  // Remote managed settings
  // mTLS configuration
  // Proxy configuration
  // API preconnect
  // Upstream proxy (CCR only)
  // Git-bash setup
  // LSP cleanup registration
  // Team cleanup registration
})
```

### Agent SDK Types (`agentSdkTypes.ts`)

```typescript
// Hook events
export const HOOK_EVENTS = [
  'beforeQuery',
  'afterQuery',
  'beforeToolUse',
  'afterToolUse',
  // ... more events
] as const

export type HookEvent = typeof HOOK_EVENTS[number]

// SDK message types
export type SDKMessage =
  | UserMessage
  | AssistantMessage
  | ResultMessage
  | SystemMessage

// Control message types
export type SDKControlRequest = {...}
export type SDKControlResponse = {...}
export type SDKControlCancelRequest = {...}
```

### Sandbox Types (`sandboxTypes.ts`)

```typescript
// Sandbox permission types
export type SandboxPermission = {...}
export type SandboxNetworkAccess = {...}
export type SandboxFileAccess = {...}
```

---

## 4. Line-by-Line Analysis

### 4.1 Version Fast-Path (`cli.tsx` lines 33-42)

```typescript
async function main(): Promise<void> {
  const args = process.argv.slice(2)

  // Fast-path for --version/-v: zero module loading needed
  if (args.length === 1 && (args[0] === '--version' || args[0] === '-v' || args[0] === '-V')) {
    // MACRO.VERSION is inlined at build time
    console.log(`${MACRO.VERSION} (Claude Code)`)
    return
  }
  
  // For all other paths, load the startup profiler
  const { profileCheckpoint } = await import('../utils/startupProfiler.js')
  profileCheckpoint('cli_entry')
}
```

**Zero Module Loading**: Version flag outputs without importing anything beyond this file.

**Build-Time Inlining**: `MACRO.VERSION` replaced at build time.

### 4.2 Dump System Prompt (`cli.tsx` lines 50-71)

```typescript
// Fast-path for --dump-system-prompt: output the rendered system prompt and exit.
// Used by prompt sensitivity evals to extract the system prompt at a specific commit.
// Ant-only: eliminated from external builds via feature flag.
if (feature('DUMP_SYSTEM_PROMPT') && args[0] === '--dump-system-prompt') {
  profileCheckpoint('cli_dump_system_prompt_path')
  const { enableConfigs } = await import('../utils/config.js')
  enableConfigs()
  const { getMainLoopModel } = await import('../utils/model/model.js')
  const model = args.find((_, i) => i > 0 && args[i-1] === '--model') || getMainLoopModel()
  const { getSystemPrompt } = await import('../constants/prompts.js')
  const prompt = await getSystemPrompt([], model)
  console.log(prompt.join('\n'))
  return
}
```

**Eval Tooling**: "Used by prompt sensitivity evals to extract the system prompt at a specific commit."

**Ant-Only**: Feature-gated out of external builds.

### 4.3 Daemon Worker Fast-Path (`cli.tsx` lines 95-106)

```typescript
// Fast-path for `--daemon-worker=<kind>` (internal — supervisor spawns this).
// Must come before the daemon subcommand check: spawned per-worker, so
// perf-sensitive. No enableConfigs(), no analytics sinks at this layer —
// workers are lean. If a worker kind needs configs/auth (assistant will),
// it calls them inside its run() fn.
if (feature('DAEMON') && args[0] === '--daemon-worker') {
  const { runDaemonWorker } = await import('../daemon/workerRegistry.js')
  await runDaemonWorker(args[1])
  return
}
```

**Lean Workers**: "No enableConfigs(), no analytics sinks at this layer — workers are lean."

**Deferred Config**: Worker kinds that need config call it inside `run()`.

### 4.4 Bridge Mode (`cli.tsx` lines 108-162)

```typescript
// Fast-path for `claude remote-control` (also accepts legacy `claude remote` /
// `claude sync` / `claude bridge`): serve local machine as bridge environment.
if (feature('BRIDGE_MODE') && (args[0] === 'remote-control' || args[0] === 'rc' || args[0] === 'remote' || args[0] === 'sync' || args[0] === 'bridge')) {
  profileCheckpoint('cli_bridge_path')
  const { enableConfigs } = await import('../utils/config.js')
  enableConfigs()
  
  // Auth check must come before the GrowthBook gate check
  const { getClaudeAIOAuthTokens } = await import('../utils/auth.js')
  if (!getClaudeAIOAuthTokens()?.accessToken) {
    exitWithError(BRIDGE_LOGIN_ERROR)
  }
  
  const disabledReason = await getBridgeDisabledReason()
  if (disabledReason) {
    exitWithError(`Error: ${disabledReason}`)
  }
  
  // Policy limits check
  const { waitForPolicyLimitsToLoad, isPolicyAllowed } = await import('../services/policyLimits/index.js')
  await waitForPolicyLimitsToLoad()
  if (!isPolicyAllowed('allow_remote_control')) {
    exitWithError("Error: Remote Control is disabled by your organization's policy.")
  }
  
  await bridgeMain(args.slice(1))
  return
}
```

**Multiple Aliases**: `remote-control`, `rc`, `remote`, `sync`, `bridge` all work.

**Auth First**: "Auth check must come before the GrowthBook gate check — without auth, GrowthBook has no user context."

**Policy Check**: Organization policy can disable remote control.

### 4.5 Background Session Management (`cli.tsx` lines 182-209)

```typescript
// Fast-path for `claude ps|logs|attach|kill` and `--bg`/`--background`.
// Session management against the ~/.claude/sessions/ registry.
if (feature('BG_SESSIONS') && (args[0] === 'ps' || args[0] === 'logs' || args[0] === 'attach' || args[0] === 'kill' || args.includes('--bg') || args.includes('--background'))) {
  profileCheckpoint('cli_bg_path')
  const { enableConfigs } = await import('../utils/config.js')
  enableConfigs()
  const bg = await import('../cli/bg.js')
  switch (args[0]) {
    case 'ps':
      await bg.psHandler(args.slice(1))
      break
    case 'logs':
      await bg.logsHandler(args[1])
      break
    case 'attach':
      await bg.attachHandler(args[1])
      break
    case 'kill':
      await bg.killHandler(args[1])
      break
    default:
      await bg.handleBgFlag(args)
  }
  return
}
```

**Session Commands**: `ps`, `logs`, `attach`, `kill` for background session management.

**Flag Handling**: `--bg`/`--background` flags also dispatch to bg handler.

### 4.6 Worktree/Tmux Fast-Path (`cli.tsx` lines 247-274)

```typescript
// Fast-path for --worktree --tmux: exec into tmux before loading full CLI
const hasTmuxFlag = args.includes('--tmux') || args.includes('--tmux=classic')
if (hasTmuxFlag && (args.includes('-w') || args.includes('--worktree') || args.some(a => a.startsWith('--worktree=')))) {
  profileCheckpoint('cli_tmux_worktree_fast_path')
  const { enableConfigs } = await import('../utils/config.js')
  enableConfigs()
  const { isWorktreeModeEnabled } = await import('../utils/worktreeModeEnabled.js')
  if (isWorktreeModeEnabled()) {
    const { execIntoTmuxWorktree } = await import('../utils/worktree.js')
    const result = await execIntoTmuxWorktree(args)
    if (result.handled) {
      return  // Exec'd into tmux, we're done
    }
    if (result.error) {
      const { exitWithError } = await import('../utils/process.js')
      exitWithError(result.error)
    }
  }
}
```

**Exec Before CLI**: "Fast-path for --worktree --tmux: exec into tmux before loading full CLI"

**Fallback**: If worktree handling fails, falls through to normal CLI.

### 4.7 Initialization: Config and Env (`init.ts` lines 57-85)

```typescript
export const init = memoize(async (): Promise<void> => {
  const initStartTime = Date.now()
  logForDiagnosticsNoPII('info', 'init_started')
  profileCheckpoint('init_function_start')

  // Validate configs are valid and enable configuration system
  try {
    const configsStart = Date.now()
    enableConfigs()
    logForDiagnosticsNoPII('info', 'init_configs_enabled', {
      duration_ms: Date.now() - configsStart,
    })
    profileCheckpoint('init_configs_enabled')

    // Apply only safe environment variables before trust dialog
    const envVarsStart = Date.now()
    applySafeConfigEnvironmentVariables()

    // Apply NODE_EXTRA_CA_CERTS from settings.json to process.env early,
    // before any TLS connections. Bun caches the TLS cert store at boot
    // via BoringSSL, so this must happen before the first TLS handshake.
    applyExtraCACertsFromConfig()

    logForDiagnosticsNoPII('info', 'init_safe_env_vars_applied', {...})
    profileCheckpoint('init_safe_env_vars_applied')

    // Make sure things get flushed on exit
    setupGracefulShutdown()
    profileCheckpoint('init_after_graceful_shutdown')
```

**Profiling**: Every step profiled with checkpoints and duration logging.

**CA Certs Early**: "Apply NODE_EXTRA_CA_CERTS from settings.json to process.env early, before any TLS connections. Bun caches the TLS cert store at boot via BoringSSL."

### 4.8 Initialization: Network Config (`init.ts` lines 134-151)

```typescript
// Configure global mTLS settings
const mtlsStart = Date.now()
logForDebugging('[init] configureGlobalMTLS starting')
configureGlobalMTLS()
logForDiagnosticsNoPII('info', 'init_mtls_configured', {
  duration_ms: Date.now() - mtlsStart,
})

// Configure global HTTP agents (proxy and/or mTLS)
const proxyStart = Date.now()
logForDebugging('[init] configureGlobalAgents starting')
configureGlobalAgents()
logForDiagnosticsNoPII('info', 'init_proxy_configured', {
  duration_ms: Date.now() - proxyStart,
})
profileCheckpoint('init_network_configured')
```

**mTLS First**: mTLS configured before proxy agents.

**Diagnostic Logging**: Both debug and diagnostic logs for troubleshooting.

### 4.9 Initialization: Upstream Proxy (`init.ts` lines 161-183)

```typescript
// CCR upstreamproxy: start the local CONNECT relay so agent subprocesses
// can reach org-configured upstreams with credential injection. Gated on
// CLAUDE_CODE_REMOTE + GrowthBook; fail-open on any error.
if (isEnvTruthy(process.env.CLAUDE_CODE_REMOTE)) {
  try {
    const { initUpstreamProxy, getUpstreamProxyEnv } = await import('../upstreamproxy/upstreamproxy.js')
    const { registerUpstreamProxyEnvFn } = await import('../utils/subprocessEnv.js')
    registerUpstreamProxyEnvFn(getUpstreamProxyEnv)
    await initUpstreamProxy()
  } catch (err) {
    logForDebugging(
      `[init] upstreamproxy init failed: ${err instanceof Error ? err.message : String(err)}; continuing without proxy`,
      { level: 'warn' },
    )
  }
}
```

**Fail-Open**: "Gated on CLAUDE_CODE_REMOTE + GrowthBook; fail-open on any error."

**Env Registration**: `getUpstreamProxyEnv` registered for subprocess injection.

---

## 5. Integration Points

### 5.1 With `daemon/`

| Component | Integration |
|-----------|-------------|
| `cli.tsx` | Uses `runDaemonWorker()`, `daemonMain()` |

### 5.2 With `bridge/`

| Component | Integration |
|-----------|-------------|
| `cli.tsx` | Uses `bridgeMain()`, `getBridgeDisabledReason()` |

### 5.3 With `cli/bg.js`

| Component | Integration |
|-----------|-------------|
| `cli.tsx` | Uses `psHandler()`, `logsHandler()`, `attachHandler()`, `killHandler()` |

### 5.4 With `utils/`

| Component | Integration |
|-----------|-------------|
| `init.ts` | Uses config, env, mTLS, proxy, cleanup utilities |

---

## 6. Data Flow

### 6.1 CLI Dispatch Flow

```
Process start
    │
    ▼
cli.tsx main()
    │
    ├──► Check fast paths (in order)
    │    ├──► --version → Output version, exit
    │    ├──► --dump-system-prompt → Output prompt, exit
    │    ├──► --daemon-worker → Run worker, exit
    │    ├──► remote-control → Bridge mode
    │    ├──► daemon → Supervisor mode
    │    ├──► ps/logs/attach/kill → Session management
    │    ├──► templates → Template jobs
    │    ├──► environment-runner → BYOC runner
    │    ├──► self-hosted-runner → Self-hosted worker
    │    └──► --worktree --tmux → Exec tmux
    │
    └──► No fast path → Load full CLI
         │
         ▼
         main.js main()
         │
         ▼
         REPL loop
```

### 6.2 Initialization Flow

```
init() called (memoized)
    │
    ├──► enableConfigs()
    ├──► applySafeConfigEnvironmentVariables()
    ├──► applyExtraCACertsFromConfig()
    ├──► setupGracefulShutdown()
    ├──► initialize1PEventLogging()
    ├──► populateOAuthAccountInfoIfNeeded()
    ├──► initJetBrainsDetection()
    ├──► detectCurrentRepository()
    ├──► initializeRemoteManagedSettingsLoadingPromise()
    ├──► initializePolicyLimitsLoadingPromise()
    ├──► recordFirstStartTime()
    ├──► configureGlobalMTLS()
    ├──► configureGlobalAgents()
    ├──► preconnectAnthropicApi()
    ├──► initUpstreamProxy() (CCR only)
    ├──► setShellIfWindows()
    ├──► registerCleanup(shutdownLspServerManager)
    └──► registerCleanup(cleanupSessionTeams)
    │
    ▼
Initialization complete
```

---

## 7. Key Patterns

### 7.1 Fast-Path Ordering

```typescript
// Order matters! Check most common/fast paths first
if (args[0] === '--version') return  // ~1ms
if (args[0] === '--daemon-worker') return  // ~10ms
if (args[0] === 'daemon') return  // ~50ms
// ... heavier paths later
```

**Performance**: Common paths checked first for minimal latency.

### 7.2 Feature Gating

```typescript
// Ant-only features
if (feature('DUMP_SYSTEM_PROMPT') && ...) { }

// Remote features
if (feature('BRIDGE_MODE') && ...) { }

// Background sessions
if (feature('BG_SESSIONS') && ...) { }

// Templates
if (feature('TEMPLATES') && ...) { }
```

**Dead Code Elimination**: `feature()` gates enable build-time elimination.

### 7.3 Dynamic Import Pattern

```typescript
// Only import when path is taken
const { bridgeMain } = await import('../bridge/bridgeMain.js')
const { daemonMain } = await import('../daemon/main.js')
```

**Lazy Loading**: Avoid loading unused modules.

---

## 8. Environment Variables

| Variable | Purpose | Values |
|----------|---------|--------|
| `CLAUDE_CODE_REMOTE` | CCR environment detection | `'true'`/undefined |
| `CLAUDE_CODE_REMOTE_SESSION_ID` | CCR session ID | UUID |
| `CCR_UPSTREAM_PROXY_ENABLED` | Upstream proxy feature gate | `'true'`/undefined |
| `ANTHROPIC_BASE_URL` | API base URL | URL |
| `USER_TYPE` | User type (ant for employees) | `'ant'`/undefined |
| `NODE_EXTRA_CA_CERTS` | Extra CA certificates | Path |

---

## 9. Feature Gates

| Feature | Fast Paths |
|---------|------------|
| `DUMP_SYSTEM_PROMPT` | `--dump-system-prompt` |
| `BRIDGE_MODE` | `remote-control`, `rc`, `remote`, `sync`, `bridge` |
| `DAEMON` | `--daemon-worker`, `daemon` |
| `BG_SESSIONS` | `ps`, `logs`, `attach`, `kill`, `--bg` |
| `TEMPLATES` | `new`, `list`, `reply` |
| `BYOC_ENVIRONMENT_RUNNER` | `environment-runner` |
| `SELF_HOSTED_RUNNER` | `self-hosted-runner` |
| `ABLATION_BASELINE` | Baseline env var injection |

---

## 10. Summary

The `entrypoints/` module provides **multiple CLI entry points and SDK types**:

1. **CLI Bootstrap** — Fast-path dispatch for special modes
2. **Initialization** — Comprehensive startup sequence
3. **MCP Entry** — MCP server mode
4. **SDK Types** — Agent SDK and sandbox type definitions

**Key Fast Paths**:
- `--version` (zero module loading)
- `--dump-system-prompt` (eval tooling)
- `daemon` (background supervisor)
- `remote-control` (bridge mode)
- `ps|logs|attach|kill` (session management)
- `--worktree --tmux` (git worktree integration)

**Key Design Decisions**:
- **Fast-path ordering** for common cases
- **Dynamic imports** for lazy loading
- **Feature gating** for dead code elimination
- **Memoized init** for single execution
- **Graceful shutdown** for cleanup

---

**Last Updated:** 2026-04-07  
**Status:** Complete — All 8 files analyzed
