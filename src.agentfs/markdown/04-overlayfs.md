---
title: OverlayFS — Copy-on-Write Implementation
---

# OverlayFS — Copy-on-Write Implementation

**AgentFS OverlayFS combines a read-only base layer with a writable delta layer, enabling sandboxed agents that can't modify the base project files.**

## Overlay Architecture

Source: `sdk/rust/src/filesystem/overlayfs.rs` (1,795 lines)

```mermaid
flowchart TB
    subgraph OverlayFS
        B["Base layer (read-only)"]
        D["Delta layer (writable AgentFS)"]
    end

    subgraph Read["Read Operation"]
        R1["Check delta first"]
        R2["If found: return delta data"]
        R3["If not: fall back to base"]
    end

    subgraph Write["Write Operation"]
        W1["Copy from base to delta (CoW)"]
        W2["Write to delta"]
    end

    B --> R3
    D --> R1
    R1 --> R2
    R1 --> R3
    
    B --> W1
    W1 --> W2
```

## Copy-on-Write Flow

```mermaid
sequenceDiagram
    participant App as Agent process
    participant Overlay as OverlayFS
    participant Base as Base (HostFS)
    participant Delta as Delta (AgentFS SQLite)

    App->>Overlay: open("/project/src/main.rs", O_WRONLY)
    Overlay->>Overlay: File exists in base only?
    Overlay->>Base: Copy file content to delta
    Base-->>Overlay: File data
    Overlay->>Delta: Write data to delta
    Delta-->>Overlay: File written to delta
    Overlay->>Overlay: Set copied_to_delta = true
    Overlay-->>App: Open handle to delta file
    
    App->>Overlay: write(fd, new_content)
    Overlay->>Delta: Write to delta (no base access needed)
    Delta-->>Overlay: Write complete
    Overlay-->>App: OK
```

**Aha:** The `copied_to_delta` atomic flag ensures copy-on-write happens exactly once per file. Subsequent writes go directly to the delta without re-copying. This is critical for performance — a 100MB file copied once, then modified incrementally.

## Stacking

OverlayFS can nest:

```
OverlayFS {
    base: OverlayFS {
        base: HostFS { "/project" },       ← Original project files
        delta: AgentFS { "layer1.db" }     ← First modification layer
    },
    delta: AgentFS { "layer2.db" }         ← Current working layer
}
```

Reads walk from innermost delta outward. Writes always go to the outermost delta.

## Whiteouts

When a file is deleted in the delta, OverlayFS stores a **whiteout marker** — a special entry that shadows the base file. Reads see the whiteout and return ENOENT.

## What's Next

- [05 — SDK](05-sdk.md) — TypeScript, Python, Rust SDKs
- [01 — SQLite VFS](01-sqlite-vfs.md) — Return to SQLite VFS
- [00 — Overview](00-overview.md) — Return to overview
