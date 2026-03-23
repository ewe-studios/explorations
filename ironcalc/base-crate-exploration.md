# ironcalc_base Crate Deep Dive

## Overview

`ironcalc_base` is the core spreadsheet engine crate containing all the fundamental components for spreadsheet calculation.

**Path**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.ironcalc/IronCalc/base/`

## Cargo.toml

```toml
[package]
name = "ironcalc_base"
version = "0.2.0"
edition = "2021"
license = "MIT OR Apache-2.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
ryu = "1.0"                    # Fast float to string
chrono = "0.4"
chrono-tz = "0.9"              # Timezone support
regex = "1.0"
once_cell = "1.16.0"
bitcode = "0.6.0"              # Binary serialization
csv = "1.3.0"
csv-sniffer = "0.1"

# WASM-specific dependencies
[target.'cfg(target_arch = "wasm32")'.dependencies]
js-sys = { version = "0.3.69" }

# Native-specific dependencies
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
rand = "0.8.5"
```

## Module Structure

```
base/src/
├── lib.rs                 # Public API exports
├── model.rs               # Model struct (main API)
├── types.rs               # Core type definitions
├── cell.rs                # Cell type and operations
├── worksheet.rs           # Worksheet operations
├── workbook.rs            # Workbook operations
├── actions.rs             # User actions
├── diffs.rs               # Diff tracking for sync
├── cast.rs                # Type casting utilities
├── constants.rs           # Spreadsheet constants
├── implicit_intersection.rs
├── new_empty.rs           # Model construction
├── number_format.rs       # Number formatting
├── styles.rs              # Style system
├── units.rs               # Unit calculations
├── utils.rs               # Utility functions
├── mock_time.rs           # Time mocking for tests
├── expressions/           # Formula handling
│   ├── mod.rs
│   ├── lexer/            # Tokenizer
│   ├── parser/           # AST parser
│   ├── token.rs          # Token types
│   ├── types.rs          # Expression types
│   └── utils.rs          # Expression utilities
├── functions/            # Function implementations
│   ├── mod.rs
│   ├── logical.rs
│   ├── mathematical.rs
│   ├── statistical.rs
│   ├── text.rs
│   ├── date_and_time.rs
│   ├── lookup_and_reference.rs
│   ├── financial.rs
│   ├── engineering/
│   └── ...
├── formatter/            # Number formatting
│   ├── mod.rs
│   ├── lexer.rs
│   ├── parser.rs
│   └── format.rs
├── language/             # Localized function names
├── locale/               # Locale data
├── user_model/           # UI-focused wrapper
│   ├── common.rs
│   └── history.rs
└── test/                 # Test modules
```

## Core Types (types.rs)

### Workbook

```rust
#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct Workbook {
    pub shared_strings: Vec<String>,
    pub defined_names: Vec<DefinedName>,
    pub worksheets: Vec<Worksheet>,
    pub styles: Styles,
    pub name: String,
    pub settings: WorkbookSettings,
    pub metadata: Metadata,
    pub tables: HashMap<String, Table>,
    pub views: HashMap<u32, WorkbookView>,
}
```

### Worksheet

```rust
#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct Worksheet {
    pub dimension: String,
    pub cols: Vec<Col>,
    pub rows: Vec<Row>,
    pub name: String,
    pub sheet_data: SheetData,  // HashMap<i32, HashMap<i32, Cell>>
    pub shared_formulas: Vec<String>,
    pub sheet_id: u32,
    pub state: SheetState,
    pub color: Option<String>,
    pub merge_cells: Vec<String>,
    pub comments: Vec<Comment>,
    pub frozen_rows: i32,
    pub frozen_columns: i32,
    pub views: HashMap<u32, WorksheetView>,
    pub show_grid_lines: bool,
}
```

### Cell Enum

```rust
#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub enum Cell {
    EmptyCell { s: i32 },
    BooleanCell { v: bool, s: i32 },
    NumberCell { v: f64, s: i32 },
    ErrorCell { ei: Error, s: i32 },
    SharedString { si: i32, s: i32 },
    CellFormula { f: i32, s: i32 },
    CellFormulaBoolean { f: i32, v: bool, s: i32 },
    CellFormulaNumber { f: i32, v: f64, s: i32 },
    CellFormulaString { f: i32, v: String, s: i32 },
    CellFormulaError {
        f: i32,
        ei: Error,
        s: i32,
        o: String,  // Origin
        m: String,  // Message
    },
}
```

## Model API (model.rs)

### Construction

```rust
impl Model {
    /// Create new empty model
    pub fn new_empty(name: &str, locale: &str, timezone: &str) -> Result<Model, String>

