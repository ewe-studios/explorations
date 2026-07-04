# Wayfinder: Overview

Wayfinder is an ecosystem of tools for autonomous software verification and routing.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Wayfinder/`

## Sub-Projects

| Project | Files | Purpose |
|---------|-------|---------|
| proofkeeper | 91 | Autonomous QA with AI-driven testing, coverage, fidelity gates |
| rac-core | 211 | RAC (Routing And Control) framework |
| wayfinder-router | 72 | Routing system with decision/scorer/gateway |
| rac-benchmarks | 101 | Benchmarks for routing and control |
| rac-ci | 2 | CI integration |
| rac-editors | — | Editor integrations |
| rac-sdk | 7 | SDK |

## proofkeeper

Autonomous verification system with:
- **Coverage read-model** — Graph-based capability coverage
- **Compiler** — Test generation from recorded sessions
- **Fidelity gate** — Acceptance bar for AI-generated tests
- **Runner** — Playwright test execution
- **Learning store** — Remember failed attempts
- **QA loop** — Select → drive → compile → fidelity → run → write-back
- **Scoped QA** — PR-triggered, diff-scoped verification

## wayfinder-router

Routing with:
- **Decision** — Routing decisions
- **Gateway** — Gateway abstraction
- **Scorer** — Route scoring
- **Theme** — UI theming

## Cross-References

- [proofkeeper](01-proofkeeper.md) — Autonomous QA
- [rac-core](02-rac-core.md) — RAC framework
- [wayfinder-router](03-wayfinder-router.md) — Routing
- [rac-benchmarks](04-rac-benchmarks.md) — Benchmarks
- [rac-sdk](05-rac-sdk.md) — SDK

[Next → proofkeeper](01-proofkeeper.md)
