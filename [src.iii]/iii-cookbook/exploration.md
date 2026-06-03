---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii-cookbook
repository: git@github.com:iii-hq/iii-cookbook
explored_at: 2026-06-03T00:00:00Z
language: TypeScript, Python, Rust (planned)
---

# Project Exploration: iii Cookbook — Runnable Recipes

## Overview

The iii Cookbook is a **planned collection of runnable iii SDK samples** across IoT, Edge, Agents, Backend APIs, and more. The repository defines the structure and conventions for future examples but currently contains no actual scenario content — it is a freshly initialized skeleton repository.

```
iii-cookbook/          # ── Skeleton repository (directories planned, not created) ──
├── README.md          # Conventions, install instructions, contributing guide
└── LICENSE            # Apache-2.0 (Copyright 2026 iii-hq)
# Planned: agents/, iot/, edge/, backend-api/, data-pipelines/, realtime/, workflows/
```

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii-cookbook`
- **Remote:** `git@github.com:iii-hq/iii-cookbook`
- **Primary Language:** TypeScript, Python, Rust (planned)
- **License:** Apache-2.0 (Copyright 2026 iii-hq)

## Directory Structure

```
iii-cookbook/
├── README.md    # Conventions, install instructions, contributing guide
└── LICENSE      # Apache-2.0
```

No scenario subdirectories or example code exist yet. The repository contains only the README and LICENSE.

## Planned Structure (from README)

> **Note:** The scenario directories below do NOT exist yet. The repository contains only `README.md` and `LICENSE`. The directory tree below shows the intended future structure.

### Layout Convention

Each scenario follows the pattern: `<scenario>/<example>/`

Each example directory contains:
- Its own `README.md`
- Source code
- Dependencies (pinned engine/SDK versions)
- Minimal tests

### Planned Scenario Categories

| Category | Description |
|----------|-------------|
| **agents** | AI agent patterns and integrations |
| **iot** | IoT device scenarios |
| **edge** | Edge deployment patterns |
| **backend-api** | REST API and HTTP triggers |
| **data-pipelines** | Data processing workflows |
| **realtime** | Real-time/streaming with iii |
| **workflows** | Workflow orchestration patterns |

### Requirements

| Component | Version |
|-----------|---------|
| iii engine | 0.11.2+ |
| Node.js | 20+ |
| Python | 3.10+ |
| Rust | 1.75+ (depending on sample) |

### Engine Installation

```bash
# Via install script
curl -fsSL https://install.iii.dev/iii/main/install.sh | sh

# Or via Docker
docker run -p 49134:49134 iiidev/iii:latest
```

### SDK Packages

| Package | Registry |
|---------|----------|
| `iii-sdk` | npm |
| `iii-sdk` | PyPI |
| `iii-sdk` | crates.io |

### Conventions

1. **Self-contained examples.** Each example has its own functions/triggers, pinned engine/SDK versions, and dependencies.
2. **Secrets management.** Secrets load from `.env`; never inline keys.
3. **External services.** Samples requiring external services ship a `docker-compose.yml`.
4. **Registry publishing.** Reusable workers should be published to the iii registry, not kept in the cookbook.
5. **Official workers.** Official iii-hq workers have an `official` flag in the registry.

### Contributing

1. Fork the repository
2. Add a scenario folder with examples
3. Include README + code + minimal tests
4. One sample per PR

### Release Cadence

- Refreshed every major iii release
- Breaking samples tagged `needs-update`

## Key Insights

1. **Skeleton repository with clear conventions.** The README fully describes the intended structure, but no actual examples have been written yet. This is a greenfield documentation project waiting for content.

2. **Registry-based distribution model.** The cookbook explicitly states that reusable workers should be published to the iii registry rather than kept in the cookbook. The cookbook is for learning, not for distributing production code.

3. **Pinned versions per example.** Each example pins its engine and SDK versions, preventing version drift from breaking examples independently.

4. **`needs-update` tagging.** Breaking samples are tagged `needs-update` rather than being immediately fixed — this provides transparency about compatibility while allowing time for updates.

## Open Questions

1. **Timeline.** When will the first scenarios be added? The repository is fresh with only 2 commits.

2. **Scenario priority.** Which of the 7 planned categories will be populated first?

3. **Relationship to examples repo.** The `examples/` directory in src.iii already contains 4 working examples (ai-chat-agent, human-in-the-loop, property-search-agent, todo-app). How does the cookbook differ in scope and depth?

## Related Explorations

- [Examples](../examples/exploration.md) — Standalone iii SDK examples (4 working examples)
- [iii Engine](../iii/exploration.md) — The iii engine
- [Workers](../workers/exploration.md) — iii worker modules

## Next Steps

1. Re-explore once scenarios are populated
2. Deep-dive into each scenario category as examples are added
3. Compare cookbook examples with the standalone examples repo
