# Reproducing IronCalc in Rust - Implementation Guide

## Overview

This guide provides a comprehensive roadmap for building a production-level spreadsheet engine in Rust, based on the IronCalc architecture. It covers the essential components, design patterns, and implementation strategies.

## Project Structure

```
spreadsheet-engine/
├── Cargo.toml (workspace)
├── engine/           # Core engine (like ironcalc_base)
├── format/           # XLSX import/export (like xlsx)
├── wasm/             # WASM bindings
├── python/           # Python bindings (optional)
└── cli/              # CLI application (optional)
```

### Workspace Cargo.toml

```toml
[workspace]
resolver = "2"

members = [
    "engine",
    "format",
    "wasm",
]

[profile.release]
lto = true
opt-level = 3
```

## Phase 1: Core Data Model

### 1.1 Cell Types

```rust
// engine/src/cell.rs

use bitcode::{Encode, Decode};
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub enum Cell {
    EmptyCell { style: i32 },
    BooleanCell { value: bool, style: i32 },
    NumberCell { value: f64, style: i32 },
    ErrorCell { error: FormulaError, style: i32 },
    SharedString { string_index: i32, style: i32 },
    CellFormula { formula_index: i32, style: i32 },
    CellFormulaNumber { formula_index: i32, value: f64, style: i32 },
    CellFormulaString { formula_index: i32, value: String, style: i32 },
    CellFormulaBoolean { formula_index: i32, value: bool, style: i32 },
    CellFormulaError {
        formula_index: i32,
        error: FormulaError,
        origin: String,
        message: String,
        style: i32,
    },
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub enum FormulaError {
    Null,
    Div,
    Value,
    Ref,
    Name,
    Num,
    NA,
    Error,
    Circ,
    NImpl,
}

impl Default for Cell {
    fn default() -> Self {
        Cell::EmptyCell { style: 0 }
    }
}
```

### 1.2 Worksheet and Workbook

```rust
// engine/src/workbook.rs

use std::collections::HashMap;
use bitcode::{Encode, Decode};

pub type SheetData = HashMap<i32, HashMap<i32, Cell>>;

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct Worksheet {
    pub name: String,
    pub sheet_id: u32,
    pub dimension: Dimension,
    pub sheet_data: SheetData,
    pub shared_formulas: Vec<String>,
    pub cols: Vec<ColumnDefinition>,
    pub rows: Vec<RowDefinition>,
    pub merge_cells: Vec<String>,
    pub frozen_rows: i32,
    pub frozen_columns: i32,
    pub state: SheetState,
    pub color: Option<String>,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct Dimension {
    pub min_row: i32,
    pub max_row: i32,
    pub min_column: i32,
    pub max_column: i32,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct ColumnDefinition {
    pub min: i32,
    pub max: i32,
    pub width: f64,
    pub custom_width: bool,
    pub style: Option<i32>,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct RowDefinition {
    pub row: i32,
    pub height: f64,
    pub custom_height: bool,
    pub style: i32,
    pub hidden: bool,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub enum SheetState {
    Visible,
    Hidden,
    VeryHidden,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct Workbook {
    pub name: String,
    pub worksheets: Vec<Worksheet>,
    pub shared_strings: Vec<String>,
    pub defined_names: Vec<DefinedName>,
    pub styles: Styles,
    pub settings: WorkbookSettings,
    pub tables: HashMap<String, Table>,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct DefinedName {
    pub name: String,
    pub formula: String,
    pub sheet_id: Option<u32>,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct WorkbookSettings {
    pub locale: String,
    pub timezone: String,
}
```

### 1.3 Styles

