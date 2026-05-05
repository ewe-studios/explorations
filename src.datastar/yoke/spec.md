# yoke -- Spec

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.datastar/yoke/`
- **Repository:** https://github.com/cablehead/yoke
- **Language:** Rust
- **Version:** 0.4.1-dev (yoke), 0.7.5 (yoagent)
- **License:** MIT

## What This Project Is

yoke is a headless LLM agent harness that operates as a Unix pipe: JSONL in, JSONL out. It drives one agent turn to completion (tool call loop), then exits. No TUI, no REPL, no daemon. Built on the `yoagent` library which provides the core agent loop, multi-provider streaming, tool execution, context management, and retry logic.

## Documentation Structure

```
src.datastar/yoke/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-jsonl-protocol.md
│   ├── 03-agent-loop.md
│   ├── 04-providers.md
│   ├── 05-tools.md
│   ├── 06-context-management.md
│   ├── 07-nushell-tool.md
│   └── 08-integration-patterns.md
├── html/
└── (uses parent build.py)
```
