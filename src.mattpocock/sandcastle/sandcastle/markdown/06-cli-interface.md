# Sandcastle — CLI Interface

## Entry Point

Source: `src/main.ts` — Effect platform entry point

```typescript
// src/main.ts
import { NodeContext, NodeRuntime } from "@effect/platform-node";
import { Effect, Layer } from "effect";
import { cli } from "./cli.js";
import { ClackDisplay } from "./Display.js";
import { withFriendlyErrors } from "./ErrorHandler.js";
import { setupTerminalCleanup } from "./terminalCleanup.js";

setupTerminalCleanup();  // Restore terminal state on any exit

const mainLayer = Layer.merge(NodeContext.layer, ClackDisplay.layer);

cli(process.argv).pipe(
  withFriendlyErrors,
  Effect.provide(mainLayer),
  NodeRuntime.runMain,
);
```

**Aha:** The terminal cleanup hook (`setupTerminalCleanup`) is registered before anything else runs. `@clack/prompts`' spinner/taskLog set stdin to raw mode and hide the cursor. If the process exits via a signal handler that calls `process.exit()` directly (e.g. the Ctrl-C handler in SandboxFactory), clack's own cleanup is bypassed, leaving the terminal broken. The exit hook fixes this.

## CLI Commands

Source: `src/cli.ts` — `@effect/cli` command definitions

### `sandcastle init`

Scaffolds the `.sandcastle/` config directory and builds the container image. This is a **wizard** that prompts for choices, with paired `--flags` for non-interactive use.

**Interactive prompts:**
1. Agent (claude-code, pi, codex, cursor, opencode, copilot)
2. Model (e.g. `claude-sonnet-4-6`)
3. Sandbox provider (docker, podman)
4. Issue tracker (github-issues, beads, custom)
5. Template (blank, simple-loop, sequential-reviewer, parallel-planner, parallel-planner-with-review)
6. Create Sandcastle GitHub label (yes/no, only with github-issues)
7. Build image (yes/no)
8. Install template dependencies (yes/no, e.g. `zod` for planner templates)

**Non-interactive flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--image-name` | `sandcastle:<repo-dir-name>` | Docker image name |
| `--agent` | Prompt | Agent to use |
| `--model` | Agent default | Model to use |
| `--sandbox` | Prompt | Sandbox provider |
| `--template` | Prompt | Template to scaffold |
| `--issue-tracker` | Prompt | Issue tracker to use |
| `--create-label` | Prompt | Create Sandcastle GitHub label |
| `--build-image` | Prompt | Build sandbox image now |
| `--install-template-deps` | Prompt | Install template host deps (e.g. zod) |

**When stdin is not a TTY and a required flag is missing, init fails fast with a clear error rather than wedging on a prompt.**

**What init creates:**
```
.sandcastle/
├── Dockerfile        # or Containerfile for Podman
├── prompt.md         # Agent instructions
├── .env.example      # Token placeholders
└── .gitignore        # Ignores .env, logs/
```

Plus template files (e.g. `main.mts`, additional prompt files, `CODING_STANDARDS.md`).

**Package manager detection:** Init detects the host package manager (npm, pnpm, yarn, or bun) from a `packageManager` field or lockfile, defaulting to npm. Templates whose `main` file imports a host dependency (e.g. Zod for planner templates) prompt you to install it when it isn't already in `package.json`.

**Errors if `.sandcastle/` already exists** to prevent overwriting customizations.

### `sandcastle docker build-image`

Rebuilds the Docker image from `.sandcastle/Dockerfile`:

| Flag | Default | Description |
|------|---------|-------------|
| `--image-name` | `sandcastle:<repo-dir-name>` | Image name |
| `--dockerfile` | `.sandcastle/Dockerfile` | Custom Dockerfile path |

**Aha:** On Linux/macOS, the build automatically passes `--build-arg AGENT_UID=$(id -u)` and `AGENT_GID=$(id -g)` so the image's `agent` user matches the host UID. This prevents permission errors on image-built files without runtime chown.

### `sandcastle docker remove-image`

Removes the Docker image. `--image-name` to override.

### `sandcastle podman build-image`

Builds the Podman image from `.sandcastle/Containerfile`. Same flags as Docker but `--containerfile` instead of `--dockerfile`.

### `sandcastle podman remove-image`

Removes the Podman image. `--image-name` to override.

## InitService

Source: `src/InitService.ts` — scaffolding engine

The InitService manages the registries for templates, agents, issue trackers, and sandbox providers:

- **Template registry**: 5 templates with metadata (name, description, main file, prompt files)
- **Agent registry**: 6 agents with per-agent Dockerfile templates and `{{ISSUE_TRACKER_TOOLS}}` placeholder
- **Issue tracker registry**: GitHub Issues, Beads, Custom with template args (`LIST_TASKS_COMMAND`, `VIEW_TASK_COMMAND`, etc.)
- **Sandbox provider registry**: Docker, Podman with image build commands

The `scaffold()` method:
1. Creates `.sandcastle/` directory
2. Writes Dockerfile/Containerfile with issue tracker tools substituted
3. Writes prompt.md, .env.example, .gitignore
4. Copies template files
5. Rewrites main file imports/calls to match selected agent/model/provider

## Display System

Source: `src/Display.ts` — `ClackDisplay`, `FileDisplay`, `SilentDisplay`, `Display` Effect Context

Three display implementations via Effect Context:

| Implementation | Purpose | When Used |
|---------------|---------|-----------|
| `ClackDisplay` | Interactive terminal UI via `@clack/prompts` | Default in `main.ts` for all CLI output |
| `FileDisplay` | Writes to log file with run delimiters, mid-line tracking | Inside `run()` when `logging.type === "file"` — agent output only |
| `SilentDisplay` | Accumulates entries in a Ref | Tests |

**How display selection works:** `main.ts` wires `ClackDisplay.layer` unconditionally for the CLI's own output (intro, summary, status). Inside `run()`, a separate Display layer is built: `FileDisplay` when logging to file (the agent's progress output goes to the log), or `ClackDisplay` when logging to stdout (the agent's output uses the same terminal UI). The two layers are merged so the CLI's own messages and the agent's output share the same terminal.

**FileDisplay** writes human-readable output to the log file:
- Run delimiters at start/end
- Mid-line tracking for streamed chunks (handles partial lines from agent)
- Status messages with severity (success/warn/error)

**Aha:** In file-logging mode, `run()` prints a startup message with a `tail -f` command so you can watch progress in another terminal. When `cwd` differs from `process.cwd()`, the log path is shown as absolute so it can be pasted into any terminal.

## Terminal Cleanup

Source: `src/terminalCleanup.ts` — `setupTerminalCleanup()`

Registers an `exit` hook that restores terminal state. Without this, signal-caught exits (Ctrl-C, SIGTERM) can leave the terminal in a broken state because clack's cleanup is bypassed.

---

Next: [07-utilities.md](./07-utilities.md) — BoundedTail, raceAbortSignal, shutdownRegistry, ErrorHandler, errors, TextDeltaBuffer, and other utility modules.