```rust
// engine/src/styles.rs

use bitcode::{Encode, Decode};

#[derive(Encode, Decode, Debug, Clone, PartialEq, Default)]
pub struct Styles {
    pub num_fmts: Vec<NumberFormat>,
    pub fonts: Vec<Font>,
    pub fills: Vec<Fill>,
    pub borders: Vec<Border>,
    pub cell_xfs: Vec<CellXf>,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq)]
pub struct Style {
    pub num_fmt: String,
    pub font: Font,
    pub fill: Fill,
    pub border: Border,
    pub alignment: Alignment,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Default)]
pub struct Font {
    pub name: String,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub color: Option<String>,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Default)]
pub struct Alignment {
    pub horizontal: HorizontalAlignment,
    pub vertical: VerticalAlignment,
    pub wrap_text: bool,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Default)]
pub enum HorizontalAlignment {
    #[default]
    General,
    Left,
    Center,
    Right,
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Default)]
pub enum VerticalAlignment {
    #[default]
    Bottom,
    Center,
    Top,
}
```

## Phase 2: Formula Lexer

### 2.1 Token Types

```rust
// engine/src/formula/token.rs

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Operators
    Addition(OpSum),
    Product(OpProduct),
    Power,
    Percent,
    Concatenate,
    Compare(OpCompare),
    Unary(UnaryOp),

    // Delimiters
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Colon,
    Comma,
    Semicolon,
    Bang,

    // Literals
    Number(f64),
    String(String),

    // References
    Reference {
        sheet: Option<String>,
        absolute_row: bool,
        absolute_col: bool,
        row: i32,
        col: i32,
    },
    Range {
        sheet: Option<String>,
        left: CellRef,
        right: CellRef,
    },

    // Identifiers
    Function(String),
    Variable(String),

    // Errors
    Error(FormulaError),
    Illegal(String),
    EndOfFile,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpSum {
    Add,
    Subtract,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpProduct {
    Multiply,
    Divide,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OpCompare {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Plus,
    Minus,
}
```

### 2.2 Lexer Implementation

```rust
// engine/src/formula/lexer.rs

use crate::locale::Locale;
use crate::language::Language;

pub enum LexerMode {
    A1,    // User-facing formulas
    R1C1,  // Internal representation
}

pub struct Lexer {
    chars: Vec<char>,
    position: usize,
    len: usize,
    mode: LexerMode,
    locale: Locale,
    language: Language,
    peek_buffer: Option<TokenType>,
}

impl Lexer {
    pub fn new(formula: &str, mode: LexerMode, locale: Locale, language: Language) -> Self {
        let chars: Vec<char> = formula.chars().collect();
        let len = chars.len();
        Lexer {
            chars,
            position: 0,
            len,
            mode,
            locale,
            language,
            peek_buffer: None,
        }
    }

    pub fn next_token(&mut self) -> TokenType {
        if let Some(token) = self.peek_buffer.take() {
            return token;
        }

        self.consume_whitespace();

        match self.current_char() {
            None => TokenType::EndOfFile,
            Some(c) => match c {
                '+' => self.consume_plus(),
                '-' => self.consume_minus(),
                '*' => TokenType::Product(OpProduct::Multiply),
                '/' => TokenType::Product(OpProduct::Divide),
                '^' => TokenType::Power,
                '%' => TokenType::Percent,
                '&' => TokenType::Concatenate,
                '=' => TokenType::Compare(OpCompare::Equal),
                '<' => self.consume_less_than(),
                '>' => self.consume_greater_than(),
                '(' => TokenType::LeftParen,
                ')' => TokenType::RightParen,
                '{' => TokenType::LeftBrace,
                '}' => TokenType::RightBrace,
                ':' => TokenType::Colon,
                ',' => self.consume_comma(),
                '!' => TokenType::Bang,
                '"' => self.consume_string(),
                '\'' => self.consume_sheet_reference(),
                '0'..='9' => self.consume_number(),
                c if c.is_alphabetic() || c == '_' => self.consume_identifier(),
                _ => TokenType::Illegal(format!("Unexpected character: {}", c)),
            }
        }
    }

    pub fn peek_token(&mut self) -> TokenType {
        let token = self.next_token();
        self.peek_buffer = Some(token.clone());
        token
    }

    fn current_char(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.current_char();
        self.position += 1;
        c
    }

    fn consume_whitespace(&mut self) {
        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn consume_number(&mut self) -> TokenType {
        let start = self.position;
        let has_decimal = self.locale.numbers.decimal == '.';
        let decimal_char = self.locale.numbers.decimal.chars().next().unwrap();

        let mut num_str = String::new();
        while let Some(c) = self.current_char() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else if c == decimal_char && !num_str.contains(decimal_char) {
                num_str.push('.');
                self.advance();
            } else {
                break;
            }
        }

        match num_str.parse::<f64>() {
            Ok(n) => TokenType::Number(n),
            Err(_) => TokenType::Illegal(format!("Invalid number: {}", num_str)),
        }
    }

    fn consume_string(&mut self) -> TokenType {
        self.advance(); // consume opening quote
        let mut value = String::new();

        while let Some(c) = self.advance() {
            if c == '"' {
                if self.current_char() == Some('"') {
                    value.push('"');
                    self.advance();
                } else {
                    break;
                }
            } else {
                value.push(c);
            }
        }

        TokenType::String(value)
    }

    fn consume_identifier(&mut self) -> TokenType {
        let start = self.position - 1;

        while let Some(c) = self.current_char() {
            if c.is_alphanumeric() || c == '_' || c == '.' {
                self.advance();
            } else {
                break;
            }
        }

        let ident: String = self.chars[start..self.position].iter().collect();

        // Check if it's a function, variable, or reference
        if self.language.is_function(&ident) {
            TokenType::Function(ident)
        } else if self.language.is_boolean(&ident) {
            TokenType::Boolean(ident.to_lowercase() == "true")
        } else if self.language.is_error(&ident) {
            TokenType::Error(self.language.parse_error(&ident))
        } else {
            TokenType::Variable(ident)
        }
    }

    // ... implement other consume methods
}
```

