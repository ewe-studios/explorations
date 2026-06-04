---
title: Supervisor — PID-1 Supervision of Worker Processes
---

# Supervisor — PID-1 Supervision of Worker Processes

**iii-init's supervisor mode manages the user worker process with host-driven restart capability via a virtio-console control channel.**

## Two Modes

Source: `supervisor.rs:9-32`

```mermaid
flowchart TD
    A[exec_worker] --> B{III_CONTROL_PORT set?}
    B -->|No| C[Legacy mode]
    B -->|Yes| D[Supervisor mode]
    
    C --> E[Spawn worker via /bin/sh -c]
    E --> F[Signal forwarding loop]
    F --> G[waitpid reap loop]
    G --> H[Exit with child code]
    
    D --> I[Spawn worker]
    I --> J[Signal forwarding loop]
    I --> K[Control thread: RPC channel]
    K --> L{RPC command?}
    L -->|Restart| M[Kill child, respawn]
    L -->|Shutdown| N[Signal child to exit]
    L -->|Ping| O[Return OK]
    L -->|Status| P[Return child status]
    M --> K
    N --> H
```

**Aha:** Supervisor mode replaced the earlier architecture where a separate `iii-supervisor` binary lived at `/opt/iii/supervisor` inside the rootfs. The init binary already runs as PID 1, so absorbing the control-channel loop removes the extra binary, the extra exec hop, and the install plumbing.

## Signal Forwarding

Source: `supervisor.rs:67-80`

```mermaid
sequenceDiagram
    participant Kernel as Signal
    participant Handler as signal_handler
    participant Child as Worker process

    Kernel->>Handler: SIGTERM/SIGINT
    Handler->>Handler: atomic_load(CHILD_PID)
    alt PID > 0
        Handler->>Child: kill(pid, sig)
    else PID == 0
        Handler->>Handler: _exit(128 + sig)
    end
```

Only calls async-signal-safe functions: atomic load, `libc::kill`, `libc::_exit`.

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `III_CONTROL_PORT` | Virtio-console port name for control channel (triggers supervisor mode) |
| `III_WORKER_WORKDIR` | Working directory for supervisor mode (defaults to `/workspace`) |
| `III_SHELL_PORT` | Virtio-console port for `iii worker exec` shell channel |
| `III_WORKER_CMD` | Command to execute (legacy mode) |

## What's Next

- [05 — Shell Dispatcher](05-shell-dispatcher.md) — virtio-console shell channel
- [01 — Boot Sequence](01-boot-sequence.md) — Return to boot sequence
- [00 — Overview](00-overview.md) — Return to overview
