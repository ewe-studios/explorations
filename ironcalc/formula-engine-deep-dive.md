# Formula Engine Deep Dive

## Overview

IronCalc's formula engine is a complete implementation of spreadsheet formula parsing and evaluation. It supports Excel-compatible syntax, 200+ functions, cell references, ranges, and proper error handling with circular reference detection.

## Architecture

```
User Input (=SUM(A1:A10))
       │
       ▼
┌──────────────┐
│    Lexer     │ → Tokens
└──────────────┘
       │
       ▼
┌──────────────┐
│    Parser    │ → AST (Node tree)
└──────────────┘
       │
       ▼
┌──────────────┐
│  Evaluator   │ → CalcResult
└──────────────┘
       │
       ▼
┌──────────────┐
│  Formatter   │ → Display String
└──────────────┘
```

## Lexer (Tokenizer)

### Token Types

```rust
pub enum TokenType {
    // Operators
    Addition(OpSum),        // +, -
    Product(OpProduct),     // *, /
    Power,                  // ^
    Percent,                // %
    And,                    // &
    Compare(OpCompare),     // =, <, >, <=, >=, <>

    // Delimiters
    LeftParenthesis,
    RightParenthesis,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Colon,                  // :
    Comma,
    Semicolon,
    Bang,                   // !

    // Literals
    Number(f64),
    String(String),

    // References
    Reference { /* cell reference */ },
    Range { /* range reference */ },

    // Special
    Function(String),
    Variable(String),
    Error(Error),
    Illegal(LexerError),
}
```

### Lexer Modes

The lexer supports two modes:

1. **A1 Mode (Display Mode)**: User-facing formulas like `=A1+B2`
2. **R1C1 Mode (Internal Mode)**: Runtime representation like `=R[1]C[1]+R[2]C[2]`

```rust
pub enum LexerMode {
    A1,    // User formulas: D4, D$4, F5:T10
    R1C1,  // Internal: R1C1 = $A$1, R[2]C[5] = relative reference
}
```

### Locale-Aware Tokenization

The lexer handles locale-specific syntax:

```rust
// Different argument separators
=IF(A1, B1, NA())   // English locale (comma)
=IF(A1; B1; NA())   // European locale (semicolon)

// Different number formats
1,123.45   // US locale
1.123,45   // German locale
```

### Lexer Implementation

```rust
pub struct Lexer {
    position: usize,
    next_token_position: Option<usize>,
    len: usize,
    chars: Vec<char>,
    mode: LexerMode,
    locale: Locale,
    language: Language,
}

impl Lexer {
    pub fn new(formula: &str, mode: LexerMode, locale: &Locale, language: &Language) -> Lexer {
        let chars: Vec<char> = formula.chars().collect();
        let len = chars.len();
        Lexer {
            chars,
            position: 0,
            next_token_position: None,
            len,
            mode,
            locale: locale.clone(),
            language: language.clone(),
        }
    }

    pub fn next_token(&mut self) -> TokenType {
        self.next_token_position = None;
        self.consume_whitespace();

        match self.read_next_char() {
            Some(char) => {
                match char {
                    '+' => TokenType::Addition(OpSum::Add),
                    '-' => TokenType::Addition(OpSum::Minus),
                    '*' => TokenType::Product(OpProduct::Times),
                    '/' => TokenType::Product(OpProduct::Divide),
                    '(' => TokenType::LeftParenthesis,
                    ')' => TokenType::RightParenthesis,
                    '=' => TokenType::Compare(OpCompare::Equal),
                    '^' => TokenType::Power,
                    '%' => TokenType::Percent,
                    '&' => TokenType::And,
                    ':' => TokenType::Colon,
                    ',' => {
                        // Locale-aware: comma or decimal separator
                        if self.locale.numbers.symbols.decimal == "," {
                            match self.consume_number(',') {
                                Ok(number) => TokenType::Number(number),
                                Err(error) => TokenType::Illegal(error),
                            }
                        } else {
                            TokenType::Comma
                        }
                    }
                    '"' => TokenType::String(self.consume_string()),
                    '\'' => self.consume_quoted_sheet_reference(),
                    '0'..='9' => {
                        match self.consume_number(char) {
                            Ok(number) => TokenType::Number(number),
                            Err(error) => TokenType::Illegal(error),
                        }
                    }
                    _ => {
                        if char.is_alphabetic() || char == '_' {
                            self.consume_identifier()
                            // Could be: function name, defined name, or reference
                        } else {
                            // Handle other cases
                        }
                    }
                }
            }
            None => TokenType::EndOfFile,
        }
    }
}
```

