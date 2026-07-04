# rust-modql: Query Language for SeaORM/rusqlite

Query language with filters, fields, ordering, and pagination. Supports SeaORM and rusqlite backends.

Source: `rust-modql/src/` — 7 modules.

## Module Structure

```
rust-modql/src/
├── lib.rs            # Error, field, filter, includes modules
├── error.rs          # Error type
├── field/            # Field selection
├── filter/           # Filter expressions
├── includes.rs       # Include/nested loading
├── sqlite/           # rusqlite backend (feature: with-rusqlite)
└── sea_utils/        # SeaORM utilities (feature: with-sea-query)
```

## Features

| Feature | Enables |
|---------|---------|
| `with-rusqlite` | rusqlite backend |
| `with-sea-query` | SeaORM query support |

## Cross-References

- [Overview](00-overview.md) — All 21 projects

[Next → rust-sqlb](04-rust-sqlb.md)
