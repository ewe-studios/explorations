---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.n0-computer/iroh-gossip
repository: git@github.com:n0-computer/iroh-gossip
revised_at: 2026-06-03T00:00:00Z
workspace: iroh-gossip
---

# iroh-gossip — Spec File

## Source Codebase

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.WebTransport/src.n0-computer/iroh-gossip`
- **Language:** Rust
- **Edition:** 2021
- **Rust Version:** 1.91
- **License:** MIT OR Apache-2.0
- **Version:** 0.100.0
- **Remote:** `git@github.com:n0-computer/iroh-gossip`

## What the Project Is

Iroh-gossip implements a P2P gossip protocol for broadcasting messages among peers subscribed to a topic. It is based on epidemic broadcast trees, combining HyParView (for swarm membership management) and PlumTree (for epidemic broadcast tree optimization). The protocol is implemented as an IO-less state machine in `proto/` with optional iroh-based networking in `net/`.

## Documentation Goal

After reading, a reader should understand:
1. How HyParView manages peer membership with active/passive views
2. How PlumTree builds and optimizes epidemic broadcast trees
3. How the two protocols combine in the topic state machine
4. The IO-less state machine design pattern (proto vs net separation)
5. The networking layer: connection loops, dialer, topic subscriber loop
6. The API layer: GossipApi, GossipTopic, events, and commands
7. The simulation framework for testing at scale
8. Message formats, serialization (postcard), and signing

## Documentation Structure

```
iroh-gossip/
├── spec.md
├── markdown/
│   ├── README.md
│   ├── 00-overview.md
│   ├── 01-architecture.md
│   ├── 02-hyparview.md
│   ├── 03-plumtree.md
│   ├── 04-topic-state.md
│   ├── 05-networking.md
│   ├── 06-api.md
│   ├── 07-simulation.md
│   ├── 09-data-flow.md
│   └── 10-cross-cutting.md
├── html/
└── build.py
```

## Tasks

| Phase | Document | Status |
|-------|----------|--------|
| Foundation | README.md | DONE |
| Foundation | 00-overview.md | DONE |
| Architecture | 01-architecture.md | DONE |
| Deep Dive | 02-hyparview.md | DONE |
| Deep Dive | 03-plumtree.md | DONE |
| Deep Dive | 04-topic-state.md | DONE |
| Deep Dive | 05-networking.md | DONE |
| Deep Dive | 06-api.md | DONE |
| Deep Dive | 07-simulation.md | DONE |
| Cross-Cutting | 09-data-flow.md | DONE |
| Cross-Cutting | 10-cross-cutting.md | DONE |
| Grandfather Review | All documents | DONE |
| HTML Generation | build.py | DONE |

## Build System

```bash
cd iroh-gossip && python3 build.py
```

Script: `build.py` (zero dependencies, Python 3.12+ stdlib only).

## Quality Requirements

All 10 iron rules from the markdown directive: detailed sections with code snippets, teach key facts quickly, clear articulation, 2+ mermaid diagrams per document, visual assets, generated HTML, cross-references, source path references, Aha moments, prev/next navigation.

## Expected Outcome

After reading, a developer can:
- Set up iroh-gossip with iroh
- Understand the HyParView/PlumTree protocol state machines
- Debug gossip issues using metrics and simulation
- Extend the protocol with custom message types

## Resume Point

Write documents in order: 00 → 01 → 02 → ... → 10. Each from actual source code, then grandfather review.
