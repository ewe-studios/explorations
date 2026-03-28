---
title: "Query Execution Deep Dive: Turso/libSQL"
subtitle: "SQLite VM bytecode, query planning, and embedded replica execution"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.turso
explored_at: 2026-03-28
related: 00-zero-to-db-engineer.md, 01-storage-engine-deep-dive.md
---

# 02 - Query Execution Deep Dive

## Overview

This document explains how SQLite executes queries internally - from SQL text to bytecode to results. We'll cover the VDBE (Virtual Database Engine), query planning, and how libSQL extends this for embedded replicas.

## Part 1: SQL Compilation Pipeline

### From SQL to Bytecode

```
┌─────────────────────────────────────────────────────────────────┐
│                    SQL Text                                      │
│              "SELECT name FROM users WHERE id = ?"               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 1. Tokenizer                                                     │
│    Splits into tokens: SELECT, name, FROM, users, WHERE, id, =, ?│
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. Parser                                                        │
│    Builds AST (Abstract Syntax Tree)                            │
│                                                                 │
│         SELECT                                                  │
│          │                                                      │
│      ┌───┴───┐                                                  │
│      │       │                                                  │
│    name   WHERE                                                 │
│              │                                                  │
│          ┌───┴───┐                                              │
│          │       │                                              │
│          =      id                                              │
│          │                                                      │
│          ?                                                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. Code Generator                                                │
│    Converts AST to VDBE bytecode                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. VDBE Program                                                  │
│    addr  opcode     P1    P2    P3    P4             P5         │
│    ───────────────────────────────────────────────────────────  │
│    0     Init       0     28    0     null           0          │
│    1     OpenRead   0     2     0     2              0          │
│    2     Rewind     0     27    0     null           0          │
│    3       Column   0     0     1     null           0          │
│    4       IdxGE    1     26    1     binary:69643   0          │
│    ...                                                          │
└─────────────────────────────────────────────────────────────────┘
```

### Tokenizer Details

```rust
#[derive(Debug, PartialEq, Clone)]
enum SqlToken {
    // Keywords
    Select,
    From,
    Where,
    Insert,
    Update,
    Delete,
    Create,
    // Operators
    Eq,        // =
    Ne,        // <> or !=
    Lt,        // <
    Gt,        // >
    Le,        // <=
    Ge,        // >=
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    // Punctuation
    LParen,    // (
    RParen,    // )
    Comma,     // ,
    Semi,      // ;
    // Literals
    Integer(i64),
    Float(f64),
    String(String),
    // Identifiers
    Id(String),
    // Parameter placeholder
    Variable(i32),  // ? or ?NNN or :NAME
}

struct Tokenizer {
    input: Vec<char>,
    pos: usize,
}

impl Tokenizer {
    fn new(sql: &str) -> Self {
        Self {
            input: sql.chars().collect(),
            pos: 0,
        }
    }

    fn tokenize(&mut self) -> Result<Vec<SqlToken>, TokenError> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            self.skip_whitespace();

            if self.pos >= self.input.len() {
                break;
            }

            let ch = self.current_char();

            match ch {
                '(' => tokens.push(SqlToken::LParen),
                ')' => tokens.push(SqlToken::RParen),
                ',' => tokens.push(SqlToken::Comma),
                ';' => tokens.push(SqlToken::Semi),
                '=' => tokens.push(SqlToken::Eq),
                '+' => tokens.push(SqlToken::Plus),
                '-' => {
                    // Could be minus or negative number
                    if self.peek_number() {
                        tokens.push(self.read_number()?);
                    } else {
                        tokens.push(SqlToken::Minus);
                    }
                }
                '*' => tokens.push(SqlToken::Star),
                '/' => tokens.push(SqlToken::Slash),
                '?' => tokens.push(self.read_variable()?),
                ':' | '@' | '$' => tokens.push(self.read_named_variable()?),
                '\'' => tokens.push(self.read_string()?),
                '"' | '`' | '[' => tokens.push(self.read_quoted_id()?),
                '0'..='9' => tokens.push(self.read_number()?),
                'a'..='z' | 'A'..='Z' | '_' => {
                    tokens.push(self.read_keyword_or_id()?)
                }
                '<' | '>' => tokens.push(self.read_comparison_op()?),
                '!' => tokens.push(self.read_not_op()?),
                '|' => tokens.push(self.read_pipe_op()?),
                '&' => tokens.push(self.read_amp()),
                '~' => tokens.push(self.read_tilde()),
                '.' => tokens.push(self.read_dot()?),
                _ => return Err(TokenError::UnexpectedChar(ch)),
            }
        }

        Ok(tokens)
    }

    fn read_keyword_or_id(&mut self) -> Result<SqlToken, TokenError> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' {
                self.pos += 1;
            } else {
                break;
            }
        }

        let word: String = self.input[start..self.pos].iter().collect();
        let upper = word.to_uppercase();

        // Check against keyword table
        let keyword = match upper.as_str() {
            "SELECT" => SqlToken::Select,
            "FROM" => SqlToken::From,
            "WHERE" => SqlToken::Where,
            "INSERT" => SqlToken::Insert,
            "UPDATE" => SqlToken::Update,
            "DELETE" => SqlToken::Delete,
            "CREATE" => SqlToken::Create,
            // ... more keywords
            _ => SqlToken::Id(word),
        };

        Ok(keyword)
    }
}
```

## Part 2: Parser and AST

### AST Node Types

```rust
#[derive(Debug)]
enum AstNode {
    /// SELECT statement
    Select(SelectStmt),

