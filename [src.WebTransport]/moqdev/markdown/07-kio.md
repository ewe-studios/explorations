---
title: kio — Async Producer/Consumer Channels
---

# kio — Async Producer/Consumer Channels

kio is a minimal async producer/consumer library with waker-based notification. Only depends on `smallvec`.

## Core Types

```rust
// moq/rs/kio/src/
pub struct Producer<T> { ... }
pub struct Consumer<T> { ... }
pub struct Shared<T> { ... }
```

Source: `moq/rs/kio/src/` — Core channel types.

## Design

kio uses waker-based notification for async wake:
- Producer sends items into shared state
- Consumer awaits on shared state
- Waker notifies consumer when items arrive

Only dependency: `smallvec` for inline storage.

Source: `moq/rs/kio/Cargo.toml:1` — Only dependency is `smallvec`.

**Aha:** kio's minimal design (single dependency) is intentional — it's used throughout the MoQ stack for async channels between protocol layers. A heavyweight dependency like `tokio::sync::mpsc` would pull in the entire tokio runtime, but kio lets the MoQ stack remain runtime-agnostic.

## Related Documents

- [Architecture](../markdown/01-architecture.md) — Module map
- [moq-net](../markdown/02-moq-net.md) — Uses kio for async channels
