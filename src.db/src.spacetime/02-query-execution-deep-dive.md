# SpacetimeDB Query Execution Deep Dive

## Overview

This document explores query execution in SpacetimeDB:
- SQL parsing and AST
- Query planning (logical and physical)
- Expression evaluation
- Incremental view maintenance for subscriptions
- Multi-table joins

---

## 1. SQL Parser

### 1.1 Parser Architecture

SpacetimeDB uses `sqlparser-rs` with custom extensions:

```rust
use sqlparser::{
    ast::{Statement, Query, Select, TableFactor},
    dialect::PostgreSqlDialect,
    parser::Parser,
};

struct SpacetimeParser {
    dialect: PostgreSqlDialect,
}

impl SpacetimeParser {
    fn parse(&self, sql: &str) -> Result<SpacetimeAst> {
        let statements = Parser::parse_sql(&self.dialect, sql)?;

        // Validate SpacetimeDB-specific constraints
        for stmt in &statements {
            self.validate_statement(stmt)?;
        }

        Ok(SpacetimeAst { statements })
    }

    /// Validate SpacetimeDB-specific SQL rules
    fn validate_statement(&self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::Query(query) => {
                // No subqueries in subscriptions
                self.validate_no_unsupported_features(query)?;
            }
            Statement::Insert { table, .. } => {
                // Verify table exists
                self.validate_table_exists(table)?;
            }
            _ => {}
        }
        Ok(())
    }
}
```

### 1.2 AST Structure

```rust
/// SpacetimeDB AST (extends sqlparser)
enum SpacetimeStatement {
    /// SELECT with subscription support
    Select {
        projection: Vec<SelectItem>,
        from: TableFactor,
        joins: Vec<Join>,
        selection: Option<Expr>,
        order_by: Vec<OrderByExpr>,
        limit: Option<Expr>,
    },

    /// INSERT with return values
    Insert {
        table: String,
        columns: Vec<String>,
        values: Vec<Vec<Expr>>,
        returning: Vec<Expr>,
    },

    /// Reducer call (SpacetimeDB-specific)
    Reducer {
        name: String,
        args: Vec<Expr>,
    },
}
```

---

## 2. Query Planning

### 2.1 Logical Plan

```rust
/// Logical query plan (what to compute)
enum LogicalPlan {
    /// Table scan
    Scan {
        table: String,
        columns: Vec<String>,
        filter: Option<Expr>,
    },

    /// Filter rows
    Filter {
        predicate: Expr,
        input: Box<LogicalPlan>,
    },

    /// Project columns
    Project {
        expressions: Vec<Expr>,
        input: Box<LogicalPlan>,
    },

    /// Join two inputs
    Join {
        left: Box<LogicalPlan>,
        right: Box<LogicalPlan>,
        condition: JoinCondition,
        join_type: JoinType,
    },

    /// Sort
    Sort {
        order_by: Vec<OrderByExpr>,
        input: Box<LogicalPlan>,
    },

    /// Limit
    Limit {
        limit: usize,
        input: Box<LogicalPlan>,
    },

    /// Aggregate (GROUP BY)
    Aggregate {
        group_by: Vec<Expr>,
        aggregates: Vec<AggregateExpr>,
        input: Box<LogicalPlan>,
    },
}
```

### 2.2 Physical Plan

```rust
/// Physical query plan (how to compute)
enum PhysicalPlan {
    /// Table scan with index selection
    TableScanExec {
        table: TableId,
        index: Option<IndexId>,
        columns: Vec<ColumnId>,
        bounds: Option<RangeBounds>,
    },

    /// Filter with predicate pushdown
    FilterExec {
        predicate: CompiledExpr,
        input: Box<PhysicalPlan>,
    },

    /// Projection
    ProjectExec {
        expressions: Vec<CompiledExpr>,
        input: Box<PhysicalPlan>,
    },

    /// Hash join
    HashJoinExec {
        left: Box<PhysicalPlan>,
        right: Box<PhysicalPlan>,
        left_key: ColumnId,
        right_key: ColumnId,
        join_type: JoinType,
    },

    /// Nested loop join (for small inputs)
    NestedLoopJoinExec {
        left: Box<PhysicalPlan>,
        right: Box<PhysicalPlan>,
        predicate: CompiledExpr,
    },

    /// Sort with spill-to-disk for large datasets
    SortExec {
        order_by: Vec<SortKey>,
        input: Box<PhysicalPlan>,
        memory_limit: usize,
    },

    /// Limit with early termination
    LimitExec {
        limit: usize,
        input: Box<PhysicalPlan>,
    },
}
```