## Phase 3: Formula Parser

### 3.1 AST Definition

```rust
// engine/src/formula/ast.rs

use super::token::*;

#[derive(Debug, Clone, PartialEq)]
pub enum AstNode {
    Literal(Literal),
    Reference(CellReference),
    Range(CellReference, CellReference),
    Unary { op: UnaryOp, expr: Box<AstNode> },
    Binary {
        op: BinaryOp,
        left: Box<AstNode>,
        right: Box<AstNode>,
    },
    Function {
        name: String,
        args: Vec<AstNode>,
    },
    Array(Vec<AstNode>),
    Error(FormulaError),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f64),
    String(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Power,
    Concatenate,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CellReference {
    pub sheet: Option<String>,
    pub row: i32,
    pub column: i32,
    pub absolute_row: bool,
    pub absolute_column: bool,
}
```

### 3.2 Recursive Descent Parser

```rust
// engine/src/formula/parser.rs

use super::lexer::{Lexer, LexerMode};
use super::ast::*;
use super::token::*;

pub struct Parser {
    lexer: Lexer,
    current_token: TokenType,
}

impl Parser {
    pub fn new(formula: &str, locale: Locale, language: Language) -> Self {
        let mut lexer = Lexer::new(formula, LexerMode::A1, locale, language);
        let current_token = lexer.next_token();
        Parser { lexer, current_token }
    }

    pub fn parse(&mut self) -> AstNode {
        self.parse_expression()
    }

    fn advance(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    fn expect(&mut self, expected: TokenType) -> Result<(), String> {
        if std::mem::discriminant(&self.current_token) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, found {:?}", expected, self.current_token))
        }
    }

    // expr => concat (compare_op concat)*
    fn parse_expression(&mut self) -> AstNode {
        let mut left = self.parse_concat();

        while let TokenType::Compare(op) = &self.current_token {
            let op = op.clone();
            self.advance();
            let right = self.parse_concat();
            left = AstNode::Binary {
                op: match op {
                    OpCompare::Equal => BinaryOp::Equal,
                    OpCompare::NotEqual => BinaryOp::NotEqual,
                    OpCompare::LessThan => BinaryOp::LessThan,
                    OpCompare::GreaterThan => BinaryOp::GreaterThan,
                    OpCompare::LessThanOrEqual => BinaryOp::LessThanOrEqual,
                    OpCompare::GreaterThanOrEqual => BinaryOp::GreaterThanOrEqual,
                },
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        left
    }

    // concat => term ('&' term)*
    fn parse_concat(&mut self) -> AstNode {
        let mut left = self.parse_term();

        while matches!(self.current_token, TokenType::Concatenate) {
            self.advance();
            let right = self.parse_term();
            left = AstNode::Binary {
                op: BinaryOp::Concatenate,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        left
    }

    // term => factor (add_op factor)*
    fn parse_term(&mut self) -> AstNode {
        let mut left = self.parse_factor();

        loop {
            let op = match &self.current_token {
                TokenType::Addition(OpSum::Add) => Some(BinaryOp::Add),
                TokenType::Addition(OpSum::Subtract) => Some(BinaryOp::Subtract),
                _ => None,
            };

            if let Some(op) = op {
                self.advance();
                let right = self.parse_factor();
                left = AstNode::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        left
    }

    // factor => power (mul_op power)*
    fn parse_factor(&mut self) -> AstNode {
        let mut left = self.parse_power();

        loop {
            let op = match &self.current_token {
                TokenType::Product(OpProduct::Multiply) => Some(BinaryOp::Multiply),
                TokenType::Product(OpProduct::Divide) => Some(BinaryOp::Divide),
                _ => None,
            };

            if let Some(op) = op {
                self.advance();
                let right = self.parse_power();
                left = AstNode::Binary {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        left
    }

    // power => unary ('^' power)?
    fn parse_power(&mut self) -> AstNode {
        let base = self.parse_unary();

        if matches!(self.current_token, TokenType::Power) {
            self.advance();
            let exponent = self.parse_power(); // Right associative
            AstNode::Binary {
                op: BinaryOp::Power,
                left: Box::new(base),
                right: Box::new(exponent),
            }
        } else {
            base
        }
    }

    // unary => ('+' | '-') unary | percent
    fn parse_unary(&mut self) -> AstNode {
        match &self.current_token {
            TokenType::Addition(OpSum::Add) => {
                self.advance();
                let expr = self.parse_unary();
                AstNode::Unary { op: UnaryOp::Plus, expr: Box::new(expr) }
            }
            TokenType::Addition(OpSum::Subtract) => {
                self.advance();
                let expr = self.parse_unary();
                AstNode::Unary { op: UnaryOp::Minus, expr: Box::new(expr) }
            }
            _ => self.parse_percent(),
        }
    }

    fn parse_percent(&mut self) -> AstNode {
        let expr = self.parse_primary();

        if matches!(self.current_token, TokenType::Percent) {
            self.advance();
            AstNode::Binary {
                op: BinaryOp::Divide,
                left: Box::new(expr),
                right: Box::new(AstNode::Literal(Literal::Number(100.0))),
            }
        } else {
            expr
        }
    }

    // primary => number | string | bool | function | reference | range | (expr)
    fn parse_primary(&mut self) -> AstNode {
        match &self.current_token {
            TokenType::Number(n) => {
                let n = *n;
                self.advance();
                AstNode::Literal(Literal::Number(n))
            }
            TokenType::String(s) => {
                let s = s.clone();
                self.advance();
                AstNode::Literal(Literal::String(s))
            }
            TokenType::Boolean(b) => {
                let b = *b;
                self.advance();
                AstNode::Literal(Literal::Boolean(b))
            }
            TokenType::Function(name) => {
                let name = name.clone();
                self.advance();
                self.parse_function(name)
            }
            TokenType::Reference { .. } | TokenType::Range { .. } => {
                self.parse_reference()
            }
            TokenType::LeftParen => {
                self.advance();
                let expr = self.parse_expression();
                self.expect(TokenType::RightParen).unwrap();
                expr
            }
            TokenType::LeftBrace => {
                self.parse_array()
            }
            TokenType::Error(e) => {
                let e = e.clone();
                self.advance();
                AstNode::Error(e)
            }
            _ => AstNode::Error(FormulaError::Error),
        }
    }

    fn parse_function(&mut self, name: String) -> AstNode {
        self.expect(TokenType::LeftParen).unwrap();

        let mut args = Vec::new();

        if !matches!(self.current_token, TokenType::RightParen) {
            args.push(self.parse_expression());

            while matches!(self.current_token, TokenType::Comma | TokenType::Semicolon) {
                self.advance();
                args.push(self.parse_expression());
            }
        }

        self.expect(TokenType::RightParen).unwrap();

        AstNode::Function { name, args }
    }

    fn parse_reference(&mut self) -> AstNode {
        // Implement reference parsing
        // Handle sheet!A1, $A$1, A1:B2, etc.
        unimplemented!()
    }

    fn parse_array(&mut self) -> AstNode {
        self.expect(TokenType::LeftBrace).unwrap();

        let mut elements = Vec::new();

        if !matches!(self.current_token, TokenType::RightBrace) {
            elements.push(self.parse_expression());

            while matches!(self.current_token, TokenType::Comma | TokenType::Semicolon) {
                self.advance();
                elements.push(self.parse_expression());
            }
        }

        self.expect(TokenType::RightBrace).unwrap();

        AstNode::Array(elements)
    }
}
```

