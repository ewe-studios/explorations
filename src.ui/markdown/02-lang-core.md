# OpenUI -- Lang Core (Lexer and AST)

OpenUI Lang is a custom DSL with its own lexer, parser, AST, and evaluator. The lexer produces 36 token types from source text, the parser builds an AST, and the evaluator interprets it. The DSL is designed for LLM output — compact, streaming-friendly, and unambiguous.

**Aha:** The lexer uses a `const enum` for token types — this is erased at compile time to literal numbers in JavaScript, so there is zero runtime cost for token type comparison. `T.Ident` becomes just `3` in the output. This matters because the lexer runs on every character of streamed input, and token type comparisons happen thousands of times per second during active streaming.

Source: `openui/packages/lang-core/src/parser/lexer.ts` — hand-written tokenizer
Source: `openui/packages/lang-core/src/parser/ast.ts` — AST types

## Lexer

The lexer is a hand-written recursive scanner that processes source text character by character:

```typescript
const enum T {
  Newline = 0,
  LParen = 1,
  RParen = 2,
  Ident = 3,
  Type = 4,
  StateVar = 5,       // $name
  BuiltinCall = 6,    // @function
  // ... 36 token types total
}
```

### Token Types (Key Categories)

| Token Type | Example | Purpose |
|-----------|---------|---------|
| `Ident` | `Button`, `label` | Component and prop names |
| `Type` | `string`, `number` | Type annotations |
| `StateVar` | `$count` | Reactive state variable references |
| `BuiltinCall` | `@Count($items)` | Built-in function calls |
| `String` | `"hello"` | String literals with escape sequences |
| `Number` | `42`, `-3.14` | Numeric literals |
| `Bool` | `true`, `false` | Boolean literals |
| `Null` | `null` | Null literal |
| `LParen`/`RParen` | `(`, `)` | Expression grouping |
| `LBrack`/`RBrack` | `[`, `]` | Array literals |
| `LBrace`/`RBrace` | `{`, `}` | Object literals |
| Individual operators | `+`, `-`, `*`, `/`, `EqEq`, `And`, `Or` | Binary operators |

### Lexer Implementation

```typescript
// lexer.ts — simplified
function tokenize(source: string): Token[] {
  const tokens: Token[] = [];
  let pos = 0;

  while (pos < source.length) {
    const ch = source[pos];

    if (ch === '$') {
      tokens.push({ type: T.StateVar, value: scanStateVar() });
    } else if (ch === '@') {
      tokens.push({ type: BuiltinCall, value: scanBuiltin() });
    } else if (ch === '"') {
      tokens.push({ type: String, value: scanString() });
    } else if (isDigit(ch) || (ch === '-' && isDigit(source[pos + 1]))) {
      tokens.push({ type: Number, value: scanNumber() });
    } else if (isAlpha(ch)) {
      const word = scanIdent();
      tokens.push({ type: isKeyword(word) ? Type : Ident, value: word });
    } else if (ch === '\n') {
      tokens.push({ type: Newline });
      pos++;
    } else {
      // Single-character tokens: ( ) [ ] { } + - * == && ...
      tokens.push({ type: singleCharToken(ch) });
      pos++;
    }
  }

  return tokens;
}
```

**Aha:** The lexer handles negative number disambiguation: `-3` is a single Number token, but `x - 3` is an Ident `x` followed by a BinOp `-` and a Number `3`. The lexer checks if the preceding token could be the left-hand side of a binary operation — if not, the `-` is part of the number.

## AST

Source: `openui/packages/lang-core/src/ast.ts`

The AST is a discriminated union with the `k` field as the discriminator:

```typescript
type ASTNode =
  | { k: 'Comp'; n: string; args: ASTNode[]; mappedProps?: ... }
  | { k: 'Str'; v: string }
  | { k: 'Num'; v: number }
  | { k: 'Bool'; v: boolean }
  | { k: 'Null' }
  | { k: 'Arr'; items: ASTNode[] }
  | { k: 'Obj'; entries: { key: string; value: ASTNode }[] }
  | { k: 'Ref'; n: string }
  | { k: 'Ph'; n: string }  // Placeholder (unresolved during streaming)
  | { k: 'StateRef'; n: string }
  | { k: 'RuntimeRef'; refType: 'query' | 'mutation'; n: string }
  | { k: 'BinOp'; op: string; left: ASTNode; right: ASTNode }
  | { k: 'UnaryOp'; op: string; operand: ASTNode }
  | { k: 'Ternary'; cond: ASTNode; then: ASTNode; else: ASTNode }
  | { k: 'Member'; obj: ASTNode; field: string }
  | { k: 'Index'; object: ASTNode; index: ASTNode }
  | { k: 'Assign'; target: string; value: ASTNode };
```

### Expression vs Statement Nodes

Statements are a typed union (not a flat struct):

```typescript
type Statement =
  | { kind: 'value'; id: string; expr: ASTNode }
  | { kind: 'state'; id: string; expr: ASTNode }
  | { kind: 'query'; id: string; expr: ASTNode }
  | { kind: 'mutation'; id: string; expr: ASTNode };
```

A statement is a component call or a top-level expression. Component calls have a `name`; expressions don't.

### Placeholder Nodes

During streaming, the parser may encounter incomplete expressions:

```
<Button label=$cou
```

The parser can't resolve `$cou` to a complete state variable (it might be `$count` or `$course`). Instead of failing, it creates a `Ph` (placeholder) node:

```typescript
{ k: 'Comp', n: 'Button', args: [{ k: 'Obj', entries: [{ key: 'label', value: { k: 'Ph', n: 'count' } }] }] }
```

Placeholders are re-parsed on the next push when more characters arrive.

**Aha:** The streaming parser never produces invalid ASTs. When it encounters incomplete input, it creates placeholder nodes instead of failing. This means the renderer always has a valid (possibly incomplete) tree to display. The renderer handles placeholders by rendering nothing or a loading indicator for that prop.

## Parser

Source: `openui/packages/lang-core/src/parser/parser.ts`, `parser/expressions.ts`, `parser/statements.ts`

The parser is a recursive descent parser with two modes:

### Full Parse

```typescript
function parse(source: string): ParseResult {
  const tokens = tokenize(source);
  return parseStatements(tokens);
}
```

### Streaming Parse

```typescript
function createStreamParser(): StreamParser {
  let buffer = '';
  let completedEnd = 0;  // Watermark
  let completedStatements: Statement[] = [];
  let pending: Statement[] = [];

  return {
    push(text: string) {
      buffer += text;
      // Scan from watermark for complete statements
      const { completed, pending } = scanFromWatermark(buffer, completedEnd);
      completedStatements = completed;
      this.pending = pending;
      completedEnd = buffer.length - pendingSource.length;
    },
    buildResult(): ParseResult {
      return mergeStatements(completedStatements, pending);
    }
  };
}
```

The streaming parser caches completed statements and only re-parses the pending (incomplete) portion on each push.

See [Streaming Parser](03-streaming-parser.md) for the detailed streaming algorithm.
See [Materializer](04-materializer.md) for how the AST is resolved and validated.
See [Evaluator](05-evaluator.md) for how expressions are interpreted.
