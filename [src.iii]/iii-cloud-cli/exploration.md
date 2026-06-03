---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii-cloud-cli
repository: git@github.com:iii-hq/iii-cloud-cli
explored_at: 2026-06-03T00:00:00Z
language: N/A (binary releases, source is private)
---

# Project Exploration: iii Cloud CLI

## Overview

The iii Cloud CLI is a **closed-source CLI tool for managing iii cloud resources**. The source code is private — this repository exists solely to host binary release assets on GitHub Releases. The CLI is installed via `iii cloud` command or by downloading binaries from the [Releases page](https://github.com/iii-hq/iii-cloud-cli/releases).

```
┌─────────────────────────────────────┐
│         iii cloud CLI              │
│                                     │
│  Install:  iii cloud                │
│  Source:   Private (not in repo)    │
│  Releases: GitHub Releases          │
│  Versions: v0.2.8 → v0.13.1        │
└─────────────────────────────────────┘
```

## Repository

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii-cloud-cli`
- **Remote:** `git@github.com:iii-hq/iii-cloud-cli`
- **Primary Language:** N/A (source is private)
- **License:** N/A (closed source)
- **Versions:** 24 version tags, v0.2.8 through v0.13.1

## Directory Structure

```
iii-cloud-cli/
└── README.md    # Installation instructions only
```

This repository contains only a `README.md` with installation instructions. No source code, build scripts, or configuration files are present.

## Version History

The repository has 24 version tags indicating active development:

| Version Range | Notes |
|--------------|-------|
| `v0.2.8`, `v0.3.0`, `v0.3.1`, `v0.4.0` | Early releases |
| `v0.7.1`, `v0.8.0`, `v0.8.1` | Mid-development |
| `v0.9.0`–`v0.9.12` | 13 patch releases — stabilization period |
| `v0.10.0`, `v0.11.1`, `v0.12.0`, `v0.13.1` | Maturing feature set |

> **Note:** Versions `v0.5.x` and `v0.6.x` have no tags — they are gaps in the release sequence.

## Installation

```bash
# Via iii command
iii cloud

# Or download from GitHub Releases
# https://github.com/iii-hq/iii-cloud-cli/releases
```

## Key Insights

1. **Source is intentionally private.** The iii Cloud CLI is a commercial product — the source code is not available, only pre-built binaries. This is consistent with the iii engine's ELv2 license model.

2. **Repository is a release artifact host.** The repo exists purely to provide a GitHub Releases page for binary downloads. The README is a single installation instruction.

3. **Active versioning.** 24 version tags across a v0.2.8 → v0.13.1 range indicates the CLI has been actively developed and iterated upon.

## Open Questions

1. **CLI capabilities.** What commands does the iii cloud CLI provide? Authentication, deployment, monitoring, resource management?

2. **Platform support.** Which platforms/architectures are the binaries built for?

3. **Relationship to iii-tools.** How does the cloud CLI relate to the open-source `iii-tools` CLI in the cli-tooling repository?

## Related Explorations

- [iii Engine](../iii/exploration.md) — The iii engine
- [CLI Tooling](../cli-tooling/exploration.md) — Open-source iii project management CLI
- [Workers](../workers/exploration.md) — iii worker modules

## Next Steps

1. Source code access would be needed for a deeper exploration
2. Document the CLI command surface if access becomes available