## Phase 4: Evaluation Engine

### 4.1 Evaluation Result

```rust
// engine/src/eval/result.rs

#[derive(Debug, Clone, PartialEq)]
pub enum EvalResult {
    Number(f64),
    String(String),
    Boolean(bool),
    Empty,
    Range {
        start: CellPosition,
        end: CellPosition,
    },
    Error {
        error: FormulaError,
        origin: CellPosition,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CellPosition {
    pub sheet: u32,
    pub row: i32,
    pub column: i32,
}
```

### 4.2 Evaluation Context

```rust
// engine/src/eval/context.rs

use std::collections::HashMap;
use crate::workbook::*;
use crate::formula::ast::*;

pub enum CellState {
    NotEvaluated,
    Evaluating,  // For cycle detection
    Evaluated,
}

pub struct EvaluationContext<'a> {
    pub workbook: &'a Workbook,
    pub cell_states: HashMap<CellPosition, CellState>,
    pub parsed_formulas: HashMap<CellPosition, AstNode>,
}

impl<'a> EvaluationContext<'a> {
    pub fn new(workbook: &'a Workbook) -> Self {
        EvaluationContext {
            workbook,
            cell_states: HashMap::new(),
            parsed_formulas: HashMap::new(),
        }
    }

    pub fn evaluate_all(&mut self) {
        for (sheet_idx, sheet) in self.workbook.worksheets.iter().enumerate() {
            for (row, row_data) in &sheet.sheet_data {
                for (col, cell) in row_data {
                    let pos = CellPosition {
                        sheet: sheet_idx as u32,
                        row: *row,
                        column: *col,
                    };
                    self.evaluate_cell(&pos);
                }
            }
        }
    }

    pub fn evaluate_cell(&mut self, position: &CellPosition) -> EvalResult {
        let cell = match self.get_cell(position) {
            Some(c) => c,
            None => return EvalResult::Empty,
        };

        if let Some(formula) = cell.get_formula() {
            // Check for cycles
            match self.cell_states.get(position) {
                Some(CellState::Evaluating) => {
                    return EvalResult::Error {
                        error: FormulaError::Circ,
                        origin: position.clone(),
                        message: "Circular reference detected".to_string(),
                    };
                }
                Some(CellState::Evaluated) => {
                    return self.get_cached_result(position);
                }
                _ => {
                    self.cell_states.insert(position.clone(), CellState::Evaluating);
                }
            }

            let ast = self.parsed_formulas.get(position).unwrap();
            let result = self.evaluate_ast(ast, position);

            // Cache result
            self.cache_result(position, &result);
            self.cell_states.insert(position.clone(), CellState::Evaluated);

            result
        } else {
            self.cell_value_to_result(cell, position)
        }
    }

    fn evaluate_ast(&mut self, node: &AstNode, position: &CellPosition) -> EvalResult {
        match node {
            AstNode::Literal(Literal::Number(n)) => EvalResult::Number(*n),
            AstNode::Literal(Literal::String(s)) => EvalResult::String(s.clone()),
            AstNode::Literal(Literal::Boolean(b)) => EvalResult::Boolean(*b),

            AstNode::Reference { sheet, row, column, absolute_row, absolute_column } => {
                // Resolve reference
                let resolved_row = if *absolute_row { *row } else { row + position.row };
                let resolved_col = if *absolute_column { *column } else { column + position.column };

                let ref_pos = CellPosition {
                    sheet: sheet.unwrap_or(position.sheet),
                    row: resolved_row,
                    column: resolved_col,
                };

                self.evaluate_cell(&ref_pos)
            }

            AstNode::Binary { op, left, right } => {
                let left_val = self.evaluate_ast(left, position);
                let right_val = self.evaluate_ast(right, position);
                self.evaluate_binary(op, left_val, right_val)
            }

            AstNode::Unary { op, expr } => {
                let val = self.evaluate_ast(expr, position);
                self.evaluate_unary(op, val)
            }

            AstNode::Function { name, args } => {
                self.evaluate_function(name, args, position)
            }

            // ... handle other cases
            _ => EvalResult::Error {
                error: FormulaError::NImpl,
                origin: position.clone(),
                message: "Not implemented".to_string(),
            },
        }
    }

    fn evaluate_function(&mut self, name: &str, args: &[AstNode], position: &CellPosition) -> EvalResult {
        match name.to_uppercase().as_str() {
            "SUM" => self.fn_sum(args, position),
            "IF" => self.fn_if(args, position),
            "AVERAGE" => self.fn_average(args, position),
            // ... implement all functions
            _ => EvalResult::Error {
                error: FormulaError::Name,
                origin: position.clone(),
                message: format!("Unknown function: {}", name),
            },
        }
    }

    fn fn_sum(&mut self, args: &[AstNode], position: &CellPosition) -> EvalResult {
        let mut sum = 0.0;

        for arg in args {
            match self.evaluate_ast(arg, position) {
                EvalResult::Number(n) => sum += n,
                EvalResult::Range { start, end } => {
                    for row in start.row..=end.row {
                        for col in start.column..=end.column {
                            let cell_pos = CellPosition {
                                sheet: start.sheet,
                                row,
                                column: col,
                            };
                            if let EvalResult::Number(n) = self.evaluate_cell(&cell_pos) {
                                sum += n;
                            }
                        }
                    }
                }
                EvalResult::Empty | EvalResult::EmptyArg => {}
                EvalResult::Error { .. } => return EvalResult::Number(sum),
                _ => {}
            }
        }

        EvalResult::Number(sum)
    }

    fn fn_if(&mut self, args: &[AstNode], position: &CellPosition) -> EvalResult {
        if args.is_empty() {
            return EvalResult::Error {
                error: FormulaError::Value,
                origin: position.clone(),
                message: "IF requires at least 1 argument".to_string(),
            };
        }

        let condition = self.evaluate_ast(&args[0], position);
        let is_truthy = match condition {
            EvalResult::Boolean(b) => b,
            EvalResult::Number(n) => n != 0.0,
            EvalResult::String(s) => !s.is_empty(),
            _ => false,
        };

        if is_truthy && args.len() > 1 {
            self.evaluate_ast(&args[1], position)
        } else if args.len() > 2 {
            self.evaluate_ast(&args[2], position)
        } else {
            EvalResult::Boolean(false)
        }
    }

    // ... implement other helper methods
}
```

