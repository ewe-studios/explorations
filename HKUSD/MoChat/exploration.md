---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.HKUSD/MoChat
repository: https://github.com/HKUDS/MoChat
explored_at: 2026-03-20T00:00:00Z
language: Python/TypeScript
---

# MoChat Exploration - Multi-Platform Chat Framework

## Overview

MoChat is a multi-platform chat framework that enables unified messaging across different chat platforms and protocols.

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.HKUSD/MoChat`
- **Remote:** `git@github.com:HKUDS/MoChat.git`
- **Primary Languages:** Python, TypeScript
- **License:** MIT

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      MoChat Core                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Adapter Layer                       │   │
│  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐           │   │
│  │  │WeChat│ │Slack │ │Teams │ │Custom│           │   │
│  │  └──────┘ └──────┘ └──────┘ └──────┘           │   │
│  └─────────────────────────────────────────────────┘   │
│                        │                                │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Message Router                      │   │
│  └─────────────────────────────────────────────────┘   │
│                        │                                │
│  ┌─────────────────────────────────────────────────┐   │
│  │           Bot/Agent Integration                  │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Components

### Adapters
- Platform-specific message handlers
- Unified message format conversion
- Authentication and session management

### Docs
- API documentation
- Integration guides
- Usage examples

### Assets
- Branding and logos
- UI components
- Configuration templates

## Features

- **Multi-Platform Support**: WeChat, Slack, Teams, and more
- **Unified API**: Consistent interface across platforms
- **Bot Integration**: Easy integration with AI agents
- **Message Routing**: Intelligent message distribution