## Parser

### Grammar

```
expr    => concat (opComp concat)*
concat  => term ('&' term)*
term    => factor (opFactor factor)*
factor  => prod (opProd prod)*
prod    => power ('^' power)*
power   => (unaryOp)* range '%'*
range   => primary (':' primary)?
primary => '(' expr ')'
        => number
        => function '(' f_args ')'
        => name (variable/reference)
        => string
        => '{' a_args '}' (array)
        => bool
        => error (#N/A, #DIV/0!, etc.)

f_args  => e (',' e)*  // or ';' based on locale
```

### Operator Precedence

From lowest to highest:

1. Comparison: `=`, `<`, `>`, `<=`, `>=`, `<>`
2. Concatenation: `&`
3. Addition/Subtraction: `+`, `-`
4. Multiplication/Division: `*`, `/`
5. Exponentiation: `^`
6. Percentage: `%`
7. Unary: `-`, `+`

### AST Node Types

```rust
#[derive(PartialEq, Clone, Debug)]
pub enum Node {
    // Literals
    BooleanKind(bool),
    NumberKind(f64),
    StringKind(String),

    // References
    ReferenceKind {
        sheet_name: Option<String>,
        sheet_index: u32,
        absolute_row: bool,
        absolute_column: bool,
        row: i32,
        column: i32,
    },
    RangeKind {
        sheet_name: Option<String>,
        sheet_index: u32,
        absolute_row1: bool,
        absolute_column1: bool,
        row1: i32,
        column1: i32,
        absolute_row2: bool,
        absolute_column2: bool,
        row2: i32,
        column2: i32,
    },

    // Operators
    OpSumKind { kind: OpSum, left: Box<Node>, right: Box<Node> },
    OpProductKind { kind: OpProduct, left: Box<Node>, right: Box<Node> },
    OpPowerKind { left: Box<Node>, right: Box<Node> },
    OpConcatenateKind { left: Box<Node>, right: Box<Node> },
    CompareKind { kind: OpCompare, left: Box<Node>, right: Box<Node> },
    UnaryKind { kind: OpUnary, right: Box<Node> },
    OpRangeKind { left: Box<Node>, right: Box<Node> },

    // Functions
    FunctionKind { kind: Function, args: Vec<Node> },
    InvalidFunctionKind { name: String, args: Vec<Node> },

    // Other
    VariableKind(String),  // Defined names
    ArrayKind(Vec<Node>),
    ErrorKind(Error),
    EmptyArgKind,
    ParseErrorKind { formula: String, message: String, position: usize },
}
```

### Parser Implementation

```rust
pub struct Parser {
    lexer: Lexer,
    worksheets: Vec<String>,
    context: Option<CellReferenceRC>,
    tables: HashMap<String, Table>,
}

impl Parser {
    pub fn parse(&mut self, formula: &str, context: &Option<CellReferenceRC>) -> Node {
        self.lexer.set_formula(formula);
        self.context.clone_from(context);
        self.parse_expr()
    }

    fn parse_expr(&mut self) -> Node {
        let mut t = self.parse_concat();
        if let Node::ParseErrorKind { .. } = t {
            return t;
        }
        let mut next_token = self.lexer.peek_token();
        while let TokenType::Compare(op) = next_token {
            self.lexer.advance_token();
            let p = self.parse_concat();
            if let Node::ParseErrorKind { .. } = p {
                return p;
            }
            t = Node::CompareKind {
                kind: op,
                left: Box::new(t),
                right: Box::new(p),
            };
            next_token = self.lexer.peek_token();
        }
        t
    }

    fn parse_concat(&mut self) -> Node {
        let mut t = self.parse_term();
        let mut next_token = self.lexer.peek_token();
        while next_token == TokenType::And {
            self.lexer.advance_token();
            let p = self.parse_term();
            t = Node::OpConcatenateKind {
                left: Box::new(t),
                right: Box::new(p),
            };
            next_token = self.lexer.peek_token();
        }
        t
    }

    fn parse_term(&mut self) -> Node {
        let mut t = self.parse_factor();
        let mut next_token = self.lexer.peek_token();
        while let TokenType::Addition(op) = next_token {
            self.lexer.advance_token();
            let p = self.parse_factor();
            t = Node::OpSumKind {
                kind: op,
                left: Box::new(t),
                right: Box::new(p),
            };
            next_token = self.lexer.peek_token();
        }
        t
    }

    // ... continues for factor, prod, power, range, primary
}
```

