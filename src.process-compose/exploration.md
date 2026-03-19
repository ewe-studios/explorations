---
name: process-compose repository structure
description: Repository at /home/darkvoid/Boxxed/@formulas/src.rust/src.process-compose containing Go-based process orchestrator and C++ activity framework
type: reference
---

## Repository: src.process-compose

**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.process-compose`

**Note:** Despite `src.rust` in path, repository contains **no Rust code** - primarily Go and C++.

### Projects in Repository

#### 1. process-compose (Main - Go 1.22)
Docker-compose-like orchestrator for non-containerized processes.

**Key capabilities:**
- Process dependency management with startup order
- Recovery policies (restart: always/on_failure/no/exit_on_failure)
- Health checks (liveness/readiness probes)
- TUI using tview/tcell
- REST API (Gin + Swagger)
- Process scaling/replication
- Hot-reload project updates

**Core modules:**
- `src/app/` - ProjectRunner, Process orchestration with mutex-protected state
- `src/cmd/` - Cobra CLI commands
- `src/api/` - Gin HTTP server + WebSocket
- `src/tui/` - Terminal UI
- `src/loader/` - YAML loading, merging, validation
- `src/health/` - Health check probes
- `src/command/` - Process execution with PTY
- `src/pclog/` - Log buffering and observers
- `src/types/` - Project, ProcessConfig, ProcessState types

**Config example:** YAML with processes, depends_on conditions (process_completed, process_completed_successfully, process_started, process_healthy, process_log_ready)

#### 2. Activity (C++ with Boost)
Threading framework for concurrent activities with recovery.

**Components:**
- `Activity` - Base class with boost::thread, retry, recovery
- `SimpleActivity` - Implementation with function lists
- `Executor` - Worker executing bound functions
- `ActivityEventHandler` - Observer pattern for status events
- `RecoveryStarterQueue` - Recovery queue

**Pattern:** Parent-child activity synchronization via condition variables, retry counts, recovery callbacks.

#### 3. diskbench (Go) - Disk benchmarking tool
#### 4. glippy (Go) - Cross-platform clipboard library
