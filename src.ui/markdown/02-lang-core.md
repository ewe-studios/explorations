# OpenUI -- Lang Core (Lexer and AST)

OpenUI Lang is a custom DSL with its own lexer, parser, AST, and evaluator. The lexer produces 36 token types from source text, the parser builds an AST, and the evaluator interprets it. The DSL is designed for LLM output — compact, streaming-friendly, and unambiguous.

**Aha:** The lexer uses a `const enum` for token types — this is erased at compile time to literal numbers in JavaScript, so there is zero runtime cost for token type comparison. `T.Ident` becomes just `16` in the output. This matters because the lexer runs on every character of streamed input, and token type comparisons happen thousands of times per second during active streaming.

Source: `openui/packages/lang-core/src/parser/lexer.ts` — hand-written tokenizer
Source: `openui/packages/lang-core/src/parser/tokens.ts` — token type enum
Source: `openui/packages/lang-core/src/parser/ast.ts` — AST types

## Lexer

The lexer is a hand-written recursive scanner that processes source text character by character:

```typescript
const enum T {
  Newline = 0,
  LParen = 1,   // (
  RParen = 2,   // )
  LBrack = 3,   // [
  RBrack = 4,   // ]
  LBrace = 5,   // {
  RBrace = 6,   // }
  Comma = 7,    // ,
  Colon = 8,    // :
  Equals = 9,   // =
  True = 10,
  False = 11,
  Null = 12,
  EOF = 13,
  Str = 14,     // string literal
  Num = 15,     // numeric literal
  Ident = 16,   // lowercase identifier → reference
  Type = 17,    // PascalCase identifier → component name
  StateVar = 18, // $identifier → reactive state
  // ... operators 19-34
  BuiltinCall = 35, // @identifier → builtin function
}

type Token = { t: T; v?: string | number };
```

### Token Types (Key Categories)

| Token Type | Example | Purpose |
|-----------|---------|---------|
| `Type` | `Button`, `Stack` | PascalCase identifier — component name or type reference |
| `Ident` | `label`, `myTable` | Lowercase identifier — variable reference |
| `StateVar` | `$count` | Reactive state variable reference (`$` prefix) |
| `BuiltinCall` | `@Count` | Built-in function call (`@` prefix) |
| `Str` | `"hello"` | String literal (both `"` and `'` quotes) |
| `Num` | `42`, `-3.14` | Numeric literal (integers, floats, exponents) |
| `True`/`False` | `true`, `false` | Boolean literals |
| `Null` | `null` | Null literal |
| `LParen`/`RParen` | `(`, `)` | Component arguments / grouping |
| `LBrack`/`RBrack` | `[`, `]` | Array literals |
| `LBrace`/`RBrace` | `{`, `}` | Object literals |
| Operators | `+`, `-`, `*`, `/`, `==`, `!=`, `&&`, `||`, `?` | Binary/unary/ternary operators |

### Lexer Implementation

```typescript
// lexer.ts — simplified from actual implementation
function tokenize(src: string): Token[] {
  const tokens: Token[] = [];
  let i = 0;

  while (i < src.length) {
    const c = src[i];

    // Skip horizontal whitespace (not newlines — they're statement separators)
    if (c === ' ' || c === '\t' || c === '\r') { i++; continue; }

    if (c === '\n') { tokens.push({ t: T.Newline }); i++; continue; }

    // Single-char punctuation
    if (c === '(') { tokens.push({ t: T.LParen }); i++; continue; }
    // ... same for ), [, ], {, }, ','

    // State variable: $identifier
    if (c === '$') { tokens.push({ t: T.StateVar, v: scanStateVar() }); continue; }

    // Builtin call: @identifier
    if (c === '@') { tokens.push({ t: T.BuiltinCall, v: scanBuiltin() }); continue; }

    // Strings: "..." (JSON.parse for unescaping) and '...' (manual unescape)
    if (c === '"' || c === "'") { tokens.push({ t: T.Str, v: scanString() }); continue; }

    // Identifier or keyword
    if (isAlpha(c)) {
      const word = scanWord();
      if (word === 'true') tokens.push({ t: T.True });
      else if (word === 'false') tokens.push({ t: T.False });
      else if (word === 'null') tokens.push({ t: T.Null });
      // PascalCase → Type (component), lowercase → Ident (reference)
      else tokens.push({ t: c >= 'A' && c <= 'Z' ? T.Type : T.Ident, v: word });
      continue;
    }

    // Number: 42, -3.14, 1e10
    if (isDigit(c) || (c === '-' && isNegativeNumber())) {
      tokens.push({ t: T.Num, v: scanNumber() });
      continue;
    }
  }
  return tokens;
}
```

**Aha:** The lexer handles negative number disambiguation: `-3` is a single Num token, but `x - 3` is an Ident `x` followed by a Minus and a Num `3`. The lexer checks if the preceding token could be the right-hand side of a binary operation (`Num`, `Str`, `Ident`, `Type`, `RParen`, `RBrack`, `True`, `False`, `Null`, `StateVar`, `BuiltinCall`) — if so, `-` is a subtraction operator; otherwise, it's part of a negative number literal.