    /// INSERT statement
    Insert(InsertStmt),

    /// UPDATE statement
    Update(UpdateStmt),

    /// DELETE statement
    Delete(DeleteStmt),

    /// CREATE TABLE
    CreateTable(CreateTableStmt),

    /// Expression
    Expr(Box<Expr>),
}

#[derive(Debug)]
struct SelectStmt {
    /// Columns to select
    columns: Vec<ResultColumn>,

    /// FROM clause (tables)
    from: Option<FromClause>,

    /// WHERE clause
    where_clause: Option<Box<Expr>>,

    /// GROUP BY clause
    group_by: Vec<Box<Expr>>,

    /// HAVING clause
    having: Option<Box<Expr>>,

    /// ORDER BY clause
    order_by: Vec<OrderByTerm>,

    /// LIMIT clause
    limit: Option<Box<Expr>>,

    /// OFFSET clause
    offset: Option<Box<Expr>>,
}

#[derive(Debug)]
enum ResultColumn {
    /// SELECT *
    Star,

    /// SELECT table.*
    TableStar(String),

    /// SELECT expr [AS alias]
    Expr {
        expr: Box<Expr>,
        alias: Option<String>,
    },
}

#[derive(Debug)]
enum Expr {
    /// Column reference: name or table.name
    Column {
        table: Option<String>,
        column: String,
    },

    /// Literal value
    Literal(Literal),

    /// Parameter placeholder: ? or :name
    Variable(i32),

    /// Binary operation: left op right
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },

    /// Function call: name(args)
    Function {
        name: String,
        args: Vec<Box<Expr>>,
    },

    /// Subquery: (SELECT ...)
    Subquery(Box<SelectStmt>),

    /// CASE expression
    Case {
        operand: Option<Box<Expr>>,
        when_then: Vec<(Box<Expr>, Box<Expr>)>,
        else_result: Option<Box<Expr>>,
    },

    /// IN expression: x IN (...)
    In {
        expr: Box<Expr>,
        list: Vec<Box<Expr>>,
        negated: bool,
    },

    /// BETWEEN: x BETWEEN a AND b
    Between {
        expr: Box<Expr>,
        start: Box<Expr>,
        end: Box<Expr>,
        negated: bool,
    },

    /// LIKE: x LIKE pattern
    Like {
        expr: Box<Expr>,
        pattern: Box<Expr>,
        escape: Option<Box<Expr>>,
        negated: bool,
    },
}

