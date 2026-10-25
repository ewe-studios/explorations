# anycable-delivery -- Index

A technical exploration of how the `anycable-go` server (Go, version 1.6.20) enforces
message ordering and at-least-once delivery. Source: `/Users/darkvoid/GitSources/src.Ruby/anycable/`.

- [00-the-delivery-question.md](./00-the-delivery-question.md) — What "ordered" and "at-least-once" mean here, and the honest boundary of the guarantee.
- [01-stream-model-subscriptions.md](./01-stream-model-subscriptions.md) — The stream as a string name, subscribe/unsubscribe, and the broker reference tracker.
- [02-hub-fanout-send-queue.md](./02-hub-fanout-send-queue.md) — The sharded hub, the per-gate broadcast loop, and the per-connection send queue.
- [03-cross-node-broadcasts.md](./03-cross-node-broadcasts.md) — NATS and Redis broadcast and pub/sub, and what that does to cross-node ordering.
- [04-broker-history-replay.md](./04-broker-history-replay.md) — The three broker backends, the in-memory append-only hot cache, offsets, and replay on reconnect.
- [05-session-resume-offsets.md](./05-session-resume-offsets.md) — Session restore, resubscribe, and the client-driven offset replay protocol.
- [06-where-the-guarantee-holds.md](./06-where-the-guarantee-holds.md) — The map of where ordering and at-least-once hold, and where they break.