## Phase 5: Function Library

### 5.1 Function Categories

```rust
// engine/src/functions/mod.rs

pub mod logical;
pub mod mathematical;
pub mod statistical;
pub mod text;
pub mod date_time;
pub mod lookup;
pub mod financial;
pub mod engineering;
pub mod information;

pub enum FunctionCategory {
    Logical,
    Mathematical,
    Statistical,
    Text,
    DateTime,
    Lookup,
    Financial,
    Engineering,
    Information,
}
```

### 5.2 Implementing Functions

```rust
// engine/src/functions/mathematical.rs

use crate::eval::result::EvalResult;

pub fn abs(args: &[EvalResult]) -> EvalResult {
    if args.len() != 1 {
        return error_value("ABS requires 1 argument");
    }
    match &args[0] {
        EvalResult::Number(n) => EvalResult::Number(n.abs()),
        _ => error_value("ABS requires a number"),
    }
}

pub fn sum(args: &[EvalResult]) -> EvalResult {
    let mut sum = 0.0;
    for arg in args {
        if let EvalResult::Number(n) = arg {
            sum += n;
        }
    }
    EvalResult::Number(sum)
}

pub fn round(args: &[EvalResult]) -> EvalResult {
    if args.len() < 1 || args.len() > 2 {
        return error_value("ROUND requires 1 or 2 arguments");
    }
    match &args[0] {
        EvalResult::Number(n) => {
            let digits = if args.len() > 1 {
                match &args[1] {
                    EvalResult::Number(d) => *d as i32,
                    _ => return error_value("Second argument must be a number"),
                }
            } else {
                0
            };
            let factor = 10_f64.powi(digits);
            EvalResult::Number((n * factor).round() / factor)
        }
        _ => error_value("First argument must be a number"),
    }
}

fn error_value(msg: &str) -> EvalResult {
    EvalResult::Error {
        error: FormulaError::Value,
        origin: CellPosition { sheet: 0, row: 0, column: 0 },
        message: msg.to_string(),
    }
}
```

