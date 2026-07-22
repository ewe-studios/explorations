# Sandcastle — Agent Integration

## The AgentProvider Interface

Every agent factory returns an `AgentProvider` object with a consistent interface:

Source: `src/AgentProvider.ts` — `AgentProvider` interface, agent factories

```typescript
interface AgentProvider {
  readonly name: string;                    // Display name (e.g. "claude-code")
  readonly env: Record<string, string>;     // Injected env vars
  readonly captureSessions: boolean;        // Whether to capture session JSONL
  readonly sessionStorage?: AgentSessionStorage;  // Resume capability (undefined = non-resumable)
  buildPrintCommand(options: AgentCommandOptions): PrintCommand;  // AFK command
  buildInteractiveArgs?(options): string[];  // Interactive CLI args
  parseStreamLine(line: string): ParsedStreamEvent[];  // Stdout line parser
}
```

**Aha:** `ParsedStreamEvent` is a discriminated union: `{ type: "text" | "result" | "tool_call" | "session_id" | "usage" }`. Each agent provider's `parseStreamLine()` parses that agent's specific stdout format into this shared type. This is how Sandcastle normalizes six completely different agent output formats into a single stream of text chunks, tool calls, and session IDs.

## Six Agent Providers

| Factory | Resume? | Session Capture | Stream Format |
|---------|---------|-----------------|---------------|
| `claudeCode(model, opts)` | Yes | Yes (`~/.claude/projects/`) | JSONL (`--output-format stream-json`) |
| `codex(model, opts)` | Yes | Yes (`~/.codex/sessions/`) | JSONL events |
| `pi(model, opts)` | Yes | Yes (`~/.pi/agent/sessions/`) | JSONL events (`--mode json`) |
| `cursor(model)` | No | No | `tool_call` events (JSONL) |
| `opencode(model, opts?)` | No | No | `text`/`tool_use` events (JSONL) |
| `copilot(model, opts?)` | No | No | `assistant.message_delta` events (JSONL) |

### Claude Code Provider

```typescript
// src/AgentProvider.ts — claudeCode factory
claudeCode("claude-opus-4-8", {
  effort: "high",           // Reasoning effort level
  env: {},                  // Injected env vars
  captureSessions: true,    // Capture session JSONL
  permissionMode: "auto",   // Maps to --permission-mode flag
});
```

**AFK command:** `claude --print --verbose --dangerously-skip-permissions --output-format stream-json --model <model> -p -` (prompt piped via stdin). With resume: adds `--resume <id>`; with fork: adds `--fork-session`.

**Stream parsing** (`parseStreamLine`): Parses JSONL lines from `stream-json` output:
- `assistant` messages with `text` blocks → text events
- `assistant` messages with `tool_use` blocks → tool call events
- `init` events → session_id extraction
- `system` events → `result` events for completion detection

**Token usage:** Extracted from the last assistant message via `parseSessionUsage()`:
```typescript
interface IterationUsage {
  inputTokens: number;
  cacheCreationInputTokens: number;
  cacheReadInputTokens: number;
  outputTokens: number;
}
```

### Codex Provider

```typescript
codex("gpt-5.4", {
  effort: "high",                    // model_reasoning_effort
  approvalsReviewer: "auto_review",  // Swaps --dangerously-bypass-approvals-and-sandbox
});
```

**AFK command:** `codex exec run --json -m <model>` (prompt piped via stdin). With resume: `codex exec resume <id> --json -m <model> -`. With fork: `codex exec fork <id> --json -m <model>`.

**Stream parsing:** Parses JSONL events:
- `thread.started` → session_id (via `thread_id`)
- `item.completed` with `agent_message` → text content + result
- `item.started` with `command_execution` → tool call (Bash)
- `turn.completed` → token usage
- `error` → error events (auth failures, rate limits, API errors)

**Resume:** `codex exec resume <id>` with prompt piped over stdin.
**Fork:** `codex exec fork <id>`.

**Aha:** The `approvalsReviewer` option changes the CLI flags entirely. When set to `"auto_review"`, it swaps `--dangerously-bypass-approvals-and-sandbox` for `-a on-request -s danger-full-access`, enabling Codex's built-in reviewer agent to evaluate each approval prompt instead of bypassing them.

### Pi Provider

```typescript
pi("claude-sonnet-4-6", {
  thinking: "high",       // --thinking flag
  env: {},
  captureSessions: true,
});
```

**Stream parsing:**
- `session` event → session_id (via `obj.id`)
- `message_update` with `text_delta` → text deltas (single-token chunks, buffered by `TextDeltaBuffer`)
- `tool_execution_start` → tool call events (allowlisted tools only)
- `agent_end` → result (complete assistant message)
- `agent_error` / `error` → error events

