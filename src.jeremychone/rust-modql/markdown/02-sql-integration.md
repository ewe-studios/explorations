# rust-modql — SQL Integration (Sea-Query & SQLite)

**Source:** `field/sea/`, `field/sqlite/`, `sea_utils/`, `sqlite/` — 13 files. SQL query building with sea-query and rusqlite backends.

## SeaField — sea-query Field Representation

```rust
// field/sea/sea_field.rs:8-12
pub struct SeaField {
    pub iden: DynIden,          // column identifier
    pub column_ref: ColumnRef,  // for WHERE clause
    pub value: SimpleExpr,      // value expression
}
```

### Construction

```rust
SeaField::new(SIden("name"), "Alice")              // basic
SeaField::siden("id", 42)                          // static str shortcut
SeaField::new_with_options(iden, value, SeaFieldOptions { cast_as: Some("json") })
```

`SeaFieldOptions` supports:
- `cast_as` — SQL cast: `value::json`
- `write_placeholder` — was removed (sea-query 1.0.0-rc.23 dropped it)

### Value Extraction

```rust
SeaField::sea_value() -> Option<&Value>           // if value is SimpleExpr::Value
SeaField::value_into::<T>() -> Result<T>           // convert to specific Rust type
```

## SeaFields — Collection for Insert/Update

```rust
// field/sea/sea_fields.rs:5-57
pub struct SeaFields(Vec<SeaField>);
```

### Insert Pattern

```rust
let (idens, values) = sea_fields.for_sea_insert();
// → (Vec<DynIden>, Vec<SimpleExpr>)

sea_query::InsertStatement::new()
    .into_table(SIden("users"))
    .columns(idens)
    .values_parentheses(values)?;
```

### Update Pattern

```rust
for (iden, value) in sea_fields.for_sea_update() {
    update_stmt.value(iden, value);
}
// → Iterator<(DynIden, SimpleExpr)>
```

### Builder API

```rust
SeaFields::new(vec![...])
    .append(SeaField::siden("name", "Alice"))
    .append_siden("age", 30)
```

## HasSeaFields Trait

```rust
// field/sea/has_sea_fields.rs:4-28
pub trait HasSeaFields: HasFields {
    fn not_none_sea_fields(self) -> SeaFields;    // only Some values
    fn all_sea_fields(self) -> SeaFields;          // all values including None
    fn sea_idens() -> Vec<DynIden>;
    fn sea_column_refs() -> Vec<ColumnRef>;
    fn sea_column_refs_with_rel(rel: impl IntoIden) -> Vec<ColumnRef>;
    fn sea_apply_select_columns(&self, select: &mut SelectStatement);
}
```

Typically derived via `#[derive(Fields, SeaFieldValue)]`.

## SqliteField — rusqlite Field Representation

```rust
// field/sqlite/sqlite_field.rs:5-11
pub struct SqliteField {
    pub iden: &'static str,
    pub column_ref: SqliteColumnRef,
    pub value: SqliteValue,
    pub meta: Option<&'static FieldMeta>,  // compile-time metadata
}
```

### SQL Generation

```rust
SqliteField::sql_column() -> String           // → `"name"` or `"table"."name"`
SqliteField::sql_column_for_select() -> String // → `"name"` or `json("usage") as usage`
SqliteField::sql_placehoder_for_write() -> &'static str // → `?` or `json(?)` or `jsonb(?)`
```

The `sql_placehoder_for_write` method inspects `FieldMeta` for `cast_as` and `write_placeholder`:

| FieldMeta config | Placeholder |
|-----------------|-------------|
| `cast_as: None` | `?` |
| `cast_as: Some("json")` | `json(?)` |
| `cast_as: Some("jsonb")` | `jsonb(?)` |
| `write_placeholder: Some("CUSTOM")` | `CUSTOM` |

**Aha:** The `sql_column_for_select` method wraps JSON-cast columns with `json()`: if `cast_as` starts with `"json"`, it generates `json("column") as prop_name`. This is needed because SQLite stores JSON as TEXT, and the `json()` function validates/parses it on read.

## SqliteValue — Dual-Value Wrapper

```rust
// field/sqlite/sqlite_value.rs:7-10
pub enum SqliteValue {
    RusqliteValue(RusqliteValue),  // native rusqlite types
    SerdeValue(JsonValue),         // arbitrary JSON (serialized to TEXT)
}
```

The `ToSql` impl for `SqliteValue` converts `SerdeValue` by JSON-serializing to a string:

```rust
// field/sqlite/sqlite_value.rs:69-76
fn json_value_to_rusqlite_value(json_value: &JsonValue) -> RusqliteValue {
    let json_str = serde_json::to_string(json_value).unwrap_or_default();
    if json_str.is_empty() {
        RusqliteValue::Null
    } else {
        RusqliteValue::Text(json_str)
    }
}
```

**Aha:** `from_serializable` uses `serde_json::to_value` and falls back to `JsonValue::Null` on error — the comment notes "TODO: Need to error!() trace when fail to serialize happen." This is a silent data loss scenario if serialization fails.

## SqliteFields — Collection for SQLite Operations

```rust
// field/sqlite/sqlite_fields.rs:5-93
pub struct SqliteFields(Vec<SqliteField>);
```

