---
title: Go Printer -- Code Generation from AST to JavaScript
---

# Go Printer: Code Generation from AST to JavaScript

The Go printer walks the parsed AST and emits JavaScript wrapped in a `$$render` template literal, preserving frontmatter code verbatim while transforming template nodes into runtime function calls. The printer operates in two passes: first it renders the intermediate JS (including AST walk), then it strips TypeScript syntax and composes sourcemaps.

## Printer Structure

```go
// internal/printer/printer.go:26-40
type printer struct {
    sourcetext         string                    // Original .astro source
    opts               transform.TransformOptions // Transform options
    output             []byte                    // Output buffer
    builder            sourcemap.ChunkBuilder    // Sourcemap builder
    handler            *handler.Handler          // Error/warning handler
    hasFuncPrelude     bool                      // Whether component wrapper started
    hasInternalImports bool                      // Whether runtime imports printed
    hasCSSImports      bool                      // Whether CSS imports printed
    needsTransitionCSS bool                      // Whether transitions.css needed
    hasTemplateElement bool                      // Whether <template> exists in AST
    
    ranges TSXRanges  // TSX output ranges (for convertToTSX)
}
```

Source: `internal/printer/printer.go:26-40`

The `PrintResult` is the output container:

```go
// internal/printer/printer.go:19-24
type PrintResult struct {
    Output         []byte              // Generated JavaScript
    SourceMapChunk sourcemap.Chunk    // Sourcemap data
    TSXRanges      TSXRanges          // TSX conversion ranges
}
```

Source: `internal/printer/printer.go:19-24`

## Runtime Constants

The printer defines all runtime helper names as constants:

```go
// internal/printer/printer.go:42-65
var TEMPLATE_TAG = "$$render"
var CREATE_ASTRO = "$$createAstro"
var CREATE_COMPONENT = "$$createComponent"
var RENDER_COMPONENT = "$$renderComponent"
var RENDER_HEAD = "$$renderHead"
var MAYBE_RENDER_HEAD = "$$maybeRenderHead"
var UNESCAPE_HTML = "$$unescapeHTML"
var RENDER_SLOT = "$$renderSlot"
var MERGE_SLOTS = "$$mergeSlots"
var ADD_ATTRIBUTE = "$$addAttribute"
var RENDER_TRANSITION = "$$renderTransition"
var CREATE_TRANSITION_SCOPE = "$$createTransitionScope"
var SPREAD_ATTRIBUTES = "$$spreadAttributes"
var DEFINE_STYLE_VARS = "$$defineStyleVars"
var DEFINE_SCRIPT_VARS = "$$defineScriptVars"
var CREATE_METADATA = "$$createMetadata"
var RENDER_SCRIPT = "$$renderScript"
var TEMPLATE_ENTER = "$$templateEnter"
var TEMPLATE_EXIT = "$$templateExit"
var METADATA = "$$metadata"
var RESULT = "$$result"
var SLOTS = "$$slots"
var FRAGMENT = "Fragment"
var BACKTICK = "`"
```

Source: `internal/printer/printer.go:42-66`

## Internal Imports

The `printInternalImports` method (`internal/printer/printer.go:180-239`) emits the runtime import block:

```go
// internal/printer/printer.go:186-236
p.print("import {\n  ")
p.print(FRAGMENT + ",\n  ")
p.print("render as " + TEMPLATE_TAG + ",\n  ")
p.print("createAstro as " + CREATE_ASTRO + ",\n  ")
// ... all 15+ runtime helpers
p.print("\n} from \"" + importSpecifier + "\";\n")

// Conditional imports
if p.hasTemplateElement {
    p.print("templateEnter as " + TEMPLATE_ENTER + ",\n  ")
    p.print("templateExit as " + TEMPLATE_EXIT + ",\n  ")
}
if opts.ResolvePath == nil {
    p.print("createMetadata as " + CREATE_METADATA)
}
```

Source: `internal/printer/printer.go:180-239`

The import specifier defaults to `"astro/runtime/server/index.js"` but can be overridden via `opts.InternalURL`.

## render1: The AST Walk

The core function `render1` (`internal/printer/print-to-js.go:121`) dispatches on node type:

```mermaid
flowchart TD
    A[render1 called with node] --> B{Node Type?}
    B -->|DocumentNode| C[printInternalImports]
    C --> D[iterate children]
    D --> E[render1 each child]
    E --> F[printReturnClose + printFuncSuffix]
    
    B -->|FrontmatterNode| G[hoist imports with js_scanner]
    G --> H[printComponentMetadata]
    H --> I[printTopLevelAstro if needed]
    I --> J[emit exports separately]
    J --> K[print remaining frontmatter in component]
    
    B -->|ElementNode| L{component?}
    L -->|Yes| M[printComponent]
    L -->|No| N[printHTMLElement]
    M --> O[$$renderComponent call]
    N --> P[template literal output]
    
    B -->|ExpressionNode| Q[expression container]
    Q --> R[${expression_content}]
    
    B -->|TextNode| S[escape and emit]
    B -->|CommentNode| T[<!-- ... -->]
