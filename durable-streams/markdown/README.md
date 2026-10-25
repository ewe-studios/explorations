# durable-streams -- Index

A technical exploration of Durable Streams (beta, pre-1.0) — the data
primitive for the agent loop. Source:
`/Users/darkvoid/GitSources/src.DurableStreams/durable-streams/`.

- [00-overview.md](./00-overview.md) — What Durable Streams is, the problem it solves, and the shape of the protocol.
- [01-stream-model.md](./01-stream-model.md) — The stream, offset, epoch, addressing, and closure.
- [02-catch-up-reads.md](./02-catch-up-reads.md) — Offset-based catch-up reads.
- [03-live-tailing.md](./03-live-tailing.md) — Live tailing: long-poll and SSE.
- [04-appends.md](./04-appends.md) — Appending bytes, content types, and the write path.
- [05-idempotent-producers.md](./05-idempotent-producers.md) — Exactly-once via `(producerId, epoch, seq)`.
- [06-json-mode.md](./06-json-mode.md) — JSON messages and array flattening.
- [07-forking.md](./07-forking.md) — Stream forking and fork boundaries.
- [08-store-layer.md](./08-store-layer.md) — The store abstraction: memory and file backends, bbolt metadata; segments and offsets.
- [09-caddy-server.md](./09-caddy-server.md) — The Go Caddy plugin production server.
- [10-cloudflare-server.md](./10-cloudflare-server.md) — Workers and Durable Objects server.
- [11-durable-proxy.md](./11-durable-proxy.md) — Durable Proxy for AI token streams.
- [12-durable-state.md](./12-durable-state.md) — Durable State over streams.
- [13-stream-db.md](./13-stream-db.md) — StreamDB, a reactive database in a stream.
- [14-stream-fs.md](./14-stream-fs.md) — StreamFS, a filesystem in a stream.
- [15-yjs-ai-transports.md](./15-yjs-ai-transports.md) — Yjs CRDT sync and the AI SDK transports.
- [16-clients-conformance-deployment.md](./16-clients-conformance-deployment.md) — The ten clients, conformance suites, and CDN fan-out.