    /// Load from binary representation
    pub fn from_bytes(s: &[u8]) -> Result<Model, String>

    /// Load from Workbook
    pub fn from_workbook(workbook: Workbook) -> Result<Model, String>
}
```

### Cell Operations

```rust
impl Model {
    /// Set user input (formula or value)
    pub fn set_user_input(&mut self, sheet: u32, row: i32, column: i32, input: String)

    /// Get formatted cell value for display
    pub fn get_formatted_cell_value(&self, sheet: u32, row: i32, column: i32) -> Result<String, String>

    /// Get cell formula if any
    pub fn get_cell_formula(&self, sheet: u32, row: i32, column: i32) -> Result<Option<String>, String>

    /// Get cell value
    pub fn get_cell_value_by_index(&self, sheet: u32, row: i32, column: i32) -> Result<CellValue, String>

    /// Check if cell is empty
    pub fn is_empty_cell(&self, sheet: u32, row: i32, column: i32) -> Result<bool, String>

    /// Clear cell contents
    pub fn cell_clear_contents(&mut self, sheet: u32, row: i32, column: i32) -> Result<(), String>

    /// Clear all (contents and style)
    pub fn cell_clear_all(&mut self, sheet: u32, row: i32, column: i32) -> Result<(), String>
}
```

### Evaluation

```rust
impl Model {
    /// Evaluate all cells
    pub fn evaluate(&mut self)

    /// Evaluate single cell
    pub(crate) fn evaluate_cell(&mut self, cell_reference: CellReferenceIndex) -> CalcResult

    /// Evaluate AST node
    pub(crate) fn evaluate_node_in_context(&mut self, node: &Node, cell: CellReferenceIndex) -> CalcResult
}
```

### Sheet Operations

```rust
impl Model {
    pub fn new_sheet(&mut self) -> Result<(), String>
    pub fn delete_sheet(&mut self, sheet: u32) -> Result<(), String>
    pub fn rename_sheet(&mut self, sheet: u32, name: &str) -> Result<(), String>
    pub fn set_sheet_color(&mut self, sheet: u32, color: &str) -> Result<(), String>
}
```

### Row/Column Operations

```rust
impl Model {
    pub fn insert_row(&mut self, sheet: u32, row: i32) -> Result<(), String>
    pub fn insert_column(&mut self, sheet: u32, column: i32) -> Result<(), String>
    pub fn delete_row(&mut self, sheet: u32, row: i32) -> Result<(), String>
    pub fn delete_column(&mut self, sheet: u32, column: i32) -> Result<(), String>
    pub fn set_row_height(&mut self, sheet: u32, row: i32, height: f64) -> Result<(), String>
    pub fn set_column_width(&mut self, sheet: u32, column: i32, width: f64) -> Result<(), String>
    pub fn get_row_height(&mut self, sheet: u32, row: i32) -> Result<f64, String>
    pub fn get_column_width(&mut self, sheet: u32, column: i32) -> Result<f64, String>
}
```

### Serialization

```rust
impl Model {
    pub fn to_bytes(&self) -> Vec<u8>
    pub fn from_bytes(bytes: &[u8]) -> Result<Model, String>
}
```

## Expression Module (expressions/)

### Lexer (lexer/mod.rs)

```rust
pub struct Lexer {
    position: usize,
    chars: Vec<char>,
    mode: LexerMode,  // A1 or R1C1
    locale: Locale,
    language: Language,
}

impl Lexer {
    pub fn new(formula: &str, mode: LexerMode, locale: &Locale, language: &Language) -> Lexer
    pub fn next_token(&mut self) -> TokenType
    pub fn peek_token(&mut self) -> TokenType
    pub fn advance_token(&mut self)
}
```

### Parser (parser/mod.rs)

```rust
pub struct Parser {
    lexer: Lexer,
    worksheets: Vec<String>,
    context: Option<CellReferenceRC>,
    tables: HashMap<String, Table>,
}