### 2.3 Query Optimizer

```rust
struct QueryOptimizer {
    /// Table statistics
    stats: TableStatistics,

    /// Available indexes
    indexes: IndexCatalog,
}

impl QueryOptimizer {
    /// Convert logical plan to optimized physical plan
    fn optimize(&self, logical: LogicalPlan) -> PhysicalPlan {
        // Generate candidate plans
        let candidates = self.generate_candidates(&logical);

        // Estimate cost for each
        let mut best_plan = None;
        let mut best_cost = f64::INFINITY;

        for plan in candidates {
            let cost = self.estimate_cost(&plan);
            if cost < best_cost {
                best_cost = cost;
                best_plan = Some(plan);
            }
        }

        best_plan.expect("No valid plans")
    }

    /// Estimate cost of physical plan
    fn estimate_cost(&self, plan: &PhysicalPlan) -> f64 {
        match plan {
            PhysicalPlan::TableScanExec { table, index, bounds, .. } => {
                // Cost = I/O + CPU
                let cardinality = self.estimate_cardinality(*table, bounds);
                let io_cost = if index.is_some() {
                    cardinality * 0.001  // Index lookup
                } else {
                    self.stats[*table].rows as f64  // Full table scan
                };
                let cpu_cost = cardinality * 0.0001;
                io_cost + cpu_cost
            }

            PhysicalPlan::HashJoinExec { left, right, .. } => {
                // Hash join cost = build + probe
                let left_cost = self.estimate_cost(left);
                let right_cost = self.estimate_cost(right);
                let left_rows = self.estimate_output_rows(left);
                let right_rows = self.estimate_output_rows(right);

                left_cost + right_cost +
                (left_rows * 0.001) +  // Build hash table
                (right_rows * 0.0001)  // Probe
            }

            PhysicalPlan::FilterExec { predicate, input } => {
                let input_cost = self.estimate_cost(input);
                let input_rows = self.estimate_output_rows(input);
                let selectivity = self.estimate_selectivity(predicate);

                input_cost + (input_rows * selectivity * 0.0001)
            }

            _ => 0.0,  // Simplified
        }
    }
}
```

### 2.4 Optimization Rules

```rust
impl QueryOptimizer {
    /// Apply optimization rules
    fn apply_rules(&self, plan: LogicalPlan) -> LogicalPlan {
        let plan = self.push_down_filters(plan);
        let plan = self.push_down_projections(plan);
        let plan = self.eliminate_redundant_operations(plan);
        let plan = self.merge_consecutive_operations(plan);
        plan
    }

    /// Push filters as close to table scan as possible
    fn push_down_filters(&self, plan: LogicalPlan) -> LogicalPlan {
        match plan {
            LogicalPlan::Filter { predicate, input } => {
                match *input {
                    LogicalPlan::Join { left, right, condition, join_type } => {
                        // Can filter be pushed to one side?
                        let left_cols = self.get_columns(&predicate);
                        if left_cols.iter().all(|c| self.is_from_table(c, &left)) {
                            // Push to left
                            LogicalPlan::Join {
                                left: Box::new(LogicalPlan::Filter {
                                    predicate,
                                    input: left,
                                }),
                                right,
                                condition,
                                join_type,
                            }
                        } else {
                            // Keep filter above join
                            LogicalPlan::Filter {
                                predicate,
                                input: Box::new(LogicalPlan::Join {
                                    left, right, condition, join_type,
                                }),
                            }
                        }
                    }
                    _ => LogicalPlan::Filter {
                        predicate,
                        input: Box::new(self.push_down_filters(*input)),
                    }
                }
            }
            _ => plan,
        }
    }

    /// Merge consecutive filters
    fn merge_consecutive_operations(&self, plan: LogicalPlan) -> LogicalPlan {
        match plan {
            LogicalPlan::Filter { predicate, input } => {
                if let LogicalPlan::Filter { predicate: inner_pred, input: inner_input } = *input {
                    // Merge: (p1 AND p2)
                    let merged = Expr::And {
                        left: Box::new(predicate),
                        right: Box::new(inner_pred),
                    };
                    LogicalPlan::Filter {
                        predicate: merged,
                        input: inner_input,
                    }
                } else {
                    LogicalPlan::Filter {
                        predicate,
                        input: Box::new(self.merge_consecutive_operations(*input)),
                    }
                }
            }
            _ => plan,
        }
    }
}
```