## AST

Source: `openui/packages/lang-core/src/parser/ast.ts`

The AST is a discriminated union with the `k` field as the discriminator:

```typescript
type ASTNode =
  | { k: "Comp"; name: string; args: ASTNode[]; mappedProps?: Record<string, ASTNode> }
  | { k: "Str"; v: string }
  | { k: "Num"; v: number }
  | { k: "Bool"; v: boolean }
  | { k: "Null" }
  | { k: "Arr"; els: ASTNode[] }
  | { k: "Obj"; entries: [string, ASTNode][] }
  | { k: "Ref"; n: string }
  | { k: "Ph"; n: string }       // Placeholder (unresolved during streaming)
  | { k: "StateRef"; n: string }
  | { k: "RuntimeRef"; n: string; refType: "query" | "mutation" }
  | { k: "BinOp"; op: string; left: ASTNode; right: ASTNode }
  | { k: "UnaryOp"; op: string; operand: ASTNode }
  | { k: "Ternary"; cond: ASTNode; then: ASTNode; else: ASTNode }
  | { k: "Member"; obj: ASTNode; field: string }
  | { k: "Index"; obj: ASTNode; index: ASTNode }
  | { k: "Assign"; target: string; value: ASTNode };
```

### Expression vs Statement Nodes

Statements are a typed union classified at parse time from token type and expression shape:

```typescript
interface CallNode { callee: string; args: ASTNode[] }

type Statement =
  | { kind: "value"; id: string; expr: ASTNode }
  | { kind: "state"; id: string; init: ASTNode }
  | { kind: "query"; id: string; call: CallNode; expr: ASTNode; deps?: string[] }
  | { kind: "mutation"; id: string; call: CallNode; expr: ASTNode };
```

Classification rules (determined in `classifyStatement`):
- If the expression is `Comp` with `name === "Query"` → `query` (extracts `CallNode`, pre-computes `$var` deps)
- If the expression is `Comp` with `name === "Mutation"` → `mutation` (extracts `CallNode`)
- If the identifier token was `$variable` → `state` (field is `init`, not `expr`)
- Everything else → `value`

### Placeholder Nodes

During streaming, the parser may encounter references to identifiers not yet defined:

```
root = Stack(header, table)
header = TextContent("Title")
// table not yet streamed...
```

The materializer can't resolve `table` because it doesn't exist yet. Instead of failing, it creates a `Ph` (placeholder) node:

```typescript
{ k: "Ph", n: "table" }
```

Placeholders also arise when a `Ref` node points to an identifier that creates a cycle (detected by the `visited` set in the materializer). They are replaced with real nodes on subsequent pushes when more text arrives.

**Aha:** The streaming parser never produces invalid ASTs. When the materializer encounters an unresolvable reference, it creates a placeholder node and records it in `meta.unresolved`. This means the renderer always has a valid (possibly incomplete) tree to display. The `Ph` nodes render as nothing — they're invisible gaps that fill in as the stream progresses.

## Parser

Source: `openui/packages/lang-core/src/parser/parser.ts`, `parser/expressions.ts`, `parser/statements.ts`

The parser has two modes: one-shot and streaming.

### Full Parse

```typescript
// parse() preprocesses (strips markdown fences, comments), autoclosing incomplete brackets,
// then splits into statements, parses expressions, classifies, and materializes.
function parse(input: string, cat: ParamMap, rootName?: string): ParseResult {
  const trimmed = preprocess(input);         // stripFences + stripComments
  const { text, wasIncomplete } = autoClose(trimmed);  // balance brackets
  const stmts = split(tokenize(text));       // split on depth-0 newlines
  // classifyStatement → buildResult → materializeValue → return
}
```

### Streaming Parse

```typescript
interface StreamParser {
  push(chunk: string): ParseResult;   // Append chunk, return latest result
  set(fullText: string): ParseResult; // Set full text (resets if not prefix-compatible)
  getResult(): ParseResult;           // Get latest result without new data
}

function createStreamParser(cat: ParamMap, rootName?: string): StreamParser {
  let buf = "";
  let completedEnd = 0;                        // Watermark
  const completedStmtMap = new Map<string, Statement>();

  // On each push: append to buffer, scan from watermark for depth-0 newlines,
  // parse completed statements into cache, re-parse only the pending tail.
}
```

The streaming parser caches completed statements in a `Map<string, Statement>` and only re-parses the pending (incomplete) tail on each push. The `set()` method auto-detects when text was replaced (not appended) and resets the cache.

See [Streaming Parser](03-streaming-parser.md) for the detailed streaming algorithm.
See [Materializer](04-materializer.md) for how the AST is resolved and validated.
See [Evaluator](05-evaluator.md) for how expressions are interpreted.