impl Parser {
    pub fn new(worksheets: Vec<String>, tables: HashMap<String, Table>) -> Parser
    pub fn parse(&mut self, formula: &str, context: &Option<CellReferenceRC>) -> Node
}
```

### AST Node

```rust
pub enum Node {
    BooleanKind(bool),
    NumberKind(f64),
    StringKind(String),
    ReferenceKind { /* ... */ },
    RangeKind { /* ... */ },
    OpSumKind { kind: OpSum, left: Box<Node>, right: Box<Node> },
    OpProductKind { kind: OpProduct, left: Box<Node>, right: Box<Node> },
    OpPowerKind { left: Box<Node>, right: Box<Node> },
    FunctionKind { kind: Function, args: Vec<Node> },
    VariableKind(String),
    CompareKind { kind: OpCompare, left: Box<Node>, right: Box<Node> },
    // ... more variants
}
```

## Functions Module (functions/)

### Function Enum

```rust
pub enum Function {
    // Logical (11 functions)
    And, False, If, Iferror, Ifna, Ifs, Not, Or, Switch, True, Xor,

    // Mathematical (30+ functions)
    Abs, Acos, Acosh, Asin, Asinh, Atan, Atan2, Atanh,
    Choose, Column, Columns, Cos, Cosh,
    Max, Min, Pi, Power, Product,
    Rand, Randbetween, Round, Rounddown, Roundup,
    Sin, Sinh, Sqrt, Sqrtpi, Sum, Sumif, Sumifs,
    Tan, Tanh,

    // Statistical (10+ functions)
    Average, Averagea, Averageif, Averageifs,
    Count, Counta, Countblank, Countif, Countifs,
    Maxifs, Minifs,

    // Text (20+ functions)
    Concat, Concatenate, Exact, Find, Left, Len, Lower,
    Mid, Rept, Right, Search, Substitute, T, Text,
    Textafter, Textbefore, Textjoin, Trim, Upper,
    Value, Valuetotext,

    // Date/Time (8 functions)
    Date, Day, Edate, Eomonth, Month, Now, Today, Year,

    // Lookup (10 functions)
    Hlookup, Index, Indirect, Lookup, Match, Offset,
    Row, Rows, Vlookup, Xlookup,

    // Financial (25+ functions)
    Cumipmt, Cumprinc, Db, Ddb, Dollarde, Dollarfr,
    Effect, Fv, Ipmt, Irr, Ispmt, Mirr, Nominal,
    Nper, Npv, Pduration, Pmt, Ppmt, Pv, Rate,
    Rri, Sln, Syd, Tbilleq, Tbillprice, Tbillyield,
    Xirr, Xnpv,

    // Engineering (30+ functions)
    Besseli, Besselj, Besselk, Bessely,
    Erf, Erfc, ErfcPrecise, ErfPrecise,
    Bin2dec, Bin2hex, Bin2oct, Dec2Bin, Dec2hex,
    // ... complex number functions

    // Information (15+ functions)
    ErrorType, Isblank, Iserr, Iserror, Iseven,
    Isformula, Islogical, Isna, Isnontext,
    Isnumber, Isodd, Isref, Istext, Na, Sheet, Type,
}
```

### Function Implementation Pattern

```rust
impl Model {
    fn evaluate_function(&mut self, function: &Function, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        match function {
            Function::Sum => self.fn_sum(args, cell),
            Function::If => self.fn_if(args, cell),
            Function::Vlookup => self.fn_vlookup(args, cell),
            // ... all functions
        }
    }

    fn fn_sum(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        let mut sum = 0.0;
        for arg in args {
            match self.evaluate_node_in_context(arg, cell) {
                CalcResult::Number(n) => sum += n,
                CalcResult::Range { left, right } => {
                    // Sum range
                }
                CalcResult::Error { .. } => return /* error */,
                _ => {}
            }
        }
        CalcResult::Number(sum)
    }
}
```

## Formatter Module (formatter/)

```rust
pub struct Parser {
    parts: Vec<ParsePart>,
}