---

## 3. Expression Evaluation

### 3.1 Compiled Expressions

```rust
/// Compiled expression for efficient evaluation
enum CompiledExpr {
    /// Column reference
    Column {
        row_offset: usize,
        data_type: DataType,
    },

    /// Literal value
    Literal {
        value: DbValue,
    },

    /// Binary operation
    BinaryOp {
        op: BinaryOperator,
        left: Box<CompiledExpr>,
        right: Box<CompiledExpr>,
    },

    /// Unary operation
    UnaryOp {
        op: UnaryOperator,
        operand: Box<CompiledExpr>,
    },

    /// Function call
    Function {
        func: BuiltinFunction,
        args: Vec<CompiledExpr>,
    },

    /// CASE expression
    Case {
        when_then: Vec<(CompiledExpr, CompiledExpr)>,
        else_result: Option<Box<CompiledExpr>>,
    },
}

impl CompiledExpr {
    /// Evaluate expression for a row
    fn evaluate(&self, row: &Row) -> DbValue {
        match self {
            CompiledExpr::Column { row_offset, .. } => {
                row.get_value_at(*row_offset)
            }
            CompiledExpr::Literal { value } => {
                value.clone()
            }
            CompiledExpr::BinaryOp { op, left, right } => {
                let left_val = left.evaluate(row);
                let right_val = right.evaluate(row);
                self.eval_binary_op(op, left_val, right_val)
            }
            // ... other cases
        }
    }

    /// Evaluate as boolean (for filters)
    fn evaluate_bool(&self, row: &Row) -> bool {
        match self.evaluate(row) {
            DbValue::Bool(b) => b,
            DbValue::Null => false,
            _ => false,
        }
    }
}
```

### 3.2 Vectorized Expression Evaluation

```rust
/// Vectorized evaluation for batch processing
struct VectorizedExpr {
    /// SIMD-compatible operations
    compiled: CompiledExpr,
}

impl VectorizedExpr {
    /// Evaluate for batch of rows
    fn evaluate_batch(&self, rows: &[Row], output: &mut [DbValue]) {
        match &self.compiled {
            CompiledExpr::BinaryOp { op, left, right } => {
                match op {
                    BinaryOperator::Plus => {
                        // SIMD addition for numeric types
                        if left.is_numeric() && right.is_numeric() {
                            self.evaluate_batch_add(left, right, rows, output);
                        } else {
                            // Scalar fallback
                            for (i, row) in rows.iter().enumerate() {
                                output[i] = self.compiled.evaluate(row);
                            }
                        }
                    }
                    BinaryOperator::Eq => {
                        // SIMD comparison
                        self.evaluate_batch_eq(left, right, rows, output);
                    }
                    _ => {
                        for (i, row) in rows.iter().enumerate() {
                            output[i] = self.compiled.evaluate(row);
                        }
                    }
                }
            }
            _ => {
                for (i, row) in rows.iter().enumerate() {
                    output[i] = self.compiled.evaluate(row);
                }
            }
        }
    }
}
```

---

## 4. Incremental View Maintenance (IVM)

### 4.1 Subscription System

