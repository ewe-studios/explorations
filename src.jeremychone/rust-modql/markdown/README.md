# rust-modql — Documentation

**Source:** `src/` — 41 Rust files across 6 modules. Model query language with filtering, field metadata, and SQL generation for sea-query and rusqlite.

`modql` provides expressive filtering (inspired by [joql.org](https://joql.org)), field metadata extraction via `#[derive(Fields)]`, and SQL generation for both sea-query and rusqlite backends. It is serialization-agnostic but provides JSON deserialization for convenient filter parsing.

## Documentation

- [Overview](00-overview.md) — Architecture, filter system (FilterNode/Group/Groups), field system, ListOptions, OpVal types, feature flags
- [Filter Operators](01-filter-ops.md) — OpValString (28 operators), OpValBool, OpValValue, JSON deserialization, sea-query condition generation
- [SQL Integration](02-sql-integration.md) — SeaField/SeaFields, SqliteField/SqliteFields, SqliteValue dual-wrapper, RusqliteBinder bridge
