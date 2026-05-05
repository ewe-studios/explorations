# yoke -- Overview

## What Is yoke?

yoke is a static Rust binary that drives one LLM agent turn to completion. It runs tool calls in a loop until the model is satisfied, then exits. Context window in as JSONL on stdin, new context + live stream out as JSONL on stdout.

```
context.jsonl ──> yoke ──> tee ──> store context for follow-ups
                               └─> real-time view
```

No TUI, no REPL, no daemon, no persistence. Just a JSONL-in / JSONL-out primitive you compose with shell tools.

## Design Philosophy

1. **Unix pipe** — stdin/stdout. Compose with `tee`, `cat`, pipes, and Nushell.
2. **Stateless binary** — No config files, no database, no daemon. State lives in the JSONL stream.
3. **Provider-agnostic** — Same interface for Anthropic, OpenAI, Gemini, Ollama, OpenRouter.
4. **Round-trippable** — Output is valid input for the next turn. Save to file, replay later.
5. **Observable** — Two line types: context (durable state) and observations (live stream). Tee observations to a renderer for real-time display.

## Two Crates

| Crate | Version | Role |
|-------|---------|------|
| `yoke` | 0.4.1-dev | CLI binary: arg parsing, JSONL I/O, provider setup, Nu tool |
| `yoagent` | 0.7.5 | Library: agent loop, providers, tools, context management, retry |

## Core Concepts

### Agent Turn

One invocation of yoke = one "turn". A turn consists of:
1. Read context from stdin (JSONL with `role` field)
2. Optionally append a trailing prompt from CLI args
3. Send to LLM provider
4. If model requests tool calls → execute them → feed results back → repeat
5. When model produces a final response (no tool calls) → emit and exit

### Context Lines vs Observation Lines

**Context lines** have a `role` field. They're the durable conversation state:
- `system` — System prompt
- `user` — User messages
- `assistant` — Model responses (with tool calls, thinking, text)
- `toolResult` — Tool execution results

**Observation lines** have a `type` field. They're the live event stream:
- `agent_start` / `agent_end` — Lifecycle boundaries
- `turn_start` / `turn_end` — Per-turn boundaries
- `delta` — Streaming text/thinking/tool_call deltas
- `tool_execution_start` / `tool_execution_end` — Tool lifecycle
- `progress_message` — Tool progress updates

Observation lines are skipped on input. Only context lines round-trip.

## Quick Usage

```nushell
# One-shot question
yoke --provider gemini --model gemini-2.5-flash "what files are here?"

# Pipe context, save for continuation
yoke --provider anthropic --model claude-sonnet-4-20250514 "refactor main.rs" \
  | tee { save -f session.jsonl }

# Continue conversation
cat session.jsonl \
  | yoke --provider anthropic --model claude-sonnet-4-20250514 "now add tests"

# Replay against different model
cat session.jsonl \
  | yoke --provider openai --model gpt-5.4-mini "summarize what happened"
```

## Key Dependencies

| Crate | Role |
|-------|------|
| yoagent | Agent loop, providers, tools, context |
| tokio | Async runtime |
| clap 4 | CLI argument parsing |
| reqwest | HTTP client for provider APIs |
| serde/serde_json | JSONL serialization |
| nu-* (0.112.1) | Embedded Nushell engine for `nu` tool |
| chrono | Timestamp formatting |
