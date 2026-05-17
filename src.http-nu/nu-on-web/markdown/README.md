# nu-on-web — Nushell in the Browser

Nushell (v0.107.0) compiled to WASM32 for browser execution. Provides a persistent REPL session where shell commands (`ls`, `cat`, `rm`) operate on a ZenFS virtual filesystem.

## Source

- **Package:** `nushell-wasm` v0.1.0
- **Language:** Rust (WASM32 target)
- **10 source files, ~400 LOC**

## Documentation

| Document | Description |
|----------|-------------|
| [00-overview](markdown/00-overview.md) | Architecture, 6 WASM exports, singleton pattern, dependencies |
| [01-wasm-engine](markdown/01-wasm-engine.md) | Engine struct, parse-eval-convert pipeline, completions, AST traversal |
| [02-commands-zenfs](markdown/02-commands-zenfs.md) | Custom commands (ls/cat/rm), ZenFS FFI bridge, type conversion |

## Quick Reference

### WASM API

```typescript
import * as nushell from 'nushell-wasm';

nushell.runCode("ls");                          // Execute Nushell code
nushell.getCommandsDescriptions("ls | where");  // Get command descriptions
nushell.findPipelineElementByOffset(code, pos); // Find expression at cursor
nushell.fetchCompletions("l", 1);               // Get completions
nushell.getDeclarationNameFromId(declId);       // Resolve command ID to name
nushell.getNextSpanStart();                     // Get span offset
```

### Architecture

- **Engine singleton** — persistent `EngineState` + `Stack` across calls (REPL behavior)
- **Auto-variables** — results stored as `_1`, `_2`, `_3` for cross-call reference
- **ZenFS bridge** — `@zenfs/core` virtual filesystem via `wasm-bindgen` module imports
- **HTML fallback** — complex types (lists, records) rendered via `to html --dark --partial`
- **Tsify types** — automatic TypeScript generation from Rust enums/structs
