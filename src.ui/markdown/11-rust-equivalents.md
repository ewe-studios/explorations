# OpenUI -- Rust Equivalents

This document maps every OpenUI concept to Rust equivalents, showing how to replicate the parser, streaming, materialization, and rendering patterns in Rust.

**Aha:** The most natural Rust equivalent for the streaming parser is `nom` or `winnow` — parser combinator libraries that support incremental parsing. However, for a hand-written approach (matching OpenUI's style), you'd use a `Reader` trait that supports `read_partial()` and a state machine that tracks parse progress. The key difference from TypeScript: Rust can't create `Function` objects at runtime, so expression evaluation must use an explicit AST interpreter (which OpenUI already does — the evaluator is already an interpreter, not a compiler).

## Lexer

```rust
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Newline,
    LParen,
    RParen,
    Ident(String),
    Type(String),
    StateVar(String),
    BuiltinCall(String),
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    BinOp(String),
    // ... 36 token types
}

struct Lexer<'a> {
    source: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn next_token(&mut self) -> Option<Token> {
        let ch = self.source.chars().nth(self.pos)?;
        match ch {
            '$' => Some(self.scan_state_var()),
            '@' => Some(self.scan_builtin()),
            '"' => Some(self.scan_string()),
            c if c.is_ascii_digit() || (c == '-' && self.peek().is_some_and(|c| c.is_ascii_digit())) => {
                Some(self.scan_number())
            }
            c if c.is_ascii_alphabetic() => Some(self.scan_ident()),
            '\n' => { self.pos += 1; Some(Token::Newline) }
            '(' => { self.pos += 1; Some(Token::LParen) }
            ')' => { self.pos += 1; Some(Token::RParen) }
            _ => { self.pos += 1; single_char_token(ch) }
        }
    }
}
```

**Production pattern:** Use `logos` for a derived lexer:

```rust
use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
enum Token {
    #[token("\n")]
    Newline,
    #[regex(r"\$[a-zA-Z_][a-zA-Z0-9_]*")]
    StateVar,
    #[regex(r"@[a-zA-Z_][a-zA-Z0-9_]*")]
    BuiltinCall,
    #[regex(r#""[^"]*""#)]
    Str,
    #[regex(r"-?[0-9]+(\.[0-9]+)?")]
    Num,
    // ...
}
```

`logos` generates an efficient DFA-based lexer from regex patterns.

## Streaming Parser

```rust
struct StreamParser {
    buffer: String,
    watermark: usize,
    completed: Vec<Statement>,
}

impl StreamParser {
    fn push(&mut self, text: &str) {
        self.buffer.push_str(text);
        // Scan from watermark for newline at depth 0
        if let Some(pos) = self.find_complete_boundary() {
            let completed_source = &self.buffer[self.watermark..=pos];
            let statements = parse_statements(completed_source);
            self.completed.extend(statements);
            self.watermark = pos + 1;
        }
    }

    fn build_result(&self) -> ParseResult {
        let pending_source = &self.buffer[self.watermark..];
        let pending = parse_statements_allow_incomplete(pending_source);
        ParseResult {
            statements: self.completed.iter().chain(&pending).cloned().collect(),
        }
    }
}
```

**Production pattern:** Use `winnow` for incremental parsing:

```rust
use winnow::stream::ReplayableStream;

fn parse_streaming(input: &mut impl ReplayableStream) -> PResult<Vec<Statement>> {
    repeat(0.., parse_statement).parse_next(input)
}
```

## AST

```rust
#[derive(Debug, Clone)]
enum ASTNode {
    Comp { name: String, args: Vec<ASTNode>, mapped_props: Option<HashMap<String, ASTNode>> },
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
    Arr(Vec<ASTNode>),
    Obj(Vec<(String, ASTNode)>),         // Tuple pairs, not struct entries
    Ref(String),
    Placeholder(String),                  // Ph — unresolvable reference
    StateRef(String),                     // $variable
    RuntimeRef { name: String, ref_type: RefType },  // Query/Mutation result reference
    BinOp { op: String, left: Box<ASTNode>, right: Box<ASTNode> },
    UnaryOp { op: String, operand: Box<ASTNode> },
    Ternary { cond: Box<ASTNode>, then: Box<ASTNode>, else_: Box<ASTNode> },
    Member { obj: Box<ASTNode>, field: String },
    Index { obj: Box<ASTNode>, index: Box<ASTNode> },
    Assign { target: String, value: Box<ASTNode> },
}

#[derive(Debug, Clone)]
enum RefType { Query, Mutation }
```

## Materializer

```rust
struct MaterializeCtx<'a> {
    syms: HashMap<String, &'a ASTNode>,  // Statement id → AST
    cat: &'a ParamMap,                    // Component param definitions
    errors: Vec<ValidationError>,
    unresolved: Vec<String>,
    visited: HashSet<String>,             // Cycle detection
}

impl<'a> MaterializeCtx<'a> {
    fn resolve_ref(&mut self, name: &str) -> Option<ElementNode> {
        if self.visited.contains(name) {
            self.unresolved.push(name.to_string());
            return None;  // Cycle → placeholder
        }
        let target = self.syms.get(name)?;
        self.visited.insert(name.to_string());
        let result = self.materialize_value(target);
        self.visited.remove(name);
        result
    }

    fn materialize_value(&mut self, node: &'a ASTNode) -> Option<ElementNode> {
        match node {
            ASTNode::Comp { name, args, .. } => {
                let def = self.cat.get(name)?;
                let props = self.map_positional_to_named(args, &def.params);
                Some(ElementNode { type_name: name.clone(), props, partial: false })
            }
            ASTNode::Ref(name) => self.resolve_ref(name),
            // ... other cases
        }
    }
}
```

## Evaluator

```rust
fn evaluate(node: &ElementNode, ctx: &EvalContext) -> Result<Value, Error> {
    match node {
        ElementNode::Str(s) => Ok(Value::Str(s.clone())),
        ElementNode::Num(n) => Ok(Value::Num(*n)),
        ElementNode::StateRef(name) => ctx.store.get(name),
        ElementNode::BinOp { op, left, right } => {
            let l = evaluate(left, ctx)?;
            let r = evaluate(right, ctx)?;
            eval_bin_op(op, l, r)
        }
        // ...
    }
}
```

## React Renderer → Rust Renderer

For a native Rust UI framework, use `iced` or `dioxus`:

```rust
// Dioxus (React-like Rust UI framework)
fn App(cx: Scope) -> Element {
    let state = use_ref(cx, || HashMap::new());
    let statements = parse_and_materialize(state.read().as_str());

    cx.render(rsx! {
        for node in statements {
            render_node(node, state)
        }
    })
}
```

## Cross-Reference Table

| OpenUI Concept | TypeScript | Rust Equivalent |
|---------------|-----------|----------------|
| Lexer | Hand-written `tokenize()` | `logos` derived lexer |
| Streaming parser | Watermark + re-parse | `winnow` incremental parser |
| AST | Discriminated union (`k` field) | Rust `enum` |
| Materializer | Schema-aware lowering | Pattern matching + HashMap lookup |
| Evaluator | Recursive `evaluate()` function | Same — Rust enum pattern matching |
| Store | Map + subscribe/notify | `Arc<RwLock<HashMap>>` + channel |
| Component library | JSON Schema + React components | `Component` trait + schema crate |
| WebSocket RPC | GatewaySocket class | `tokio-tungstenite` + serde |

See [Lang Core](02-lang-core.md) for the TypeScript implementation.
See [Streaming Parser](03-streaming-parser.md) for the streaming algorithm.
See [Production Patterns](12-production-patterns.md) for broader production considerations.