```rust
/// Subscription maintains materialized query results
struct Subscription {
    /// Subscription ID
    id: SubscriptionId,

    /// SQL query text
    query_sql: String,

    /// Compiled query plan
    query_plan: PhysicalPlan,

    /// Materialized results (for delta computation)
    materialized: MaterializedView,

    /// Clients subscribed to this query
    clients: HashSet<ClientId>,
}

struct MaterializedView {
    /// Current results (row_id -> row)
    rows: HashMap<RowId, Row>,

    /// Indexes for efficient delta computation
    indexes: Vec<Index>,
}

impl Subscription {
    /// Compute delta when table changes
    fn compute_delta(&mut self, table_change: &TableChange) -> ViewDelta {
        match table_change {
            TableChange::Insert { row } => {
                if self.query_plan.matches(row) {
                    ViewDelta::Insert { row: row.clone() }
                } else {
                    ViewDelta::NoChange
                }
            }

            TableChange::Delete { row_id, old_row } => {
                if self.materialized.rows.contains_key(row_id) {
                    ViewDelta::Delete { row_id: *row_id }
                } else {
                    ViewDelta::NoChange
                }
            }

            TableChange::Update { row_id, old_row, new_row } => {
                let was_matching = self.query_plan.matches(old_row);
                let is_matching = self.query_plan.matches(new_row);

                match (was_matching, is_matching) {
                    (true, true) => ViewDelta::Update { row_id: *row_id, new_row: new_row.clone() },
                    (true, false) => ViewDelta::Delete { row_id: *row_id },
                    (false, true) => ViewDelta::Insert { row: new_row.clone() },
                    (false, false) => ViewDelta::NoChange,
                }
            }
        }
    }
}
```

### 4.2 Delta Propagation

```rust
/// Delta sent to clients
struct ViewDelta {
    /// Rows inserted
    inserts: Vec<Row>,

    /// Rows deleted
    deletes: Vec<RowId>,

    /// Rows updated (old_id -> new_row)
    updates: HashMap<RowId, Row>,
}

impl SubscriptionIndex {
    /// Process table change and propagate to all affected subscriptions
    fn propagate_change(&mut self, table_id: TableId, change: &TableChange) {
        // Find all subscriptions affected by this table
        let affected = self.find_affected_subscriptions(table_id);

        for sub_id in affected {
            let subscription = &mut self.subscriptions[sub_id];

            // Compute delta for this subscription
            let delta = subscription.compute_delta(change);

            if !delta.is_empty() {
                // Update materialized view
                subscription.materialized.apply_delta(&delta);

                // Send delta to all clients
                for client_id in &subscription.clients {
                    self.send_delta(*client_id, &delta);
                }
            }
        }
    }
}
```

### 4.3 Join Maintenance

```rust
/// Incremental join maintenance
struct IncrementalJoin {
    /// Left input materialized
    left_state: HashMap<JoinKey, Vec<RowId>>,

    /// Right input materialized
    right_state: HashMap<JoinKey, Vec<RowId>>,

    /// Current join results
    results: HashSet<(RowId, RowId)>,
}

impl IncrementalJoin {
    /// Handle left input insert
    fn on_left_insert(&mut self, row: &Row) -> Vec<(RowId, RowId)> {
        let key = self.extract_key(row);
        let mut new_matches = Vec::new();

        // Find matching right rows
        if let Some(right_rows) = self.right_state.get(&key) {
            for &right_id in right_rows {
                new_matches.push((row.id(), right_id));
            }
        }

        // Update state
        self.left_state.entry(key).or_default().push(row.id());

        new_matches
    }

    /// Handle right input insert
    fn on_right_insert(&mut self, row: &Row) -> Vec<(RowId, RowId)> {
        let key = self.extract_key(row);
        let mut new_matches = Vec::new();

        // Find matching left rows
        if let Some(left_rows) = self.left_state.get(&key) {
            for &left_id in left_rows {
                new_matches.push((left_id, row.id()));
            }
        }

        // Update state
        self.right_state.entry(key).or_default().push(row.id());

        new_matches
    }
}
```

---

## 5. Join Algorithms

### 5.1 Hash Join

