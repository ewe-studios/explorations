# Tools and Sandboxing

Moltis runs tools in isolated sandboxes — Docker/Podman, Apple Container, or WASM — to ensure security and resource isolation.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/crates/tools/`

## Tool Execution

The tools crate (~37.0K LoC) handles:

- Tool definition and registration
- Tool execution with sandboxing
- WASM tool support
- Skill execution

```
crates/tools/           ← Tool execution framework (~37.0K LoC)
crates/skills/          ← Skill definitions
crates/browser/         ← Browser automation
```

**Aha:** The tools crate is nearly as large as the gateway crate (~37K vs ~37.4K LoC), making it one of the two largest components in Moltis. This reflects the importance of tool execution in an AI agent — the agent's capabilities come primarily from its tools.

## Sandboxing

Moltis supports multiple sandbox backends:

| Backend | Purpose | Platform |
|---------|---------|----------|
| Docker/Podman | Container sandboxing | Linux, macOS |
| Apple Container | Native macOS sandboxing | macOS only |
| WASM | WebAssembly sandboxing | Cross-platform |

## WASM Tools

Moltis includes precompiled WASM tools:

| Tool | Crate | Purpose |
|------|-------|---------|
| Calculator | `moltis-wasm-tools/calc` | Math operations |
| Web Fetch | `moltis-wasm-tools/web-fetch` | Fetch web content |
| Web Search | `moltis-wasm-tools/web-search` | Search the web |

**Source:** `crates/wasm-tools/`, `crates/wasm-precompile/`.

## Browser Automation

The browser crate provides browser automation capabilities:

```
crates/browser/         ← Browser automation
```

**Source:** `crates/browser/src/`.

## Voice

The voice crate provides STT (speech-to-text) and TTS (text-to-speech):

```
crates/voice/           ← Voice I/O (STT + TTS)
```

**Source:** `crates/voice/src/`.

## Skills

The skills crate manages bundled and workspace skills:

```
crates/skills/          ← Skill definitions
```

Skills are predefined tool sets that the agent can use to accomplish tasks.

## Next

Continue to [06-memory](06-memory.md) for the SQLite + FTS + vector memory system.
