# AnyCable-Go Deep Exploration — Index

A technical exploration of AnyCable-Go 1.6.20, the Go WebSocket and SSE server for the AnyCable realtime stack (source at `/Users/darkvoid/GitSources/src.Ruby/anycable/`).

- [00-what-is-anycable.md](./00-what-is-anycable.md) — What AnyCable-Go is, why a Go server replaces the Ruby Action Cable server, and the package map.
- [01-wire-protocol.md](./01-wire-protocol.md) — The AnyCable wire protocol: protobuf over WebSocket, the Action Cable JSON messages, and the `protos/` definitions.
- [02-connection-lifecycle.md](./02-connection-lifecycle.md) — The path from handshake to welcome to disconnect, with session timers, pings, and resume.
- [03-hub-and-subscriptions.md](./03-hub-and-subscriptions.md) — The sharded in-memory hub, stream subscriptions, fan-out, and remote disconnect.
- [04-broadcasts-pubsub.md](./04-broadcasts-pubsub.md) — How broadcasts fan out over NATS or Redis pub/sub, and the broker layer for history, presence, and session cache.
- [05-rpc-layer.md](./05-rpc-layer.md) — The RPC controller back to Ruby, its retry and backoff, and the embedded mruby metrics printer.
- [06-transports.md](./06-transports.md) — The WebSocket and SSE transport servers and their encoders.
- [07-configuration-cli.md](./07-configuration-cli.md) — Configuration, command-line flags, environment variables, and TOML files.
- [08-telemetry-metrics.md](./08-telemetry-metrics.md) — Metrics, logging, and the anonymized telemetry tracker.
- [09-embedding-and-extras.md](./09-embedding-and-extras.md) — Embedding AnyCable in a Go app, plus the REST API, signed streams, Pusher, and Durable Streams.
