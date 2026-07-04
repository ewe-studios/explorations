# Ponytail Architecture: Hooks, Config Resolution, Runtime State

Ponytail runs as a set of Node.js hooks that inject instructions into AI agent sessions. Five hook files, one config resolver, one runtime state manager.

Source: `hooks/` — 5 files, `ponytail-mcp/` — 2 files, `pi-extension/` — 1 file.

## Hook Architecture

Ponytail uses three Claude Code hook events:

| Hook | File | When | Purpose |
|------|------|------|---------|
| SessionStart | `ponytail-activate.js` | Session begins | Write mode flag, emit instructions |
| UserPromptSubmit | `ponytail-mode-tracker.js` | Each user message | Detect /ponytail commands, update mode |
| (statusline) | `ponytail-statusline.sh/ps1` | CLI prompt | Show [PONYTAIL] badge |

### SessionStart Hook

Source: `hooks/ponytail-activate.js` — 80 lines.

```javascript
const mode = getDefaultMode()  // Resolve: env → config file → 'full'

if (mode === 'off') {
  clearMode()
  writeHookOutput('SessionStart', 'off', 'OK')
  process.exit(0)
}

// 1. Write flag file at $CLAUDE_CONFIG_DIR/.ponytail-active
setMode(mode)

// 2. Emit ponytail instructions (filtered to active mode)
let output = getPonytailInstructions(mode)

// 3. Detect missing statusline config → nudge user
```

### UserPromptSubmit Hook

Source: `hooks/ponytail-mode-tracker.js` — 60 lines.

Reads stdin as JSON, detects `/ponytail lite|full|ultra|off` commands, writes new mode to flag file.

```javascript
process.stdin.on('data', chunk => { input += chunk })
process.stdin.on('end', () => {
  const data = JSON.parse(input.replace(/^﻿/, ''))
  const prompt = (data.prompt || '').trim().toLowerCase()

  if (/^[/@$]ponytail/.test(prompt)) {
    const parts = prompt.split(/\s+/)
    const arg = parts[1] || ''
    // Set mode based on arg
    if (mode && mode !== 'off') setMode(mode)
    else if (mode === 'off') clearMode()
  }
})
```

## Config Resolution

Source: `hooks/ponytail-config.js` — 100+ lines.

Resolution order (highest to lowest priority):

```javascript
function getDefaultMode() {
  // 1. Environment variable
  const envMode = process.env.PONYTAIL_DEFAULT_MODE
  if (envMode && VALID_MODES.includes(envMode.toLowerCase())) {
    return envMode.toLowerCase()
  }

  // 2. Config file (~/.config/ponytail/config.json)
  try {
    const config = JSON.parse(fs.readFileSync(getConfigPath(), 'utf8'))
    if (config.defaultMode && VALID_MODES.includes(config.defaultMode.toLowerCase())) {
      return config.defaultMode.toLowerCase()
    }
  } catch (e) { /* fall through */ }

  // 3. Default: 'full'
  return DEFAULT_MODE
}
```

### Config Directory Resolution

```javascript
function getConfigDir() {
  if (process.env.XDG_CONFIG_HOME) {
    return path.join(process.env.XDG_CONFIG_HOME, 'ponytail')
  }
  if (process.platform === 'win32') {
    return path.join(process.env.APPDATA || path.join(os.homedir(), 'AppData', 'Roaming'), 'ponytail')
  }
  return path.join(os.homedir(), '.config', 'ponytail')
}
```

### Deactivation Detection

```javascript
function isDeactivationCommand(text) {
  const t = String(text || '').trim().toLowerCase().replace(/[.!?\s]+$/, '')
  return t === 'stop ponytail' || t === 'normal mode'
}
```

Aha: The deactivation check requires the **entire message** to be the command (ignoring case and trailing punctuation). Earlier versions matched the phrase anywhere in the message, which turned ponytail off mid-task for ordinary requests like "add a normal mode toggle".

### Shell Safety Check

```javascript
function isShellSafe(p) {
  return typeof p === 'string' && /^[A-Za-z0-9 _.\-:/\\~]+$/.test(p)
}
```

Validates that the plugin install path contains only safe characters before embedding it in a shell command for the statusline setup nudge.

## Runtime State

Source: `hooks/ponytail-runtime.js` — 50 lines.

```javascript
const STATE_FILE = '.ponytail-active'
const isCopilot = Boolean(process.env.COPILOT_PLUGIN_DATA)
const isCodex = !isCopilot && Boolean(process.env.PLUGIN_DATA)

let stateDir = getClaudeDir()
if (isCodex) stateDir = process.env.PLUGIN_DATA
if (isCopilot) stateDir = process.env.COPILOT_PLUGIN_DATA

const statePath = path.join(stateDir, STATE_FILE)

function setMode(mode) {
  fs.mkdirSync(path.dirname(statePath), { recursive: true })
  fs.writeFileSync(statePath, mode)
}

function clearMode() {
  try { fs.unlinkSync(statePath) } catch (e) {}
}
```

