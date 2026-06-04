---
source_location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.iii/iii/sdk/packages/node/
repository: part of git@github.com:iii-hq/iii (monorepo)
explored_at: 2026-06-04
documentation_goal: Document the iii Browser SDK and Observability SDK.
---

# Spec: iii Browser SDK + Observability SDK Documentation

## 1. Source Codebase

| Property | Value |
|----------|-------|
| Location | `iii/sdk/packages/node/iii-browser/` + `observability/` |
| Language | TypeScript |
| License | Apache-2.0 |
| Browser SDK | 2,415 LOC (11 files) |
| Observability SDK | 2,169 LOC (23 files) |
| Total | 4,584 LOC |

### Browser SDK Files

| File | LOC | Purpose |
|------|-----|---------|
| `iii.ts` | 789 | Core browser SDK — WebSocket connection, registration |
| `iii-types.ts` | 352 | Type definitions |
| `types.ts` | 332 | Internal types |
| `stream.ts` | 310 | Stream management |
| `channels.ts` | 255 | Channel reader/writer |
| `state.ts` | 119 | State operations |
| `utils.ts` | 82 | Utility functions |
| `iii-constants.ts` | 58 | Engine functions/triggers constants |
| `index.ts` | 43 | Public exports |

### Observability SDK Files

| File | LOC | Purpose |
|------|-----|---------|
| `telemetry-system/index.ts` | 326 | OTEL system setup |
| `telemetry-system/connection.ts` | 229 | WebSocket OTEL connection |
| `telemetry-system/span-ops.ts` | — | Span operations |
| `telemetry-system/span-exporter.ts` | 107 | Span exporter |
| `telemetry-system/metrics-exporter.ts` | 113 | Metrics exporter |
| `telemetry-system/log-exporter.ts` | — | Log exporter |
| `telemetry-system/fetch-instrumentation.ts` | 152 | Fetch auto-instrumentation |
| `telemetry-system/baggage-span-processor.ts` | — | Baggage propagation |
| `logger.ts` | 174 | Logger with OTEL integration |
| `worker-metrics.ts` | 154 | Worker metrics collection |
| `otel-worker-gauges.ts` | 151 | Worker gauge metrics |
| `http-instrumentation.ts` | — | HTTP instrumentation |

## 2. What These SDKs Are

**Browser SDK** (`iii-browser-sdk`) — iii SDK for the browser with no OpenTelemetry or Node.js dependencies. Connects to the iii engine via WebSocket with RBAC protection.

**Observability SDK** (`@iii-dev/observability`) — OpenTelemetry and logging primitives shared across all iii SDKs. Provides Logger, OTEL setup, span/metrics/log exporters, and auto-instrumentation.

## 3-10. Standard sections

| # | Document | Status |
|---|----------|--------|
| 0 | README.md | TODO |
| 1 | 00-overview.md | TODO |
| 2 | 01-browser-sdk.md | TODO |
| 3 | 02-observability-sdk.md | TODO |
| 4 | 03-telemetry-system.md | TODO |
| 5 | 04-cross-cutting.md | TODO |
| 6 | Grandfather review | TODO |
| 7 | Fix findings | TODO |
| 8 | Generate HTML | TODO |

Build via `python3 build.py .`. Grandfather review mandatory.
