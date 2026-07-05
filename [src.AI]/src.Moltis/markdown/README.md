# Moltis — Multi-Channel AI Agent Platform

Self-hosted Rust agent server connecting LLMs to 9 messaging channels with 68 crates and sandboxed tool execution.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Moltis/moltis/` — 1,038 Rust files, 72 workspace members, MIT

## Documents

| # | Document | What You'll Master |
|---|----------|-------------------|
| 00 | [Overview](00-overview.md) | Architecture, 68-crate map, key design decisions |
| 01 | [Gateway](01-gateway.md) | Method dispatch, session registry, channel store, webhook middleware |
| 02 | [Agent Loop](02-agent-loop.md) | Streaming execution, tool accumulation, retry, loop detection, compaction |
| 03 | [Channels](03-channels.md) | ChannelPlugin trait, 9 channel types, webhook verification, media download |
| 04 | [Providers](04-providers.md) | LlmProvider trait, 10+ backends, model discovery, local GGUF, ProviderChain |
| 05 | [Tools & Sandbox](05-tools-sandbox.md) | 30+ tools, 6-layer policy, Firecracker/Docker/WASM sandboxes, sub-agent spawning |

## Build HTML

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.Moltis
python3 build.py .
```
