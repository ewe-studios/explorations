# IronCalc Project Exploration

## Overview

IronCalc is a modern, open-source spreadsheet engine written in Rust with WASM bindings. It aims to democratize spreadsheets by providing a high-performance, embeddable calculation engine that can run in diverse environments including web browsers, terminals, desktop applications, and server-side processes.

**Repository**: https://github.com/ironcalc/IronCalc

**License**: MIT OR Apache-2.0

## Project Structure

```
/home/darkvoid/Boxxed/@formulas/src.rust/src.ironcalc/
├── IronCalc/                    # Main workspace
│   ├── base/                    # Core engine (ironcalc_base crate)
│   ├── xlsx/                    # XLSX import/export (ironcalc crate)
│   ├── bindings/
│   │   ├── wasm/               # WebAssembly bindings
│   │   └── python/             # Python bindings (pyo3)
│   ├── webapp/                 # React-based web UI
│   ├── generate_locale/        # Locale generation tooling
│   └── wiki/                   # Documentation
├── web-bindings/               # Alternative WASM bindings project
├── TironCalc/                  # Terminal UI spreadsheet (ratatui)
└── ironcalc.github.io/         # Landing page
```

## Workspace Crates

### 1. `ironcalc_base` (base/)
The core spreadsheet engine with:
- Formula parsing and evaluation
- Cell data model and types
- Function library (200+ Excel-compatible functions)
- Number formatting
- Locale and timezone support
- Undo/redo history management

**Dependencies**: serde, chrono, regex, bitcode, csv, js-sys (WASM)

### 2. `ironcalc` (xlsx/)
XLSX file format support:
- Import from Excel files
- Export to Excel files
- Internal binary format (.icalc)

**Dependencies**: zip, roxmltree, ironcalc_base, itertools

### 3. `wasm` (bindings/wasm/)
WebAssembly bindings for browser usage:
- cdylib crate type for WASM output
- wasm-bindgen integration
- Serde for JS serialization

### 4. `pyroncalc` (bindings/python/)
Python bindings using PyO3

## Key Architecture Components

### Data Model

The spreadsheet uses a hierarchical data model:

```
Workbook
├── Worksheet[] (sheets)
│   ├── SheetData (HashMap<row, HashMap<column, Cell>>)
│   ├── Cols[] (column definitions)
│   ├── Rows[] (row definitions)
│   └── WorksheetView[] (UI state per view)
├── SharedStrings[] (string interning)
├── DefinedNames[] (named ranges)
├── Tables[] (Excel tables)
├── Styles
│   ├── NumFmts[]
│   ├── Fonts[]
│   ├── Fills[]
│   └── Borders[]
└── Views[] (window state)
```

### Cell Types

```rust
pub enum Cell {
    EmptyCell { s: i32 },
    BooleanCell { v: bool, s: i32 },
    NumberCell { v: f64, s: i32 },
    ErrorCell { ei: Error, s: i32 },
    SharedString { si: i32, s: i32 },
    CellFormula { f: i32, s: i32 },              // Unevaluated
    CellFormulaBoolean { f: i32, v: bool, s: i32 },
    CellFormulaNumber { f: i32, v: f64, s: i32 },
    CellFormulaString { f: i32, v: String, s: i32 },
    CellFormulaError { f: i32, ei: Error, s: i32, o: String, m: String },
}
```

### Formula AST (Node Types)

```rust
pub enum Node {
    BooleanKind(bool),
    NumberKind(f64),
    StringKind(String),
    ReferenceKind { /* cell reference */ },
    RangeKind { /* range reference */ },
    OpSumKind { kind: OpSum, left, right },
    OpProductKind { kind: OpProduct, left, right },
    OpPowerKind { left, right },
    OpConcatenateKind { left, right },
    CompareKind { kind: OpCompare, left, right },
    UnaryKind { kind: OpUnary, right },
    FunctionKind { kind: Function, args: Vec<Node> },
    VariableKind(String),  // Defined names
    ErrorKind(token::Error),
}
```

