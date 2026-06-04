---
title: iii Browser SDK + Observability SDK
---

# iii Browser SDK + Observability SDK

**The Browser SDK connects to iii from web apps with no Node.js dependencies. The Observability SDK provides OpenTelemetry and logging primitives shared across all iii SDKs.**

## Browser SDK Architecture

```mermaid
flowchart TB
    subgraph Browser["Web App"]
        A1["iii-browser-sdk"]
        A2["WebSocket connection"]
        A3["RBAC token"]
    end

    subgraph Engine["iii Engine"]
        E1["WebSocket server"]
        E2["RBAC session"]
        E3["Function registry"]
    end

    A1 --> A2
    A2 --> A3
    A3 --> E1
    E1 --> E2
    E2 --> E3
    E3 --> A2
```

**Aha:** The browser SDK deliberately excludes OpenTelemetry and Node.js dependencies — it's designed for web apps where bundle size matters. OTEL is available separately through the Observability SDK when needed.

## Observability SDK Architecture

```mermaid
flowchart TB
    subgraph OTEL["Observability SDK"]
        O1["Logger"]
        O2["TracerProvider"]
        O3["MeterProvider"]
        O4["SpanExporter"]
        O5["MetricsExporter"]
        O6["LogExporter"]
    end

    subgraph Engine["iii Engine"]
        E1["OTEL WebSocket endpoint"]
        E2["Binary frame ingestion"]
    end

    O1 --> O2
    O1 --> O3
    O2 --> O4
    O3 --> O5
    O1 --> O6
    O4 --> E1
    O5 --> E1
    O6 --> E1
    E1 --> E2
```

## Package Comparison

| Feature | Browser SDK | Observability SDK |
|---------|------------|------------------|
| Package | `iii-browser-sdk` | `@iii-dev/observability` |
| OTEL | No | Yes |
| Node.js deps | No | Yes |
| WebSocket | Yes | Yes (for OTEL) |
| Logger | No | Yes |
| Bundle size | Minimal | Larger (OTEL libs) |
| Use case | Web apps, interactive UI | Server-side workers, full observability |

## What's Next

- [01 — Browser SDK](01-browser-sdk.md) — Core SDK, WebSocket, RBAC
- [02 — Observability SDK](02-observability-sdk.md) — Logger, OTEL setup
- [03 — Telemetry System](03-telemetry-system.md) — Exporters, instrumentation
