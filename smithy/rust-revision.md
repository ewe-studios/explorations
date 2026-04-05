---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.deployAnywhere/smithy
repository: github.com:smithy-lang/smithy
explored_at: 2026-04-04
focus: Building Smithy-like IDL and code generation in Rust
---

# Rust Revision: Building an IDL and Code Generation System in Rust

## Overview

This guide shows how to build a Smithy-like interface definition language (IDL) and code generation system in Rust. We cover parser combinators, AST design, symbol resolution, and code generation using templates and writers.

## Why Rust for IDL/Codegen?

| Feature | Smithy (Java) | Rust Equivalent |
|---------|---------------|-----------------|
| Parser | ANTLR | nom, logos, rowan |
| AST | Custom | Custom with arena allocation |
| Codegen | Java writers | askama, tera, proc-macro |
| Validation | Custom validators | Custom with thiserror |
| CLI | Smithy CLI | clap + cargo |

## Project Setup

### Cargo.toml

```toml
[package]
name = "idl-codegen"
version = "0.1.0"
edition = "2021"

[dependencies]
# Parser combinators
nom = "7.1"
logos = "0.13"              # Lexer generator
rowan = "0.15"              # Green tree IR (like rust-analyzer)

# AST and validation
thiserror = "1.0"
anyhow = "1.0"

# Code generation
askama = "0.12"             # Template engine
heck = "0.4"                # Case conversion
indoc = "2.0"               # Indentation handling

# CLI
clap = { version = "4.4", features = ["derive"] }

# Utilities
regex = "1.10"
once_cell = "1.19"
itertools = "0.12"

# File handling
walkdir = "2.4"
glob = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[dev-dependencies]
insta = "1.34"              # Snapshot testing
pretty_assertions = "1.4"
```

## Lexer with Logos

```rust
// src/lexer.rs

use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")]
#[logos(error = LexerError)]
pub enum Token {
    // Keywords
    #[token("service")]
    Service,
    
    #[token("resource")]
    Resource,
    
    #[token("operation")]
    Operation,
    
    #[token("structure")]
    Structure,
    
    #[token("union")]
    Union,
    
    #[token("list")]
    List,
    
    #[token("map")]
    Map,
    
    #[token("string")]
    String,
    
    #[token("integer")]
    Integer,
    
    #[token("long")]
    Long,
    
    #[token("float")]
    Float,
    
    #[token("double")]
    Double,
    
    #[token("boolean")]
    Boolean,
    
    #[token("timestamp")]
    Timestamp,
    
    #[token("blob")]
    Blob,
    
    #[token("enum")]
    Enum,
    
    #[token("trait")]
    Trait,
    
    #[token("null")]
    Null,
    
    #[token("true")]
    True,
    
    #[token("false")]
    False,
    
    // Punctuation
    #[token("{")]
    LBrace,
    
    #[token("}")]
    RBrace,
    
    #[token("[")]
    LBracket,
    
    #[token("]")]
    RBracket,
    
    #[token("(")]
    LParen,
    
    #[token(")")]
    RParen,
    
    #[token(":")]
    Colon,
    
    #[token("::")]
    NamespaceSeparator,
    
    #[token(",")]
    Comma,
    
    #[token("=")]
    Equals,
    
    #[token("@")]
    At,
    
    #[token("$")]
    Dollar,
    
    #[token("|>")]
    Pipe,
    
    // Identifier and literals
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,
    
    #[regex(r#""([^"\\]|\\.)*""#)]
    StringLiteral,
    
    #[regex(r"-?[0-9]+")]
    IntegerLiteral,
    
    #[regex(r"-?[0-9]+\.[0-9]+")]
    FloatLiteral,
    
    // Comments
    #[regex(r"//[^\n]*", logos::skip)]
    LineComment,
    
    #[regex(r"/\*[^*]*\*/", logos::skip)]
    BlockComment,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LexerError {
    InvalidString,
    InvalidNumber,
    UnexpectedCharacter(char),
}

/// Token with span information
pub struct SpannedToken {
    pub token: Token,
    pub span: std::ops::Range<usize>,
    pub source: String,
}

impl SpannedToken {
    pub fn lex(source: &str) -> Result<Vec<Self>, LexerError> {
        let mut lexer = Token::lexer(source);
        let mut tokens = Vec::new();
        
        while let Some(result) = lexer.next() {
            let token = result?;
            let span = lexer.span();
            
            tokens.push(SpannedToken {
                token,
                span: span.clone(),
                source: source[span.clone()].to_string(),
            });
        }
        
        Ok(tokens)
    }
}
```

