# zcoder -- Documentation Index

zcoder is an early-stage, parallel-first AI coding harness written in Rust. A terminal UI
client (`zc`) talks to a background daemon (`zc base`) over a Unix domain socket. The daemon
owns a SQLite database of workspaces, runs, and AI requests, and drives model calls through
the `genai` crate. This documentation covers the whole system end to end: the domain model,
the layered config, the persistence layer, the single-writer executor, the wire protocol, and
the terminal UI.

## Source Code

All source files are located at:

`/Users/darkvoid/GitSources/src.zcoder/zcoder`

The workspace has six crates: `zc-asset`, `zc-core`, `zc-common`, `zc-tui`, `zc-router`, and
`zc-base`, plus the `zc` binary at `src/main.rs`. There are 137 Rust source files.

## Quick Orientation: The Shape

```
zc (TUI client)  ── Unix socket ──>  zc base (daemon)
                                        │
                                        ├── SQLite (wspace, run, aixc)
                                        ├── single-writer executor ──> genai (model)
                                        └── model bus ──> pump loops ──> back to client
```

One prompt travels from a keystroke, through the event handler and the router, into the
executor, which writes rows and calls the model, then back out as events that the TUI
debounces and renders. The client and daemon are the same binary: `zc` re-runs itself as
`base` when no daemon is live.

## Documents by Category

### Foundation

These documents describe what zcoder is and how its crates fit together.

| Document | Covers | Key Modules |
|----------|--------|-------------|
| [Overview](00-overview.md) | What zcoder is, philosophy, six crates at a glance | All crates |
| [Architecture](01-architecture.md) | Crate dependency graph, layering, socket, event streams | `zc-base`, `zc-router`, `zc-tui` |

### Deep Dives

These documents go deep into one component each.

| Document | Covers | Key Modules |
|----------|--------|-------------|
| [Domain Model](02-domain-model.md) | `Air`/`Wks`/`Run` entities, `Id`, `EpochUs`, `EntityType` | `zc-core` |
| [Config System](03-config-system.md) | Layered TOML, JSON merge, model aliases, hot reload | `zc-base::config` |
| [Persistence](04-persistence.md) | SQLite schema, BMC CRUD, `ModelManager`, model bus | `zc-base::model` |
| [Executor](05-executor.md) | `RunPrompt` flow, air exec, udiffx, aiprog, pricing | `zc-base::exec` |
| [Router & Transport](06-router-transport.md) | `RouterMsg`, postcard framing, server, client | `zc-router` |
| [Event Infrastructure](07-event-infrastructure.md) | Channels, time, dirs, clipboard, cache, JSON | `zc-common` |
| [TUI Core](08-tui-core.md) | Event loop, debouncer, state processor, prompter state | `zc-tui::core` |
| [TUI Rendering](09-tui-rendering.md) | Views, tblock builder, style system | `zc-tui::view` |
| [Bootstrap & Lifecycle](10-bootstrap-lifecycle.md) | `main`, base command, spawner, socket, assets | `zc` + `zc-asset` |

### Cross-Cutting

These documents span the whole system.

| Document | Covers | Key Modules |
|----------|--------|-------------|
| [Data Flow](11-data-flow.md) | End-to-end sequence diagrams, pull-on-notify | All |
| [Cross-Cutting Concerns](12-cross-cutting.md) | Rust idioms, production-grade version, resilient storage | All |

## Document Details

### [00-overview.md](00-overview.md) -- Overview
The entry point. Describes the client/server split, the "parallel first" philosophy, the six
crates at a glance, and the ten-minute mental model: `zc` prompts, `zc base` owns state and
runs model calls, the two meet over a socket.

### [01-architecture.md](01-architecture.md) -- Architecture
The crate dependency graph and layering. Explains why `zc-core` is the shared contract crate
with no persistence or execution, why the daemon owns the database, and how the two event
streams (broadcast model changes, point-to-point exec events) flow.

### [02-domain-model.md](02-domain-model.md) -- Domain Model
The three tables `wspace`, `run`, and `aixc` and their Rust entities `Wks`, `Run`, `Air`.
Covers `Id` (UUID), `EpochUs` (microsecond timestamps), `EntityType`, the model change and
exec events, and the alias resolution with `MAX_ALIAS_DEPTH`.

### [03-config-system.md](03-config-system.md) -- Config System
Three-layer TOML config: base default, base user, workspace. JSON merge semantics, model alias
resolution with reasoning suffixes, `get_model`, and copy-on-write hot reload via `ArcSwap`.

### [04-persistence.md](04-persistence.md) -- Persistence
The SQLite schema (`STRICT`, `WITHOUT ROWID`), the BMC (Base Model Controller) CRUD pattern,
the `DbBmc` trait, `ModelManager`, and the model bus that broadcasts every write. In-memory
today, with the `create_where_not_exists` idempotency trick.

### [05-executor.md](05-executor.md) -- Executor
The single-writer executor that turns a `RunPrompt` command into a run, air requests, a model
call, and side effects. Covers `udiffx` diff parsing, `aiprog` program generation, cost
pricing, and the `RunStart`/`RunEnd`/`RunError` lifecycle.

### [06-router-transport.md](06-router-transport.md) -- Router and Transport
The `RouterMsg` envelope, postcard wire framing with `MAX_FRAME_LEN`, the attach handshake,
the `RouterServer`/`RouterClient` pair, and the correlated RPC map with `PendingRequest`.

### [07-event-infrastructure.md](07-event-infrastructure.md) -- Event Infrastructure
The `zc-common` plumbing: the `event_base` channel facade (MPSC, SPSC, `Once`), `MsgId` vs
`Id`, the socket and cache directory constants, `jsons::merge`, and the kept-alive clipboard.

### [08-tui-core.md](08-tui-core.md) -- TUI Core
The `TuiEvent` model, the `start_tui` wiring, the `run_ui_loop`, the per-frame `Debouncer`
with its per-kind coalescing laws, the `ping_timer`, the term reader, and the `StateProcessor`
reducer with the `PrompterState` byte-cursor machine.

### [09-tui-rendering.md](09-tui-rendering.md) -- TUI Rendering
The four-pane layout, the answer view with scroll zone and scrollbar, the `TBlockKind` gutter
(`"▌ "` colored by kind), the running-arrow animation, and the grayscale style system.

### [10-bootstrap-lifecycle.md](10-bootstrap-lifecycle.md) -- Bootstrap and Lifecycle
`main`, the `base` command, the self-spawning daemon, the `connect_or_spawn` retry loop, the
base parts wiring, workspace resolution, the `zc-asset` embedded zip, and the idle shutdown.

### [11-data-flow.md](11-data-flow.md) -- Data Flow
End-to-end sequence diagrams tracing one prompt from keystroke to model call and back to the
screen. The broadcast-vs-status distinction and the pull-on-notify read pattern.

### [12-cross-cutting.md](12-cross-cutting.md) -- Cross-Cutting Concerns
The Rust idioms that shape the project, what a production-grade version would change, and how
to build a resilient storage system like this from first principles.
