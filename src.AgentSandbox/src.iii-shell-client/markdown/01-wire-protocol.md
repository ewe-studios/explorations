---
title: Wire Protocol — Frame Format, Encoding, Decoding
---

# Wire Protocol — Frame Format, Encoding, Decoding

**The shell protocol uses length-prefixed binary frames with a JSON payload for multiplexed command execution.**

## Frame Wire Format

Source: `iii-shell-proto/src/lib.rs:17-54`

```mermaid
flowchart LR
    A[frame_len: u32 BE] --> B[corr_id: u32 BE]
    B --> C[flags: u8]
    C --> D[JSON payload: frame_len-5 bytes]
    D --> E[ShellMessage enum]
```

| Field | Size | Purpose |
|-------|------|---------|
| `frame_len` | 4 bytes (big-endian) | Total length of corr_id + flags + payload |
| `corr_id` | 4 bytes (big-endian) | Correlation ID for multiplexing |
| `flags` | 1 byte | Bitfield (FLAG_TERMINAL = 0x01) |
| payload | variable | UTF-8 JSON: one ShellMessage per frame |

**Aha:** `MAX_FRAME_SIZE` is 4 MiB — matching microsandbox's ceiling so a wedged session can't OOM the host relay. The 5-byte fixed header (`FRAME_HEADER_SIZE`) means the payload length is `frame_len - 5`.

## Encoding

Source: `iii-shell-proto/src/lib.rs`

```rust
pub fn encode_frame(corr_id: u32, flags: u8, msg: &ShellMessage) -> Vec<u8>
```

Steps:
1. Serialize `ShellMessage` to JSON
2. Prepend `corr_id` (4 bytes BE) + `flags` (1 byte)
3. Prepend `frame_len` (4 bytes BE = corr_id + flags + json.len())

## Decoding

Source: `iii-shell-proto/src/lib.rs`

```rust
pub fn decode_frame(data: &[u8]) -> Result<(u32, u8, ShellMessage), DecodeError>
```

Steps:
1. Read `frame_len` (4 bytes BE)
2. Read `corr_id` (4 bytes BE) + `flags` (1 byte)
3. Read JSON payload (`frame_len - 5` bytes)
4. Deserialize `ShellMessage` from JSON

## Multiplexing

Multiple sessions share the same virtio-console port, distinguished by `corr_id`:

```mermaid
sequenceDiagram
    participant Host1 as Session 1 (corr_id=1)
    participant Host2 as Session 2 (corr_id=2)
    participant Guest as Guest dispatcher

    Host1->>Guest: Exec (corr_id=1)
    Host2->>Guest: Exec (corr_id=2)
    Guest-->>Host1: Stdout (corr_id=1)
    Guest-->>Host2: Stdout (corr_id=2)
    Guest-->>Host1: Exited (corr_id=1, FLAG_TERMINAL)
    Guest-->>Host2: Exited (corr_id=2, FLAG_TERMINAL)
```

## What's Next

- [02 — Shell Client](02-shell-client.md) — Async pipe-mode client
- [03 — Filesystem Ops](03-filesystem-ops.md) — FsRequest/FsResult types
- [00 — Overview](00-overview.md) — Return to overview
