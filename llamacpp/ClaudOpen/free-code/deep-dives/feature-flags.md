# free-code Feature Flags Deep-Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code`

A comprehensive audit and explanation of all 88 feature flags in free-code.

---

## Table of Contents

1. [Overview](#overview)
2. [Build System](#build-system)
3. [Working Features](#working-features)
4. [Broken Features](#broken-features)
5. [Feature Implementation](#feature-implementation)
6. [Dead Code Elimination](#dead-code-elimination)

---

## Overview

free-code references **88 `feature('FLAG')` compile-time flags**:

| Status | Count |
|--------|-------|
| Working (bundle cleanly) | 54 |
| Broken (fail to bundle) | 34 |

**Important:** "Bundle cleanly" does not always mean "runtime-safe". Some flags depend on optional native modules, claude.ai OAuth, GrowthBook gates, or externalized `@ant/*` packages.

---

## Build System

### Feature Flag Injection

Location: `scripts/build.ts`

```typescript
import { feature } from 'bun:bundle'

const defaultFeatures = ['VOICE_MODE']

const fullExperimentalFeatures = [
  'AGENT_MEMORY_SNAPSHOT',
  'AGENT_TRIGGERS',
  'AGENT_TRIGGERS_REMOTE',
  'AWAY_SUMMARY',
  'BASH_CLASSIFIER',
  'BRIDGE_MODE',
  'BUILTIN_EXPLORE_PLAN_AGENTS',
  'CACHED_MICROCOMPACT',
  'CCR_AUTO_CONNECT',
  'CCR_MIRROR',
  'CCR_REMOTE_SETUP',
  'COMPACTION_REMINDERS',
  'CONNECTOR_TEXT',
  'EXTRACT_MEMORIES',
  'HISTORY_PICKER',
  'HOOK_PROMPTS',
  'KAIROS_BRIEF',
  'KAIROS_CHANNELS',
  'LODESTONE',
  'MCP_RICH_OUTPUT',
  'MESSAGE_ACTIONS',
  'NATIVE_CLIPBOARD_IMAGE',
  'NEW_INIT',
  'POWERSHELL_AUTO_MODE',
  'PROMPT_CACHE_BREAK_DETECTION',
  'QUICK_SEARCH',
  'SHOT_STATS',
  'TEAMMEM',
  'TOKEN_BUDGET',
  'TREE_SITTER_BASH',
  'TREE_SITTER_BASH_SHADOW',
  'ULTRAPLAN',
  'ULTRATHINK',
  'UNATTENDED_RETRY',
  'VERIFICATION_AGENT',
  'VOICE_MODE',
] as const

// Build command injection
for (const feature of features) {
  cmd.push(`--feature=${feature}`)
}
```

### Build Variants

| Command | Features | Output |
|---------|----------|--------|
| `bun run build` | `VOICE_MODE` | `./cli` |
| `bun run build:dev` | `VOICE_MODE` + dev stamp | `./cli-dev` |
| `bun run build:dev:full` | All 54 working flags | `./cli-dev` |
| `bun run compile` | `VOICE_MODE` | `./dist/cli` |

### Custom Feature Selection

```bash
# Enable specific flags
bun run ./scripts/build.ts --feature=ULTRAPLAN --feature=ULTRATHINK

# Add flag to dev build
bun run ./scripts/build.ts --dev --feature=BRIDGE_MODE
```

---

## Working Features

### Interaction and UI Experiments

#### `AWAY_SUMMARY`
**Purpose:** Adds away-from-keyboard summary behavior in the REPL.

**Location:** `src/services/awaySummary.ts`

**Usage:**
```typescript
if (feature('AWAY_SUMMARY')) {
  // Generate summary when user is idle
  const summary = generateAwaySummary(messages)
}
```

---

#### `HISTORY_PICKER`
**Purpose:** Enables the interactive prompt history picker.

**Location:** `src/hooks/useSearchInput.ts`, `src/components/PromptInput/`

**Usage:**
```typescript
if (feature('HISTORY_PICKER')) {
  // Show history picker UI
  const history = await showHistoryPicker()
}
```

---

#### `HOOK_PROMPTS`
**Purpose:** Passes the prompt/request text into hook execution flows.

**Location:** `src/utils/hooks.ts`

**Usage:**
```typescript
if (feature('HOOK_PROMPTS')) {
  await executeHook('beforeQuery', { prompt: userInput })
}
```

---

#### `KAIROS_BRIEF`
**Purpose:** Enables brief-only transcript layout and BriefTool-oriented UX without the full assistant stack.

**Location:** `src/tools/BriefTool/`, `src/commands/brief.ts`

**Related:** `KAIROS`, `KAIROS_CHANNELS`

---

#### `KAIROS_CHANNELS`
**Purpose:** Enables channel notices and channel callback plumbing around MCP/channel messaging.

**Location:** `src/services/mcp/`, `src/hooks/useMailboxBridge.ts`

---

#### `LODESTONE`
**Purpose:** Enables deep-link / protocol-registration related flows and settings wiring.

**Location:** `src/utils/deepLink/`, `src/utils/desktopDeepLink.ts`

---

#### `MESSAGE_ACTIONS`
**Purpose:** Enables message action entrypoints in the interactive UI.

**Location:** `src/components/Messages.tsx`, `src/components/MessageSelector.tsx`

**Usage:**
```typescript
if (feature('MESSAGE_ACTIONS')) {
  // Show action menu on message hover
  <MessageActions message={message} />
}
```

---

#### `NEW_INIT`
**Purpose:** Enables the newer `/init` decision path.

**Location:** `src/commands/init.ts`

---

#### `QUICK_SEARCH`
**Purpose:** Enables prompt quick-search behavior.

**Location:** `src/hooks/useSearchInput.ts`, `src/components/PromptInput/`

---

#### `SHOT_STATS`
**Purpose:** Enables additional shot-distribution stats views.

**Location:** `src/utils/stats.ts`, `src/components/Stats.tsx`

---

#### `TOKEN_BUDGET`
**Purpose:** Enables token budget tracking, prompt triggers, and token warning UI.

**Location:** `src/utils/tokenBudget.ts`, `src/bootstrap/state.ts`

**Usage:**
```typescript
if (feature('TOKEN_BUDGET')) {
  const budget = parseTokenBudget(config.tokenBudget)
  const status = checkTokenBudget(usedTokens, budget)
  
  if (status.status === 'warning') {
    displayWarning(`Approaching token budget: ${status.percentage.toFixed(0)}%`)
  }
}
```

---

#### `ULTRAPLAN`
**Purpose:** Enables `/ultraplan`, prompt triggers, and exit-plan affordances. Remote multi-agent planning on Claude Code web (Opus-class).

**Location:** `src/commands/ultraplan.tsx` (67KB), `src/utils/ultraplan/`

**Usage:**
```typescript
if (feature('ULTRAPLAN')) {
  // Register ultraplan command
  commands.push(ultraplanCommand)
}
```

---

#### `ULTRATHINK`
**Purpose:** Enables the extra thinking-depth mode switch. Type "ultrathink" to boost reasoning effort.

**Location:** `src/utils/effort.ts`, `src/utils/thinking.ts`

**Usage:**
```typescript
if (feature('ULTRATHINK') && userInput.includes('ultrathink')) {
  params.thinking = {
    type: 'enabled',
    budget_tokens: 50000, // Increased budget
  }
}
```

---

#### `VOICE_MODE`
**Purpose:** Enables voice toggling, dictation keybindings, voice notices, and voice UI.

**Location:** `src/commands/voice/`, `src/services/voice.ts` (18KB), `src/services/voiceStreamSTT.ts` (21KB)

**Note:** Bundles cleanly, but requires claude.ai OAuth and a local recording backend (SoX or native audio module).

**Usage:**
```typescript
if (feature('VOICE_MODE')) {
  const { useVoiceIntegration } = require('../hooks/useVoiceIntegration.js')
  const { voiceState, handleVoiceInput } = useVoiceIntegration()
}
```

---

### Agent, Memory, and Planning Experiments

#### `AGENT_MEMORY_SNAPSHOT`
**Purpose:** Stores extra custom-agent memory snapshot state in the app.

**Location:** `src/tools/AgentTool/`, `src/services/SessionMemory/`

---

#### `AGENT_TRIGGERS`
**Purpose:** Enables local cron/trigger tools and bundled trigger-related skills.

**Location:** `src/tools/ScheduleCronTool/`, `src/hooks/useScheduledTasks.ts`, `src/utils/cron*.ts`

**Tools enabled:**
- `CronCreateTool` — Create scheduled tasks
- `CronDeleteTool` — Delete scheduled tasks
- `CronListTool` — List scheduled tasks

---

#### `AGENT_TRIGGERS_REMOTE`
**Purpose:** Enables the remote trigger tool path.

**Location:** `src/tools/RemoteTriggerTool/RemoteTriggerTool.ts`

---

#### `BUILTIN_EXPLORE_PLAN_AGENTS`
**Purpose:** Enables built-in explore/plan agent presets.

**Location:** `src/tools/AgentTool/`, `src/commands/agents/`

---

#### `CACHED_MICROCOMPACT`
**Purpose:** Enables cached microcompact state through query and API flows.

**Location:** `src/services/compact/microCompact.ts`

---

#### `COMPACTION_REMINDERS`
**Purpose:** Enables reminder copy around compaction and attachment flows.

**Location:** `src/services/compact/`, `src/utils/messages.ts`

---

#### `EXTRACT_MEMORIES`
**Purpose:** Enables post-query automatic memory extraction hooks.

**Location:** `src/services/extractMemories/`

---

#### `PROMPT_CACHE_BREAK_DETECTION`
**Purpose:** Enables cache-break detection around compaction/query/API flow.

**Location:** `src/services/api/promptCacheBreakDetection.ts` (26KB)

---

#### `TEAMMEM`
**Purpose:** Enables team-memory files, watcher hooks, and related UI messages.

**Location:** `src/services/teamMemorySync/`, `src/utils/teamMemoryOps.ts`

**Note:** Bundles cleanly, but only does useful work when team-memory config/files are actually enabled in the environment.

---

#### `VERIFICATION_AGENT`
**Purpose:** Enables verification-agent guidance in prompts and task/todo tooling.

**Location:** `src/tools/VerifyPlanExecutionTool/` (conditionally loaded)

---

### Tools, Permissions, and Remote Experiments

#### `BASH_CLASSIFIER`
**Purpose:** Enables classifier-assisted bash permission decisions.

**Location:** `src/utils/permissions/`, `src/tools/BashTool/bashPermissions.ts`

---

#### `BRIDGE_MODE`
**Purpose:** Enables Remote Control / REPL bridge command and entitlement paths.

**Location:** `src/bridge/` (30+ files), `src/commands/bridge/`

**Note:** Bundles cleanly, but gated at runtime on claude.ai OAuth plus GrowthBook entitlement checks.

**Commands enabled:**
- `remote-control` / `rc` / `remote` / `sync` / `bridge`

---

#### `CCR_AUTO_CONNECT`
**Purpose:** Enables the CCR auto-connect default path.

**Location:** `src/hooks/useRemoteSession.ts`, `src/services/remoteManagedSettings/`

---

#### `CCR_MIRROR`
**Purpose:** Enables outbound-only CCR mirror sessions.

**Location:** `src/hooks/useDirectConnect.ts`

---

#### `CCR_REMOTE_SETUP`
**Purpose:** Enables the remote setup command path.

**Location:** `src/commands/remote-setup/`

---

#### `CHICAGO_MCP`
**Purpose:** Enables computer-use MCP integration paths and wrapper loading.

**Location:** `src/utils/computerUse/`, `src/tools/`

**Note:** Bundles cleanly, but runtime path still reaches externalized `@ant/computer-use-*` packages. Compile-safe, not fully runtime-safe in external snapshot.

---

#### `CONNECTOR_TEXT`
**Purpose:** Enables connector-text block handling in API/logging/UI paths.

**Location:** `src/utils/messages.ts`, `src/components/Messages.tsx`

---

#### `MCP_RICH_OUTPUT`
**Purpose:** Enables richer MCP UI rendering.

**Location:** `src/components/mcp/`, `src/services/mcp/`

---

#### `NATIVE_CLIPBOARD_IMAGE`
**Purpose:** Enables the native macOS clipboard image fast path.

**Location:** `src/utils/imagePaste.ts`

**Note:** Bundles cleanly, but only accelerates macOS clipboard reads when `image-processor-napi` is present.

---

#### `POWERSHELL_AUTO_MODE`
**Purpose:** Enables PowerShell-specific auto-mode permission handling.

**Location:** `src/utils/powershell/`, `src/tools/PowerShellTool/`

---

#### `TREE_SITTER_BASH`
**Purpose:** Enables the tree-sitter bash parser backend.

**Location:** `src/utils/bash/`

---

#### `TREE_SITTER_BASH_SHADOW`
**Purpose:** Enables the tree-sitter bash shadow rollout path.

**Location:** `src/utils/bash/`

---

#### `UNATTENDED_RETRY`
**Purpose:** Enables unattended retry behavior in API retry flows.

**Location:** `src/services/api/withRetry.ts`

---

### Build Support Flags

These flags bundle cleanly and are mostly rollout, platform, telemetry, or plumbing toggles:

| Flag | Purpose |
|------|---------|
| `ABLATION_BASELINE` | CLI ablation/baseline entrypoint toggle |
| `ALLOW_TEST_VERSIONS` | Allows test versions in native installer flows |
| `ANTI_DISTILLATION_CC` | Adds anti-distillation request metadata |
| `BREAK_CACHE_COMMAND` | Injects the break-cache command path |
| `COWORKER_TYPE_TELEMETRY` | Adds coworker-type telemetry fields |
| `DOWNLOAD_USER_SETTINGS` | Enables settings-sync pull paths |
| `DUMP_SYSTEM_PROMPT` | Enables the system-prompt dump path |
| `FILE_PERSISTENCE` | Enables file persistence plumbing |
| `HARD_FAIL` | Enables stricter failure/logging behavior |
| `IS_LIBC_GLIBC` | Forces glibc environment detection |
| `IS_LIBC_MUSL` | Forces musl environment detection |
| `NATIVE_CLIENT_ATTESTATION` | Adds native attestation marker text in system header |
| `PERFETTO_TRACING` | Enables perfetto tracing hooks |
| `SKILL_IMPROVEMENT` | Enables skill-improvement hooks |
| `SKIP_DETECTION_WHEN_AUTOUPDATES_DISABLED` | Skips updater detection when auto-updates disabled |
| `SLOW_OPERATION_LOGGING` | Enables slow-operation logging |
| `UPLOAD_USER_SETTINGS` | Enables settings-sync push paths |

---

## Broken Features

### Easy Reconstruction (Single File/Asset Gaps)

| Flag | Missing File | Notes |
|------|--------------|-------|
| `AUTO_THEME` | `src/utils/systemThemeWatcher.js` | OSC 11 watcher only |
| `BG_SESSIONS` | `src/cli/bg.js` | CLI fast-path dispatch wired |
| `BUDDY` | `src/commands/buddy/index.js` | UI components exist |
| `BUILDING_CLAUDE_APPS` | `src/claude-api/csharp/claude-api.md` | Asset/document gap |
| `COMMIT_ATTRIBUTION` | `src/utils/attributionHooks.js` | Setup/cache-clear code exists |
| `FORK_SUBAGENT` | `src/commands/fork/index.js` | Command slot exists |
| `HISTORY_SNIP` | `src/commands/force-snip.js` | SnipTool exists |
| `KAIROS_GITHUB_WEBHOOKS` | `src/tools/SubscribePRTool/SubscribePRTool.js` | Command slot exists |
| `KAIROS_PUSH_NOTIFICATION` | `src/tools/PushNotificationTool/PushNotificationTool.js` | Tool slot in tools.ts |
| `MCP_SKILLS` | `src/skills/mcpSkills.js` | mcpSkillBuilders.ts exists |
| `MEMORY_SHAPE_TELEMETRY` | `src/memdir/memoryShapeTelemetry.js` | Hook call sites exist |
| `OVERFLOW_TEST_TOOL` | `src/tools/OverflowTestTool/OverflowTestTool.js` | Isolated, test-only |
| `RUN_SKILL_GENERATOR` | `src/runSkillGenerator.js` | Registration path exists |
| `TEMPLATES` | `src/cli/handlers/templateJobs.js` | CLI fast-path wired |
| `TORCH` | `src/commands/torch.js` | Single command entry gap |
| `TRANSCRIPT_CLASSIFIER` | `src/utils/permissions/yolo-classifier-prompts/auto_mode_system_prompt.txt` | Prompt asset gap |

### Medium-Sized Gaps

| Flag | Missing File | Notes |
|------|--------------|-------|
| `BYOC_ENVIRONMENT_RUNNER` | `src/environment-runner/main.js` | |
| `CONTEXT_COLLAPSE` | `src/tools/CtxInspectTool/CtxInspectTool.js` | |
| `COORDINATOR_MODE` | `src/coordinator/workerAgent.js` | |
| `DAEMON` | `src/daemon/workerRegistry.js` | |
| `DIRECT_CONNECT` | `src/server/parseConnectUrl.js` | |
| `EXPERIMENTAL_SKILL_SEARCH` | `src/services/skillSearch/localSearch.js` | |
| `MONITOR_TOOL` | `src/tools/MonitorTool/MonitorTool.js` | |
| `REACTIVE_COMPACT` | `src/services/compact/reactiveCompact.js` | |
| `REVIEW_ARTIFACT` | `src/hunter.js` | |
| `SELF_HOSTED_RUNNER` | `src/self-hosted-runner/main.js` | |
| `SSH_REMOTE` | `src/ssh/createSSHSession.js` | |
| `TERMINAL_PANEL` | `src/tools/TerminalCaptureTool/TerminalCaptureTool.js` | |
| `UDS_INBOX` | `src/utils/udsMessaging.js` | |
| `WEB_BROWSER_TOOL` | `src/tools/WebBrowserTool/WebBrowserTool.js` | |
| `WORKFLOW_SCRIPTS` | `src/commands/workflows/index.js` + more | Multiple gaps |

### Large Missing Subsystems

| Flag | Missing Files | Notes |
|------|---------------|-------|
| `KAIROS` | `src/assistant/index.js` + assistant stack | Full assistant system |
| `KAIROS_DREAM` | `src/dream.js` + dream-task behavior | Dream task system |
| `PROACTIVE` | `src/proactive/index.js` + task/tool stack | Proactive task system |

---

## Feature Implementation

### Using feature() in Code

```typescript
import { feature } from 'bun:bundle'

// Simple gate
if (feature('ULTRAPLAN')) {
  // UltraPlan-specific code
}

// Conditional import (DCE-safe)
const ultraplanCommand = feature('ULTRAPLAN')
  ? require('./commands/ultraplan.js').default
  : null

// In arrays (spread with condition)
const commands = [
  helpCommand,
  ...(feature('ULTRAPLAN') ? [ultraplanCommand] : []),
]

// Top-level guard for module elimination
/* eslint-disable custom-rules/no-process-env-top-level */
const useVoiceIntegration = feature('VOICE_MODE')
  ? require('../hooks/useVoiceIntegration.js').useVoiceIntegration
  : () => ({ /* no-op */ })
/* eslint-enable */
```

### Dead Code Elimination Pattern

```typescript
// Pattern 1: Inline conditional
if (feature('FLAG')) {
  // This block is completely removed from external builds
  expensiveOperation()
}

// Pattern 2: Conditional require
const optionalModule = feature('FLAG')
  ? require('./optional-module.js')
  : null

// Pattern 3: Feature-gated import
import type { OptionalType } from './types.js'  // Types always remain

const OptionalComponent = feature('FLAG')
  ? require('./OptionalComponent.js').default
  : null

// Pattern 4: Array filtering
const features = [
  baseFeature,
  ...(feature('EXTRA') ? [extraFeature] : []),
].filter(Boolean)
```

### Build-Time Defines

```typescript
// scripts/build.ts
const defines = {
  'process.env.USER_TYPE': JSON.stringify('external'),
  'process.env.CLAUDE_CODE_FORCE_FULL_LOGO': JSON.stringify('true'),
  'process.env.CLAUDE_CODE_EXPERIMENTAL_BUILD': JSON.stringify(dev),
  'process.env.CLAUDE_CODE_VERIFY_PLAN': JSON.stringify('false'),
  'MACRO.VERSION': JSON.stringify(version),
  'MACRO.BUILD_TIME': JSON.stringify(buildTime),
} as const

// Injected via bun build --define
for (const [key, value] of Object.entries(defines)) {
  cmd.push('--define', `${key}=${value}`)
}
```

---

## Dead Code Elimination

### How It Works

The `bun:bundle` feature() function enables **build-time dead code elimination**:

1. **Build script** injects `--feature=FLAG_NAME` for enabled flags
2. **Bundler** evaluates `feature('FLAG_NAME')` at build time
3. **Dead branches** are completely removed from output
4. **Result**: Smaller binary, no runtime overhead for disabled features

### Example

```typescript
// Source code
import { feature } from 'bun:bundle'

if (feature('ULTRAPLAN')) {
  console.log('UltraPlan enabled')
}

// Build WITHOUT --feature=ULTRAPLAN
// Result: Entire if-block removed (0 bytes)

// Build WITH --feature=ULTRAPLAN
// Result: console.log retained
```

### External Modules

```typescript
// scripts/build.ts
const externals = [
  '@ant/*',              // Anthropic internal packages
  'audio-capture-napi',  // Native audio module
  'image-processor-napi', // Native image module
  'modifiers-napi',      // Native modifiers module
  'url-handler-napi',    // Native URL handler
]

// These packages are not bundled (remain as external requires)
for (const external of externals) {
  cmd.push('--external', external)
}
```

---

## References

- [FEATURES.md](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/FEATURES.md) — Original feature flag audit
- [scripts/build.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/scripts/build.ts) — Build script
- [src/commands.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/commands.ts) — Feature-gated commands
- [src/tools.ts](/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/free-code/src/tools.ts) — Feature-gated tools
