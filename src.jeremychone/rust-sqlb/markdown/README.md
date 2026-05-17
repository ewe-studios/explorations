# rust-sqlb — Documentation

**Source:** `src/` — 9 Rust files. Postgres-only SQL query builder on `sqlx`.

rust-sqlb is a lightweight SQL query builder for Postgres, built on `sqlx`. It provides a fluent, builder-pattern API for SELECT, INSERT, UPDATE, and DELETE queries with parameterized binding, RETURNING support, automatic identifier escaping, and safety guards against unqualified UPDATE/DELETE operations.

## Documentation

- [Overview](00-overview.md) — Architecture at a glance, quick start, key types
- [Core Types](01-core-types.md) — Field, HasFields, SqlBuilder, Whereable, SqlxBindable, Raw, identifier escaping
- [Builders](02-builders.md) — Select, Insert, Update, Delete, sqlx_exec execution engine

## Feature Flags

| Feature | Enables |
|---------|---------|
| `chrono-support` | `chrono::NaiveDateTime`, `NaiveDate`, `NaiveTime`, `DateTime<Utc>` binding |
| `json` | `serde_json::Value` binding |
| `decimal` | `rust_decimal::Decimal` binding |
