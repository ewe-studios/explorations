# walgit Deep Exploration — Index

walgit is a Rust git server that puts an entire repository into an object store: no database, no leader, no local state that matters. This exploration reads the source at `/Users/darkvoid/GitSources/src.s3/walgit/` and documents how the write-ahead log, the manifest compare-and-swap, the pack storage, the smart HTTP transfer, LFS, the JSON API, push policy, webhooks, and the disposable-cache scaling story actually work in the code. Read the documents in order; each one links to the next.

- [00-object-store-repository.md](./00-object-store-repository.md) — the object store is the repository: the WAL, the manifest CAS, and the invariants
- [01-proto-keys.md](./01-proto-keys.md) — the protobuf contract: Manifest, LogEntry, PackRef, framing, and bucket keys
- [02-store-crate.md](./02-store-crate.md) — the store crate: object layout, opaque versions, CAS, and leases
- [03-wal-crate.md](./03-wal-crate.md) — the WAL crate: sync levels, group commit, and the single-flight publisher
- [04-git-storage.md](./04-git-storage.md) — git object storage: packs, indexes, refs, and commit graphs
- [05-smart-http.md](./05-smart-http.md) — smart HTTP: info/refs, upload-pack v0/v2, and receive-pack
- [06-lfs.md](./06-lfs.md) — Git LFS: batch API, transfer, verification, and upstream read-through
- [07-json-api-sdk.md](./07-json-api-sdk.md) — the JSON API, its caching tiers, and the repos.js SDK
- [08-push-policy.md](./08-push-policy.md) — the policy.json rule language and its matcher
- [09-webhooks.md](./09-webhooks.md) — the WAL-to-event bridge, the durable cursor, and delivery
- [10-scaling.md](./10-scaling.md) — the disposable cache: sync levels, remote reads, and placement
- [11-config-auth-cli.md](./11-config-auth-cli.md) — configuration, authentication, roles, and the CLI