```

Source: `internal/printer/print-to-js.go:121` — `render1` function entry point.

## Frontmatter Processing

The frontmatter rendering (`internal/printer/print-to-js.go:156-280`) is the most complex part:

1. **Import hoisting**: `js_scanner.HoistImports` extracts import statements from frontmatter
2. **CSS imports**: `printCSSImports` emits `import "file.astro?astro&type=style&index=N&lang.css"` for each style
3. **Component metadata**: `printComponentMetadata` re-emits imports as `$$moduleN` namespace imports
4. **Astro global**: `printTopLevelAstro` emits `const $$Astro = $$createAstro(...)` if `Astro` appears in source
5. **Exports**: Named/default exports are hoisted after metadata
6. **Body**: Remaining frontmatter code goes inside the component function

```go
// internal/printer/print-to-js.go:169-177
render := js_scanner.HoistImports([]byte(c.Data))
if len(render.Hoisted) > 0 {
    for i, hoisted := range render.Hoisted {
        // Emit hoisted imports with source mapping
        p.printTextWithSourcemap(string(hoisted)+"\n", loc.Loc{...})
    }
}

// Print component metadata (re-emits imports as $$moduleN)
p.printComponentMetadata(n.Parent, opts.opts, []byte(p.sourcetext))
```

Source: `internal/printer/print-to-js.go:169-190`

## The js_scanner.HoistImports

The JS scanner (`internal/js_scanner/js_scanner.go`) parses frontmatter code and separates:

- **Hoisted**: Import statements (moved before component function)
- **Body**: Non-import code (goes inside component function)

It also tracks `getStaticPaths` calls for static path generation.

## Component Metadata Generation

`printComponentMetadata` (`internal/printer/printer.go:613-792`) scans frontmatter imports and generates:

```js
import * as $$module1 from '../components/Counter.jsx';
import * as $$module2 from '../components/Header.astro';

