---
title: Shell Parser — tree-sitter Bash Parsing
---

# Shell Parser — tree-sitter Bash Parsing

**Mirage parses bash commands using tree-sitter — the same parser used by GitHub, Neovim, and Helix — to build an execution tree that the Workspace can evaluate.**

## Shell Parser

Source: `typescript/packages/core/src/shell/parse.ts`

```typescript
export async function createShellParser(config: ShellParserConfig): Promise<ShellParser> {
  await Parser.init({ wasmBinary: config.engineWasm })
  const language = await Language.load(config.grammarWasm)
  const parser = new Parser()
  parser.setLanguage(language)
  return {
    parse(command: string): Node {
      return parser.parse(command).rootNode
    },
  }
}
```

**Aha:** Using tree-sitter means Mirage understands bash syntax — pipes, redirects, variables, subshells — rather than splitting on whitespace. This is why agents can write complex pipelines like `grep alert /slack/general/*.json | wc -l` and have them work correctly across mounts.

## Execution Tree

```mermaid
flowchart TD
    A["cat /s3/logs/app.json | grep error | head -5"] --> B[Parse with tree-sitter]
    B --> C[Pipe node]
    C --> D[Command: cat]
    C --> E[Command: grep]
    C --> F[Command: head]
    D --> G[PathSpec: /s3/logs/app.json]
    E --> H[Arg: "error"]
    F --> I[Arg: "-5"]
```

## Supported Bash Features

| Feature | Example | Support |
|---------|---------|---------|
| Pipes | `cat file | grep pattern` | ✅ |
| Redirects | `echo hello > file` | ✅ |
| Variables | `echo $HOME` | ✅ |
| Command substitution | `echo $(pwd)` | ✅ |
| Glob patterns | `cat /s3/logs/*.json` | ✅ |
| Background jobs | `sleep 10 &` | ✅ |
| If/else | `if [ -f file ]; then ...` | ✅ |
| For loops | `for f in *.txt; do ...` | ✅ |
| Here documents | `cat <<EOF` | ✅ |
| Python REPL | `python3` (interactive) | ✅ (via Pyodide) |

## Syntax Error Detection

Source: `typescript/packages/core/src/shell/parse.ts`

```typescript
export function findSyntaxError(command: string): SyntaxError | null {
  const tree = parser.parse(command)
  if (tree.rootNode.hasError) {
    // Return the first error node
  }
  return null
}
```

Errors are returned to the agent as feedback — "syntax error near unexpected token" — so the agent can fix the command.

## Python REPL Integration

Source: `typescript/packages/core/src/workspace/executor/python/`

When the agent runs `python3` interactively, Mirage spins up a Pyodide runtime:

```mermaid
sequenceDiagram
    participant Agent as AI Agent
    participant WS as Workspace
    participant Pyodide as Pyodide Runtime
    participant Bridge as Mirage Bridge

    Agent->>WS: execute("python3")
    WS->>Pyodide: Start REPL
    Agent->>WS: send("import mirage; mirage.cat('/data/file.txt')")
    WS->>Pyodide: Execute code
    Pyodide->>Bridge: Mirage API call
    Bridge->>WS: Dispatch to resource
    WS-->>Pyodide: Result
    Pyodide-->>Agent: Output
```

## What's Next

- [06 — Ops & Commands](06-ops-commands.md) — Operation registry, built-in commands
- [07 — Cross-Mount](07-cross-mount.md) — cp, mv, diff across backends
- [02 — Workspace](02-workspace.md) — Return to workspace