## AST Definitions

```rust
// src/ast.rs

use std::collections::HashMap;
use std::fmt;

/// Complete Smithy-like model
#[derive(Debug, Clone)]
pub struct Model {
    pub namespace: Namespace,
    pub use_declarations: Vec<UseDeclaration>,
    pub metadata: HashMap<String, Value>,
    pub shapes: Vec<Shape>,
}

/// Namespace declaration
#[derive(Debug, Clone)]
pub struct Namespace {
    pub segments: Vec<String>,
}

impl Namespace {
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
    }
    
    pub fn as_string(&self) -> String {
        self.segments.join(".")
    }
}

/// Use/import declaration
#[derive(Debug, Clone)]
pub struct UseDeclaration {
    pub shape_id: ShapeId,
    pub alias: Option<String>,
}

/// Shape identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShapeId {
    pub namespace: Namespace,
    pub name: String,
}

impl ShapeId {
    pub fn new(namespace: Namespace, name: String) -> Self {
        Self { namespace, name }
    }
    
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        if let Some(colon_pos) = s.find('$') {
            let ns_str = &s[..colon_pos];
            let name = s[colon_pos + 1..].to_string();
            
            let segments: Vec<String> = ns_str
                .split('.')
                .map(|s| s.to_string())
                .collect();
            
            Ok(ShapeId {
                namespace: Namespace::new(segments),
                name,
            })
        } else {
            // No namespace
            Ok(ShapeId {
                namespace: Namespace::new(vec![]),
                name: s.to_string(),
            })
        }
    }
}

/// Shape definitions
#[derive(Debug, Clone)]
pub enum Shape {
    Service(ServiceShape),
    Resource(ResourceShape),
    Operation(OperationShape),
    Structure(StructureShape),
    Union(UnionShape),
    Enum(EnumShape),
    String(StringShape),
    Integer(IntegerShape),
    Long(LongShape),
    Float(FloatShape),
    Double(DoubleShape),
    Boolean(BooleanShape),
    Timestamp(TimestampShape),
    Blob(BlobShape),
    List(ListShape),
    Map(MapShape),
    Member(MemberShape),
}

impl Shape {
    pub fn id(&self) -> ShapeId {
        match self {
            Shape::Service(s) => s.id.clone(),
            Shape::Resource(r) => r.id.clone(),
            Shape::Operation(o) => o.id.clone(),
            Shape::Structure(st) => st.id.clone(),
            Shape::Union(u) => u.id.clone(),
            Shape::Enum(e) => e.id.clone(),
            Shape::String(s) => s.id.clone(),
            Shape::Integer(i) => i.id.clone(),
            _ => unimplemented!(),
        }
    }
    
    pub fn shape_type(&self) -> ShapeType {
        match self {
            Shape::Service(_) => ShapeType::Service,
            Shape::Resource(_) => ShapeType::Resource,
            Shape::Operation(_) => ShapeType::Operation,
            Shape::Structure(_) => ShapeType::Structure,
            Shape::Union(_) => ShapeType::Union,
            Shape::Enum(_) => ShapeType::Enum,
            Shape::String(_) => ShapeType::String,
            Shape::Integer(_) => ShapeType::Integer,
            Shape::Long(_) => ShapeType::Long,
            Shape::Float(_) => ShapeType::Float,
            Shape::Double(_) => ShapeType::Double,
            Shape::Boolean(_) => ShapeType::Boolean,
            Shape::Timestamp(_) => ShapeType::Timestamp,
            Shape::Blob(_) => ShapeType::Blob,
            Shape::List(_) => ShapeType::List,
            Shape::Map(_) => ShapeType::Map,
            Shape::Member(_) => ShapeType::Member,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeType {
    Service,
    Resource,
    Operation,
    Structure,
    Union,
    Enum,
    String,
    Integer,
    Long,
    Float,
    Double,
    Boolean,
    Timestamp,
    Blob,
    List,
    Map,
    Member,
}

/// Service shape
#[derive(Debug, Clone)]
pub struct ServiceShape {
    pub id: ShapeId,
    pub version: String,
    pub operations: Vec<ShapeId>,
    pub resources: Vec<ShapeId>,
    pub errors: Vec<ShapeId>,
    pub traits: Vec<Trait>,
    pub documentation: Option<String>,
}

/// Resource shape
#[derive(Debug, Clone)]
pub struct ResourceShape {
    pub id: ShapeId,
    pub identifiers: HashMap<String, ShapeId>,
    pub read: Option<ShapeId>,
    pub create: Option<ShapeId>,
    pub update: Option<ShapeId>,
    pub delete: Option<ShapeId>,
    pub list: Option<ShapeId>,
    pub resources: Vec<ShapeId>,  // Nested resources
    pub operations: Vec<ShapeId>,
    pub traits: Vec<Trait>,
    pub documentation: Option<String>,
}

/// Operation shape
#[derive(Debug, Clone)]
pub struct OperationShape {
    pub id: ShapeId,
    pub input: Option<ShapeId>,
    pub output: Option<ShapeId>,
    pub errors: Vec<ShapeId>,
    pub traits: Vec<Trait>,
    pub documentation: Option<String>,
}

/// Structure shape
#[derive(Debug, Clone)]
pub struct StructureShape {
    pub id: ShapeId,
    pub members: Vec<MemberShape>,
    pub mixins: Vec<ShapeId>,
    pub traits: Vec<Trait>,
    pub documentation: Option<String>,
}

/// Union shape (tagged union)
#[derive(Debug, Clone)]
pub struct UnionShape {
    pub id: ShapeId,
    pub variants: Vec<MemberShape>,
    pub traits: Vec<Trait>,
    pub documentation: Option<String>,
}

/// Enum shape
#[derive(Debug, Clone)]
pub struct EnumShape {
    pub id: ShapeId,
    pub values: Vec<EnumValue>,
    pub traits: Vec<Trait>,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnumValue {
    pub name: String,
    pub value: Option<String>,  // Explicit value
    pub traits: Vec<Trait>,
    pub documentation: Option<String>,
}

/// Member shape (for structures, unions, lists, maps)
#[derive(Debug, Clone)]
pub struct MemberShape {
    pub name: String,
    pub target: ShapeId,
    pub traits: Vec<Trait>,
    pub documentation: Option<String>,
}

/// Trait application
#[derive(Debug, Clone)]
pub struct Trait {
    pub id: ShapeId,
    pub value: Option<Value>,
}

/// Value representation
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Boolean(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

#[derive(Debug, Clone)]
pub enum Number {
    Integer(i64),
    Float(f64),
}
```