### Reference Resolution

```rust
fn get_sheet_index_by_name(&self, name: &str) -> Option<u32> {
    let worksheets = &self.worksheets;
    for (i, sheet) in worksheets.iter().enumerate() {
        if sheet == name {
            return Some(i as u32);
        }
    }
    None
}

// Handle relative vs absolute references
// $A$1 = absolute both
// A$1  = absolute row
// $A1  = absolute column
// A1   = relative both
```

## Evaluation Engine

### Evaluation Result Type

```rust
pub enum CalcResult {
    Number(f64),
    String(String),
    Boolean(bool),
    EmptyCell,
    EmptyArg,
    Range {
        left: CellReferenceIndex,
        right: CellReferenceIndex,
    },
    Error {
        error: Error,
        origin: CellReferenceIndex,
        message: String,
    },
}
```

### Error Types

```rust
pub enum Error {
    NULL,       // #NULL! - Intersection of non-intersecting ranges
    DIV,        // #DIV/0! - Division by zero
    VALUE,      // #VALUE! - Wrong type of argument
    REF,        // #REF! - Invalid cell reference
    NAME,       // #NAME? - Unrecognized function/name
    NUM,        // #NUM! - Invalid numeric value
    NA,         // #N/A - Value not available
    ERROR,      // #ERROR! - General error
    CIRC,       // #CIRC! - Circular reference
    NIMPL,      // #NIMPL - Not implemented
}
```

### Cell Evaluation with Cycle Detection

```rust
#[derive(Clone)]
pub(crate) enum CellState {
    Evaluated,    // Cell has been evaluated
    Evaluating,   // Cell is currently being evaluated (cycle detection)
}

pub(crate) fn evaluate_cell(&mut self, cell_reference: CellReferenceIndex) -> CalcResult {
    let row_data = match self.workbook.worksheets[cell_reference.sheet as usize]
        .sheet_data
        .get(&cell_reference.row)
    {
        Some(r) => r,
        None => return CalcResult::EmptyCell,
    };

    let cell = match row_data.get(&cell_reference.column) {
        Some(c) => c,
        None => return CalcResult::EmptyCell,
    };

    match cell.get_formula() {
        Some(f) => {
            let key = (cell_reference.sheet, cell_reference.row, cell_reference.column);

            match self.cells.get(&key) {
                Some(CellState::Evaluating) => {
                    // Circular reference detected!
                    return CalcResult::new_error(
                        Error::CIRC,
                        cell_reference,
                        "Circular reference detected".to_string(),
                    );
                }
                Some(CellState::Evaluated) => {
                    // Return cached result
                    return self.get_cell_value(cell, cell_reference);
                }
                _ => {
                    // Mark as being evaluated
                    self.cells.insert(key, CellState::Evaluating);
                }
            }

            let node = &self.parsed_formulas[cell_reference.sheet as usize][f as usize];
            let result = self.evaluate_node_in_context(node, cell_reference);

            // Store result in cell
            self.set_cell_value(cell_reference, &result);

            // Mark as evaluated
            self.cells.insert(key, CellState::Evaluated);

            result
        }
        None => self.get_cell_value(cell, cell_reference),
    }
}
```

### Node Evaluation

