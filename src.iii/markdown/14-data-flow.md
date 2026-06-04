---
title: Data Flow — End-to-End Invocation, Durable Workflows, Streaming, Telemetry
---

# Data Flow — End-to-End Invocation, Durable Workflows, Streaming, Telemetry

**This document traces data through the iii system end-to-end.** We follow requests from entry point through the engine, to workers, and back — covering function invocation, durable queue workflows, streaming updates, and telemetry pipelines.

## Function Invocation Flow

```mermaid
sequenceDiagram
    participant Client as External Client
    participant HTTP as iii-http Worker
    participant Engine as Engine Core
    participant FuncReg as Functions Registry
    participant Worker as Target Worker
    participant OTEL as OTEL Tracing

    Client->>HTTP: POST /greet {name: "World"}
    HTTP->>Engine: InvokeFunction {function_id: "greet", data: {...}}

    Engine->>Engine: router_msg(InvokeFunction)

    alt Middleware registered
        Engine->>Engine: Call middleware function
    end

    Engine->>Engine: spawn_invoke_function()
    Engine->>OTEL: Create span with traceparent/baggage
    Engine->>FuncReg: get("greet")
    FuncReg-->>Engine: Function {handler}

    Engine->>Engine: Create Invocation (uuid, oneshot channel)

    alt In-process function
        Engine->>Worker: Direct handler call
    else External function (WS)
        Engine->>Worker: Forward via WebSocket
    else HTTP external function
        Engine->>Worker: HTTP request to invocation URL
    end

    Worker->>Worker: Execute business logic
    Worker-->>Engine: FunctionResult::Success({message: "Hello, World!"})

    Engine->>OTEL: Record span status = OK
    Engine->>Engine: Send result via oneshot channel
    Engine-->>HTTP: InvocationResult {result}
    HTTP-->>Client: HTTP 200 {message: "Hello, World!"}
```

## Durable Queue Workflow

The human-in-the-loop pattern demonstrates durable workflows:

```mermaid
sequenceDiagram
    participant Client as External Client
    participant HTTP as iii-http
    participant Queue as iii-queue
    participant State as iii-state
    participant Worker as SDK Worker

    Client->>HTTP: POST /orders {item: "widget"}
    HTTP->>Worker: Invoke order::submit
    Worker->>State: state::set("orders", orderId, order)
    Worker->>Queue: iii::durable::publish {topic: "order.submitted"}
    Worker-->>HTTP: HTTP 201 {id: orderId}
    HTTP-->>Client: 201 Created

    Queue->>Worker: Trigger order::analyze-risk (durable:subscriber)
    Worker->>State: state::get("orders", orderId)
    Worker->>Worker: Calculate risk score

    alt Risk ≤ 70 (auto-approve)
        Worker->>State: state::set("orders", orderId, {status: "approved"})
        Worker->>Queue: publish {topic: "order.auto_approved"}
    else Risk > 70 (pause for approval)
        Worker->>State: state::set("orders", orderId, {status: "awaiting_approval"})
        Note over Worker: NO PUBLISH — workflow paused
    end

    alt Approval received
        Client->>HTTP: POST /webhooks/orders/:id/approve
        HTTP->>Worker: Invoke order::approval-webhook
        Worker->>State: state::get (verify awaiting_approval)
        Worker->>State: state::set("orders", orderId, {status: "approved"})
        Worker->>Queue: publish {topic: "order.approved"}
    end

    Queue->>Worker: Trigger order::complete (subscribes to both topics)
    Worker->>State: state::set("orders", orderId, {status: "completed"})
    Worker->>State: stream::set("orderProgress", orderId, {status: "completed"})
```

**Aha:** The workflow pause is achieved by NOT publishing an event. The durable subscriber pattern means the workflow only continues when explicitly triggered. The state is the checkpoint, and the absence of an event is the pause signal.

## Streaming Updates

The spec-forge streaming flow demonstrates iii Channels:

