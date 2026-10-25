# Feature Toggles Deep Exploration — Index

A technical exploration of the Ruby gem `feature_toggles` 1.2.1 (sponsored by Evil Martians), a small mechanism for pending features that take longer than a single release cycle.

- [00-overview.md](./00-overview.md) — What the gem is: named Ruby blocks that gate pending features, and the map of the six-file library.
- [01-config-dsl.md](./01-config-dsl.md) — The Ruby DSL: `env` and `feature`, and how a feature is declared with a resolver and metadata.
- [02-feature-object.md](./02-feature-object.md) — The `Feature` value object: name, resolver, and metadata, with no decision logic of its own.
- [03-runtime-api.md](./03-runtime-api.md) — `enabled?`, the global environment-variable override, `for` and the Proxy, and the `to_a` / `to_h` reports.
- [04-loading-and-rails.md](./04-loading-and-rails.md) — File loading via `instance_eval`, the Rails Railtie, and the engine-versus-application order.