## Parser with nom

```rust
// src/parser.rs

use nom::{
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1, multispace0},
    sequence::tuple,
    branch::alt,
    multi::many0,
    combinator::map,
    IResult,
};

use crate::ast::*;
use crate::lexer::{Token, SpannedToken};

/// Parse a complete model
pub fn parse_model(source: &str) -> Result<Model, ParseError> {
    let tokens = SpannedToken::lex(source)?;
    parse_model_from_tokens(&tokens)
}

fn parse_model_from_tokens(tokens: &[SpannedToken]) -> Result<Model, ParseError> {
    let mut namespace = None;
    let mut use_declarations = Vec::new();
    let mut shapes = Vec::new();
    let mut metadata = HashMap::new();
    
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i].token {
            Token::Identifier => {
                if tokens[i].source == "namespace" {
                    // Parse namespace
                    let (ns, end) = parse_namespace(&tokens[i..])?;
                    namespace = Some(ns);
                    i = end;
                } else if tokens[i].source == "use" {
                    // Parse use declaration
                    let (use_decl, end) = parse_use(&tokens[i..])?;
                    use_declarations.push(use_decl);
                    i = end;
                } else if tokens[i].source == "metadata" {
                    // Parse metadata
                    let (key, value, end) = parse_metadata(&tokens[i..])?;
                    metadata.insert(key, value);
                    i = end;
                } else {
                    // Parse shape
                    let (shape, end) = parse_shape(&tokens[i..])?;
                    shapes.push(shape);
                    i = end;
                }
            }
            _ => i += 1,
        }
    }
    
    Ok(Model {
        namespace: namespace.unwrap_or(Namespace::new(vec![])),
        use_declarations,
        metadata,
        shapes,
    })
}

fn parse_namespace(tokens: &[SpannedToken]) -> Result<(Namespace, usize), ParseError> {
    // namespace com.example
    if tokens.len() < 2 {
        return Err(ParseError::UnexpectedEnd);
    }
    
    let mut segments = Vec::new();
    let mut i = 1;
    
    // Parse namespace segments
    while i < tokens.len() {
        match &tokens[i].token {
            Token::Identifier => {
                segments.push(tokens[i].source.clone());
                i += 1;
            }
            Token::NamespaceSeparator => {
                i += 1;
            }
            _ => break,
        }
    }
    
    Ok((Namespace::new(segments), i))
}

fn parse_shape(tokens: &[SpannedToken]) -> Result<(Shape, usize), ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::UnexpectedEnd);
    }
    
    // Check for traits
    let mut traits = Vec::new();
    let mut i = 0;
    
    while let Some(Token::At) = tokens.get(i).map(|t| &t.token) {
        let (trait_, end) = parse_trait(&tokens[i..])?;
        traits.push(trait_);
        i += end;
    }
    
    // Check for documentation
    let mut documentation = None;
    if let Some(Token::StringLiteral) = tokens.get(i).map(|t| &t.token) {
        documentation = Some(tokens[i].source.trim_matches('"').to_string());
        i += 1;
    }
    
    // Parse shape based on keyword
    let (shape, end) = match &tokens[i].token {
        Token::Service => parse_service(&tokens[i..])?,
        Token::Resource => parse_resource(&tokens[i..])?,
        Token::Operation => parse_operation(&tokens[i..])?,
        Token::Structure => parse_structure(&tokens[i..])?,
        Token::Union => parse_union(&tokens[i..])?,
        Token::Enum => parse_enum(&tokens[i..])?,
        Token::String => parse_string_shape(&tokens[i..])?,
        Token::Integer => parse_integer_shape(&tokens[i..])?,
        Token::List => parse_list_shape(&tokens[i..])?,
        Token::Map => parse_map_shape(&tokens[i..])?,
        _ => return Err(ParseError::UnexpectedToken(tokens[i].clone())),
    };
    
    Ok((shape, i + end))
}

fn parse_service(tokens: &[SpannedToken]) -> Result<(Shape, usize), ParseError> {
    // service Weather { ... }
    if tokens.len() < 4 {
        return Err(ParseError::UnexpectedEnd);
    }
    
    let mut i = 1;  // Skip 'service' keyword
    
    // Parse service name
    let name = match &tokens[i].token {
        Token::Identifier => tokens[i].source.clone(),
        _ => return Err(ParseError::ExpectedIdentifier),
    };
    i += 1;
    
    // Parse service body
    if !matches!(tokens.get(i).map(|t| &t.token), Some(Token::LBrace)) {
        return Err(ParseError::ExpectedLBrace);
    }
    i += 1;
    
    let mut operations = Vec::new();
    let mut resources = Vec::new();
    let mut errors = Vec::new();
    let mut version = String::new();
    
    // Parse service members
    while i < tokens.len() {
        if let Some(Token::RBrace) = tokens.get(i).map(|t| &t.token) {
            i += 1;
            break;
        }
        
        match &tokens[i].token {
            Token::Identifier => {
                let key = &tokens[i].source;
                
                if key == "version" {
                    // Parse version: "2024-01-01"
                    i += 1;
                    if let Some(Token::Equals) = tokens.get(i).map(|t| &t.token) {
                        i += 1;
                    }
                    if let Some(Token::StringLiteral) = tokens.get(i).map(|t| &t.token) {
                        version = tokens[i].source.trim_matches('"').to_string();
                        i += 1;
                    }
                } else if key == "operations" {
                    // Parse operations list
                    let (ops, end) = parse_shape_id_list(&tokens[i..])?;
                    operations = ops;
                    i += end;
                } else if key == "resources" {
                    let (res, end) = parse_shape_id_list(&tokens[i..])?;
                    resources = res;
                    i += end;
                } else if key == "errors" {
                    let (errs, end) = parse_shape_id_list(&tokens[i..])?;
                    errors = errs;
                    i += end;
                } else {
                    i += 1;  // Skip unknown
                }
            }
            _ => i += 1,
        }
    }
    
    let service = ServiceShape {
        id: ShapeId::new(Namespace::new(vec![]), name),
        version,
        operations,
        resources,
        errors,
        traits: Vec::new(),
        documentation: None,
    };
    
    Ok((Shape::Service(service), i))
}

fn parse_shape_id_list(tokens: &[SpannedToken]) -> Result<(Vec<ShapeId>, usize), ParseError> {
    // [Shape1, Shape2, ...]
    let mut i = 0;
    let mut ids = Vec::new();
    
    if !matches!(tokens.get(i).map(|t| &t.token), Some(Token::LBracket)) {
        return Err(ParseError::ExpectedLBracket);
    }
    i += 1;
    
    while i < tokens.len() {
        if let Some(Token::RBracket) = tokens.get(i).map(|t| &t.token) {
            i += 1;
            break;
        }
        
        if let Token::Identifier = &tokens[i].token {
            let id = ShapeId::parse(&tokens[i].source)?;
            ids.push(id);
            i += 1;
        }
        
        if let Some(Token::Comma) = tokens.get(i).map(|t| &t.token) {
            i += 1;
        }
    }
    
    Ok((ids, i))
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Unexpected end of input")]
    UnexpectedEnd,
    
    #[error("Unexpected token: {0:?}")]
    UnexpectedToken(SpannedToken),
    
    #[error("Expected identifier")]
    ExpectedIdentifier,
    
    #[error("Expected '{{'")]
    ExpectedLBrace,
    
    #[error("Expected '['")]
    ExpectedLBracket,
    
    #[error("Lexer error: {0:?}")]
    LexerError(#[from] crate::lexer::LexerError),
    
    #[error("Invalid shape ID: {0}")]
    InvalidShapeId(String),
}
```

