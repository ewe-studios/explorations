# walgit versus celld on the bucket — Index

A side-by-side exploration of two Rust systems that replicate state to an
object-storage bucket. walgit (`/Users/darkvoid/GitSources/src.s3/walgit`) is a
git server whose bucket is the repository; celld
(`/Users/darkvoid/GitSources/src.s3/celld`) is a Durable Objects daemon whose
bucket is the long-term home of many SQLite cells.

- [00-shared-substrate.md](./00-shared-substrate.md) — One bucket, no database, no leader: the shared substrate.
- [01-unit-of-state.md](./01-unit-of-state.md) — The unit of state: git packs and a WAL versus per-cell SQLite and LTX.
- [02-write-path.md](./02-write-path.md) — The write path: claim_log_slot and one CAS versus on_event and the output gate.
- [03-consistency-fencing.md](./03-consistency-fencing.md) — Consistency and fencing: a CAS on the manifest versus a CAS on own.json.
- [04-durability-rpo-rto.md](./04-durability-rpo-rto.md) — Durability and RPO/RTO: a disposable cache versus the RPO=0 ticket.
- [05-efficiency.md](./05-efficiency.md) — Efficiency: content addressing and single flight versus LTX compaction and paging.
- [06-when-each-wins.md](./06-when-each-wins.md) — When each wins: the trade-offs.
