# What Is src.OpenMonoAgent?

OpenMono is a .NET 10 CLI agentic coding tool that runs a local conversation loop against a llama.cpp inference server (or any OpenAI-compatible endpoint). Everything runs in Docker — your project folder bind-mounts as `/workspace`, the agent can't escape it.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.OpenMonoAgent/OpenMonoAgent.ai/`

## Architecture

```
┌────────────────────────────────────────┐
│  openmono (CLI)                        │
│  src/OpenMono.Cli/                     │
│                                        │
│  ConversationLoop                      │
│    └── ILlmClient (streaming SSE)      │
│    └── ToolDispatcher                  │
│         └── 20 built-in tools          │
│         └── MCP tools (dynamic)        │
│         └── LSP tools                  │
│         └── RoslynTool                 │
│    └── PermissionEngine                │
│    └── SessionManager (JSONL)          │
│    └── Compactor / Checkpointer        │
│    └── HookRunner                      │
│    └── IRenderer (TUI | Classic)       │
└────────────────────────────────────────┘
           │ HTTP :7474 (OpenAI-compat)
           ▼
┌────────────────────┐
│  llama-server      │
│  (llama.cpp)       │
│  Qwen3.6 GGUF      │
└────────────────────┘
```

## Source Structure

```
src/OpenMono.Cli/
├── Acp/                  ← Agent Communication Protocol
├── Agents/               ← Agent definitions
├── Commands/             ← CLI commands (run, continue, compact, etc.)
├── Config/               ← Configuration management
├── History/              ← Conversation history
├── Hooks/                ← Hook system (pre/post tool execution)
├── Llm/                  ← LLM client, SSE streaming
├── Lsp/                  ← Language Server Protocol integration
├── Mcp/                  ← Model Context Protocol
├── Memory/               ← Memory/context management
├── Permissions/          ← Permission engine
├── Playbooks/            ← Playbook system
├── Rendering/            ← Output rendering
├── Session/              ← JSONL session management
├── Tools/                ← 20+ built-in tools
├── Tui/                  ← Terminal UI
│   └── Export/           ← TUI export functions
└── Utils/                ← Utilities
```

## Key Features

| Feature | Description |
|---------|-------------|
| Local inference | Runs against llama.cpp or any OpenAI-compatible endpoint |
| 20+ built-in tools | Read, write, edit, search, shell, web, etc. |
| MCP integration | Dynamic MCP tool loading |
| LSP support | Language server protocol for code intelligence |
| Roslyn analysis | C#-specific code analysis via Roslyn |
| Permission engine | Auto-allow rules for tool execution |
| JSONL sessions | Session persistence with compaction |
| TUI rendering | Terminal UI with classic fallback |
| Hook system | Pre/post tool execution hooks |
| Playbooks | Reusable workflow templates |

## Installation

```bash
./get-openmono.sh   # Download and install
openmono run        # Start agent
```

## Docker Setup

```
docker/
├── docker-compose.yml   ← Docker compose setup
├── Dockerfile.agent     ← Agent container
├── Dockerfile.llama     ← Llama.cpp container
├── Caddyfile            ← Caddy gateway config
└── entrypoint.sh        ← Container entrypoint
```

## Configuration

Configuration via `~/.openmono/config.json` or environment variables:

| Config | Env Var | Purpose |
|--------|---------|---------|
| `llm.endpoint` | `OPENMONO_LLM_ENDPOINT` | LLM server URL |
| `llm.api_key` | `OPENMONO_LLM_API_KEY` | API key |
| `llm.model` | `OPENMONO_LLM_MODEL` | Model name |
| `web.search` | `OPENMONO_WEB_SEARCH` | Web search URL |
| `web.scrape` | `OPENMONO_WEB_SCRAPE` | Web scrape URL |

## Next

Continue to [01-cli](01-cli.md) for CLI commands and configuration.