## Symbol Resolution

```rust
// src/resolver.rs

use std::collections::{HashMap, HashSet};
use crate::ast::*;

/// Shape resolver - resolves shape references
pub struct Resolver {
    shapes: HashMap<ShapeId, Shape>,
    aliases: HashMap<String, ShapeId>,
    errors: Vec<ResolutionError>,
}

impl Resolver {
    pub fn new(model: Model) -> Self {
        let mut shapes = HashMap::new();
        let mut aliases = HashMap::new();
        
        for shape in model.shapes {
            let id = shape.id();
            shapes.insert(id.clone(), shape);
        }
        
        Self {
            shapes,
            aliases,
            errors: Vec::new(),
        }
    }
    
    /// Resolve all shape references
    pub fn resolve(&mut self) -> Result<(), Vec<ResolutionError>> {
        let mut errors = Vec::new();
        
        // Collect all shape IDs for validation
        let shape_ids: HashSet<ShapeId> = self.shapes.keys().cloned().collect();
        
        // Validate all references
        for (id, shape) in &self.shapes {
            let refs = self.collect_references(shape);
            
            for ref_id in refs {
                if !shape_ids.contains(&ref_id) && !self.aliases.contains_key(&ref_id.name) {
                    errors.push(ResolutionError::UndefinedShape {
                        shape: id.clone(),
                        reference: ref_id,
                    });
                }
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Get shape by ID
    pub fn get(&self, id: &ShapeId) -> Option<&Shape> {
        self.shapes.get(id)
    }
    
    /// Get shape by name (with namespace resolution)
    pub fn get_by_name(&self, name: &str) -> Option<&Shape> {
        // First try as fully qualified
        if let Ok(id) = ShapeId::parse(name) {
            if let Some(shape) = self.shapes.get(&id) {
                return Some(shape);
            }
        }
        
        // Try alias
        if let Some(id) = self.aliases.get(name) {
            return self.shapes.get(id);
        }
        
        None
    }
    
    /// Collect all shape references from a shape
    fn collect_references(&self, shape: &Shape) -> Vec<ShapeId> {
        let mut refs = Vec::new();
        
        match shape {
            Shape::Service(s) => {
                refs.extend(s.operations.clone());
                refs.extend(s.resources.clone());
                refs.extend(s.errors.clone());
            }
            Shape::Resource(r) => {
                if let Some(id) = &r.read { refs.push(id.clone()); }
                if let Some(id) = &r.create { refs.push(id.clone()); }
                if let Some(id) = &r.update { refs.push(id.clone()); }
                if let Some(id) = &r.delete { refs.push(id.clone()); }
                if let Some(id) = &r.list { refs.push(id.clone()); }
                refs.extend(r.resources.clone());
                refs.extend(r.operations.clone());
                refs.extend(r.identifiers.values().cloned());
            }
            Shape::Operation(o) => {
                if let Some(id) = &o.input { refs.push(id.clone()); }
                if let Some(id) = &o.output { refs.push(id.clone()); }
                refs.extend(o.errors.clone());
            }
            Shape::Structure(s) => {
                for member in &s.members {
                    refs.push(member.target.clone());
                }
                refs.extend(s.mixins.clone());
            }
            Shape::Union(u) => {
                for variant in &u.variants {
                    refs.push(variant.target.clone());
                }
            }
            Shape::List(l) => {
                // Member target handled in MemberShape
            }
            Shape::Map(m) => {
                // Key and value targets handled in MemberShape
            }
            _ => {}
        }
        
        refs
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResolutionError {
    #[error("Undefined shape {reference} referenced from {shape}")]
    UndefinedShape {
        shape: ShapeId,
        reference: ShapeId,
    },
    
    #[error("Circular dependency detected")]
    CircularDependency,
    
    #[error("Duplicate shape: {0}")]
    DuplicateShape(ShapeId),
}
```