pub enum ParsePart {
    General(GeneralPart),
    Number(NumberPart),
    Date(DatePart),
    Text(TextPart),
    Error(String),
}

pub fn format_number(value: f64, format: &str) -> String
pub fn parse_formatted_number(input: &str, format: &str) -> Option<f64>
```

## UserModel (user_model/)

### Structure

```rust
pub struct UserModel {
    pub(crate) model: Model,
    history: History,
    send_queue: Vec<QueueDiffs>,
    pause_evaluation: bool,
}
```

### Undo/Redo

```rust
impl UserModel {
    pub fn undo(&mut self) -> Result<(), String>
    pub fn redo(&mut self) -> Result<(), String>
    pub fn can_undo(&self) -> bool
    pub fn can_redo(&self) -> bool
}
```

### Diff Tracking

```rust
impl UserModel {
    pub fn flush_send_queue(&mut self) -> Vec<u8>
    pub fn apply_external_diffs(&mut self, diffs: &[u8]) -> Result<(), String>
}
```

### Evaluation Control

```rust
impl UserModel {
    pub fn pause_evaluation(&mut self)
    pub fn resume_evaluation(&mut self)
    pub fn evaluate(&mut self)
}
```

## Locale System (locale/)

```rust
pub struct Locale {
    pub code: String,
    pub numbers: NumberFormat,
    pub currency: Currency,
    pub list_separator: String,
}

pub struct NumberFormat {
    pub symbols: NumberSymbols,
    pub formats: NumberFormats,
}

pub struct NumberSymbols {
    pub decimal: String,      // "." or ","
    pub grouping: String,     // "," or "."
    pub percent: String,
    pub minus: String,
    pub plus: String,
}
```

## Language System (language/)

```rust
pub struct Language {
    pub code: String,
    pub function_translations: HashMap<String, String>,
    pub boolean_true: String,
    pub boolean_false: String,
    pub errors: HashMap<Error, String>,
}
```

## Constants (constants.rs)

```rust
pub const LAST_ROW: i32 = 1_048_576;       // Excel max rows
pub const LAST_COLUMN: i32 = 16_384;       // Excel max columns (XFD)
pub const DEFAULT_ROW_HEIGHT: f64 = 15.0;
pub const DEFAULT_COLUMN_WIDTH: f64 = 8.43;
```

## Testing

### Test Structure

```
test/
├── mod.rs
├── test_general.rs           # General tests
├── test_math.rs              # Mathematical functions
├── test_fn_sum.rs            # SUM function tests
├── test_fn_if.rs             # IF function tests
├── test_circular_references.rs
├── test_error_propagation.rs
├── test_forward_references.rs
├── user_model/
│   ├── test_evaluation.rs
│   ├── test_undo_redo.rs
│   └── ...
└── engineering/
    ├── test_bessel.rs
    ├── test_complex.rs
    └── ...
```

### Example Test

```rust
#[test]
fn test_fn_sum_basic() {
    let mut model = Model::new_empty("test", "en", "UTC").unwrap();

    model.set_user_input(0, 1, 1, "10".to_string());
    model.set_user_input(0, 2, 1, "20".to_string());
    model.set_user_input(0, 3, 1, "30".to_string());
    model.set_user_input(0, 4, 1, "=SUM(A1:A3)".to_string());

    model.evaluate();

    assert_eq!(model.get_formatted_cell_value(0, 4, 1).unwrap(), "60");
}
```

## Key Design Patterns

1. **String Interning**: Shared strings for memory efficiency
2. **Lazy Evaluation**: Cells evaluated on-demand
3. **Memoization**: Cache evaluation results
4. **Cycle Detection**: Track evaluation state to detect circular refs
5. **Diff-based Sync**: Track changes for collaboration
6. **Locale Separation**: Separate locale from language

## Performance Characteristics

- **Formula Parsing**: O(n) where n is formula length
- **Cell Evaluation**: O(d) where d is dependency depth
- **Memory**: Sparse storage - only non-empty cells stored
- **Serialization**: bitcode provides ~10x compression vs JSON
