# rust-sqlb Documentation

Simple SQL builder with sqlx integration.

## Documents

| # | Document | Description |
|---|----------|-------------|
| 0 | [Overview](00-overview.md) | Architecture, builders, SqlxBindable |

## Source

`rust-sqlb/src/` — 9 Rust files

## Usage

```rust
use sqlb::{select, insert, Field, HasFields};

let users: Vec<User> = select()
    .table("users")
    .and_where_eq("active", true)
    .fetch_all(&pool).await?;

#[derive(Fields)]
struct NewUser { name: String, email: String }

let user = insert()
    .table("users")
    .data(user.not_none_fields())
    .fetch_one(&pool).await?;
```