```rust
pub(crate) fn evaluate_node_in_context(
    &mut self,
    node: &Node,
    cell: CellReferenceIndex,
) -> CalcResult {
    use Node::*;

    match node {
        NumberKind(value) => CalcResult::Number(*value),
        StringKind(value) => CalcResult::String(value.replace(r#""""#, r#"""#)),
        BooleanKind(value) => CalcResult::Boolean(*value),

        OpSumKind { kind, left, right } => {
            let l = match self.get_number(left, cell) {
                Ok(f) => f,
                Err(s) => return s,
            };
            let r = match self.get_number(right, cell) {
                Ok(f) => f,
                Err(s) => return s,
            };
            match kind {
                OpSum::Add => CalcResult::Number(l + r),
                OpSum::Minus => CalcResult::Number(l - r),
            }
        }

        OpProductKind { kind, left, right } => {
            let l = match self.get_number(left, cell) {
                Ok(f) => f,
                Err(s) => return s,
            };
            let r = match self.get_number(right, cell) {
                Ok(f) => f,
                Err(s) => return s,
            };
            match kind {
                OpProduct::Times => CalcResult::Number(l * r),
                OpProduct::Divide => {
                    if r == 0.0 {
                        return CalcResult::new_error(
                            Error::DIV,
                            cell,
                            "Divide by Zero".to_string(),
                        );
                    }
                    CalcResult::Number(l / r)
                }
            }
        }

        OpPowerKind { left, right } => {
            let l = self.get_number(left, cell).unwrap();
            let r = self.get_number(right, cell).unwrap();
            CalcResult::Number(l.powf(r))
        }

        CompareKind { kind, left, right } => {
            let l = self.evaluate_node_in_context(left, cell);
            if l.is_error() { return l; }
            let r = self.evaluate_node_in_context(right, cell);
            if r.is_error() { return r; }

            let compare = compare_values(&l, &r);
            match kind {
                OpCompare::Equal => CalcResult::Boolean(compare == 0),
                OpCompare::LessThan => CalcResult::Boolean(compare == -1),
                OpCompare::GreaterThan => CalcResult::Boolean(compare == 1),
                // ... etc
            }
        }

        FunctionKind { kind, args } => {
            self.evaluate_function(kind, args, cell)
        }

        ReferenceKind { sheet_index, row, column, absolute_row, absolute_column, .. } => {
            // Resolve reference based on context cell
            let mut resolved_row = *row;
            let mut resolved_column = *column;

            if !absolute_row {
                resolved_row += cell.row;
            }
            if !absolute_column {
                resolved_column += cell.column;
            }

            self.evaluate_cell(CellReferenceIndex {
                sheet: *sheet_index,
                row: resolved_row,
                column: resolved_column,
            })
        }

        RangeKind { row1, column1, row2, column2, sheet_index, .. } => {
            CalcResult::Range {
                left: CellReferenceIndex {
                    sheet: *sheet_index,
                    row: *row1,
                    column: *column1,
                },
                right: CellReferenceIndex {
                    sheet: *sheet_index,
                    row: *row2,
                    column: *column2,
                },
            }
        }

        VariableKind(defined_name) => {
            // Lookup defined name
            let parsed_defined_name = self
                .parsed_defined_names
                .get(&(Some(cell.sheet), defined_name.to_lowercase()))
                .or_else(|| {
                    self.parsed_defined_names
                        .get(&(None, defined_name.to_lowercase()))
                });

            match parsed_defined_name {
                Some(ParsedDefinedName::CellReference(reference)) => {
                    self.evaluate_cell(*reference)
                }
                Some(ParsedDefinedName::RangeReference(range)) => {
                    CalcResult::Range { left: range.left, right: range.right }
                }
                _ => CalcResult::new_error(
                    Error::NAME,
                    cell,
                    format!("Defined name \"{}\" not found.", defined_name),
                ),
            }
        }

        // ... other cases
    }
}
```

## Function Implementation

### Function Enum

```rust
#[derive(PartialEq, Clone, Debug)]
pub enum Function {
    // Logical
    And, False, If, Iferror, Ifna, Not, Or, Xor,

    // Mathematical
    Abs, Acos, Asin, Atan, Cos, Sin, Tan,
    Sum, Sumif, Sumifs, Product,
    Round, Rounddown, Roundup,

    // Statistical
    Average, Averageif, Count, Counta, Countif, Max, Min,

    // Text
    Concat, Concatenate, Find, Left, Len, Lower, Mid, Right, Search,
    Substitute, Text, Trim, Upper, Value,

    // Date and Time
    Date, Day, Month, Year, Now, Today,

    // Lookup
    Vlookup, Hlookup, Index, Match, Xlookup, Offset,

    // Financial
    Fv, Pmt, Pv, Rate, Nper, Ir, Xnpv,

    // Engineering
    Besseli, Besselj, Besselk, Bessely, Erf, Erfc,

    // ... 200+ functions total
}
```

