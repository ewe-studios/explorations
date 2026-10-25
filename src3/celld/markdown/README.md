# celld

celld is a self-hosted, distributed Durable Objects daemon written in Rust. It
runs a Cloudflare Workers application from a Wrangler (Cloudflare's CLI/build
tool) `wrangler.json` file: Workers, Durable Objects, KV, Queues, D1, R2,
Workflows, Cron Triggers, and static assets.

The exploration reads the code in `/Users/darkvoid/GitSources/src.s3/celld/`.
The workspace has three crates. `crates/celld` is the executor and adapter
host. `crates/logic` is the pure decision core. `crates/ltx` is a vendored LTX
replication library.

Read the documents in order. Each one ends with a link to the next.

- [00 -- Overview](00-overview.md) — what celld is, and the shape of the
  codebase.
- [01 -- The Cell](01-the-cell.md) — one Durable Object, one SQLite database.
- [02 -- Logic and Executor](02-logic-executor-split.md) — the pure decision
  core and the effect shell.
- [03 -- Durability](03-durability.md) — how a write survives a failure.
- [04 -- Workers](04-workers-execution.md) — V8 isolates and stateless
  execution.
- [05 -- KV, Queues, D1, R2](05-kv-queues-d1-r2.md) — the storage bindings.
- [06 -- Workflows and Cron](06-workflows-cron.md) — durable steps and
  schedules.
- [07 -- Static Assets](07-static-assets.md) — serving files from the bucket.
- [08 -- Wrangler Deploy](08-wrangler-deploy.md) — from configuration to a
  deployment.
- [09 -- Fleet Operations](09-fleet-operations.md) — leases, balance, and
  shutdown.

Continue to [00-overview.md](00-overview.md).
