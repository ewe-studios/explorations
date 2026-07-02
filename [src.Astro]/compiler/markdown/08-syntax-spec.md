---
title: Syntax Specification -- The .astro File Format
---

# Syntax Specification: The .astro File Format

The .astro file format combines a TypeScript frontmatter section (between `---` fences) with a JSX-like template that supports expressions, custom directives, and CSS/script blocks. The syntax extends JSX with HTML conventions (no single-root requirement, HTML comments, unquoted attributes) and Astro-specific directives (client:*, set:html, define:vars, is:raw).

## File Structure

An `.astro` file has two sections in this order:

```
┌──────────────────────────────────────┐
│ ---                                  │
│ Component Script (TypeScript)        │
│ ---                                  │
├──────────────────────────────────────┤
│ Template (JSX-like)                  │
│   ├─ HTML elements                   │
│   ├─ Component references            │
│   ├─ Expressions { ... }            │
│   ├─ <style> blocks                 │
│   └─ <script> blocks                │
└──────────────────────────────────────┘
```

Source: `compiler/SYNTAX_SPEC.md:1-33`

## Minimal Examples

```astro
<!-- Template only, no frontmatter -->
<!-- A comment! -->
<h1>Hello, World!</h1>
```

```astro
<!-- With frontmatter, style, and template -->
---
const greeting = "Hello";
---

<h1>{greeting}, World!</h1>

<style>
  h1 { color: royalblue; }
</style>
```

Source: `compiler/SYNTAX_SPEC.md:35-51`

## Frontmatter (Component Script)

The region between `---` fences:

| Rule | Detail |
|------|--------|
| Fences | `---` (three dashes), not required on own line |
| Count | Only one per file |
| Language | TypeScript (all standard TS syntax) |
| Whitespace | Any amount before opening or after closing fence |
| Pre-fence content | Allowed but customarily ignored |

```astro
---
// Opening fence can share line with code
--- const x = 1; ---
// This is valid but unusual
```

Source: `compiler/SYNTAX_SPEC.md:54-63`

### Top-Level Return

`return` is allowed at top level (non-standard for modules):

```astro
---
import { getUser } from "../lib/auth.js";
const user = await getUser();
if (!user) {
  return Astro.redirect("/login");
}
---
```

Source: `compiler/SYNTAX_SPEC.md:66-78`

Aha: The top-level `return` is a key design choice — it enables early exits and redirects from component scripts without requiring a wrapper function. The compiler detects `await` anywhere in the frontmatter and wraps the entire component function in `async`. This means any `return` statement in frontmatter can return a `Response` (via `Astro.redirect`) that short-circuits rendering.

## Template

The template is everything after the closing `---` fence (or the entire file without frontmatter). It follows the JSX specification with modifications:

### Whitespace Between Frontmatter and Template

Any whitespace between `---` and template is ignored:

```astro
---
const greeting = "Hello";
---



<!-- These blank lines are ignored -->
<h1>{greeting}, World!</h1>
```

Source: `compiler/SYNTAX_SPEC.md:88-100`

### Differences from JSX

| Feature | Astro | JSX |
|---------|-------|-----|
| HTML comments | `<!-- ... -->` allowed | `{/* ... */}` only |
| Fragment shorthand attrs | Not allowed (`< slot="x">`) | Same |
| Fragment longhand | `<Fragment attr="x">` allowed | Same |
| Multiple roots | Allowed | Single root required |
| Attribute names | Hyphens, colons allows | Must be valid JS identifiers |
| Component namespaces | Not supported (`<A:B/>` is one name) | Not supported |
| Attribute shorthand | `{prop}` = `prop={prop}` | Same |
| Template literal attrs | `` attr=`hello ${x}` `` | Not supported |
| Empty expressions | `{}` in attrs allowed | Same |
| Comments in open tags | `{/* */}` between attrs allowed | Same |
| `<` in text | No escape needed (HTML rules) | Needs `{'<'}` or `&lt;` |
| Non-ASCII tags | Not supported (treated as text) | Supported |
| Unquoted attrs | `data-id=1234` allowed | Must be quoted |
| Unclosed tags | Allowed (HTML rules) | Must be closed |
| Void elements | `<br>` no self-close needed | `<br/>` or `<br>` |
| Whitespace in expressions | Preserved | Collapsed |