**AFK command:** `pi -p --mode json --model <model> --thinking <level>` (prompt piped via stdin). With resume: `pi -p --mode json --model <model> --session <id>`.

**Aha:** Pi emits text as single-token deltas, which makes for very choppy display output. Sandcastle's `TextDeltaBuffer` (in `src/TextDeltaBuffer.ts`) coalesces these into multi-word lines before forwarding to the display, making the output readable.

### Non-Resumable Providers (Cursor, OpenCode, Copilot)

These three agents don't support session resume. Their providers have no `sessionStorage` property and `captureSessions: false`. Attempting to use `resumeSession` or `output.maxRetries` with them throws immediately:

Source: `src/run.ts:563-576`
```typescript
if (outputMaxRetries > 0 && !provider.sessionStorage) {
  throw new Error(
    `output.maxRetries requires an agent provider that supports session resumption. ` +
    `The "${provider.name}" provider does not. ` +
    `Use claudeCode, codex, or pi, or set maxRetries to 0.`,
  );
}
```

**Cursor** AFK command: `agent --print --output-format stream-json --model <model> <prompt>` (binary is `agent`, not `cursor`; prompt passed as argv arg).

**OpenCode** options: `variant` (reasoning effort, e.g. `"high"`) and `agent` (selects agent _inside_ OpenCode, e.g. `"build"`). AFK command: `opencode run --format json --model <model> --variant <v> --agent <a> --dangerously-skip-permissions <prompt>`.

**Copilot** options: `effort` (`"low" | "medium" | "high"`). AFK command: `copilot -p <prompt> --output-format json --model <model> --allow-all-tools --effort <e>` (uses `--allow-all-tools` for bypass, not `--dangerously-skip-permissions`).

## Agent Stream Emitter

Source: `src/AgentStreamEmitter.ts` — `AgentStreamEvent` union, `agentStreamEmitterLayer()`

The `AgentStreamEmitter` is an Effect Context service that broadcasts agent events to external observers:

```typescript
type AgentStreamEvent =
  | { type: "text"; iteration: number; timestamp: Date; text: string }
  | { type: "toolCall"; iteration: number; timestamp: Date; tool: string; input: string }
  | { type: "raw"; iteration: number; timestamp: Date; line: string };
```

Registered via `logging.onAgentStreamEvent` callback. Errors thrown by the callback are swallowed — a broken forwarder must not kill the run.

## Session Capture

After each resumable provider iteration, Sandcastle automatically copies the agent's session file from the sandbox to the host:

Source: `src/SessionStore.ts` — `transferClaudeSession()`, `transferCodexSession()`, `encodeProjectPath()`

### Claude Code

Session location: `~/.claude/projects/<encoded-path>/<session-id>.jsonl`

The `encodedProjectPath()` function converts the sandbox repo path to Claude's hyphenated directory convention:
```
/home/agent/workspace → home-agent-workspace
```

**Subagent capture:** After capturing the main session, Sandcastle also captures any `Agent`-tool or `Workflow`-tool subagent transcripts from `<session-id>/subagents/agent-*.jsonl`. This is best-effort: failure on one transcript logs a warning but doesn't fail the run. However, main-session capture failure **does** fail the run.

### Codex

Session location: `~/.codex/sessions/YYYY/MM/DD/rollout-*-<session-id>.jsonl`

Located via `find` command matching `rollout-*-${id}.jsonl` since the date-based directory path varies per run.

### Pi

Session location: `~/.pi/agent/sessions/--<encoded-cwd>--/<timestamp>_<session-id>.jsonl`

### CWD Rewriting

**Aha:** When a session file is captured, all `cwd` fields inside the JSONL are rewritten to match the host repo root. This is critical — the provider's native resume command (`claude --resume <id>`, `codex exec resume <id>`, `pi --session <id>`) reads the JSONL and needs `cwd` to point to the correct directory. Without rewriting, the agent would try to resume into the sandbox path, which doesn't exist on the host.

The transfer functions are **pure string transformations** — they read the JSONL as text, regex-replace `cwd` values, and write back. No JSON parsing, no schema knowledge needed. Claude and Codex use `cwd` field matching; Pi uses `cwd` field on the header line.

## Session Resume

Pass `resumeSession` to `run()` to continue a prior conversation:

Source: `src/run.ts:805-821`
```typescript
resume: async (prompt, resumeOptions) => {
  const lastIteration = result.iterations.at(-1);
  if (!lastIteration?.sessionId) {
    throw new Error("Cannot resume: no sessionId was captured");
  }
  return run({
    ...options, ...resumeOptions,
    prompt, promptFile: undefined,
    maxIterations: 1,
    resumeSession: lastIteration.sessionId,
  });
});
```