```mermaid
sequenceDiagram
    participant Browser as Browser
    participant Engine as iii Engine
    participant Channel as Channel Manager
    participant Worker as spec-forge
    participant Claude as Claude API

    Browser->>Engine: POST /spec-forge/stream {prompt, catalog}
    Engine->>Worker: Route to stream function
    Worker->>Worker: Check SHA-256 + semantic cache

    alt Cache miss
        Worker->>Worker: Acquire rate limiter
        Worker->>Channel: create_channel() → {channel_id, access_key}
        Worker-->>Engine: Return {channel_id, access_key}
        Engine-->>Browser: Channel connection info
        Browser->>Channel: Connect to /channels/{channel_id}

        par Background
            Worker->>Claude: SSE streaming request
            loop Per accumulated patch
                Claude-->>Worker: content_block_delta tokens
                Worker->>Worker: Parse JSONL patch
                Worker->>Worker: Apply patch to spec
                Worker->>Channel: write(patch)
                Channel-->>Browser: ChannelData {patch}
                Browser->>Browser: Re-render component
            end
            Worker->>Worker: Validate final spec
            Worker->>Worker: Cache result (SHA-256 + semantic)
            Worker->>Channel: write({"type": "done"})
        end
    else Cache hit
        Worker-->>Engine: Return cached spec immediately
        Engine-->>Browser: Complete spec (0ms)
    end
```

## Telemetry Pipeline

```mermaid
flowchart LR
    subgraph Sources["Telemetry Sources"]
        SDK["SDK Workers<br/>(binary frames)"]
        Engine["Engine Internal<br/>(tracing spans)"]
        Workers["In-Process Workers<br/>(metrics)"]
    end

    subgraph Ingestion["Ingestion"]
        OTLP["/otel WebSocket<br/>OTLP/MTRC/LOGS"]
        Tracing["tracing::info_span!"]
    end

    subgraph Processing["OpenTelemetry SDK"]
        Tracer["TracerProvider"]
        Meter["MeterProvider"]
        Logger["LoggerProvider"]
    end

    subgraph Export["Export"]
        Console["Console Exporter"]
        Remote["OTLP Exporter<br/>(Jaeger, Grafana)"]
    end

    SDK -->|OTLP prefix| OTLP
    Engine -->|tracing spans| Tracing
    Workers -->|metrics| Meter

    OTLP --> Tracer & Meter & Logger
    Tracing --> Tracer

    Tracer --> Console & Remote
    Meter --> Console & Remote
    Logger --> Console & Remote
```

### Binary Frame Processing

```
┌───────────────────────────────────────────┐
│ WebSocket Binary Frame                    │
├─────────────┬─────────────────────────────┤
│ 4-byte prefix │ JSON payload              │
├─────────────┼─────────────────────────────┤
│ OTLP          │ OpenTelemetry trace spans  │
│ MTRC          │ OpenTelemetry metrics      │
│ LOGS          │ OpenTelemetry logs         │
└─────────────┴─────────────────────────────┘
```

Each frame:
1. Checked for prefix match (`bytes.starts_with(OTLP_WS_PREFIX)`)
2. Payload extracted (prefix bytes removed)
3. Parsed as UTF-8 JSON
4. Ingested via appropriate OTEL handler
5. Bypasses message routing entirely

## State Access Pattern

All examples and workers use the same state primitive:

```mermaid
sequenceDiagram
    participant Worker as SDK Worker
    participant Engine as iii Engine
    participant StateWorker as iii-state Worker

    Worker->>Engine: trigger({function_id: "state::get", payload: {scope: "orders", key: id}})
    Engine->>StateWorker: Route to state worker
    StateWorker->>StateWorker: DashMap.get(scope + "::" + key)
    StateWorker-->>Engine: Return value
    Engine-->>Worker: Value

    Worker->>Engine: trigger({function_id: "state::set", payload: {scope: "orders", key: id, value: data}})
    Engine->>StateWorker: Route to state worker
    StateWorker->>StateWorker: DashMap.insert(scope + "::" + key, value)
    StateWorker-->>Engine: ACK
    Engine-->>Worker: ACK
```

## Queue Message Flow

```mermaid
sequenceDiagram
    participant Publisher as Publisher
    participant Queue as iii-queue Worker
    participant Subscriber as Subscriber

    Publisher->>Queue: iii::durable::publish {topic: "order.submitted", data: {...}}
    Queue->>Queue: Push to topic queue
    Queue->>Queue: Check for durable:subscriber triggers

    loop Consumer picks up message
        Queue->>Subscriber: Invoke subscriber function
        Subscriber->>Subscriber: Process message
        alt Success
            Subscriber-->>Queue: Acknowledge
            Queue->>Queue: Remove from queue
        else Failure
            Subscriber-->>Queue: Return error
            Queue->>Queue: Increment retry count
            opt Retry limit exceeded
                Queue->>Queue: Move to Dead Letter Queue
            end
        end
    end
```

## What's Next

- [15 — Cross-Cutting](15-cross-cutting.md) — Security, configuration, testing, CI/CD
- [00 — Overview](00-overview.md) — Return to overview