Source: `compiler/SYNTAX_SPEC.md:102-288`

## Expression Parsing

Expressions are delimited by `{` and `}`:

```astro
{
    <div>Hello</div>
}

{   <div>test</div>   }

{   }
```

In Astro, whitespace inside expressions produces text nodes (unlike JSX where it's ignored).

Source: `compiler/SYNTAX_SPEC.md:158-173`

## Attribute Types

Six attribute types are supported:

```mermaid
classDiagram
    class AttributeType {
        <<enumeration>>
        QuotedAttribute
        EmptyAttribute
        ExpressionAttribute
        SpreadAttribute
        ShorthandAttribute
        TemplateLiteralAttribute
    }
    
    class Examples {
        +class="my-class"
        +disabled
        +onClick={handler}
        +{...props}
        +{className}
        +title=`Hello ${name}`
    }
    
    AttributeType .. Examples
```

| Type | Syntax | Generated Output |
|------|--------|-----------------|
| Quoted | `class="my-class"` | `class="my-class"` |
| Empty | `disabled` | `disabled` |
| Expression | `onClick={() => fn()}` | `${$$addAttribute(() => fn(), "onClick")}` |
| Spread | `{...props}` | `${$$spreadAttributes(props)}` |
| Shorthand | `{className}` | `${$$addAttribute(className, "className")}` |
| Template literal | `` title=`Hello ${x}` `` | `${$$addAttribute(\`Hello ${x}\`, "title")}` |

Source: `compiler/internal/token.go:80-87` — Token type definitions.
Source: `compiler/internal/token.go:464-556` — Attribute parsing in `readTagAttrVal`.

### Template Literal Attributes

Backticks allow interpolation without opening an expression:

```astro
<Component attribute=`hello ${value}` />
```

Source: `compiler/SYNTAX_SPEC.md:231-238`

### Unquoted Attributes

No quotes needed for alphanumeric, hyphens, underscores, periods:

```astro
<Component data-id=12345 class=my-class />
```

Source: `compiler/SYNTAX_SPEC.md:270-277`

### Empty Expressions in Attributes

```astro
<Component attribute={} />
```

Source: `compiler/SYNTAX_SPEC.md:240-246`

## is:raw Directive

The `is:raw` attribute disables expression parsing inside an element:

```astro
<Component is:raw>{<% non-JS content %>}</Component>

<div is:raw>
  {not an expression, just text}
</div>
```

Source: `compiler/SYNTAX_SPEC.md:131-140`

Source: `compiler/internal/token.go:1203` — `raw = z.hasAttribute("is:raw")`.

## Elements That Disable Expression Parsing

These elements treat `{` and `}` as literal text:

- `<math>`
- `<iframe>`
- `<noembed>`
- `<noframes>`
- `<plaintext>`
- `<xmp>`

Source: `compiler/SYNTAX_SPEC.md:308-329`

```astro
<math xmlns="http://www.w3.org/1998/Math/MathML">
  <annotation encoding="application/x-tex">
    f\colon X \to \mathbb{R}^{2x}
  </annotation>
</math>
```

Here `{R}` and `{2x}` are literal text, not expressions.

## Elements With Partial Expression Support

`<title>` and `<textarea>` treat inner tags as text but still support expressions:

```astro
<title>{pageTitle} - Site with <b>bold</b></title>
<!-- {pageTitle} is an expression, <b>bold</b> is text -->
```

Source: `compiler/SYNTAX_SPEC.md:333-343`

## Style Blocks

Multiple `<style>` blocks per file:

```astro
<style>
  h1 { color: red; }
</style>

<style lang="scss">
  $accent: #1d4ed8;
  .card { border-color: $accent; }
</style>
```

| Attribute | Purpose |
|-----------|---------|
| `lang` | Preprocessor language (`scss`, `sass`, `postcss`, `stylus`, `less`) |
| `is:global` | Disable CSS scoping |
| `define:vars={...}` | Inject JS variables into CSS |

Source: `compiler/SYNTAX_SPEC.md:347-373`

## Script Blocks

Multiple `<script>` blocks per file:

```astro
<!-- Bare script = TypeScript -->
<script>
  interface User { id: number; name: string; }
</script>

<!-- With attributes = standard HTML script rules -->
<script defer>/* JavaScript */</script>
<script type="module">import { foo } from "./foo.js";</script>
<script type="application/json">{ "key": "value" }</script>
```

Source: `compiler/SYNTAX_SPEC.md:377-416`

The bare `<script>` (no attributes) accepts TypeScript. Any attribute switches to standard HTML `<script>` rules.

## Fragment Syntax

Both shorthand and longhand fragments supported:

```astro
<!-- Shorthand (no attributes allowed) -->
<>
  <div>Item 1</div>
  <div>Item 2</div>
</>

<!-- Longhand (attributes supported) -->
<Fragment class="wrapper">
  <div>Item 1</div>
</Fragment>
```

Source: `compiler/SYNTAX_SPEC.md:118-130`

The shorthand form cannot accept attributes — attempting `< slot="named">` produces an error suggesting `<Fragment slot="named">`.

Source: `compiler/internal/token.go:1776-1786` — Error for fragment shorthand with attrs.

## HTML Doctype

```astro
<!DOCTYPE html>
```

Source: `compiler/SYNTAX_SPEC.md:143-149`

## Directive Reference

| Directive | Element | Purpose |
|-----------|---------|---------|
| `client:load` | Component | Hydrate on page load |
| `client:idle` | Component | Hydrate when main thread idle |
| `client:visible` | Component | Hydrate when element visible |
| `client:media` | Component | Hydrate on media query match |
| `client:only` | Component | Client-only (no SSR) |
| `is:raw` | Any | Disable expression parsing |
| `is:global` | `<style>` | Disable CSS scoping |
| `set:html` | Any | Render raw HTML |
| `set:text` | Any | Render text (escaped) |
| `define:vars` | `<style>`, `<script>` | Inject JS variables |
| `define:vars` (script) | `<script>` | Script-scoped variables |
| `transition:animate` | Any | View transition animation |
| `transition:name` | Any | View transition name mapping |
| `transition:persist` | Any | Persist element across transitions |
| `transition:persist-props` | Any | Persist with props (preserves component props) |
| `server:defer` | Component | Defer to client for rendering |
| `data-astro-reload` | `<a>`, `<form>`, `<area>` | Full page reload instead of VT |

## Parser Token Flow

```mermaid
flowchart LR
    A[source text] --> B{First char?}
    B -->|---| C[FrontmatterFenceToken]
    B -->|{| D[StartExpressionToken]
    B -->|<| E{Next char?}
    E -->|a-z/A-Z| F[StartTagToken]
    E -->|/| G[EndTagToken]
    E -->|! | H[CommentToken or DoctypeToken]
    E -->|>| I[StartTagToken Fragment <>]
    B -->|text| J[TextToken]
    C -->|inside frontmatter| K[TextToken TS code]
    K -->|---| L[FrontmatterFenceToken closing]
    L --> M[back to normal parsing]
    D -->|inside expression| N[nested expression or element]
    F --> O[readTagAttrKey]
    O --> P[readTagAttrVal]
    P --> Q[check for > or />]
```

Source: `compiler/internal/token.go:1676-2098` — `Next()` method.

## Next Steps

Continue to cross-cutting concerns: [Cross-Cutting](./09-cross-cutting.md)

Related documents:
- [Go Parser](./01-go-parser.md) — How this syntax is tokenized and parsed
- [AST Transforms](./05-ast-transforms.md) — How directives are normalized
- [Overview](./00-overview.md) — Architecture context
