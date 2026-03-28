# SurrealDB: Query Execution Deep Dive

## Overview

This document explores SurrealDB's query execution:
- SurrealQL parsing
- Query planning
- Graph traversals
- Document operations
- Function evaluation

---

## 1. SurrealQL Parser

### Parser Architecture

```rust
use nom::{IResult, bytes::complete::tag, character::complete::alpha1};

/// SurrealQL statement
pub enum Statement {
    Select(SelectStatement),
    Create(CreateStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Insert(InsertStatement),
    Define(DefineStatement),
    Info(InfoStatement),
}

/// SELECT statement
pub struct SelectStatement {
    pub expr: Vec<Expr>,
    pub from: FromClause,
    pub where_clause: Option<Expr>,
    pub order: Vec<OrderBy>,
    pub limit: Option<usize>,
    pub start: Option<usize>,
}

/// FROM clause (supports graph traversals)
pub enum FromClause {
    /// Simple table
    Table(String),

    /// Graph traversal
    Graph(GraphTraversal),

    /// Subquery
    Subquery(Box<SelectStatement>),
}

/// Graph traversal
pub struct GraphTraversal {
    pub start: Expr,
    pub direction: Direction,
    pub edges: Vec<EdgeMatcher>,
}

#[derive(Clone, Copy)]
pub enum Direction {
    In,
    Out,
    Both,
}
```

### Parsing Examples

```rust
/// Parse SELECT statement
fn parse_select(input: &str) -> IResult<&str, Statement> {
    let (input, _) = tag("SELECT")(input)?;
    let (input, _) = multispace1(input)?;

    // Parse expressions
    let (input, expr) = parse_expr_list(input)?;
    let (input, _) = multispace1(input)?;

    // Parse FROM
    let (input, _) = tag("FROM")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, from) = parse_from_clause(input)?;

    // Optional WHERE
    let (input, where_clause) = parse_where(input)?;

    // Optional ORDER BY, LIMIT, etc.
    let (input, order) = parse_order_by(input)?;
    let (input, limit) = parse_limit(input)?;

    Ok((input, Statement::Select(SelectStatement {
        expr,
        from,
        where_clause,
        order,
        limit,
        start: None,
    })))
}

/// Parse graph traversal: ->edge->table
fn parse_graph_traversal(input: &str) -> IResult<&str, GraphTraversal> {
    let (input, start) = parse_expr(input)?;

    let mut edges = Vec::new();
    let mut direction = Direction::Out;
    let mut remaining = input;

    while !remaining.is_empty() {
        // Parse direction: -> or <-
        if let Ok((rest, _)) = tag::<_, _, nom::error::Error<&str>>("->")(remaining) {
            direction = Direction::Out;
            remaining = rest;
        } else if let Ok((rest, _)) = tag("<-")(remaining) {
            direction = Direction::In;
            remaining = rest;
        } else {
            break;
        }

        // Parse edge/table name
        let (rest, name) = alpha1(remaining)?;
        edges.push(EdgeMatcher {
            name: name.to_string(),
            direction,
        });
        remaining = rest;
    }

    Ok((remaining, GraphTraversal { start, direction, edges }))
}
```

---

## 2. Query Planning

### Logical Plan

```rust
/// Logical query plan
pub enum LogicalPlan {
    /// Table scan
    TableScan {
        table: String,
        projection: Vec<String>,
        filter: Option<Expr>,
    },

    /// Index scan
    IndexScan {
        table: String,
        index: String,
        bounds: RangeBounds,
        projection: Vec<String>,
    },

    /// Graph traversal
    GraphTraversal {
        start: RecordId,
        edges: Vec<EdgeStep>,
        filter: Option<Expr>,
    },

    /// Filter
    Filter {
        predicate: Expr,
        input: Box<LogicalPlan>,
    },

    /// Projection
    Project {
        expressions: Vec<Expr>,
        input: Box<LogicalPlan>,
    },

    /// Sort
    Sort {
        order_by: Vec<OrderBy>,
        input: Box<LogicalPlan>,
    },

    /// Limit
    Limit {
        limit: usize,
        offset: usize,
        input: Box<LogicalPlan>,
    },

    /// Join
    Join {
        left: Box<LogicalPlan>,
        right: Box<LogicalPlan>,
        condition: JoinCondition,
    },
}
```

### Query Planner