Mode is stored as a single string in a file (`.ponytail-active`) in the agent's config directory. Different agents use different directories:

| Agent | State Directory |
|-------|----------------|
| Claude Code | `~/.claude/` (or `$CLAUDE_CONFIG_DIR`) |
| Codex CLI | `$PLUGIN_DATA` |
| Copilot CLI | `$COPILOT_PLUGIN_DATA` |

### Agent-Specific Output

```javascript
function writeHookOutput(event, mode, context = '') {
  if (isCopilot) {
    // Copilot reads additionalContext on SessionStart
    process.stdout.write(JSON.stringify(
      event === 'SessionStart' && context ? { additionalContext: context } : {}))
    return
  }
  if (isCodex) {
    const output = { systemMessage: `PONYTAIL:${mode.toUpperCase()}` }
    if (context) output.hookSpecificOutput = { hookEventName: event, additionalContext: context }
    process.stdout.write(JSON.stringify(output))
    return
  }
  // Claude Code: raw text
  process.stdout.write(context)
}
```

Each agent has a different expected output format. Copilot wants `{ additionalContext: "..." }`, Codex wants `{ systemMessage: "PONYTAIL:FULL" }`, Claude Code wants raw text.

## Instruction Building

Source: `hooks/ponytail-instructions.js` — 150+ lines.

```javascript
function getPonytailInstructions(mode) {
  const configuredMode = normalizePersistedMode(mode) || DEFAULT_MODE

  if (INDEPENDENT_MODES.has(configuredMode)) {
    // 'review' mode uses a separate skill file
    return `PONYTAIL MODE ACTIVE — level: ${configuredMode}. Behavior defined by /ponytail-${configuredMode} skill.`
  }

  // Try to read the skill file (SKILL.md) and filter for the active mode
  try {
    const skillBody = fs.readFileSync(SKILL_PATH, 'utf8')
    return filterSkillBodyForMode(skillBody, mode)
  } catch {
    return getFallbackInstructions(mode)  // Hardcoded fallback
  }
}
```

### Skill File Filtering

```javascript
function filterSkillBodyForMode(body, mode) {
  const effectiveMode = normalizeMode(mode) || DEFAULT_MODE
  const withoutFrontmatter = String(body || '').replace(/^---[\s\S]*?---\s*/, '')

  return withoutFrontmatter
    .split(/\r?\n/)
    .filter((line) => {
      // Table rows: only keep the row matching the active mode
      const tableLabel = line.match(/^\|\s*\*\*(.+?)\*\*\s*\|/)
      if (tableLabel) {
        const labelMode = normalizeMode(tableLabel[1].trim())
        if (labelMode) return labelMode === effectiveMode
      }
      // Example sections: only keep the example matching the active mode
      const exampleLabel = line.match(/^-\s*([^:]+):\s*/)
      if (exampleLabel) {
        const labelMode = normalizeMode(exampleLabel[1].trim())
        if (labelMode) return labelMode === effectiveMode
      }
      // All other lines (general rules) are kept
      return true
    })
    .join('\n')
}
```

Aha: The skill file (SKILL.md) contains mode-specific intensity tables and worked examples. The filter removes frontmatter, then keeps only the table row and example section that match the active mode. Non-mode-specific rules (like "No unrequested abstractions") are kept verbatim for all modes.

### Fallback Instructions

When the skill file is unavailable, hardcoded instructions are emitted:

```
PONYTAIL MODE ACTIVE — level: full

You are a lazy senior developer. Lazy means efficient, not careless.

## The ladder
1. Does this need to be built at all? (YAGNI)
2. Does the standard library do this? Use it.
3. Does a native platform feature cover it? Use it.
4. Does an already-installed dependency solve it? Use it.
5. Can this be one line? Make it one line.
6. Only then: write the minimum code that works.
```

## MCP Server

Source: `ponytail-mcp/` — 2 files.

Provides ponytail as an MCP (Model Context Protocol) server for agents that support MCP.

```javascript
// ponytail-mcp/index.js
// Exposes /ponytail commands as MCP tools
// ponytail-mcp/instructions.js — MCP-specific instruction formatting
```

## Pi Extension

Source: `pi-extension/index.js` — Pi plugin integration.

Registers ponytail as a Pi extension with skills directory.

## Cross-References

- [Overview](00-overview.md) — What ponytail is
- [Benchmarks](02-benchmarks.md) — Agentic benchmarks
- [Integrations](03-integrations.md) — Agent plugins, hooks

[Next → Benchmarks](02-benchmarks.md)
