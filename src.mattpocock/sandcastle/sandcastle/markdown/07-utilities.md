# Sandcastle — Utilities

## BoundedTail

Source: `src/boundedTail.ts` — `BoundedTail` class, `MAX_TAIL_CHARS`

A bounded, rolling tail of streamed output — shared by every sandbox provider that streams `exec` output:

```typescript
// src/boundedTail.ts
export const MAX_TAIL_CHARS = 64 * 1024;  // 64KiB — chars, not bytes

class BoundedTail {
  private readonly items: string[] = [];
  private totalChars = 0;
  private readonly maxChars: number;
  private readonly separator: string;

  constructor(maxChars = MAX_TAIL_CHARS, separator = "") {
    this.maxChars = maxChars;
    this.separator = separator;
  }

  push(item: string): void {
    // Single oversized item truncated to its own tail
    const bounded = item.length > this.maxChars
      ? item.slice(item.length - this.maxChars)
      : item;
    this.totalChars += bounded.length + (this.items.length > 0 ? this.separator.length : 0);
    this.items.push(bounded);
    // Evict oldest items to stay within budget
    while (this.totalChars > this.maxChars && this.items.length > 1) {
      const dropped = this.items.shift()!;
      this.totalChars -= dropped.length + this.separator.length;
    }
  }

  toString(): string {
    return this.items.join(this.separator);  // length <= maxChars
  }
}
```