## Phase 6: Serialization

### 6.1 Binary Serialization with bitcode

```rust
// engine/src/serial.rs

use bitcode::{Encode, Decode};

#[derive(Encode, Decode)]
pub struct WorkbookBinary {
    pub version: u32,
    pub data: Vec<u8>,
}

pub fn to_bytes(workbook: &Workbook) -> Vec<u8> {
    bitcode::encode(workbook)
}

pub fn from_bytes(bytes: &[u8]) -> Result<Workbook, String> {
    bitcode::decode(bytes).map_err(|e| format!("Deserialization error: {}", e))
}
```

### 6.2 XLSX Export

```rust
// format/src/export/mod.rs

use zip::ZipWriter;
use std::io::Write;

pub fn save_to_xlsx(workbook: &Workbook, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let mut zip = ZipWriter::new(file);

    // Write [Content_Types].xml
    zip.start_file("[Content_Types].xml", zip::write::FileOptions::default())?;
    zip.write_all(generate_content_types(workbook).as_bytes())?;

    // Write docProps
    zip.start_file("docProps/app.xml", zip::write::FileOptions::default())?;
    zip.write_all(generate_app_xml(workbook).as_bytes())?;

    // Write xl/workbook.xml
    zip.start_file("xl/workbook.xml", zip::write::FileOptions::default())?;
    zip.write_all(generate_workbook_xml(workbook).as_bytes())?;

    // Write worksheets
    for (idx, sheet) in workbook.worksheets.iter().enumerate() {
        zip.start_file(
            format!("xl/worksheets/sheet{}.xml", idx + 1),
            zip::write::FileOptions::default(),
        )?;
        zip.write_all(generate_worksheet_xml(sheet).as_bytes())?;
    }

    // Write sharedStrings.xml
    zip.start_file("xl/sharedStrings.xml", zip::write::FileOptions::default())?;
    zip.write_all(generate_shared_strings_xml(workbook).as_bytes())?;

    // Write styles.xml
    zip.start_file("xl/styles.xml", zip::write::FileOptions::default())?;
    zip.write_all(generate_styles_xml(workbook).as_bytes())?;

    zip.finish()?;
    Ok(())
}
```