### Function Implementation Pattern

```rust
impl Model {
    fn evaluate_function(
        &mut self,
        function: &Function,
        args: &[Node],
        cell: CellReferenceIndex,
    ) -> CalcResult {
        match function {
            Function::Sum => self.fn_sum(args, cell),
            Function::If => self.fn_if(args, cell),
            Function::Vlookup => self.fn_vlookup(args, cell),
            // ... etc
        }
    }

    fn fn_sum(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let mut sum = 0.0;

        for arg in args {
            let result = self.evaluate_node_in_context(arg, cell);
            match result {
                CalcResult::Number(n) => sum += n,
                CalcResult::Range { left, right } => {
                    // Sum all cells in range
                    for row in left.row..=right.row {
                        for col in left.column..=right.column {
                            let cell_result = self.evaluate_cell(CellReferenceIndex {
                                sheet: left.sheet,
                                row,
                                column: col,
                            });
                            if let CalcResult::Number(n) = cell_result {
                                sum += n;
                            }
                        }
                    }
                }
                CalcResult::EmptyCell | CalcResult::EmptyArg => {}
                CalcResult::Error { .. } => return result,
                _ => {}
            }
        }

        CalcResult::Number(sum)
    }

    fn fn_if(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 2 || args.len() > 3 {
            return CalcResult::new_error(
                Error::VALUE,
                cell,
                "IF requires 2 or 3 arguments".to_string(),
            );
        }

        let condition = self.evaluate_node_in_context(&args[0], cell);

        let is_truthy = match condition {
            CalcResult::Boolean(b) => b,
            CalcResult::Number(n) => n != 0.0,
            CalcResult::String(s) => !s.is_empty(),
            _ => false,
        };

        if is_truthy {
            self.evaluate_node_in_context(&args[1], cell)
        } else if args.len() == 3 {
            self.evaluate_node_in_context(&args[2], cell)
        } else {
            CalcResult::Boolean(false)
        }
    }
}
```

### Engineering Functions Example (Bessel)

```rust
// From functions/engineering/transcendental/bessel_i.rs

pub fn besseli(x: f64, n: i32) -> f64 {
    // Modified Bessel function of the first kind
    // Implementation uses series expansion
    if x == 0.0 {
        if n == 0 {
            return 1.0;
        } else {
            return 0.0;
        }
    }

    let mut sum = 0.0;
    let mut term = (x / 2.0).powi(n) / factorial(n) as f64;

    for k in 0..100 {
        sum += term;
        term *= (x / 2.0).powi(2) / ((k + 1) * (k + n + 1)) as f64;
        if term.abs() < 1e-10 {
            break;
        }
    }

    sum
}
```

## Cell Dependency Graph

### Implicit Dependencies

IronCalc doesn't build an explicit dependency graph. Instead, it uses:

1. **Lazy evaluation**: Cells are evaluated on-demand
2. **Memoization**: Results are cached after evaluation
3. **Cycle detection**: Tracking evaluation state prevents infinite loops

### Evaluation Order

```rust
pub fn evaluate(&mut self) {
    // Clear all computation artifacts
    self.cells.clear();

    // Get all cells
    let cells = self.get_all_cells();

    // Evaluate each cell
    // Note: This is row-major order, but dependencies are resolved on-demand
    for cell in cells {
        self.evaluate_cell(CellReferenceIndex {
            sheet: cell.index,
            row: cell.row,
            column: cell.column,
        });
    }
}
```

### Reference Resolution in Formulas

When a formula references another cell:

```rust
// If cell B2 contains =A1+1
// Evaluating B2 triggers evaluation of A1

Node::ReferenceKind { sheet_index, row, column, .. } => {
    // Recursively evaluate the referenced cell
    self.evaluate_cell(CellReferenceIndex {
        sheet: *sheet_index,
        row: *row,
        column: *column,
    })
}
```

