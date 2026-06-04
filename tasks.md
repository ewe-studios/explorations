# Todo - Exploration Tasks Status

## CRITICAL: Depth Requirement

All must follow our markdown engineering directive, writing the markdown and building the html with ./markdown_engineering/documentation_directive.md and build.py. Each must be detailed, not light, must be deep, pull the AHA! moments and be detailed and clear so someone junior in technical expertise can understand things fully and properly. Write fundamental documentation files to help engineers level up quickly with the gaps, ideas, technical design, data structures and processes used. See examples like ./pi and ./hermes. Follow the documentation directive.

**CRITICAL: Depth is non-negotiable.** A 200-line markdown for a large, multi-file project is unacceptable. The purpose of exploration is to teach. Read every source file. Document every significant function, type, algorithm, and data structure. Include actual code snippets with file paths. Length is not a constraint — write as much as needed to fully teach the project. If a project needs 50 pages, write 50 pages. Short documents are a failure of thoroughness, not a virtue. Grandfather review is mandatory, not optional.

Do it one by one, ensure you finished each, done grandfather review and have fixed all issues before moving to the next one.

## Sequential Task List

### TASK 1: [src.iii]/ — Comprehensive Ecosystem Documentation (DONE ✓)
The iii ecosystem has been documented with 16 documents covering engine, protocol, workers, functions, triggers, observability, SDKs, ecosystem workers, agentmemory, spec-forge, cli-tooling, skills-validation, examples, data flow, and cross-cutting concerns. Grandfather review completed. HTML generated.

**KNOWN GAPS FROM TASK 1 — must be addressed in follow-up tasks below.**

### TASK 2: [src.iii]/ — Deep Dive: iii-worker crate (42,998 LOC)
**Location:** `iii/crates/iii-worker/`
**Status:** NOT EXPLORED. This is the SINGLE LARGEST crate in the entire monorepo (43K LOC) — bigger than the engine core itself. Was completely missed in Task 1. Must document:
- Sandbox/VM architecture (likely krun-based)
- Worker runtime lifecycle
- How workers are spawned and managed
- Relationship to iii-supervisor and iii-init
- Integration with iii-filesystem for VFS

### TASK 3: [src.iii]/ — Deep Dive: iii-filesystem crate (4,421 LOC)
**Location:** `iii/crates/iii-filesystem/`
**Status:** NOT EXPLORED. This is the VFS layer. Must document:
- `PassthroughFs` backend (exposes host dir to guest VM via virtio-fs)
- `inode.rs` (702 lines) — inode table management
- `dir_ops.rs` (363 lines) — directory operations
- `file_ops.rs` (194 lines) — file read/write/create
- `create_ops.rs` (216 lines) — creation operations
- `remove_ops.rs` (201 lines) — removal operations
- `init_binary.rs` (197 lines) — init binary injection
- `platform.rs` (839 lines) — platform abstraction
- `inode_table.rs` (227 lines) — inode table shared
- `handle_table.rs` (16 lines) — file handle management
- `name_validation.rs` (77 lines) — name validation rules
- `special.rs` (103 lines) — special file handling
- `builder.rs` (227 lines) — PassthroughFsBuilder
- `metadata.rs` (199 lines) — metadata operations
- How it integrates with iii-worker VM sandboxes
- virtio-fs protocol usage

### TASK 4: [src.iii]/ — Deep Dive: iii-init crate (6,429 LOC)
**Location:** `iii/crates/iii-init/`
**Status:** NOT EXPLORED. Must document:
- VM initialization sequence
- Init binary that runs inside guest VMs
- How it sets up the sandbox environment
- Integration with iii-worker and iii-filesystem

### TASK 5: [src.iii]/ — Deep Dive: iii-supervisor crate (1,201 LOC)
**Location:** `iii/crates/iii-supervisor/`
**Status:** NOT EXPLORED. Must document:
- Process supervision for worker VMs
- Lifecycle management (start, stop, restart, health checks)
- How it detects and recovers from worker crashes

### TASK 6: [src.iii]/ — Deep Dive: iii-network crate (2,661 LOC)
**Location:** `iii/crates/iii-network/`
**Status:** NOT EXPLORED. Must document:
- Network stack for worker VMs
- How VMs connect back to the engine
- Network isolation / firewalling
- virtio-net configuration

### TASK 7: [src.iii]/ — Deep Dive: iii-shell-client (1,157 LOC) + iii-shell-proto (1,026 LOC)
**Location:** `iii/crates/iii-shell-client/` and `iii/crates/iii-shell-proto/`
**Status:** NOT EXPLORED. Must document:
- Shell client protocol and implementation
- How the shell worker (engine/src/workers/shell/) connects to the shell-client
- Shell protocol message types