## Phase 7: WASM Bindings

### 7.1 WASM Module

```rust
// wasm/src/lib.rs

use wasm_bindgen::prelude::*;
use spreadsheet_engine::{Model, UserModel};

#[wasm_bindgen]
pub struct Spreadsheet {
    model: UserModel,
}

#[wasm_bindgen]
impl Spreadsheet {
    #[wasm_bindgen(constructor)]
    pub fn new(name: &str, locale: &str, timezone: &str) -> Result<Spreadsheet, JsError> {
        let model = UserModel::new_empty(name, locale, timezone)
            .map_err(|e| JsError::new(&e))?;
        Ok(Spreadsheet { model })
    }

    pub fn set_user_input(&mut self, sheet: u32, row: i32, col: i32, value: &str) -> Result<(), JsError> {
        self.model.set_user_input(sheet, row, col, value)
            .map_err(|e| JsError::new(&e))
    }

    pub fn get_formatted_cell_value(&self, sheet: u32, row: i32, col: i32) -> Result<String, JsError> {
        self.model.get_formatted_cell_value(sheet, row, col)
            .map_err(|e| JsError::new(&e))
    }

    pub fn evaluate(&mut self) {
        self.model.evaluate();
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.model.to_bytes()
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Spreadsheet, JsError> {
        let model = UserModel::from_bytes(bytes)
            .map_err(|e| JsError::new(&e))?;
        Ok(Spreadsheet { model })
    }

    pub fn undo(&mut self) -> Result<(), JsError> {
        self.model.undo().map_err(|e| JsError::new(&e))
    }

    pub fn redo(&mut self) -> Result<(), JsError> {
        self.model.redo().map_err(|e| JsError::new(&e))
    }

    pub fn pause_evaluation(&mut self) {
        self.model.pause_evaluation();
    }

    pub fn resume_evaluation(&mut self) {
        self.model.resume_evaluation();
    }
}
```

