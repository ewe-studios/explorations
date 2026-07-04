# rust-sqlb: SQL Builder

SQL builder for insert, select, update, and delete operations.

Source: `rust-sqlb/src/` — 9 modules.

## Module Structure

```
rust-sqlb/src/
├── lib.rs            # Public API
├── core.rs           # Core SQL types
├── insert.rs         # INSERT builder
├── select.rs         # SELECT builder
├── update.rs         # UPDATE builder
├── delete.rs         # DELETE builder
├── utils.rs          # SQL utilities
├── val.rs            # Value types
└── sqlx_exec.rs      # sqlx execution
```

## Cross-References

- [Overview](00-overview.md) — All 21 projects
- [rust-modql](03-rust-modql.md) — Query language (uses sqlb)

[Next → rust-udiffx](05-rust-udiffx.md)
