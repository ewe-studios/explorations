# Moltis — Multi-Channel AI Agent Platform

Self-hosted Rust agent server connecting LLMs to 9 messaging channels with 64 crates and sandboxed tool execution.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/` -- 1,038 Rust files, 68 crates + 3 apps, MIT

## Documents

| # | Document | What You'll Master |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | Architecture, 64-crate map, key design decisions |
| 01 | [Architecture](01-architecture.md) | Workspace structure, crate map, dependency graph, layered architecture |
| 01 | [Gateway](01-gateway.md) | Method dispatch, session registry, channel store, webhook middleware |
| 02 | [Config](02-gateway.md) | Config loading, validation, schema, environment substitution |
| 02 | [Agent Loop](02-agent-loop.md) | Streaming execution, tool accumulation, retry, loop detection, compaction |
| 03 | [Channels](03-channels.md) | ChannelPlugin trait, 9 channel types, webhook verification, media download |
| 03 | [Agents & Chat](03-agents-chat.md) | Complete message flow: channel -> agent loop -> provider -> reply |
| 03 | [Gateway Deep](03-gateway-deep.md) | Deep dive: voice, channels, agents, providers, native apps, startup |
| 04 | [Channels Deep](04-channels.md) | Channel event flow, WhatsApp pairing, Slack HMAC, Discord intents |
| 04 | [Providers](04-providers.md) | LlmProvider trait, 10+ backends, model discovery, local GGUF, ProviderChain |
| 05 | [Voice Pipeline](05-agents-deep.md) | 11 STT + 6 TTS providers, auto-speak modes, SSML directives |
| 05 | [Tools & Sandbox](05-tools-sandbox.md) | 30+ tools, 6-layer policy, Firecracker/Docker/WASM sandboxes, sub-agent spawning |
| 06 | [Memory](06-memory.md) | Chunking, embeddings, hybrid FTS5+vector search, RRF merge |
| 06 | [Providers Deep](06-providers-deep.md) | Provider implementations, model capabilities, grammar-constrained tools |
| 07 | [Native Apps](07-native-apps.md) | iOS/macOS Swift apps, FFI bridges, callback systems |
| 08 | [Security](08-security.md) | Auth, sandboxing, unsafe code isolation, network filter |
| 09 | [Tools Deep](09-tools-deep.md) | Tool registration, sandbox routing, background tasks, provider discovery |

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.Moltis
python3 build.py .
```