#[derive(Debug)]
enum BinaryOp {
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    Is,
    IsNot,
}

#[derive(Debug)]
enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Blob(Vec<u8>),
    Null,
}
```

### Parser Implementation (Simplified)

```rust
struct Parser {
    tokens: Vec<SqlToken>,
    pos: usize,
}

impl Parser {
    fn parse(&mut self) -> Result<AstNode, ParseError> {
        match self.peek() {
            Some(SqlToken::Select) => self.parse_select(),
            Some(SqlToken::Insert) => self.parse_insert(),
            Some(SqlToken::Update) => self.parse_update(),
            Some(SqlToken::Delete) => self.parse_delete(),
            Some(SqlToken::Create) => self.parse_create(),
            _ => Err(ParseError::UnexpectedToken(self.peek().cloned())),
        }
    }

    fn parse_select(&mut self) -> Result<AstNode, ParseError> {
        self.expect(SqlToken::Select)?;

        // Parse columns
        let columns = self.parse_result_columns()?;

        // Optional FROM clause
        let from = if self.peek() == Some(&SqlToken::From) {
            self.advance();
            Some(self.parse_from_clause()?)
        } else {
            None
        };

        // Optional WHERE clause
        let where_clause = if self.peek() == Some(&SqlToken::Where) {
            self.advance();
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };

        // Parse ORDER BY, GROUP BY, LIMIT, etc.
        // ... (more parsing logic)

        Ok(AstNode::Select(SelectStmt {
            columns,
            from,
            where_clause,
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
        }))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or_expression()
    }