```rust
/// Hash join implementation
struct HashJoinExec {
    /// Build side (smaller input)
    build_side: PhysicalPlan,

    /// Probe side
    probe_side: PhysicalPlan,

    /// Join keys
    build_key: ColumnId,
    probe_key: ColumnId,

    /// Join type
    join_type: JoinType,
}

impl HashJoinExec {
    fn execute(&self) -> Result<Vec<Row>> {
        // Phase 1: Build hash table from build side
        let build_rows = self.build_side.execute()?;
        let mut hash_table = HashMap::new();

        for row in build_rows {
            let key = row.get(self.build_key);
            hash_table.entry(key).or_insert(Vec::new()).push(row);
        }

        // Phase 2: Probe with probe side
        let probe_rows = self.probe_side.execute()?;
        let mut results = Vec::new();

        for probe_row in probe_rows {
            let key = probe_row.get(self.probe_key);

            if let Some(build_rows) = hash_table.get(&key) {
                for build_row in build_rows {
                    results.push(self.combine_rows(build_row, probe_row));
                }
            } else if self.join_type == JoinType::Left {
                // Left join: keep probe row with NULLs
                results.push(self.combine_with_nulls(probe_row));
            }
        }

        Ok(results)
    }
}
```

### 5.2 Sort-Merge Join

```rust
/// Sort-merge join for pre-sorted inputs
struct SortMergeJoinExec {
    left: PhysicalPlan,
    right: PhysicalPlan,
    left_key: ColumnId,
    right_key: ColumnId,
}

impl SortMergeJoinExec {
    fn execute(&self) -> Result<Vec<Row>> {
        let mut left_iter = self.left.execute()?.into_iter();
        let mut right_iter = self.right.execute()?.into_iter();

        let mut results = Vec::new();

        let mut left_row = left_iter.next();
        let mut right_row = right_iter.next();

        while let (Some(l), Some(r)) = (&left_row, &right_row) {
            let l_key = l.get(self.left_key);
            let r_key = r.get(self.right_key);

            match l_key.cmp(&r_key) {
                Ordering::Less => {
                    left_row = left_iter.next();
                }
                Ordering::Greater => {
                    right_row = right_iter.next();
                }
                Ordering::Equal => {
                    // Handle duplicate keys
                    let l_key = l_key.clone();
                    let mut left_matches = vec![l.clone()];
                    let mut right_matches = vec![r.clone()];

                    // Collect all matching left rows
                    for l in left_iter.by_ref() {
                        if l.get(self.left_key) == l_key {
                            left_matches.push(l);
                        } else {
                            left_row = Some(l);
                            break;
                        }
                    }

                    // Collect all matching right rows
                    for r in right_iter.by_ref() {
                        if r.get(self.right_key) == l_key {
                            right_matches.push(r);
                        } else {
                            right_row = Some(r);
                            break;
                        }
                    }

                    // Cross product of matches
                    for l in &left_matches {
                        for r in &right_matches {
                            results.push(self.combine_rows(l, r));
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}
```

---

## 6. Query Execution Statistics

```rust
/// Runtime statistics for query
struct ExecutionStats {
    /// Rows produced
    rows_produced: u64,

    /// Time spent (microseconds)
    elapsed_us: u64,

    /// Memory used (bytes)
    memory_bytes: usize,

    /// Per-operator stats
    operator_stats: Vec<OperatorStats>,
}

struct OperatorStats {
    /// Operator name
    name: String,

    /// Rows input
    rows_in: u64,

    /// Rows output
    rows_out: u64,

    /// Time spent
    elapsed_us: u64,
}

impl PhysicalPlan {
    /// Execute with statistics
    fn execute_with_stats(&self) -> Result<(Vec<Row>, ExecutionStats)> {
        let start = std::time::Instant::now();

        let rows = self.execute()?;

        Ok((rows, ExecutionStats {
            rows_produced: rows.len() as u64,
            elapsed_us: start.elapsed().as_micros() as u64,
            memory_bytes: 0,  // Would track during execution
            operator_stats: vec![],
        }))
    }
}
```

---

## Document History

| Date | Change |
|------|--------|
| 2026-03-27 | Initial query execution deep dive created |

---

*This exploration is a living document. Revisit sections as concepts become clearer through implementation.*