**Constraints:**
- Incompatible with `maxIterations > 1` (throws before sandbox creation)
- Provider's host session file must exist (throws if missing — precheck in `src/resumePrecheck.ts`)
- Only iteration 1 receives the resume flag; subsequent iterations start fresh
- Session file is transferred into the sandbox with `cwd` fields rewritten before sandbox starts

## Session Fork

Source: `RunResult.fork()` in `src/run.ts:822-839`

`fork()` is the sibling of `resume()` — continues from the last captured session but leaves the parent JSONL untouched and writes the child under a new session ID:

```typescript
fork: async (prompt, forkOptions) => {
  const lastIteration = result.iterations.at(-1);
  if (!lastIteration?.sessionId) {
    throw new Error("Cannot fork: no sessionId was captured");
  }
  return run({
    ...options, ...forkOptions,
    prompt, promptFile: undefined,
    maxIterations: 1,
    resumeSession: lastIteration.sessionId,
    forkSession: true,  // Internal flag, wired by fork() method
  });
});
```

**Mechanism:** Uses the agent's native fork flag — `claude --resume <id> --fork-session` for Claude Code, `codex exec fork <id>` for Codex.

**Aha:** Fork isolates the session JSONL only — it does **not** isolate the branch, worktree, or sandbox. Safe concurrent fan-out (`Promise.all([r.fork(a), r.fork(b)])`) requires the caller to give each child a distinct `branch` via `branchStrategy`. The default `head` and `merge-to-head` strategies are **not** safe for concurrent forks: `head` shares the host working directory, and `merge-to-head` races `git merge` against the same HEAD (ADR 0018).

```typescript
const parent = await run({
  agent: claudeCode("claude-opus-4-8"),
  sandbox: docker(),
  prompt: "Read the codebase and summarise the data model",
});

const [reviewA, reviewB] = await Promise.all([
  parent.fork?.("Review the migration plan", {
    branchStrategy: { type: "branch", branch: "review-a" },
  }),
  parent.fork?.("Audit the auth layer", {
    branchStrategy: { type: "branch", branch: "review-b" },
  }),
]);
```

## Structured Output

Source: `src/Output.ts` — `Output`, `StructuredOutputError`; `src/extractStructuredOutput.ts`

Extract a typed payload from the agent's stdout via XML tags:

```typescript
import { Output } from "@ai-hero/sandcastle";
import { z } from "zod";

// JSON + schema extraction
Output.object({
  tag: "result",
  schema: z.object({ summary: z.string(), score: z.number() }),
  maxRetries: 2,  // Built-in retry via session resume
});

// Plain string extraction
Output.string({ tag: "summary" });
```

**Extraction flow** (`src/extractStructuredOutput.ts`):
1. Finds the **last** occurrence of `<tag>...</tag>` in stdout (regex)
2. Unwraps Markdown code fences if present
3. For `Output.object`: JSON parses, validates against Standard Schema
4. For `Output.string`: returns trimmed text

**Constraints:**
- `maxIterations` must be `1` (the default)
- The resolved prompt must contain the configured opening tag literal
- When extraction or validation fails, `run()` throws a `StructuredOutputError`

**Aha:** The `StructuredOutputError` carries recovery data: `commits`, `branch`, `preservedWorktreePath`, `sessionId`, `sessionFilePath`. This means even when extraction fails, the caller knows exactly what the agent produced and can resume the session manually.

### Built-in Retry

When `maxRetries > 0`, Sandcastle handles the retry loop automatically:

Source: `src/run.ts:862-889`
```typescript
if (error instanceof StructuredOutputError && outputMaxRetries > 0 && error.sessionId) {
  return run({
    ...options,
    prompt: buildStructuredOutputRetryFeedback(error, retriesRemainingAfter),
    promptFile: undefined, promptArgs: undefined,
    resumeSession: error.sessionId,
    output: { ...options.output, maxRetries: retriesRemainingAfter },
  });
}
```

The retry prompt is token-efficient — it tells the agent what went wrong, shows the previous output, and asks for a corrected tag without redoing the work:

```
Your previous response did not produce valid structured output.

Retries remaining after this attempt: 1.

Problem: ...
Parser detail: ...
Previous matched output: ...

Emit only a corrected <result> block. Do not change files or run commands.
```

Each retry resumes the same agent session and decrements `maxRetries` so the recursion terminates.

---

Next: [04-orchestration-workflow.md](./04-orchestration-workflow.md) — The Orchestrator, iteration loop, SandboxFactory, worktree management, and sync operations.