### 7.2 Build Configuration

```toml
# wasm/Cargo.toml

[package]
name = "spreadsheet-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"
spreadsheet_engine = { path = "../engine" }
```

## Production Considerations

### Performance Optimizations

1. **Sparse Storage**: Use HashMap for cell storage
2. **String Interning**: Shared strings for memory efficiency
3. **Formula Caching**: Cache parsed ASTs
4. **Lazy Evaluation**: Evaluate on-demand
5. **LTO**: Enable link-time optimization

### Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use crate::Model;

    #[test]
    fn test_basic_formulas() {
        let mut model = Model::new_empty("test", "en", "UTC").unwrap();
        model.set_user_input(0, 1, 1, "=2+2").unwrap();
        model.evaluate();
        assert_eq!(model.get_formatted_cell_value(0, 1, 1).unwrap(), "4");
    }

    #[test]
    fn test_circular_reference() {
        let mut model = Model::new_empty("test", "en", "UTC").unwrap();
        model.set_user_input(0, 1, 1, "=B2").unwrap();
        model.set_user_input(0, 1, 2, "=A1").unwrap();
        model.evaluate();
        // Should not hang
    }

    #[test]
    fn test_function_implementation() {
        let mut model = Model::new_empty("test", "en", "UTC").unwrap();
        for i in 1..=5 {
            model.set_user_input(0, i, 1, format!("{}", i)).unwrap();
        }
        model.set_user_input(0, 6, 1, "=SUM(A1:A5)").unwrap();
        model.evaluate();
        assert_eq!(model.get_formatted_cell_value(0, 6, 1).unwrap(), "15");
    }
}
```

### Dependencies Summary

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
bitcode = "0.6"
chrono = "0.4"
chrono-tz = "0.9"
regex = "1.0"
thiserror = "1.0"
zip = "0.6"
roxmltree = "0.19"
csv = "1.3"

[target.'cfg(target_arch = "wasm32")'.dependencies]
js-sys = "0.3"
wasm-bindgen = "0.2"

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
rand = "0.8"
```

## Implementation Checklist

- [ ] Core data model (Cell, Worksheet, Workbook)
- [ ] Styles system
- [ ] Formula lexer
- [ ] Formula parser (AST)
- [ ] Evaluation engine
- [ ] Cell dependency tracking
- [ ] Circular reference detection
- [ ] Function library (start with 20 most common)
- [ ] Number formatting
- [ ] Binary serialization
- [ ] XLSX import/export
- [ ] WASM bindings
- [ ] Undo/redo system
- [ ] Unit tests for all functions
- [ ] Integration tests
- [ ] Documentation

## Conclusion

Building a production spreadsheet engine is a significant undertaking. IronCalc demonstrates that Rust is an excellent choice due to:

1. **Memory safety**: No segfaults in calculation
2. **Performance**: Native speed for both native and WASM
3. **Type system**: Catch formula errors at compile time
4. **Serialization**: Efficient binary formats with bitcode

Start with the core data model and formula evaluation, then incrementally add features. The modular architecture allows you to build and test components independently.