    fn parse_or_expression(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and_expression()?;

        while self.peek() == Some(&SqlToken::And) {
            self.advance();
            let right = self.parse_and_expression()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality_expression()?;

        while self.peek() == Some(&SqlToken::And) {
            self.advance();
            let right = self.parse_equality_expression()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_equality_expression(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_relational_expression()?;

        if let Some(op) = self.peek_equality_op() {
            self.advance();
            let right = self.parse_relational_expression()?;
            return Ok(Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().cloned() {
            Some(SqlToken::Integer(n)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Integer(n)))
            }
            Some(SqlToken::Float(n)) => {
                self.advance();
                Ok(Expr::Literal(Literal::Float(n)))
            }
            Some(SqlToken::String(s)) => {
                self.advance();
                Ok(Expr::Literal(Literal::String(s)))
            }
            Some(SqlToken::Variable(n)) => {
                self.advance();
                Ok(Expr::Variable(n))
            }
            Some(SqlToken::Id(name)) => {
                self.advance();
                // Check if it's a function call
                if self.peek() == Some(&SqlToken::LParen) {
                    self.advance();
                    let args = self.parse_expr_list()?;
                    self.expect(SqlToken::RParen)?;
                    return Ok(Expr::Function { name, args });
                }
                Ok(Expr::Column {
                    table: None,
                    column: name,
                })
            }
            Some(SqlToken::LParen) => {
                self.advance();
                if self.peek() == Some(&SqlToken::Select) {
                    // Subquery
                    let subquery = self.parse_select()?;
                    self.expect(SqlToken::RParen)?;
                    return Ok(Expr::Subquery(Box::new(subquery)));
                }
                // Parenthesized expression
                let expr = self.parse_expr()?;
                self.expect(SqlToken::RParen)?;
                Ok(expr)
            }
            Some(SqlToken::Star) => {
                self.advance();
                Ok(Expr::Literal(Literal::Integer(0))) // Handle specially
            }
            _ => Err(ParseError::UnexpectedToken(self.peek().cloned())),
        }
    }
}
```

## Part 3: VDBE Bytecode Instructions

### Key Opcodes

```rust
/// VDBE Opcodes (simplified subset)
#[derive(Debug, Clone)]
enum VdbeOpcode {
    // Program Control
    Init { addr: usize },           // Jump to P2
    Halt { err_code: i32 },         // Stop execution
    Goto { addr: usize },           // Unconditional jump
    If { cond_reg: usize, addr: usize },  // Jump if P1 is true
    IfNot { cond_reg: usize, addr: usize },  // Jump if P1 is false

    // Cursor Operations
    OpenRead { cursor: usize, root_page: usize },  // Open read cursor
    OpenWrite { cursor: usize, root_page: usize },  // Open write cursor
    Close { cursor: usize },         // Close cursor
    Rewind { cursor: usize, addr: usize },  // Rewind cursor, jump if empty
    Next { cursor: usize, addr: usize },  // Advance cursor, jump if not EOF

    // Data Access
    Column { cursor: usize, col: usize, dest: usize },  // Read column into P3
    Rowid { cursor: usize, dest: usize },  // Read rowid into P2
    Seek { cursor: usize, key_reg: usize, addr: usize },  // Seek to key
    Insert { cursor: usize, key_reg: usize, data_reg: usize },  // Insert row
    Delete { cursor: usize },        // Delete current row

    // Register Operations
    Null { dest: usize },            // Set P1 to NULL
    Integer { value: i64, dest: usize },  // Store integer in P2
    Real { value: f64, dest: usize },  // Store float in P2
    String { value: String, dest: usize },  // Store string in P2
    Copy { src: usize, dest: usize },  // Copy P1 to P2
    Move { src: usize, dest: usize, count: usize },  // Move P3 registers

    // Arithmetic
    Add { lhs: usize, rhs: usize, dest: usize },
    Subtract { lhs: usize, rhs: usize, dest: usize },
    Multiply { lhs: usize, rhs: usize, dest: usize },
    Divide { lhs: usize, rhs: usize, dest: usize },

    // Comparison
    Eq { lhs: usize, rhs: usize, dest: usize },
    Ne { lhs: usize, rhs: usize, dest: usize },
    Lt { lhs: usize, rhs: usize, dest: usize },
    Gt { lhs: usize, rhs: usize, dest: usize },
    Le { lhs: usize, rhs: usize, dest: usize },
    Ge { lhs: usize, rhs: usize, dest: usize },

    // Jump on Comparison
    IdxGE { cursor: usize, addr: usize, key_reg: usize },  // Jump if key >= cursor key
    SeekGT { cursor: usize, addr: usize, key_reg: usize },  // Seek > key

    // Aggregate Functions
    AggStep { func: AggFunc, arg_reg: usize, accum_reg: usize },
    AggFinal { accum_reg: usize, dest: usize },

    // Sorting
    SortOpen { cursor: usize, columns: usize },  // Open sorter
    SortInsert { cursor: usize, record_reg: usize },  // Insert into sorter
    SortSort { cursor: usize, addr: usize },  // Sort and iterate
    SortData { cursor: usize, col: usize, dest: usize },  // Get sorted column

    // Subquery
    Gosub { addr: usize, link_reg: usize },  // Call subroutine
    Return { link_reg: usize },        // Return from subroutine

    // Misc
    MakeRecord { start_reg: usize, count: usize, dest: usize },  // Create record
    ResultRow { start_reg: usize, count: usize },  // Yield result row
    Explain { msg: String },         // For EXPLAIN query plan
}
```

### Example: Compiling SELECT

```sql
SELECT name FROM users WHERE id = 42
```

Generates this bytecode:

```rust
let program = vec![
    // Initialize
    (0, Init { addr: 9 }),

    // Open cursor on users table (root page = 2)
    (1, OpenRead { cursor: 0, root_page: 2 }),

    // Rewind cursor - jump to halt if empty
    (2, Rewind { cursor: 0, addr: 8 }),

    // Loop body:
    //   Read id column (column 0) into register 1
    (3, Column { cursor: 0, col: 0, dest: 1 }),

    //   Compare id != 42, jump to Next if not equal
    (4, Integer { value: 42, dest: 2 }),
    (5, Ne { lhs: 1, rhs: 2, dest: 3 }),
    (6, If { cond_reg: 3, addr: 7 }),

    //   Read name column into result register
    (7, Column { cursor: 0, col: 1, dest: 4 }),
    (8, ResultRow { start_reg: 4, count: 1 }),

    // Advance to next row
    (9, Next { cursor: 0, addr: 3 }),

    // Halt
    (10, Halt { err_code: 0 }),
];
```

### Bytecode Execution Loop

```rust
struct Vdbe {
    /// Program instructions
    program: Vec<(usize, VdbeOpcode)>,

    /// Program counter
    pc: usize,

    /// Register file (values)
    registers: Vec<Value>,

    /// Open cursors
    cursors: HashMap<usize, Cursor>,

    /// Result rows
    result: Vec<Vec<Value>>,

    /// Halt flag
    halted: bool,

    /// Error state
    error: Option<String>,
}

impl Vdbe {
    fn execute(&mut self) -> Result<Vec<Vec<Value>>, String> {
        while !self.halted {
            let (addr, opcode) = self.program[self.pc].clone();

            match opcode {
                VdbeOpcode::Init { addr } => {
                    self.pc = addr;
                }

                VdbeOpcode::Halt { err_code } => {
                    self.halted = true;
                    if err_code != 0 {
                        return Err(format!("SQL error: {}", err_code));
                    }
                }

                VdbeOpcode::Goto { addr } => {
                    self.pc = addr;
                }

                VdbeOpcode::OpenRead { cursor, root_page } => {
                    let table = self.open_table(root_page)?;
                    self.cursors.insert(cursor, Cursor::Read(table));
                    self.pc += 1;
                }

                VdbeOpcode::Rewind { cursor, addr } => {
                    if let Some(Cursor::Read(ref mut table)) = self.cursors.get_mut(&cursor) {
                        if table.rewind() {
                            self.pc += 1;
                        } else {
                            self.pc = addr;  // Empty table, jump
                        }
                    }
                }

                VdbeOpcode::Next { cursor, addr } => {
                    if let Some(Cursor::Read(ref mut table)) = self.cursors.get_mut(&cursor) {
                        if table.next() {
                            self.pc += 1;
                        } else {
                            self.pc = addr;  // EOF, jump
                        }
                    }
                }

                VdbeOpcode::Column { cursor, col, dest } => {
                    if let Some(Cursor::Read(ref table)) = self.cursors.get(&cursor) {
                        let value = table.get_column(col).clone();
                        self.registers[dest] = value;
                    }
                    self.pc += 1;
                }

                VdbeOpcode::Integer { value, dest } => {
                    self.registers[dest] = Value::Integer(value);
                    self.pc += 1;
                }

                VdbeOpcode::Ne { lhs, rhs, dest } => {
                    let result = self.registers[lhs] != self.registers[rhs];
                    self.registers[dest] = Value::Boolean(result);
                    self.pc += 1;
                }

                VdbeOpcode::If { cond_reg, addr } => {
                    if let Value::Boolean(true) = self.registers[cond_reg] {
                        self.pc = addr;
                    } else {
                        self.pc += 1;
                    }
                }

                VdbeOpcode::ResultRow { start_reg, count } => {
                    let row: Vec<Value> = (start_reg..start_reg + count)
                        .map(|i| self.registers[i].clone())
                        .collect();
                    self.result.push(row);
                    self.pc += 1;
                }

                _ => {
                    // Handle other opcodes...
                    self.pc += 1;
                }
            }
        }

        Ok(self.result.clone())
    }
}
```

## Part 4: Query Planning

### What is Query Planning?

Query planning determines **how** to execute a query efficiently:

```sql
SELECT * FROM users u
JOIN orders o ON u.id = o.user_id
WHERE u.country = 'US'
ORDER BY o.date DESC
LIMIT 100;
```

Questions the planner must answer:
1. Which index to use for `u.country = 'US'`?
2. How to join `users` and `orders`? (nested loop, hash join, merge join)
3. Which table to scan first?
4. When to apply the ORDER BY? (before or after join)
5. When to apply the LIMIT?

### Plan Generation

```rust
#[derive(Debug)]
enum QueryPlan {
    /// Full table scan
    Scan {
        table: String,
        filter: Option<Expr>,
    },

    /// Index lookup
    IndexSeek {
        table: String,
        index: String,
        key: Box<Expr>,
    },

    /// Index range scan
    IndexRange {
        table: String,
        index: String,
        range: Range<Expr>,
    },

    /// Nested loop join
    NestedLoop {
        left: Box<QueryPlan>,
        right: Box<QueryPlan>,
        condition: Option<Expr>,
    },

    /// Hash join
    HashJoin {
        left: Box<QueryPlan>,
        right: Box<QueryPlan>,
        left_key: Box<Expr>,
        right_key: Box<Expr>,
    },

    /// Sort operation
    Sort {
        input: Box<QueryPlan>,
        keys: Vec<OrderByTerm>,
    },

    /// Limit operation
    Limit {
        input: Box<QueryPlan>,
        limit: usize,
        offset: usize,
    },
}

impl QueryPlanner {
    fn plan(&self, query: &SelectStmt) -> Result<QueryPlan, PlanError> {
        // Start with base tables
        let mut plan = self.plan_from_clause(&query.from)?;

        // Add WHERE filter
        if let Some(where_clause) = &query.where_clause {
            plan = self.add_filter(plan, where_clause)?;
        }

        // Add JOINs
        // ... (join planning logic)

        // Add ORDER BY
        if !query.order_by.is_empty() {
            plan = QueryPlan::Sort {
                input: Box::new(plan),
                keys: query.order_by.clone(),
            };
        }

        // Add LIMIT
        if let Some(limit) = &query.limit {
            plan = QueryPlan::Limit {
                input: Box::new(plan),
                limit: self.eval_limit(limit)?,
                offset: query.offset.as_ref().map(|o| self.eval_limit(o)).unwrap_or(0),
            };
        }

        Ok(plan)
    }

    fn add_filter(&self, plan: QueryPlan, filter: &Expr) -> Result<QueryPlan, PlanError> {
        match &plan {
            QueryPlan::Scan { table, .. } => {
                // Check if filter can use an index
                if let Some(index) = self.find_index_for_filter(table, filter) {
                    // Convert to index seek
                    Ok(self.plan_index_seek(table, index, filter))
                } else {
                    // Keep as scan with filter
                    Ok(QueryPlan::Scan {
                        table: table.clone(),
                        filter: Some(filter.clone()),
                    })
                }
            }
            _ => {
                // Push filter down or apply on top
                Ok(QueryPlan::Scan {
                    table: "derived".to_string(),
                    filter: Some(filter.clone()),
                })
            }
        }
    }
}
```

### Cost Estimation

```rust
#[derive(Debug)]
struct PlanCost {
    /// Estimated rows to scan
    rows: f64,

    /// Estimated I/O operations
    io_ops: f64,

    /// Estimated CPU operations
    cpu_ops: f64,
}

impl QueryPlanner {
    fn estimate_cost(&self, plan: &QueryPlan, stats: &TableStats) -> PlanCost {
        match plan {
            QueryPlan::Scan { table, filter } => {
                let rows = stats.row_count(table);
                let selectivity = filter.map(|f| self.estimate_selectivity(f)).unwrap_or(1.0);

                PlanCost {
                    rows: rows,
                    io_ops: rows / stats.rows_per_page(table),
                    cpu_ops: rows,
                }
            }

            QueryPlan::IndexSeek { index, .. } => {
                // Index seek: log2(index_size) + 1 page fetch
                let index_pages = stats.index_pages(index);
                let height = (index_pages as f64).log2().ceil() as f64;

                PlanCost {
                    rows: 1.0,
                    io_ops: height + 1.0,  // Index traversal + table lookup
                    cpu_ops: height,
                }
            }

            QueryPlan::NestedLoop { left, right, .. } => {
                let left_cost = self.estimate_cost(left, stats);
                let right_cost = self.estimate_cost(right, stats);

                // Nested loop: for each left row, scan all right rows
                PlanCost {
                    rows: left_cost.rows * right_cost.rows,
                    io_ops: left_cost.io_ops + (left_cost.rows * right_cost.io_ops),
                    cpu_ops: left_cost.cpu_ops + (left_cost.rows * right_cost.cpu_ops),
                }
            }

            _ => PlanCost {
                rows: 0.0,
                io_ops: 0.0,
                cpu_ops: 0.0,
            },
        }
    }

    fn choose_best_plan(&self, alternatives: Vec<QueryPlan>, stats: &TableStats) -> QueryPlan {
        alternatives
            .into_iter()
            .min_by(|a, b| {
                let cost_a = self.estimate_cost(a, stats);
                let cost_b = self.estimate_cost(b, stats);
                cost_a.total().partial_cmp(&cost_b.total()).unwrap()
            })
            .unwrap()
    }
}

impl PlanCost {
    fn total(&self) -> f64 {
        self.io_ops * 10.0 + self.cpu_ops  // I/O is expensive
    }
}
```

## Part 5: libSQL Extensions

### Embedded Replica Query Execution

```rust
/// libSQL execution flow for embedded replicas
enum ExecutionMode {
    /// Execute against local replica
    Local,

    /// Execute against primary (for writes)
    Remote,

    /// Execute locally after syncing
    LocalAfterSync,
}

impl LibSqlDatabase {
    async fn execute(&self, query: &str, params: &[Value]) -> Result<ResultSet, Error> {
        let stmt = self.parse(query)?;
        let plan = self.plan(&stmt)?;

        // Determine execution mode based on query type
        let mode = match &stmt {
            AstNode::Select { .. } => {
                // Reads can use local replica
                if self.replica.is_fresh() {
                    ExecutionMode::Local
                } else {
                    ExecutionMode::LocalAfterSync
                }
            }
            AstNode::Insert { .. }
            | AstNode::Update { .. }
            | AstNode::Delete { .. } => {
                // Writes go to primary
                ExecutionMode::Remote
            }
        };

        match mode {
            ExecutionMode::Local => {
                self.execute_local(&plan, params)
            }
            ExecutionMode::LocalAfterSync => {
                self.sync().await?;
                self.execute_local(&plan, params)
            }
            ExecutionMode::Remote => {
                self.execute_remote(query, params).await
            }
        }
    }

    fn execute_local(&self, plan: &QueryPlan, params: &[Value]) -> Result<ResultSet, Error> {
        // Use local SQLite with VDBE
        let mut vdbe = Vdbe::new(&self.local_db);
        vdbe.bind_params(params)?;
        let result = vdbe.execute()?;
        Ok(ResultSet::from_vdbe_result(result))
    }

    async fn execute_remote(&self, query: &str, params: &[Value]) -> Result<ResultSet, Error> {
        // Send to primary via HTTP
        let response = self.http_client
            .post(&self.primary_url)
            .json(&ExecuteRequest {
                query: query.to_string(),
                params: params.to_vec(),
            })
            .send()
            .await?;

        let result: ExecuteResponse = response.json().await?;
        Ok(ResultSet::from_remote_result(result))
    }
}
```

### Sync-Aware Query Planning

```rust
impl QueryPlanner {
    /// For embedded replicas, consider sync status in planning
    fn plan_with_sync_awareness(
        &self,
        query: &SelectStmt,
        replica_state: &ReplicaState,
    ) -> Result<QueryPlan, PlanError> {
        match query {
            // Simple select - can always use local
            SelectStmt {
                from,
                where_clause: None,
                limit: Some(limit),
                ..
            } if self.is_simple_select(query) => {
                Ok(QueryPlan::Scan {
                    table: self.get_table_name(from),
                    filter: None,
                })
            }

            // Select with filters that might be stale
            SelectStmt { where_clause: Some(filter), .. } => {
                // Check if filter involves recently-modified data
                if self.involves_hot_data(filter, replica_state) {
                    // Force sync before query
                    Ok(QueryPlan::Sync {
                        input: Box::new(self.plan(query)?),
                    })
                } else {
                    // Safe to query locally
                    self.plan(query)
                }
            }

            _ => self.plan(query),
        }
    }

    fn involves_hot_data(&self, filter: &Expr, state: &ReplicaState) -> bool {
        // Check if filter involves tables/rows modified in recent sync
        match filter {
            Expr::Column { table, column } => {
                state.recently_modified_tables.contains(table)
            }
            _ => false,
        }
    }
}
```

## Part 6: Optimization Techniques

### Prepared Statements

```rust
/// Prepared statement = compiled VDBE program that can be reused
struct PreparedStatement {
    /// Compiled bytecode
    program: Vec<VdbeOpcode>,

    /// Parameter count
    param_count: usize,

    /// Column names in result
    columns: Vec<String>,

    /// Bound parameter values
    params: Vec<Value>,
}

impl Database {
    fn prepare(&self, sql: &str) -> Result<PreparedStatement, Error> {
        // Parse once
        let ast = self.parser.parse(sql)?;

        // Plan once
        let plan = self.planner.plan(&ast)?;

        // Compile to bytecode once
        let program = self.compiler.compile(&plan)?;

        Ok(PreparedStatement {
            program,
            param_count: self.count_parameters(&ast),
            columns: self.get_result_columns(&ast),
            params: vec![],
        })
    }
}

// Usage:
// BAD: Compile every time
for id in ids {
    let stmt = db.prepare("SELECT * FROM users WHERE id = ?")?;
    stmt.execute([id])?;
}

// GOOD: Prepare once, execute many times
let stmt = db.prepare("SELECT * FROM users WHERE id = ?")?;
for id in ids {
    stmt.execute([id])?;
}
```

### Covering Indexes

```sql
-- Query:
SELECT name FROM users WHERE country = 'US';

-- Without covering index:
-- 1. Seek index on country = 'US' → get rowids
-- 2. For each rowid, fetch from users table → get name
-- Total: index lookup + table lookups

-- With covering index:
CREATE INDEX idx_users_country_name ON users(country, name);

-- 1. Seek index on country = 'US' → names are already in index!
-- Total: index lookup only (no table access)
```

### Query Rewriting

```rust
impl QueryOptimizer {
    /// Rewrite queries for better performance
    fn optimize(&self, query: &SelectStmt) -> SelectStmt {
        let mut optimized = query.clone();

        // Constant folding: 1 + 1 → 2
        self.fold_constants(&mut optimized);

        // Predicate pushdown: move filters closer to data source
        self.pushdown_predicates(&mut optimized);

        // Remove redundant operations
        self.eliminate_redundancy(&mut optimized);

        // Decorrelate subqueries where possible
        self.decorrelate_subqueries(&mut optimized);

        optimized
    }

    fn fold_constants(&self, query: &mut SelectStmt) {
        // Find expressions like (5 + 3) and replace with 8
        // Find expressions like (TRUE AND x) and replace with x
    }

    fn pushdown_predicates(&self, query: &mut SelectStmt) {
        // Move WHERE clause conditions as close to tables as possible
        // Especially important for JOINs and subqueries
    }

    fn decorrelate_subqueries(&self, query: &mut SelectStmt) {
        // Convert correlated subquery to JOIN where possible
        // FROM users u WHERE EXISTS (SELECT 1 FROM orders o WHERE o.user_id = u.id)
        // → JOIN with DISTINCT
    }
}
```

---

*This document is part of the Turso/libSQL exploration series. See [exploration.md](./exploration.md) for the complete index.*