### TASK 8: [src.iii]/ — Deep Dive: Console (23,051 LOC total)
**Location:** `iii/console/packages/console-frontend/` (20,656 LOC) + `console-rust/` (2,395 LOC)
**Status:** Surface-level only in Task 1. Must document:
- Full React component tree (command palette, trace visualization, keyboard shortcuts, resizable panels)
- OTEL trace data hooks and transforms
- Rust backend: HTTP server, WebSocket proxy, embedded assets
- How the console connects to the engine and routes API calls
- Route structure (TanStack Router / vite-based)

### TASK 9: [src.iii]/ — Deep Dive: Engine Workers Not Documented (10,518 LOC)
**Location:** `iii/engine/src/workers/`
**Status:** Surface-level only in Task 1. These in-process workers need individual deep dives:
- `configuration/` (2,693 LOC) — Configuration worker, dynamic config reloading
- `engine_fn/` (2,617 LOC) — Engine function registrations and routing
- `rest_api/` (4,810 LOC — views.rs: 2,644 lines) — Full REST API surface, all endpoints
- `pubsub/` (903 LOC) — Pub/sub messaging implementation
- `http_functions/` (592 LOC) — HTTP function invocation worker
- `shell/` (973 LOC) — Shell execution in-engine worker
- `bridge_client/` (541 LOC) — Bridge client for external communication

### TASK 10: [src.iii]/ — Deep Dive: Python SDK (10,884 LOC)
**Location:** `iii/sdk/packages/python/iii/`
**Status:** Surface-level only in Task 1. Must document:
- Full Python SDK implementation (10,884 lines!)
- Async event loop design
- All function/trigger registration patterns
- How it differs from Node.js SDK
- `python/observability/` (1,674 LOC) — Python observability SDK
- `python/iii-example/` (616 LOC) — Python examples

### TASK 11: [src.iii]/ — Deep Dive: Node.js Browser SDK (4,314 LOC) + Observability (2,186 LOC)
**Location:** `iii/sdk/packages/node/iii-browser/` + `node/observability/`
**Status:** NOT EXPLORED. Must document:
- How iii works in the browser (WebSocket, no Node.js runtime)
- Browser-specific limitations and adaptations
- Observability SDK for Node.js (2,186 LOC)
- `node/iii-example/` (1,030 LOC) — Node.js examples

### TASK 12: [src.iii]/ — Deep Dive: Skills System
**Location:** `iii/skills/`
**Status:** NOT EXPLORED. Must document:
- What skills are (agent-readable reference material)
- How skills are bundled and distributed
- SKILL.md format
- Relationship to skills-and-validation project
- `new_skills/` directory

### TASK 13: [src.iii]/ — Deep Dive: Infrastructure (Terraform)
**Location:** `iii/infra/terraform/`
**Status:** NOT EXPLORED. Must document:
- Cloud infrastructure setup
- Deployment patterns
- Terraform modules

### TASK 14: [src.strukto-ai]/ — Comprehensive Documentation
**Location:** `src.strukto-ai/`
**Status:** NOT STARTED. Per tasks.md item 2 — needs full exploration following directive.

### TASK 15: [src.Uncloud]/ — Deep Dive
**Location:** `@formulas/src.rust/src.cloud_providers/src.Uncloud/`
**Status:** NOT STARTED. Per tasks.md item 3.

## Completion Tracker

| Task | Scope | Status | Documents | Grandfather Review |
|------|-------|--------|-----------|-------------------|
| 1 | iii ecosystem (surface) | ✓ DONE | 16 docs, 4,715 lines, 33 mermaid, 45 aha moments | ✓ DONE (partial — gaps identified) |
| 2 | iii-worker (43K LOC) | NOT STARTED | — | — |
| 3 | iii-filesystem (4.4K LOC) | NOT STARTED | — | — |
| 4 | iii-init (6.4K LOC) | NOT STARTED | — | — |
| 5 | iii-supervisor (1.2K LOC) | NOT STARTED | — | — |
| 6 | iii-network (2.7K LOC) | NOT STARTED | — | — |
| 7 | iii-shell-client+proto (2.2K LOC) | NOT STARTED | — | — |
| 8 | Console (23K LOC) | NOT STARTED | — | — |
| 9 | Engine workers deep dive (10.5K LOC) | NOT STARTED | — | — |
| 10 | Python SDK (10.9K LOC) | NOT STARTED | — | — |
| 11 | Node browser+obs SDK (6.5K LOC) | NOT STARTED | — | — |
| 12 | Skills system | NOT STARTED | — | — |
| 13 | Infra/terraform | NOT STARTED | — | — |
| 14 | src.strukto-ai | NOT STARTED | — | — |
| 15 | src.Uncloud | NOT STARTED | — | — |

**Total unexplored LOC across tasks 2-13: ~135,000 lines**