### SQL Generation Methods

| Method | Output | Use Case |
|--------|--------|----------|
| `sql_columns()` | `"id", "name", "content"` | INSERT column list |
| `sql_columns_for_select()` | `"id", json("data") as data` | SELECT column list |
| `sql_placeholders()` | `?, json(?), ?` | INSERT/UPDATE values |
| `sql_setters()` | `"id" = ?, "name" = json(?)` | UPDATE SET clause |
| `for_insert()` | `(Vec<&str>, Vec<SqliteValue>)` | INSERT (columns + values) |
| `for_update()` | `Vec<(&str, SqliteValue)>` | UPDATE (column, value) pairs |
| `into_values()` | `Vec<SqliteValue>` | Bound parameters |
| `values_as_dyn_to_sql_vec()` | `Vec<&dyn ToSql>` | rusqlite query execution |

### Example: Building INSERT

```rust
let fields = SqliteFields::new(vec![
    SqliteField::new("name", "Alice"),
    SqliteField::new("email", "alice@example.com"),
]);

let sql = format!(
    "INSERT INTO users ({}) VALUES ({})",
    fields.sql_columns(),
    fields.sql_placeholders()
);
// → INSERT INTO users ("name", "email") VALUES (?, ?)

let (cols, vals) = fields.for_insert();
// → (["name", "email"], [SqliteValue::RusqliteValue("Alice"), ...])
```

## HasSqliteFields Trait

```rust
// field/sqlite/has_sqlite_fields.rs:3-30
pub trait HasSqliteFields: HasFields {
    fn sqlite_not_none_fields(self) -> SqliteFields;
    fn sqlite_all_fields(self) -> SqliteFields;
    fn sqlite_columns_for_select() -> String;
    fn sqlite_column_refs_with_rel(rel: &'static str) -> Vec<SqliteColumnRef>;
}
```

## SqliteFromRow — Row Parsing

```rust
// sqlite/mod.rs:24-36
pub trait SqliteFromRow {
    fn sqlite_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self>;
    fn sqlite_from_row_partial(row: &rusqlite::Row<'_>, prop_names: &[&str]) -> rusqlite::Result<Self>;
}
```

Derived via `#[derive(SqliteFromRow)]` from `modql-macros`. The `partial` variant reads only specified columns.

## Sea-Query Utilities

### StringIden and SIden

```rust
// sea_utils/sea_types.rs:5-44
pub struct StringIden(pub String);       // runtime string Iden
pub struct SIden(pub &'static str);      // compile-time static Iden

impl Iden for StringIden { fn unquoted(&self) -> &str { &self.0 } }
impl Iden for SIden { fn unquoted(&self) -> &str { &self.0 } }
impl IdenStatic for SIden { fn as_str(&self) -> &'static str { self.0 } }
```

`SIden` is more efficient — no allocation, works with `'static` column names. `StringIden` is needed for dynamic names (e.g., from filter node names).

### Expression Builders

```rust
// sea_utils/sea_types.rs:46-65
pub fn into_node_value_expr<T>(val: T, options: &FilterNodeOptions) -> SimpleExpr
where T: Into<Value> {
    let mut vxpr = SimpleExpr::Value(val.into());
    if let Some(cast_as) = options.cast_as {
        vxpr = vxpr.cast_as(StringIden(cast_as.into()));
    }
    vxpr
}

pub fn into_node_column_expr(col: ColumnRef, options: &FilterNodeOptions) -> SimpleExpr {
    match &options.cast_column_as {
        Some(cast) => SimpleExpr::Column(col).cast_as(StringIden(cast.into())),
        None => SimpleExpr::Column(col),
    }
}
```

Used by filter operators to build value expressions with optional SQL casting.

## Rusqlite Bridge — sea_rusqlite.rs

```rust
// sea_utils/sea_rusqlite.rs:5-104
pub struct RusqliteValue(pub sea_query::Value);
pub struct RusqliteValues(pub Vec<RusqliteValue>);

pub trait RusqliteBinder {
    fn build_rusqlite<T: QueryBuilder>(&self, query_builder: T) -> (String, RusqliteValues);
}
```

**Aha:** The comment at the top explains why this exists: "to avoid the dependency catch-22 with rusqlite and sea-query by inlining the binding lib." The `sea-query-rusqlite` crate has its own dependency graph that can conflict. This file cherry-picks just what's needed.

### Implemented For

- `SelectStatement`
- `UpdateStatement`
- `InsertStatement`
- `DeleteStatement`
- `WithQuery`

### ToSql Implementation

Maps all sea-query `Value` variants to rusqlite equivalents:

```rust
match &self.0 {
    Value::Bool(v) => opt_to_sql!(v),
    Value::Int(v) => opt_to_sql!(v),
    Value::String(v) => box_to_sql!(v),  // boxed String → &str
    Value::BigUnsigned(v) => v.map(|v| v as i64),  // downcast to i64
    Value::Enum(_) => todo!(),
}
```

**Aha:** `BigUnsigned` is cast to `i64` — this can silently truncate values above `i64::MAX`. The `Enum` variant is `todo!()` — using enums with sea-query + rusqlite will panic.
