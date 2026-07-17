---
title: Boop Agent Documentation
---

# Boop Agent Documentation

An iMessage-based personal agent powered by Claude Agent SDK or OpenAI Codex.

## Foundation

- [Overview](00-overview.md) — What Boop is, why it exists, architecture at a glance
- [Architecture](01-architecture.md) — Full system diagram, module map, layer diagram

## Core Components

- [Interaction Agent](02-interaction-agent.md) — The dispatcher: decision engine, tools, prompts
- [Execution Agent](03-execution-agent.md) — Sub-agent workers: per-task execution, integrations
- [Memory System](04-memory-system.md) — Tiers, extraction, decay, vector search, cleaning

## Runtime & Integrations

- [Runtimes](05-runtimes.md) — Claude Agent SDK and Codex app-server adapters
- [Integrations & Composio](06-integrations-composio.md) — 1000+ toolkits, OAuth, identity
- [Automations](07-automations.md) — Cron scheduling, recurring tasks, timezone support

## Advanced Systems

- [Consolidation](08-consolidation.md) — Adversarial memory consolidation pipeline
- [Sendblue & iMessage](09-sendblue-imessage.md) — iMessage bridge, webhooks, typing indicators
- [Convex Data Model](10-convex-data-model.md) — Schema, tables, indexes, queries

## Optional Features

- [Optional Features](11-optional-features.md) — Local browser, Apple data, Mac integrations
- [Cost Tracking](12-cost-tracking.md) — Usage logging, cost estimation, dashboard metrics

## Cross-Cutting & Operations

- [Cross-Cutting Concerns](13-cross-cutting.md) — Privacy, heartbeat, broadcast, WebSocket
- [Setup & Deploy](14-setup-deploy.md) — Interactive setup, dev, desktop app, production
