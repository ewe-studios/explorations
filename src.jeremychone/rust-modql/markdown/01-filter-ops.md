# rust-modql — Filter Operators and JSON Deserialization

**Source:** `filter/ops/` + `filter/json/` — 11 files. Type-specific operator values, JSON parsing, and sea-query conversion.

## OpValString — String Operators

```rust
// filter/ops/op_val_string.rs:9-55
pub enum OpValString {
    // Equality
    Eq(String), Not(String),

    // Set membership
    In(Vec<String>), NotIn(Vec<String>),

    // Comparison
    Lt(String), Lte(String), Gt(String), Gte(String),

    // Substring matching
    Contains(String), NotContains(String),
    ContainsAny(Vec<String>), NotContainsAny(Vec<String>),
    ContainsAll(Vec<String>),

    // Prefix matching
    StartsWith(String), NotStartsWith(String),
    StartsWithAny(Vec<String>), NotStartsWithAny(Vec<String>),

    // Suffix matching
    EndsWith(String), NotEndsWith(String),
    EndsWithAny(Vec<String>), NotEndsWithAny(Vec<String>),

    // Null/empty checks
    Empty(bool), Null(bool),

    // Case-insensitive (Ci variants)
    ContainsCi(String), NotContainsCi(String),
    StartsWithCi(String), NotStartsWithCi(String),
    EndsWithCi(String), NotEndsWithCi(String),

    // PostgreSQL ILIKE
    Ilike(String),
}
```

30 operators total. The `*Any` variants accept a list and produce `OR` conditions — any one match satisfies the filter.

### JSON Mapping

| JSON Operator | OpValString Variant | Value Type |
|--------------|---------------------|------------|
| `$eq` | Eq | String |
| `$not` | Not | String |
| `$in` | In | Array[String] |
| `$notIn` | NotIn | Array[String] |
| `$lt` | Lt | String |
| `$lte` | Lte | String |
| `$gt` | Gt | String |
| `$gte` | Gte | String |
| `$contains` | Contains | String |
| `$notContains` | NotContains | String |
| `$containsAny` | ContainsAny | Array[String] |
| `$containsAll` | ContainsAll | Array[String] |
| `$notContainsAny` | NotContainsAny | Array[String] |
| `$startsWith` | StartsWith | String |
| `$notStartsWith` | NotStartsWith | String |
| `$startsWithAny` | StartsWithAny | Array[String] |
| `$notStartsWithAny` | NotStartsWithAny | Array[String] |
| `$endsWith` | EndsWith | String |
| `$notEndsWith` | NotEndsWith | String |
| `$endsWithAny` | EndsWithAny | Array[String] |
| `$notEndsWithAny` | NotEndsWithAny | Array[String] |
| `$empty` | Empty | Bool |
| `$null` | Null | Bool |
| `$containsCi` | ContainsCi | String |
| `$notContainsCi` | NotContainsCi | String |
| `$startsWithCi` | StartsWithCi | String |
| `$notStartsWithCi` | NotStartsWithCi | String |
| `$endsWithCi` | EndsWithCi | String |
| `$notEndsWithCi` | NotEndsWithCi | String |
| `$ilike` | Ilike | String |

### Example JSON Filter

```json
{
  "name": { "$contains": "World", "$startsWith": "Hello" },
  "status": { "$in": ["active", "pending"] },
  "email": { "$endsWithCi": "@GMAIL.COM" }
}
```

Deserializes to `OpValsString` with 3 operators for `name`, 1 for `status`, 1 for `email`.

## OpValBool — Boolean Operators

```rust
// filter/ops/op_val_bool.rs:7-11
pub enum OpValBool {
    Eq(bool),
    Not(bool),
    Null(bool),
}
```

Only 3 operators — boolean fields don't need containment or pattern matching. JSON operators: `$eq`, `$not`, `$null` (all take Bool values).

## OpValValue — Generic JSON Value Operators

```rust
// filter/ops/op_val_value.rs:8-22
pub enum OpValValue {
    Eq(Value), Not(Value),
    In(Vec<Value>), NotIn(Vec<Value>),
    Lt(Value), Lte(Value), Gt(Value), Gte(Value),
    Null(bool),
}
```

The catch-all for non-scalar JSON values. Used with `#[modql(to_sea_value_fn = "...")]` to convert arbitrary JSON to sea-query values. **Aha:** There is no `From<Value>` impl for `OpValValue` because `serde_json::Value` could be any type — the library cannot safely assume a default conversion.

## OpValInt64, OpValInt32, OpValFloat64 — Numeric Operators

All numeric types share the same operator set:

```rust
pub enum OpValInt64 {
    Eq(i64), Not(i64),
    In(Vec<i64>), NotIn(Vec<i64>),
    Lt(i64), Lte(i64), Gt(i64), Gte(i64),
    Null(bool),
}
```

No string-like operators (no `Contains`, `StartsWith`, etc.).

## JSON Deserialization — Visitor Pattern

```rust
// filter/json/ovs_de_string.rs:7-56
impl<'de> Deserialize<'de> for OpValsString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StringOpValsVisitor)
    }
}
```

The `StringOpValsVisitor` handles three input types:

1. **String value** → `OpValString::Eq(value)` (shorthand)
2. **Map of operators** → parses each `{"$contains": "x", "$startsWith": "y"}` into multiple `OpValString` variants
3. The visitor calls `OpValueToOpValType::op_value_to_op_val_type(op, value)` for each key-value pair

```rust
// filter/json/ovs_json.rs:4-9
pub trait OpValueToOpValType {
    fn op_value_to_op_val_type(op: &str, value: Value) -> Result<Self>
    where Self: Sized;
}
```