export const $$metadata = $$createMetadata("src/pages/index.astro", {
  modules: [
    { module: $$module1, specifier: '../components/Counter.jsx', assert: {} },
    { module: $$module2, specifier: '../components/Header.astro', assert: {} }
  ],
  hydratedComponents: [Counter],
  clientOnlyComponents: [],
  hydrationDirectives: new Set(['load']),
  hoisted: []
});
```

The metadata is built by:
1. Scanning imports with `js_scanner.NextImportStatement`
2. Matching component names to import specifiers
3. Building the `$$metadata` export

Source: `internal/printer/printer.go:613-792`

## Attribute Printing

The `printAttribute` method (`internal/printer/printer.go:464-545`) handles all 6 attribute types:

```go
// internal/printer/printer.go:464-545 (simplified)
func (p *printer) printAttribute(attr astro.Attribute, n *astro.Node) {
    if _, ok := skippedAttributes[attr.Key]; ok { return }  // Skip define:vars, set:html, etc.
    
    switch attr.Type {
    case astro.QuotedAttribute:
        p.print(` class="` + escape(attr.Val) + `"`)
    case astro.EmptyAttribute:
        p.print(` disabled`)
    case astro.ExpressionAttribute:
        p.print(`${$$addAttribute(` + attr.Val + `, "` + attr.Key + `")}`)
    case astro.SpreadAttribute:
        p.print(`${$$spreadAttributes(` + attr.Key + `)}`)
    case astro.ShorthandAttribute:
        p.print(`${$$addAttribute(` + attr.Key + `, "` + attr.Key + `")}`)
    case astro.TemplateLiteralAttribute:
        p.print(`${$$addAttribute(` + "`" + attr.Val + "`" + `, "` + attr.Key + `")}`)
    }
}
```

Source: `internal/printer/printer.go:464-545`

Skipped attributes (not emitted directly):

```go
// internal/printer/printer.go:384-392
var skippedAttributes = map[string]bool{
    "define:vars":        true,  // Handled via $$defineStyleVars/$$defineScriptVars
    "set:text":           true,  // Converted to expression child
    "set:html":           true,  // Converted to $$unescapeHTML call
    "is:raw":             true,  // Parser directive, not runtime
    "transition:animate": true,  // Converted to data-astro-transition-scope
    "transition:name":    true,  // Converted to data-astro-transition-scope
    "transition:persist": true,  // Converted to data-astro-transition-persist
}
```

Source: `internal/printer/printer.go:384-392`

## Component Element Printing

Components (uppercase or dotted names) render as `$$renderComponent` calls:

```go
// printComponent dispatches to $$renderComponent emission
p.print("${" + RENDER_COMPONENT + "(" + RESULT + "," + componentExpr + "," + props + "," + slots + ")}")
```

The component name is resolved by matching against frontmatter imports. The props object is built by iterating attributes and skipping directive attributes.

## define:vars Handling

The `printDefineVarsOpen` method (`internal/printer/printer.go:300-336`) injects `$$defineStyleVars` or `$$defineScriptVars`:

```go
// internal/printer/printer.go:300-336
func (p *printer) printDefineVarsOpen(n *astro.Node) {
    if !(n.DataAtom == atom.Script || n.DataAtom == atom.Style) { return }
    if !transform.HasAttr(n, "define:vars") { return }
    
    for _, attr := range n.Attr {
        if attr.Key == "define:vars" {
            var defineCall string
            if n.DataAtom == atom.Script {
                defineCall = DEFINE_SCRIPT_VARS  // $$defineScriptVars
            } else {
                defineCall = DEFINE_STYLE_VARS   // $$defineStyleVars
            }
            p.print("${" + defineCall + "(" + attr.Val + ")}")
            return
        }
    }
}
```

For scripts without `type="module"`, an IIFE wrapper is added:

```js
<script>(function(){${$$defineScriptVars(vars)}...code...})();</script>
```

Aha: The IIFE wrapper is necessary because `$$defineScriptVars` declares variables that the script body needs, and without `type="module"` the script runs in classic (non-ESM) scope. With `type="module"`, the IIFE is omitted because modules already have their own scope (and imports inside functions would be illegal).

## Transition Handling

The `maybeConvertTransition` function (`internal/printer/printer.go:574-611`) converts transition attributes to data attributes:

```go
// internal/printer/printer.go:574-611
func maybeConvertTransition(n *astro.Node) {
    if transform.HasAttr(n, "transition:animate") || transform.HasAttr(n, "transition:name") {
        n.Attr = append(n.Attr, astro.Attribute{
            Key:  "data-astro-transition-scope",
            Val:  fmt.Sprintf(`%s(%s, "%s", %s, %s)`, 
                RENDER_TRANSITION, RESULT, n.TransitionScope, animationExpr, transitionExpr),
            Type: astro.ExpressionAttribute,
        })
    }
    // transition:persist → data-astro-transition-persist
    if transform.HasAttr(n, "transition:persist") {
        // ... similar conversion
    }
}
```

Source: `internal/printer/printer.go:574-611`

## Whitespace Handling

Whitespace collapsing in Go happens in the transform pass (`internal/transform/transform.go:327-394`), not the printer. The printer just emits whatever text is in the AST after transform.

## CSS Import Generation

The `printCSSImports` method (`internal/printer/printer.go:241-258`) emits one import per style tag:

```go
// internal/printer/printer.go:241-258
func (p *printer) printCSSImports(cssLen int) {
    i := 0
    for i < cssLen {
        p.print(fmt.Sprintf("import \"%s?astro&type=style&index=%v&lang.css\";", 
            p.opts.Filename, i))
        i++
    }
    if p.needsTransitionCSS {
        p.print(fmt.Sprintf(`import "%s";`, p.opts.TransitionsAnimationURL))
    }
    p.print("\n")
}
```

Source: `internal/printer/printer.go:241-258`

## Full Output Structure

The final generated output follows this structure:

```mermaid
flowchart LR
    A[Hoisted Imports] --> B[Module Imports $$moduleN]
    B --> C[$$metadata Export]
    C --> D[Named/Default Exports]
    D --> E[Component Function]
    E --> F[$$createAstro]
    F --> G[Frontmatter Body]
    G --> H[Template Literals]
    H --> I[$$renderComponent Calls]
    I --> J[Return $$render result]
```

## Next Steps

Continue to AST transforms: [AST Transforms](./05-ast-transforms.md)

Related documents:
- [Go Parser](./01-go-parser.md) — What produces the AST the printer consumes
- [Code Generation](./06-code-generation.md) — Rust equivalent code generation
- [Compiler API](./07-compiler-api.md) — How to invoke the printer programmatically
