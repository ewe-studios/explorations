# Publisher Deep Dive: render, renderNode, dynamicDetection

This document provides source-level detail of the three core publisher files: `render.ts` (page orchestrator), `renderNode.ts` (recursive node renderer), and `dynamicDetection.ts` (static/dynamic classification).

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.Instatic/Instatic/src/core/publisher/`

## File Map

| File | Purpose |
|------|---------|
| `render.ts` | `publishPage()` — page-level orchestration, `<head>`/`<body>` assembly |
| `renderNode.ts` | `renderNode()` — recursive bottom-up node walk |
| `renderConfig.ts` | `RenderConfig`, `RenderAccumulators`, `RenderNodeFn` types |
| `renderVisualComponentRef.ts` | Specialized renderer for `base.visual-component-ref` |
| `renderLoop.ts` | Specialized renderer for `base.loop` — data iteration, round-robin |
| `dynamicDetection.ts` | `findDynamicNodeIds()` — 4-rule auto-detection |
| `escapeProps.ts` | Constraint #211: escape string props before render() |
| `classInjection.ts` | Inject author classIds, node IDs, inline styles |
| `classCss.ts` | Compile StyleRule entries → CSS, bag to inline style |
| `cssCollector.ts` | `CssCollector` — dedup CSS by moduleId (~60-80% reduction) |
| `cspPlan.ts` | Content-Security-Policy plan generation |
| `frameworkCss.ts` | Site framework CSS (spacing scale, typography) |
| `reset.ts` | `PUBLISHER_RESET_CSS` — cross-browser baseline |
| `siteCssBundle.ts` | Hash-named bundle composition |
| `userStylesheets.ts` | Site-level user stylesheets |
| `sizesResolver.ts` | `<img sizes>` from layout (caps, fractions, grid tracks) |
| `holeSubtreeModules.ts` | Collect module IDs inside hole subtrees |
| `utils.ts` | `escapeHtml`, `isSafeUrl`, `sanitiseCssValue` |

## publishPage: Page Orchestrator

**Source:** `src/core/publisher/render.ts`

```typescript
// src/core/publisher/render.ts
interface PublishPageOptions {
  breakpointId?: string
  templateContext?: TemplateRenderDataContext
  runtimeAssets?: PublishedPageRuntimeAssets
  loopData?: Map<string, ResolvedLoopRenderData>
  mediaAssets?: Map<string, RenderResolvedMedia>
  loopEndpointBaseUrl?: string
  cssMode?: 'inline' | 'external'
}