Implemented for each `OpVal*` type. The `(_, v)` catch-all returns `JsonOpValNotSupported { operator, value }`.

## Sea-Query Condition Generation

### OpValString → Condition

```rust
// filter/ops/op_val_string.rs:210-322
impl OpValString {
    pub fn into_sea_cond_expr(self, col: &ColumnRef, options: &FilterNodeOptions) -> SeaResult<Condition> {
        match self {
            OpValString::Eq(s) => binary_fn(BinOper::Equal, s),
            OpValString::Contains(s) => binary_fn(BinOper::Like, format!("%{s}%")),
            OpValString::ContainsAll(values) => {
                let mut cond = Condition::all();
                for value in values {
                    cond = cond.add(binary_fn(BinOper::Like, format!("%{value}%")));
                }
                cond
            }
            OpValString::ContainsAny(values) => cond_any_of_fn(BinOper::Like, values, "%", "%"),
            OpValString::Empty(empty) => {
                // Empty checks both NULL and "":
                Condition::any()
                    .add(sea_is_col_value_null(col.clone(), empty))
                    .add(binary_fn(op, "".to_string()))
            }
            OpValString::Ilike(s) => {
                #[cfg(feature = "with-ilike")]
                { pg_binary_fn(PgBinOper::ILike, format!("%{s}%")) }
                #[cfg(not(feature = "with-ilike"))]
                { case_insensitive_fn(BinOper::Like, format!("%{s}%")) }
            }
            // ... other operators
        }
    }
}
```

Key patterns:

| Operator | SQL Generated |
|----------|--------------|
| `Contains("x")` | `col LIKE '%x%'` |
| `ContainsAll(["a","b"])` | `col LIKE '%a%' AND col LIKE '%b%'` |
| `ContainsAny(["a","b"])` | `col LIKE '%a%' OR col LIKE '%b%'` |
| `StartsWith("x")` | `col LIKE 'x%'` |
| `EndsWith("x")` | `col LIKE '%x'` |
| `Empty(true)` | `col IS NULL OR col = ''` |
| `Ilike("x")` | `col ILIKE '%x%'` (with `with-ilike`) or `LOWER(col) LIKE '%x%'` (fallback) |

### Case-Insensitive Functions

```rust
let case_insensitive_fn = |op: BinOper, v: String| {
    let col_expr = SimpleExpr::FunctionCall(Func::lower(Expr::col(col.clone())));
    let value_expr = SimpleExpr::FunctionCall(Func::lower(SimpleExpr::Value(v.into())));
    SimpleExpr::binary(col_expr, op, value_expr).into()
};
```

Wraps both column and value in `LOWER()`: `LOWER(col) LIKE LOWER('pattern')`.

### OpValValue → Condition (with custom conversion)

```rust
// filter/ops/op_val_value.rs:95-147
impl OpValValue {
    pub fn into_sea_cond_expr_with_json_to_sea(
        self, col: &ColumnRef, options: &FilterNodeOptions,
        to_sea_value: &ToSeaValueFnHolder,
    ) -> SeaResult<Condition> {
        match self {
            OpValValue::Eq(json_value) => {
                let sea_value = to_sea_value.call(json_value)?;  // custom conversion
                binary_fn(BinOper::Equal, sea_value)
            }
            // ... other operators
        }
    }
}
```

The `ToSeaValueFnHolder` wraps a `fn(serde_json::Value) -> SeaResult<sea_query::Value>` function pointer, allowing custom JSON-to-SQL type conversions (e.g., JSON strings → JSONB columns).

## FilterNode → Sea-Query

```rust
// filter/nodes/node.rs:162-201 (with-sea-query)
impl FilterNode {
    pub fn into_sea_cond_expr_list(self) -> SeaResult<Vec<Condition>> {
        let col = match self.rel {
            Some(rel) => ColumnRef::Column(ColumnName(Some(rel.into()), StringIden(self.name).into_iden())),
            None => StringIden(self.name).into_column_ref(),
        };

        for op_val in self.opvals {
            let cond_expr = match op_val {
                OpVal::String(ov) => ov.into_sea_cond_expr(&col, &self.options)?,
                OpVal::Int64(ov) => ov.into_sea_cond_expr(&col, &self.options)?,
                OpVal::Value(ov) => {
                    // Requires ForSeaCondition from #[modql] attribute
                    match &self.for_sea_condition {
                        Some(ForSeaCondition::ToSeaValue(fn_holder)) =>
                            ov.into_sea_cond_expr_with_json_to_sea(&col, &self.options, fn_holder)?,
                        Some(ForSeaCondition::ToSeaCondition(fn_holder)) =>
                            fn_holder.call(&col, ov)?,
                        None => Err("OpValsValue must have #[modql] attribute"),
                    }
                }
                // ...
            };
        }
    }
}
```

The `rel` field enables cross-table references: `project.title` → `ColumnRef::Column("project"."title")`.

## ForSeaCondition — Custom Value Conversion

```rust
// filter/into_sea/mod.rs:12-66
pub enum ForSeaCondition {
    ToSeaValue(ToSeaValueFnHolder),       // json Value → sea_query Value
    ToSeaCondition(ToSeaConditionFnHolder), // full custom condition
}
```

Two approaches:
1. **ToSeaValue** — provide a function that converts JSON to sea-query Value; the router builds the condition
2. **ToSeaCondition** — provide a function that builds the entire `Condition` directly

Set via `#[modql(to_sea_value_fn = "my_conversion_fn")]` or `#[modql(to_sea_condition_fn = "my_condition_fn")]` on a field.
