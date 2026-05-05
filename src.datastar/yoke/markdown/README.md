# yoke Documentation

Headless LLM agent harness. JSONL in, JSONL out. A single agent turn as a Unix pipe.

## Documents

1. [Overview](00-overview.md) -- What yoke is, design philosophy, core concepts
2. [Architecture](01-architecture.md) -- yoke binary + yoagent library, module layout
3. [JSONL Protocol](02-jsonl-protocol.md) -- Input/output format, context lines, observation lines
4. [Agent Loop](03-agent-loop.md) -- The core loop: prompt → stream → tool exec → repeat
5. [Providers](04-providers.md) -- Anthropic, OpenAI, Gemini, Ollama, OpenRouter
6. [Tools](05-tools.md) -- Built-in tools, tool groups, AgentTool trait
7. [Context Management](06-context-management.md) -- Token estimation, compaction, execution limits
8. [Nushell Tool](07-nushell-tool.md) -- Embedded Nushell engine, plugins, modules
9. [Integration Patterns](08-integration-patterns.md) -- With xs, http-nu, skills, web UI
