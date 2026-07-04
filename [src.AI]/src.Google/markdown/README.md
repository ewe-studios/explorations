# src.Google — Google's AI Agent Ecosystem

12 Google projects: agent frameworks, UI design SDKs, cloud infrastructure, and knowledge management tools.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Google/` — 12 projects, Apache 2.0/MIT

## Documents

### Overview
| # | Document | What You'll Learn |
|---|----------|-------------------|
| 00 | [Project Map](00-overview.md) | All 12 projects at a glance, quick summaries |

### Deep Dives — Agent Framework
| # | Document | What You'll Master |
|---|----------|-------------------|
| 01 | [adk-go](01-adk-go.md) | Agent interface, LLMAgent config + callback lifecycle, runner orchestration, findAgentToRun(), InvocationContext, session/memory/artifacts, ParallelAgent, A2A remote agents |

### Deep Dives — Design & UI
| # | Document | What You'll Master |
|---|----------|-------------------|
| 02 | [stitch-sdk](02-stitch-sdk.md) | Lazy-loading domain model (Stitch→Project→Screen→DesignSystem), MCP transport with schema repair, virtual tools, AI SDK + ADK adapters, code generation architecture |
| 04 | [design.md](04-design-md.md) | DESIGN.md format (YAML tokens + Markdown prose), CLI (lint/diff/export), 9 validation rules, Tailwind/DTCG export, prose-over-tokens philosophy |

### Deep Dives — Infrastructure & Tools
| # | Document | What You'll Master |
|---|----------|-------------------|
| 03 | [Centravity](03-centravity.md) | Agentic IDE: ModelGateway (5 providers), AgentOrchestrator (swarms), Liquid Working Memory (ODE-based + Hebbian learning), 14-table SQLite, tool registry, Z3 formal verification |
| 05 | [gcsfuse](05-gcsfuse.md) | FUSE adapter: streaming writes (40% faster), parallel downloads (9x faster), auto machine-type detection, 3 ML profiles, code-generated config, CSI driver |
| 06 | [generative-ai](06-generative-ai.md) | 30+ Gemini notebook topics: function calling, context caching, multimodal live API, controlled generation, RAG grounding, Imagen, Veo, Chirp, embeddings |

## Building

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/src.Google
python3 build.py .
```