interface PublishedPage {
  filename: string    // e.g. "index.html", "about-us.html"
  html: string        // Complete <!DOCTYPE html> document
  jsModuleIds: string[] // Module-JS candidates for script injection
}
```

The `publishPage()` function:

1. **Resolve template context** — page/site/route frames via `buildPageFrame`, `buildSiteFrame`, `buildRouteFrame`
2. **Detect dynamic nodes** — `findDynamicNodeIds(page, site, registry)` returns set of dynamic node IDs
3. **Check Layer A viability** — if no dynamic nodes and no loops, page can be fully static
4. **Build `<body>` classes** — `classNamesForClassIds(rootNode.classIds)` injects root node classes
5. **Build `<head>`** — title, description, favicon, lang, importmap, runtime `<script>`s, CSP
6. **Call `renderNode(rootNodeId)`** — recursive bottom-up walk
7. **Collect CSS** — `collectClassCSS()` + `sanitizeModuleCSS()` + `collectUserStylesheetCss()`
8. **Emit CSS** — inline `<style>` or external `<link>` tags based on `cssMode`
9. **Assemble document** — `<!DOCTYPE html>` + `<html>` + `<head>` + `<body>`

### CSS Emission Modes

| Mode | Output | Use case |
|------|--------|----------|
| `inline` (default) | Single `<style>` block with reset + framework + module + user CSS | Self-contained exports, iframe preview, tests |
| `external` | Three `<link rel="stylesheet">` tags | Server-served pages where CSS is cached separately |

### CSP Generation

```typescript
// src/core/publisher/cspPlan.ts
// createBaseCspPlan() → Content-Security-Policy meta tag
// Handles script-src, style-src, img-src, font-src, connect-src
```

## renderNode: Recursive Walker

**Source:** `src/core/publisher/renderNode.ts`

The `renderNode()` function is the recursive bottom-up walker. Two paths:

### Path 1: Specialized Renderers

```typescript
// renderNode checks for specialized renderers by moduleId:
if (moduleId === 'base.visual-component-ref') {
  return renderVisualComponentRef(nodeId, config, acc, renderNode)
}
if (moduleId === 'base.loop') {
  return renderLoop(nodeId, config, acc, renderNode)
}
```

**Aha:** Specialized renderers receive `renderNode` as a callback so the file graph stays acyclic — only `renderNode.ts` knows both ends of the recursion. This prevents circular dependencies between the walker and specialized renderers.

### Path 2: renderStandardNode

For all other nodes, the bottom-up flow:

1. **Render children** — `renderNode(childId)` for each child
2. **Resolve page refs** — `resolvePageRefProps()` converts `cms:page:<id>` to public paths
3. **Resolve effective props** — `resolveProps()` from page-tree
4. **Resolve dynamic props** — `resolveDynamicProps()` for request-dependent bindings
5. **Attach media** — `attachResolvedMediaByKey()` for multi-media props
6. **Resolve sizes** — `resolveAutoSizes()` for responsive images
7. **Escape props** — `escapeProps()` (Constraint #211)
8. **Validate** — `validateNodeProps()` against module schema
9. **Call module.render()** — pure function returning HTML (Constraint #179)
10. **Collect CSS** — `collectClassCSS()` deduped by moduleId
11. **Inject classes** — `injectNodeClassIds()`, `injectNodeId()`, `injectNodeInlineStyles()`
12. **Sanitize richtext** — `sanitizeRichtext()` via DOMPurify

```typescript
// renderNode.ts:46-76 — Page ref resolution
function resolvePageRefProps(
  props: Record<string, unknown>,
  pages: RenderConfig['site']['pages'],
): Record<string, unknown> {
  let out: Record<string, unknown> | null = null
  for (const [key, value] of Object.entries(props)) {
    if (!isPageRef(value)) continue
    const resolved = resolvePageRef(value, pages)
    if (resolved === null) continue
    if (!out) out = { ...props }
    out[key] = resolved
  }
  return out ?? props
}
```

## dynamicDetection: Four-Rule Classifier

**Source:** `src/core/publisher/dynamicDetection.ts`

The `findDynamicNodeIds()` function walks the page tree once and classifies every node as static or dynamic. It's the single source of truth used by both Layer A (should we bake to disk?) and Layer C (should we emit `<instatic-hole>`?).

### Detection Rules

| Rule | Condition | Effect |
|------|-----------|--------|
| 1 | Module flagged `dynamic: true` in registry | Node is dynamic |
| 2 | `dynamicBindings` entry with request-dependent source (`route.query.*`) | Node is dynamic |
| 2b | String prop value contains `{source.field}` token with request-dependent source | Node is dynamic |
| 3 | `base.loop` with `requestDependent: true` source | Node is dynamic |
| 4 | `base.visual-component-ref` whose VC tree contains any dynamic node | Outer VC ref is dynamic |

### Request-Dependent Binding Sources

| Source | Request-Dependent? | Reason |
|--------|-------------------|--------|
| `route.query.*` | Yes | Varies per visitor URL |
| `currentEntry.*` | No | Entry known at publish |
| `parentEntry.*` | No | Parent entry known at publish |
| `page.*` | No | Page-level, static |
| `site.*` | No | Site-level, static |
| `route.path` | No | Fixed per static route |
| `route.slug` | No | Fixed per static route |

### VC Ref Subtlety

When a VC definition tree is dynamic, the **outer VC ref node ID** (in the page tree) goes into `dynamicPageNodeIds` — not the inner VC node IDs. The hole boundary is the VC ref, not any inner node. Diagnostic reasons collected from inner VC traversal are still appended to the reason list so authors can see WHICH inner construct made the VC dynamic.

```typescript
// dynamicDetection.ts:63-80 — Binding source classification
function isBindingSourceRequestDependent(source: string, field: string): boolean {
  switch (source) {
    case 'route':
      return field === 'query' || field.startsWith('query.')
    case 'currentEntry':
    case 'parentEntry':
    case 'page':
    case 'site':
      return false
    default:
      // Unknown source — conservatively treated as publish-time deterministic
      // to avoid false positives forcing unnecessary dynamic rendering.
      return false
  }
}
```

**Aha:** Unknown binding sources are conservatively treated as publish-time deterministic. This avoids false positives where every plugin-registered source would force dynamic rendering. Plugin authors adding request-dependent sources should update this function or register sources via a future plugin-source registry.

### Hole Subtree Collection

After identifying dynamic nodes, `collectHoleSubtreeModuleIds()` collects all module IDs inside hole subtrees. This ensures the correct module-JS bundles are injected for pages with dynamic content.

## escapeProps: Constraint #211

**Source:** `src/core/publisher/escapeProps.ts`

Every node's props pass through `escapeProps()` before `module.render()`. This dispatches per-prop based on the schema control `type`:

- Strings are HTML-escaped (`escapeHtml`)
- URLs are validated (`isSafeUrl`)
- Numbers, booleans, and objects pass through unchanged

This prevents XSS at the render boundary — even if a module accidentally returns user-controlled HTML, it gets escaped.

## CSS Collector: Decision #308

**Source:** `src/core/publisher/cssCollector.ts`

CSS is deduplicated by `moduleId`. If two nodes use the same module with the same classIds, the CSS is emitted once. This achieves ~60-80% size reduction on typical pages.

```typescript
// CssCollector — accumulates class CSS, deduped by moduleId
// collectClassCSS(modules, classIds, registry) → CSS string
// sanitizeModuleCSS(moduleId, css) → safe CSS for injection
```

## Next

Continue to [06-plugin-deep](06-plugin-deep.md) for the plugin host, VM, protocol, and RPC deep dive.
