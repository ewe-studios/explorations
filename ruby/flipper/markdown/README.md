# Flipper Deep Exploration — Index

A technical exploration of the Flipper feature-flag monorepo, version 1.4.2 (source at `/Users/darkvoid/GitSources/src.Ruby/flipper/`).

- [00-overview.md](./00-overview.md) — What Flipper is: one flag question over many storage backends, and the layered map of the whole gem.
- [01-feature-flag-core-api.md](./01-feature-flag-core-api.md) — The feature flag and the core API: `enable`, `disable`, `enabled?`, actors, and the module-level delegators.
- [02-adapter-storage-contract.md](./02-adapter-storage-contract.md) — The Adapter contract and the Memory adapter as the reference implementation of it.
- [03-groups-percentage-rollouts.md](./03-groups-percentage-rollouts.md) — Groups and percentage-of-actors rollouts: deterministic bucketing with CRC32.
- [04-gate-layer.md](./04-gate-layer.md) — The six stateless Gates and how one `enabled?` check is evaluated.
- [05-adapter-stack-instrumentation.md](./05-adapter-stack-instrumentation.md) — The decorated adapter stack: Memoizable, Instrumented, Strict, and the AdapterBuilder.
- [06-activerecord-redis-adapters.md](./06-activerecord-redis-adapters.md) — Two real adapters, ActiveRecord (two tables) and Redis (hashes), showing how storage plugs in.
- [07-expressions.md](./07-expressions.md) — The Expression gate: a condition tree of Any, All, Equal, and Property as a feature check.
- [08-cloud-api-ui.md](./08-cloud-api-ui.md) — The web front ends: Flipper Cloud (HTTP adapter plus sync), the REST API, and the web UI.
- [09-composition.md](./09-composition.md) — How it all composes: the adapter stack, sync, import/export, the Rails engine, and testing.