**Why it exists:** When a provider streams output line-by-line, it accumulates the stream to build `ExecResult.stdout`/`stderr`. Consumers read just the tail (last lines of an error, or a fallback for the agent's final result), so retaining the whole stream is unnecessary. Once the accumulated string passes V8's ~512MB max string length, a naive `chunks.join()` throws `RangeError: Invalid string length`, crashing the whole orchestration. 64KiB sits comfortably above any agent completion signal or structured-output payload while staying far below V8's limit.

## shutdownRegistry

Source: `src/shutdownRegistry.ts` — `registerShutdown()`

Process-wide shutdown registry. Sandboxes need synchronous cleanup (remove a container, print worktree-recovery guidance) when the host process is interrupted. Registering `SIGINT`/`SIGTERM`/`exit` listeners per sandbox makes Node emit `MaxListenersExceededWarning` once more than ~10 sandboxes exist concurrently.

This module installs exactly **one listener per signal** — no matter how many sandboxes are alive — and fans out to a set of teardown callbacks:

```typescript
// src/shutdownRegistry.ts
export type ShutdownCallback = () => void;  // Synchronous only

export const registerShutdown = (teardown: ShutdownCallback): (() => void) => {
  teardownCallbacks.add(teardown);
  attachListeners();  // First registration installs the shared listeners
  let active = true;
  return () => {  // Idempotent unregister function
    if (!active) return;
    active = false;
    teardownCallbacks.delete(teardown);
    if (teardownCallbacks.size === 0) detachListeners();
  };
};
```

**Key properties:**
- Callbacks are **synchronous** (`ShutdownCallback = () => void`) — signal handlers cannot await
- On `SIGINT`/`SIGTERM`: runs every teardown, then `process.exit(1)`
- On plain `exit`: runs every teardown without forcing exit code
- Detaches listeners on signal so the subsequent `exit` event doesn't run teardowns twice
- Last unregistration removes the listeners, restoring default signal behavior

**Aha:** Each sandbox registers its own `removeContainerSync` callback on creation, and the unregister function is called when the container is removed normally. This means concurrent sandboxes share a single exit listener instead of each registering their own.

## ErrorHandler

Source: `src/ErrorHandler.ts` — `formatErrorMessage()`, `withFriendlyErrors`

Two functions for converting tagged `SandboxError`s into user-friendly terminal output:

```typescript
// src/ErrorHandler.ts
export const formatErrorMessage = (error: SandboxError): string => {
  switch (error._tag) {
    case "ExecError":
      return `Command failed in sandbox (${error.command}): ${error.message}`;
    case "DockerError":
      return `Docker operation failed: ${error.message}. Is Docker running?`;
    case "PodmanError":
      return `Podman operation failed: ${error.message}. Is Podman running?`;
    case "AgentIdleTimeoutError":
    case "WorktreeTimeoutError":
    case "SessionCaptureError":
      return error.message;
    // ... 20+ error types, all handled
  }
};

export const withFriendlyErrors = <A, E, R>(
  effect: Effect.Effect<A, E, R>,
): Effect.Effect<A, Exclude<E, SandboxError>, R | Display> =>
  Effect.catchTags(effect as Effect.Effect<A, SandboxError, R>, {
    ExecError: showErrorAndExit,
    DockerError: showErrorAndExit,
    // ... all 23 error types mapped to showErrorAndExit
  });
```

`withFriendlyErrors` is the top-level wrapper in `main.ts`:

```typescript
// src/main.ts
cli(process.argv).pipe(
  withFriendlyErrors,  // catches all SandboxError, displays via Display, exits 1
  Effect.provide(mainLayer),
  NodeRuntime.runMain,
);
```

## Errors

Source: `src/errors.ts` — 23 tagged error classes

All errors extend `SandboxError` via `Data.TaggedError`:

```typescript
// src/errors.ts — error classes (all Data.TaggedError)
class ExecError extends Data.TaggedError("ExecError")<{
  readonly message: string;
  readonly command: string;
  readonly exitCode?: number;
}> {}

class ExecHostError extends Data.TaggedError("ExecHostError")<{
  readonly message: string;
  readonly command: string;
}> {}

class PromptError extends Data.TaggedError("PromptError")<{
  readonly message: string;
  readonly exitCode?: number;
}> {}

class AgentError extends Data.TaggedError("AgentError")<{
  readonly message: string;
  readonly preservedWorktreePath?: string;
}> {}

class AgentIdleTimeoutError extends Data.TaggedError("AgentIdleTimeoutError")<{
  readonly message: string;
  readonly timeoutMs: number;
  readonly preservedWorktreePath?: string;
}> {}

class SessionCaptureError extends Data.TaggedError("SessionCaptureError")<{
  readonly message: string;
  readonly sessionId?: string;
}> {}
```

**All 23 error types:**
| Error | Key Fields | When Thrown |
|-------|-----------|-------------|
| `ExecError` | `command`, `exitCode?` | Command failed inside sandbox |
| `ExecHostError` | `command` | Command failed on host |
| `CopyError` | — | File copy failed |
| `DockerError` | — | Docker operation failed |
| `PodmanError` | — | Podman operation failed |
| `SyncError` | — | Git sync (sync-in/sync-out) failed |
| `WorktreeError` | — | Git worktree operation failed |
| `PromptError` | `exitCode?` | Prompt resolution failed |
| `AgentError` | `preservedWorktreePath?` | Agent invocation failed |
| `ConfigDirError` | — | Config directory invalid |
| `InitError` | — | Init scaffolding failed |
| `AgentIdleTimeoutError` | `timeoutMs`, `preservedWorktreePath?` | No output for idle timeout |
| `WorktreeTimeoutError` | `path`, `operation`, `timeoutMs` | Worktree operation timed out |
| `ContainerStartTimeoutError` | — | Container didn't start in time |
| `CopyToWorktreeTimeoutError` | — | File copy to worktree timed out |
| `CopyToWorktreeError` | `path`, `stderr`, `exitCode` | File copy to worktree failed |
| `SyncInTimeoutError` | — | Sync-in (git bundle transfer) timed out |
| `HookTimeoutError` | `command`, `timeoutMs` | Lifecycle hook timed out |
| `GitSetupTimeoutError` | — | Git setup in sandbox timed out |
| `PromptExpansionTimeoutError` | `expression`, `elapsedMs` | `` !`command` `` expansion timed out |
| `CommitCollectionTimeoutError` | — | Collecting commits timed out |
| `MergeToHostTimeoutError` | `sourceBranch`, `targetBranch` | Merging to host timed out |
| `SessionCaptureError` | `sessionId?` | Capturing session JSONL failed |

**Aha:** The `withTimeout()` helper (also in `errors.ts`) wraps any Effect with a custom timeout error, so `ExecError` can carry the command that timed out, `HookTimeoutError` can carry the hook command, etc. This is better than a generic timeout — every timeout error is context-specific.

## TextDeltaBuffer

Source: `src/TextDeltaBuffer.ts`

Buffers token-by-token chunks into multi-word lines for readable display (especially Pi, which emits single-token deltas):

```typescript
// src/TextDeltaBuffer.ts — flush thresholds
const LENGTH_THRESHOLD = 80;   // Flush when buffer >= 80 chars
const DEBOUNCE_MS = 50;        // Flush after 50ms of silence

// Flush triggers (any one fires):
// 1. Newline detected in buffer
// 2. Sentence boundary: /[.!?] $/ 
// 3. Buffer length >= 80 chars
// 4. 50ms debounce timer fires
```

The buffer has `push()`, `flush()`, and `dispose()` methods. The running counter is encapsulated so callers can't desync it.

## EnvResolver + mergeProviderEnv

Source: `src/EnvResolver.ts` — `resolveEnv()`
Source: `src/mergeProviderEnv.ts` — `mergeProviderEnv()`

**EnvResolver** reads `.sandcastle/.env` from the host repo directory and merges with `process.env`. Uses `FileSystem` Effect service for testability.

**mergeProviderEnv** merges resolver output with agent + sandbox provider env:

```typescript
// src/mergeProviderEnv.ts — merge rules
// 1. Start with resolvedEnv output (.sandcastle/.env + process.env)
// 2. Overlay agent provider env
// 3. Overlay sandbox provider env
// 4. Validate: agent env and sandbox env must NOT share any key → throws if overlap
```

This means provider-level `env` options (e.g. `claudeCode("...", { env: { ANTHROPIC_API_KEY: "..." } })`) override `.sandcastle/.env` values, but agent and sandbox provider envs must be disjoint.

## SessionStore

Source: `src/SessionStore.ts` — session transfer primitives

Session capture and transfer functions:
- `encodeProjectPath(cwd)` — converts repo path to Claude's hyphenated convention (`/home/agent/workspace` → `home-agent-workspace`)
- `claudeHostSessionPath(cwd, id)` — computes `~/.claude/projects/<encoded>/<id>.jsonl`
- `claudeSandboxSessionPath(cwd, id)` — computes `/home/agent/.claude/projects/<encoded>/<id>.jsonl`
- `transferClaudeSession(jsonl, fromCwd, toCwd)` — pure string transformation, rewrites `cwd` fields in JSONL
- `findClaudeSessionOnHost(id)` — locates session by id on host
- `findCodexSessionOnHost(id)` — uses `find` command matching `rollout-*-${id}.jsonl` under `~/.codex/sessions/`
- `piSessionDirPath(cwd)` — computes `~/.pi/agent/sessions/<encoded-cwd>/`
- `findPiSessionOnHost(id)` — scans `~/.pi/agent/sessions/` directories for session by id

**Aha:** Pi resolves `--session <id>` against the *current project's* encoded directory first. A transferred file in any other directory hits a "fork session?" prompt that hangs in print/json mode. So Sandcastle lands captured Pi files in the current project's directory regardless of where the sandbox wrote them.

## templates

Source: `src/templates.ts` — `SKELETON_PROMPT`

Contains `SKELETON_PROMPT` — a reusable prompt template with `# Context`, `# Task`, `# Done` sections. The actual template registry (5 templates with metadata) lives in `InitService.ts`, not here.

## resumePrecheck

Source: `src/resumePrecheck.ts` — `assertResumeSessionExists()`

Pre-flight check: validates that the session file exists on the host before sandbox creation. Uses `existsOnHost()` from the provider's `sessionStorage` to check.

## extractStructuredOutput

Source: `src/extractStructuredOutput.ts` — `extractStructuredOutput()`

1. Finds the **last** occurrence of `<tag>...</tag>` in stdout (regex)
2. Unwraps Markdown code fences if present
3. For `Output.object`: JSON parses, validates against Standard Schema
4. For `Output.string`: returns trimmed text

On failure: throws `StructuredOutputError` with `commits`, `branch`, `preservedWorktreePath`, `sessionId`, `sessionFilePath`.

## version

Source: `src/version.ts` — `VERSION` constant

Exports `VERSION` from `__SANDCASTLE_VERSION__` (injected by tsup from `package.json` at build time).

## MountConfig + mountUtils

Source: `src/MountConfig.ts` — `MountConfig` type
Source: `src/mountUtils.ts` — path utilities

Mount configuration types and utilities:
- SELinux label handling (`z`, `Z`, false)
- Path normalization (tilde expansion, absolute resolution)
- Git mount resolution (follows worktree `.git` file to actual gitdir)
- Windows worktree path patching (ADR-0006)
- `formatVolumeMount()` — builds `-v` mount strings with SELinux labels

## resolveCwd

Source: `src/resolveCwd.ts` — `resolveCwd()` (Effect)

Resolves and validates the `cwd` option:
- Relative paths → resolved against `process.cwd()`
- Absolute paths → used as-is
- Invalid path → `CwdError` thrown

Uses `FileSystem` Effect service for testability.

## raceAbortSignal

Source: `src/raceAbortSignal.ts`

Races an Effect against an AbortSignal using `Effect.raceFirst(effect, Deferred.await(abortDeferred))`:
- Checks `signal.aborted` upfront and `Effect.die`s immediately if already aborted
- Creates a `Deferred` that fires on abort via `signal.addEventListener("abort", ...)`
- Uses `Effect.ensuring` to clean up the event listener
- On abort: `Effect.die` propagates as a defect (not a catchable error)

**Aha:** This uses Effect's `die` channel (defect propagation), not a normal error. This means abort isn't caught by `Effect.catchTags` — it propagates as an unhandled defect, which is intentional: an abort should not be silently swallowed by error handling.

## terminalCleanup

Source: `src/terminalCleanup.ts` — `setupTerminalCleanup()`

Registers an `exit` hook that restores terminal state. `@clack/prompts`' spinner/taskLog set stdin to raw mode and hide the cursor. If the process exits via a signal handler that calls `process.exit()` directly (e.g. the Ctrl-C handler in SandboxFactory), clack's own cleanup is bypassed, leaving the terminal broken.

## sandboxExec + startSandbox

Source: `src/sandboxExec.ts` — `sandboxExec()`
Source: `src/startSandbox.ts` — `startSandbox()`

**sandboxExec**: Wrapper for sandbox exec with options (cwd, onLine streaming).

**startSandbox**: Dispatches on `provider.tag`:
- `bind-mount`: Builds mount list (worktree + git mounts + user mounts), calls `provider.create()`
- `isolated`: Calls `provider.create()`, then `syncIn()` via git bundle, then `copyIn()` for optional paths
- `none`: Calls `provider.create()` directly

## interactive

Source: `src/interactive.ts` — `interactive()`

Top-level function for TUI agent sessions. Defaults to `noSandbox()` when no sandbox provided. Validates `copyToWorktree` incompatible with `head` strategy.

---

Next: [08-cross-cutting.md](./08-cross-cutting.md) — Error handling patterns, AbortSignal, shutdown lifecycle, Windows handling, testing, CI/CD.