## Code Generation

```rust
// src/codegen.rs

use askama::Template;
use std::collections::HashMap;

/// Code generator trait
pub trait CodeGenerator {
    fn generate(&self, model: &Model) -> Result<GeneratedCode, CodegenError>;
}

/// Generated code output
pub struct GeneratedCode {
    pub files: Vec<GeneratedFile>,
}

pub struct GeneratedFile {
    pub path: String,
    pub content: String,
}

/// Rust code generator
pub struct RustCodeGenerator {
    settings: RustSettings,
}

#[derive(Debug, Clone)]
pub struct RustSettings {
    pub crate_name: String,
    pub module_path: String,
    pub derive_debug: bool,
    pub derive_clone: bool,
    pub derive_serde: bool,
}

impl Default for RustSettings {
    fn default() -> Self {
        Self {
            crate_name: "generated".to_string(),
            module_path: "crate::model".to_string(),
            derive_debug: true,
            derive_clone: true,
            derive_serde: false,
        }
    }
}

impl RustCodeGenerator {
    pub fn new(settings: RustSettings) -> Self {
        Self { settings }
    }
}

impl CodeGenerator for RustCodeGenerator {
    fn generate(&self, model: &Model) -> Result<GeneratedCode, CodegenError> {
        let mut files = Vec::new();
        
        // Generate lib.rs
        files.push(GeneratedFile {
            path: "lib.rs".to_string(),
            content: self.generate_lib(model)?,
        });
        
        // Generate types module
        files.push(GeneratedFile {
            path: "types.rs".to_string(),
            content: self.generate_types(model)?,
        });
        
        // Generate client module
        files.push(GeneratedFile {
            path: "client.rs".to_string(),
            content: self.generate_client(model)?,
        });
        
        Ok(GeneratedCode { files })
    }
}

impl RustCodeGenerator {
    fn generate_lib(&self, model: &Model) -> Result<String, CodegenError> {
        let mut output = String::new();
        
        output.push_str("// Generated code - do not edit\n\n");
        output.push_str("pub mod types;\n");
        output.push_str("pub mod client;\n");
        output.push_str("\n");
        output.push_str("pub use types::*;\n");
        output.push_str("pub use client::Client;\n");
        
        Ok(output)
    }
    
    fn generate_types(&self, model: &Model) -> Result<String, CodegenError> {
        use heck::ToPascalCase;
        
        let mut output = String::new();
        output.push_str("// Generated type definitions\n\n");
        
        if self.settings.derive_serde {
            output.push_str("use serde::{Deserialize, Serialize};\n\n");
        }
        
        for shape in &model.shapes {
            match shape {
                Shape::Structure(s) => {
                    let name = s.id.name.to_pascal_case();
                    
                    // Derive macros
                    output.push_str("#[derive(");
                    if self.settings.derive_debug { output.push_str("Debug, "); }
                    if self.settings.derive_clone { output.push_str("Clone, "); }
                    if self.settings.derive_serde { output.push_str("Serialize, Deserialize, "); }
                    output.push_str(")]\n");
                    
                    // Struct definition
                    output.push_str("pub struct ");
                    output.push_str(&name);
                    output.push_str(" {\n");
                    
                    for member in &s.members {
                        let field_name = member.name.to_snake_case();
                        let field_type = self.rust_type(&member.target);
                        output.push_str(&format!("    pub {}: {},\n", field_name, field_type));
                    }
                    
                    output.push_str("}\n\n");
                }
                
                Shape::Enum(e) => {
                    let name = e.id.name.to_pascal_case();
                    
                    output.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n");
                    output.push_str("pub enum ");
                    output.push_str(&name);
                    output.push_str(" {\n");
                    
                    for value in &e.values {
                        let variant = value.name.to_pascal_case();
                        output.push_str(&format!("    {},\n", variant));
                    }
                    
                    output.push_str("}\n\n");
                }
                
                _ => {}
            }
        }
        
        Ok(output)
    }
    
    fn generate_client(&self, model: &Model) -> Result<String, CodegenError> {
        use heck::ToPascalCase;
        
        let mut output = String::new();
        output.push_str("// Generated client code\n\n");
        output.push_str("use crate::types::*;\n\n");
        
        // Find service shape
        for shape in &model.shapes {
            if let Shape::Service(service) = shape {
                let service_name = service.id.name.to_pascal_case();
                
                output.push_str("pub struct Client {\n");
                output.push_str("    base_url: String,\n");
                output.push_str("    client: reqwest::Client,\n");
                output.push_str("}\n\n");
                
                output.push_str("impl Client {\n");
                output.push_str("    pub fn new(base_url: impl Into<String>) -> Self {\n");
                output.push_str("        Self {\n");
                output.push_str("            base_url: base_url.into(),\n");
                output.push_str("            client: reqwest::Client::new(),\n");
                output.push_str("        }\n");
                output.push_str("    }\n\n");
                
                // Generate operation methods
                for op_id in &service.operations {
                    // Find operation shape
                    // (In real code, resolve the ID)
                    output.push_str(&format!("    pub async fn {}(&self) -> Result<(), reqwest::Error> {{\n", op_id.name.to_snake_case()));
                    output.push_str("        // TODO: implement\n");
                    output.push_str("        Ok(())\n");
                    output.push_str("    }\n\n");
                }
                
                output.push_str("}\n");
            }
        }
        
        Ok(output)
    }
    
    fn rust_type(&self, shape_id: &ShapeId) -> String {
        use heck::ToPascalCase;
        
        match shape_id.name.as_str() {
            "String" => "String".to_string(),
            "Integer" => "i32".to_string(),
            "Long" => "i64".to_string(),
            "Float" => "f32".to_string(),
            "Double" => "f64".to_string(),
            "Boolean" => "bool".to_string(),
            "Timestamp" => "time::PrimitiveDateTime".to_string(),
            "Blob" => "bytes::Bytes".to_string(),
            _ => shape_id.name.to_pascal_case(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CodegenError {
    #[error("Template error: {0}")]
    Template(#[from] askama::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Conclusion

Building a Smithy-like system in Rust provides:

1. **Type Safety**: Compile-time guarantees for AST
2. **Performance**: Fast parsing with nom/logos
3. **Template Safety**: askama compile-time templates
4. **CLI Tools**: clap for ergonomic command-line interfaces
5. **Testing**: Snapshot testing with insta