```rust
pub struct QueryPlanner {
    indexes: IndexCatalog,
    stats: TableStatistics,
}

impl QueryPlanner {
    /// Create logical plan from AST
    pub fn plan(&self, stmt: SelectStatement) -> Result<LogicalPlan> {
        // Plan FROM clause
        let mut plan = self.plan_from(&stmt.from)?;

        // Apply WHERE filter
        if let Some(where_clause) = &stmt.where_clause {
            plan = LogicalPlan::Filter {
                predicate: where_clause.clone(),
                input: Box::new(plan),
            };
        }

        // Apply projection
        plan = LogicalPlan::Project {
            expressions: stmt.expr,
            input: Box::new(plan),
        };

        // Apply ORDER BY
        if !stmt.order.is_empty() {
            plan = LogicalPlan::Sort {
                order_by: stmt.order,
                input: Box::new(plan),
            };
        }

        // Apply LIMIT
        if let Some(limit) = stmt.limit {
            plan = LogicalPlan::Limit {
                limit,
                offset: stmt.start.unwrap_or(0),
                input: Box::new(plan),
            };
        }

        Ok(plan)
    }

    /// Plan FROM clause
    fn plan_from(&self, from: &FromClause) -> Result<LogicalPlan> {
        match from {
            FromClause::Table(name) => {
                // Check if we can use an index
                if let Some(index) = self.find_suitable_index(name) {
                    Ok(LogicalPlan::IndexScan {
                        table: name.clone(),
                        index: index.name,
                        bounds: index.bounds,
                        projection: vec![],
                    })
                } else {
                    Ok(LogicalPlan::TableScan {
                        table: name.clone(),
                        projection: vec![],
                        filter: None,
                    })
                }
            }

            FromClause::Graph(traversal) => {
                Ok(LogicalPlan::GraphTraversal {
                    start: self.resolve_start(&traversal.start)?,
                    edges: traversal.edges.clone(),
                    filter: None,
                })
            }

            FromClause::Subquery(subquery) => {
                self.plan(*subquery.clone())
            }
        }
    }
}
```

---

## 3. Graph Traversal Execution

### Traversal Executor

```rust
impl Executor for GraphTraversal {
    async fn execute(&self, ctx: &ExecutionContext) -> Result<Vec<Record>> {
        let mut current = vec![self.start.clone()];
        let mut visited = HashSet::new();

        // Process each edge step
        for edge_step in &self.edges {
            let mut next = Vec::new();

            for rid in current {
                if visited.contains(&rid) {
                    continue;
                }
                visited.insert(rid.clone());

                // Get edges for this record
                let edges = match edge_step.direction {
                    Direction::Out => {
                        ctx.get_outgoing_edges(&rid, &edge_step.edge_type).await?
                    }
                    Direction::In => {
                        ctx.get_incoming_edges(&rid, &edge_step.edge_type).await?
                    }
                    Direction::Both => {
                        let mut edges = ctx.get_outgoing_edges(&rid, &edge_step.edge_type).await?;
                        edges.extend(ctx.get_incoming_edges(&rid, &edge_step.edge_type).await?);
                        edges
                    }
                };

                // Collect target records
                for edge in edges {
                    let target = match edge_step.direction {
                        Direction::Out => edge.to,
                        Direction::In => edge.from,
                        Direction::Both => {
                            if edge.from == rid { edge.to } else { edge.from }
                        }
                    };
                    next.push(target);
                }
            }

            current = next;
        }

        // Load final records
        let mut results = Vec::new();
        for rid in current {
            if let Some(record) = ctx.get_record(&rid).await? {
                results.push(record);
            }
        }

        Ok(results)
    }
}
```

---

## 4. Function Evaluation

### Built-in Functions

```rust
/// Built-in function registry
pub struct FunctionRegistry {
    functions: HashMap<String, FunctionImpl>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };

        // Register built-in functions
        registry.register("array::len", array_len);
        registry.register("string::length", string_length);
        registry.register("math::sum", math_sum);
        registry.register("time::now", time_now);
        registry.register("geo::distance", geo_distance);

        registry
    }

    pub fn call(&self, name: &str, args: Vec<Value>) -> Result<Value> {
        let func = self.functions.get(name)
            .ok_or_else(|| Error::FunctionNotFound(name.to_string()))?;

        func(args)
    }
}

// Function implementations
fn array_len(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::Array(arr) => Ok(Value::Int(arr.len() as i64)),
        _ => Err(Error::TypeMismatch),
    }
}

fn string_length(args: Vec<Value>) -> Result<Value> {
    match &args[0] {
        Value::String(s) => Ok(Value::Int(s.len() as i64)),
        _ => Err(Error::TypeMismatch),
    }
}

fn geo_distance(args: Vec<Value>) -> Result<Value> {
    let point1 = args[0].as_point()?;
    let point2 = args[1].as_point()?;

    let distance = haversine_distance(point1, point2);
    Ok(Value::Float(distance))
}
```

### User-Defined Functions

```sql
-- Define function
DEFINE FUNCTION fn::greet($name: string) {
    RETURN "Hello, " + $name + "!";
};

-- Use function
SELECT fn::greet(name) FROM user;
```

```rust
/// User-defined function
pub struct UserFunction {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Expr,
}

impl UserFunction {
    pub fn call(&self, args: Vec<Value>, ctx: &Context) -> Result<Value> {
        // Create local scope with parameters
        let mut scope = ctx.scope.clone();

        for (param, arg) in self.params.iter().zip(args) {
            scope.insert(param.name.clone(), arg);
        }

        // Evaluate body in new scope
        self.body.evaluate(&Context { scope, ..ctx.clone() })
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