## Formula Grammar

```
opComp   => '=' | '<' | '>' | '<=' | '>=' | '<>'
opFactor => '*' | '/'
unaryOp  => '-' | '+'

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
        => name
        => string
        => '{' a_args '}'
        => bool
        => bool()
        => error

f_args  => e (',' e)*
```

## Function Categories

IronCalc implements 200+ Excel-compatible functions:

| Category | Functions |
|----------|-----------|
| Logical | AND, FALSE, IF, IFERROR, IFNA, IFS, NOT, OR, SWITCH, TRUE, XOR |
| Mathematical | ABS, ACOS, ASIN, ATAN, CHOOSE, COLUMN, COS, MAX, MIN, PI, POWER, PRODUCT, RAND, ROUND, SIN, SQRT, SUM, SUMIF, SUMIFS, TAN |
| Statistical | AVERAGE, AVERAGEA, AVERAGEIF, COUNT, COUNTA, COUNTBLANK, COUNTIF, MAXIFS, MINIFS |
| Text | CONCAT, CONCATENATE, EXACT, FIND, LEFT, LEN, LOWER, MID, REPT, RIGHT, SEARCH, SUBSTITUTE, TEXT, TRIM, UPPER, VALUE |
| Date/Time | DATE, DAY, EDATE, EOMONTH, MONTH, NOW, TODAY, YEAR |
| Lookup | HLOOKUP, INDEX, INDIRECT, LOOKUP, MATCH, OFFSET, VLOOKUP, XLOOKUP |
| Financial | CUMIPMT, FV, IPMT, IRR, NPER, NPV, PMT, PPMT, PV, RATE, XIRR, XNPV |
| Engineering | BESSEL*, ERF, ERFC, BIN2DEC, DEC2BIN, HEX2DEC, complex number functions |
| Information | ERROR.TYPE, ISBLANK, ISERR, ISNUMBER, ISTEXT, SHEET, TYPE |

## Evaluation Model

### Recursive Descent Evaluation

IronCalc uses a top-down recursive evaluation algorithm:

```rust
pub fn evaluate(&mut self) {
    self.cells.clear();  // Clear evaluation state
    let cells = self.get_all_cells();

    for cell in cells {
        self.evaluate_cell(CellReferenceIndex {
            sheet: cell.index,
            row: cell.row,
            column: cell.column,
        });
    }
}
```

### Circular Reference Detection

The engine tracks cell evaluation state to detect circular references:

```rust
pub(crate) fn evaluate_cell(&mut self, cell_reference: CellReferenceIndex) -> CalcResult {
    // ... get cell from sheet_data ...

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
            self.set_cell_value(cell_reference, &result);
            self.cells.insert(key, CellState::Evaluated);
            result
        }
        None => self.get_cell_value(cell, cell_reference),
    }
}
```

## WASM Architecture

### Web Bindings Pattern

The WASM bindings expose a clean JavaScript API:

```rust
#[wasm_bindgen]
pub struct Model {
    model: BaseModel,
}

#[wasm_bindgen]
impl Model {
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str, locale: &str, timezone: &str) -> Result<Model, JsError> {
        let model = BaseModel::new_empty(name, locale, timezone).map_err(to_js_error)?;
        Ok(Model { model })
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Model, JsError> {
        let model = BaseModel::from_bytes(bytes).map_err(to_js_error)?;
        Ok(Model { model })
    }

    pub fn evaluate(&mut self) {
        self.model.evaluate();
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.model.to_bytes()
    }

    // ... 60+ more methods
}
```

### Key WASM Features

1. **State persistence**: `to_bytes()` / `from_bytes()` for saving/loading
2. **Undo/Redo**: Full history support
3. **Evaluation control**: `pause_evaluation()` / `resume_evaluation()`
4. **Diff synchronization**: `flush_send_queue()` / `apply_external_diffs()`

## Performance Characteristics

### Serialization

IronCalc uses `bitcode` for efficient binary serialization:

```rust
#[derive(Encode, Decode, Debug, PartialEq, Clone)]
pub struct Workbook {
    pub shared_strings: Vec<String>,
    pub defined_names: Vec<DefinedName>,
    pub worksheets: Vec<Worksheet>,
    // ...
}

pub fn to_bytes(&self) -> Vec<u8> {
    bitcode::encode(&self.workbook)
}
```

### Optimizations

1. **String interning**: Shared strings HashMap for O(1) lookup
2. **Lazy evaluation**: Cells only evaluated when needed
3. **Memoization**: Evaluated cells cached with CellState
4. **Column-major storage**: SheetData uses HashMap for sparse storage

### Release Profile

```toml
[profile.release]
lto = true
```

## File Format Support

### XLSX Export Structure

```
workbook.xlsx
├── [Content_Types].xml
├── docProps/
│   ├── app.xml
│   └── core.xml
├── _rels/
│   └── .rels
├── xl/
│   ├── sharedStrings.xml
│   ├── styles.xml
│   ├── workbook.xml
│   ├── _rels/
│   │   └── workbook.xml.rels
│   └── worksheets/
│       ├── sheet1.xml
│       ├── sheet2.xml
│       └── ...
```

### Internal Format (.icalc)

Binary format using bitcode encoding - significantly smaller and faster than XLSX.

## UserModel Layer

The `UserModel` wraps `Model` with additional features for UI applications:

```rust
pub struct UserModel {
    pub(crate) model: Model,
    history: History,           // Undo/redo
    send_queue: Vec<QueueDiffs>, // Sync queue
    pause_evaluation: bool,
}
```

Features:
- **Undo/Redo**: Full command history
- **Diff tracking**: Changes serialized for synchronization
- **Auto-evaluation**: Automatic recalculation on changes
- **Clipboard**: Copy/paste with formatting
- **View management**: Selected cell, frozen panes, scroll position

## Locale and Language Support

IronCalc separates locale from language:

- **Locale**: Number formatting (decimal separator, date formats)
- **Language**: Function names, error messages

This allows formulas to be stored in a canonical form (English function names) while displaying in the user's language.

```rust
pub struct Locale {
    pub numbers: NumberFormat,
    pub currency: Currency,
    // ...
}

pub struct Language {
    pub function_names: HashMap<String, String>,
    pub error_messages: HashMap<Error, String>,
}
```

## TironCalc - Terminal UI Example

TironCalc demonstrates using IronCalc in a terminal application:

```rust
use ironcalc::{
    base::{expressions::utils::number_to_column, Model},
    export::save_to_xlsx,
    import::load_from_xlsx,
};
use ratatui::{
    widgets::{Table, Row, Cell, Block, Borders},
    layout::{Layout, Constraint, Direction},
    // ...
};

// Load existing file or create new
let mut model = if args.len() > 1 {
    load_from_xlsx(file_name, "en", "UTC").unwrap()
} else {
    Model::new_empty(file_name, "en", "UTC").unwrap()
};

// Render cells
for row_index in minimum_row_index..=maximum_row_index {
    for column_index in minimum_column_index..=maximum_column_index {
        let value = model
            .get_formatted_cell_value(sheet, row_index, column_index)
            .unwrap();
    }
}
```

## Key Design Decisions

1. **Rust-first**: Core engine in Rust for safety and performance
2. **WASM-native**: Designed for web from the start
3. **Excel-compatible**: Formula syntax and functions match Excel
4. **Locale-aware**: Proper internationalization
5. **Sparse storage**: HashMap-based cell storage for memory efficiency
6. **Binary serialization**: Fast save/load with bitcode
7. **No external calc chain**: Evaluates on-demand with cycle detection

## Testing

Comprehensive test coverage including:
- Unit tests for each function category
- Integration tests for formulas and evaluation
- Test cases from Excel conformance suite
- Circular reference tests
- Undo/redo tests

```bash
make tests  # Runs all tests
make coverage  # Generates coverage report
```
