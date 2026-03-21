# Moltbook Ecosystem Exploration

## Executive Summary

The Moltbook ecosystem is a comprehensive suite of AI agent tools, infrastructure, and platforms centered around the Clawdbot/Moltbot personal AI assistant. This exploration covers the entire ecosystem including skill registries, workflow engines, deployment infrastructure, and multi-language agent implementations.

## Ecosystem Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         MOLTBOT ECOSYSTEM                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                  │
│  │   MOLTBOOK   │────│   MOLTBOT    │────│   CLAWDBOT   │                  │
│  │   Registry   │    │  Core Agent  │    │   Gateway    │                  │
│  └──────────────┘    └──────────────┘    └──────────────┘                  │
│         │                  │                   │                            │
│         ▼                  ▼                   ▼                            │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                  │
│  │   MOLTHUB    │    │   LOBSTER    │    │  CLAWDINATOR │                  │
│  │  Skills/Souls│    │  Workflows   │    │  NixOS+AWS   │                  │
│  └──────────────┘    └──────────────┘    └──────────────┘                  │
│         │                  │                   │                            │
│         ▼                  ▼                   ▼                            │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                  │
│  │   ZEROLAW    │    │   NANOBOT    │    │   THEPOPEBOT │                  │
│  │   Rust Core  │    │   Python     │    │  GitHub+Cron │                  │
│  └──────────────┘    └──────────────┘    └──────────────┘                  │
│         │                  │                                                │
│         ▼                  ▼                                                │
│  ┌──────────────┐    ┌──────────────┐                                      │
│  │  BARNACLE    │    │  MOLT.BOT    │                                      │
│  │   Discord    │    │   Landing    │                                      │
│  └──────────────┘    └──────────────┘                                      │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Projects Summary

| Project | Language | Purpose | Status |
|---------|----------|---------|--------|
| **MoltHub** | TypeScript/Convex | Public skill registry (SKILL.md + SOUL.md) | Production |
| **Moltbot** | TypeScript | Core AI agent gateway (WhatsApp, Discord, etc.) | Production |
| **Lobster** | TypeScript | Workflow automation with approval gates | Production |
| **Moltinators** | Nix/OpenTofu | NixOS on AWS deployment infrastructure | Production |
| **Zeroclaw** | Rust | Ultra-lightweight agent runtime (<5MB RAM) | Active Dev |
| **Nanobot** | Python | Ultra-lightweight assistant (~3400 lines) | Active Dev |
| **ThePopebot** | Node.js | GitHub Actions-based autonomous agents | Production |
| **Barnacle** | TypeScript/Bun | Discord bot with Carbon framework | Production |
| **molt.bot** | Astro | Landing page + installer scripts | Production |
| **OpenClaw** | TypeScript | Upstream agent framework | Active Dev |
| **Clawd** | Various | Personal agent skills/configurations | Personal |

## Core Architecture Principles

1. **Plugin Architecture**: Core stays lean; capabilities ship as plugins
2. **Declarative Configuration**: Infrastructure as code, reproducible deployments
3. **Security by Default**: Gateway pairing, sandboxing, explicit allowlists
4. **Trait-Based Design**: Swap providers, channels, tools with config changes
5. **Memory Systems**: SQLite hybrid search, vector embeddings, FTS5

## Key Integrations

### Channels (Communication)
- WhatsApp (Baileys)
- Discord
- Telegram
- Slack
- iMessage
- Mattermost
- Signal
- Email (SMTP/IMAP)
- Feishu/Lark
- Webhook API

### AI Providers
- OpenAI/GPT
- Anthropic/Claude
- OpenRouter (multi-model)
- Ollama (local)
- DeepSeek
- Groq
- Gemini

### Memory Backends
- SQLite (hybrid vector + keyword)
- PostgreSQL
- Lucid bridge
- Markdown files
- No-op (stateless)

## Document Index

- [molthub.md](./molthub.md) - MoltHub skill registry deep dive
- [moltnators.md](./moltnators.md) - Moltinators deployment infrastructure
- [molthub-deep-dive.md](./molthub-deep-dive.md) - API and implementation details
- [molbot-deep-dive.md](./molbot-deep-dive.md) - Core agent architecture
- [production-grade.md](./production-grade.md) - Production considerations
- [rust-revision.md](./rust-revision.md) - Rust implementation roadmap

---

*This exploration is part of the repo-expolorations project. Last updated: 2026-03-22*