## Formula Storage and Parsing

### Parsed Formula Cache

```rust
pub struct Model {
    pub workbook: Workbook,
    pub parsed_formulas: Vec<Vec<Node>>,  // One Vec per sheet
    pub parsed_defined_names: HashMap<(Option<u32>, String), ParsedDefinedName>,
    pub shared_strings: HashMap<String, usize>,
    pub parser: Parser,
    pub cells: HashMap<(u32, i32, i32), CellState>,
    // ...
}
```

### Formula Index

```rust
// In Cell enum:
CellFormula {
    f: i32,  // Index into parsed_formulas[sheet_index]
    s: i32,  // Style index
}
```

### Parsing All Formulas

```rust
fn parse_formulas(&mut self) {
    for (sheet_index, worksheet) in self.workbook.worksheets.iter().enumerate() {
        let mut sheet_formulas = Vec::new();

        for (_row, row_data) in &worksheet.sheet_data {
            for (_col, cell) in row_data {
                if let Some(formula_index) = cell.get_formula() {
                    // Formula string is stored in worksheet.shared_formulas
                    let formula = &worksheet.shared_formulas[formula_index as usize];

                    let context = Some(CellReferenceRC { /* cell position */ });
                    let parsed = self.parser.parse(formula, &context);

                    sheet_formulas.push(parsed);
                }
            }
        }

        self.parsed_formulas.push(sheet_formulas);
    }
}
```

## Stringification (AST to Formula)

```rust
pub fn to_string(node: &Node) -> String {
    match node {
        Node::NumberKind(n) => format!("{}", n),
        Node::StringKind(s) => format!("\"{}\"", s.replace('"', "\"\"")),
        Node::BooleanKind(true) => "TRUE".to_string(),
        Node::BooleanKind(false) => "FALSE".to_string(),

        Node::OpSumKind { kind, left, right } => {
            let l = to_string(left);
            let r = to_string(right);
            match kind {
                OpSum::Add => format!("{}+{}", l, r),
                OpSum::Minus => format!("{}-{}", l, r),
            }
        }

        Node::FunctionKind { kind, args } => {
            let func_name = format!("{:?}", kind);  // In practice, localized
            let args_str: Vec<String> = args.iter().map(|a| to_string(a)).collect();
            format!("{}({})", func_name, args_str.join(","))
        }

        // ... etc
    }
}
```

## Testing the Formula Engine

### Unit Tests

```rust
#[cfg(test)]
mod test {
    use crate::{Model, expressions::parser::Parser};

    #[test]
    fn test_simple_formula() {
        let mut model = Model::new_empty("test", "en", "UTC").unwrap();
        model.set_user_input(0, 1, 1, "=2+2");
        model.evaluate();
        assert_eq!(model.get_formatted_cell_value(0, 1, 1).unwrap(), "4");
    }

    #[test]
    fn test_circular_reference() {
        let mut model = Model::new_empty("test", "en", "UTC").unwrap();
        model.set_user_input(0, 1, 1, "=B2+1");
        model.set_user_input(0, 1, 2, "=A1+1");
        model.evaluate();
        // Should not hang, should detect cycle
        let value = model.get_formatted_cell_value(0, 1, 1).unwrap();
        assert!(value.contains("CIRC"));
    }

    #[test]
    fn test_sum_range() {
        let mut model = Model::new_empty("test", "en", "UTC").unwrap();
        for i in 1..=5 {
            model.set_user_input(0, i, 1, format!("{}", i * 10));
        }
        model.set_user_input(0, 6, 1, "=SUM(A1:A5)");
        model.evaluate();
        assert_eq!(model.get_formatted_cell_value(0, 6, 1).unwrap(), "150");
    }
}
```

## Performance Optimizations

1. **Shared Strings**: Interned strings for memory efficiency
2. **Formula Caching**: Parsed AST cached, not re-parsed on each evaluation
3. **Lazy Evaluation**: Only evaluate cells when needed
4. **Memoization**: Cache evaluation results
5. **Sparse Storage**: HashMap for sheet data (no empty cells stored)
