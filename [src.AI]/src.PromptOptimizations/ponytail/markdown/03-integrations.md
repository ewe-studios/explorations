# Ponytail Integrations: Agent Plugins, Hooks, and Skills

Ponytail integrates with 14 AI agents through three mechanisms: Claude hooks, Gemini extensions, and MCP servers.

Source: `hooks/`, `gemini-extension.json`, `ponytail-mcp/`, `pi-extension/`, `skills/`, `commands/`.

## Claude Code Integration

Via hooks in `hooks/`:

| Hook | File | Event |
|------|------|-------|
| Activate | `ponytail-activate.js` | SessionStart |
| Mode Tracker | `ponytail-mode-tracker.js` | UserPromptSubmit |
| Instructions | `ponytail-instructions.js` | Shared |
| Config | `ponytail-config.js` | Shared |
| Runtime | `ponytail-runtime.js` | Shared |

### Hook Configuration

```json
{
  "hooks": {
    "SessionStart": "hooks/ponytail-activate.js",
    "UserPromptSubmit": "hooks/ponytail-mode-tracker.js"
  }
}
```

## Gemini CLI Integration

Source: `gemini-extension.json`

Registers ponytail as a Gemini extension with its own instruction set.

## Codex CLI Integration

Via `PLUGIN_DATA` environment variable. The runtime detects `process.env.PLUGIN_DATA` and writes output in Codex format:

```javascript
if (isCodex) {
  process.stdout.write(JSON.stringify({
    systemMessage: `PONYTAIL:${mode.toUpperCase()}`,
    hookSpecificOutput: { hookEventName: event, additionalContext: context }
  }))
}
```

## Copilot CLI Integration

Via `COPILOT_PLUGIN_DATA` environment variable. Copilot reads `additionalContext` on SessionStart only:

```javascript
if (isCopilot) {
  process.stdout.write(JSON.stringify(
    event === 'SessionStart' && context ? { additionalContext: context } : {}))
}
```

## OpenCode Integration

Source: `opencode.json`

Registers ponytail as an OpenCode plugin.

## Pi Extension

Source: `pi-extension/index.js`

Registers ponytail as a Pi plugin with skills directory:

```json
{
  "pi": {
    "extensions": ["./pi-extension/index.js"],
    "skills": ["./skills"]
  }
}
```

## MCP Server

Source: `ponytail-mcp/`

Exposes ponytail commands as MCP tools for any agent supporting the Model Context Protocol.

## Skills

Source: `skills/ponytail/SKILL.md`

The skill file contains:
- Mode intensity tables (lite/full/ultra)
- Worked examples for each mode
- General rules (apply to all modes)

## Commands

Source: `commands/`

CLI commands for ponytail mode management.

## Scripts

| Script | Purpose |
|--------|---------|
| `scripts/build-openclaw-skills.js` | Build skills for OpenClaw agent |
| `scripts/check-rule-copies.js` | Verify rule files aren't duplicated |

## Cross-References

- [Overview](00-overview.md) — What ponytail is
- [Architecture](01-architecture.md) — Hooks, config resolution
- [Benchmarks](02-benchmarks.md) — Agentic benchmarks
