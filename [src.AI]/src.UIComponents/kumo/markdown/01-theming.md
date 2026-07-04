# Kumo Theming: CSS light-dark(), Semantic Tokens, and Lint Enforcement

Kumo uses CSS `light-dark()` for automatic theme handling instead of Tailwind's `dark:` variant. This is enforced by a custom oxlint plugin with 4 rules. The theme system also parses semantic color tokens from CSS files for validation.

Source: `lint/` (6 files), `packages/kumo/tailwind.config.js`, `packages/kumo/src/styles/`.

## Tailwind Configuration

Source: `packages/kumo/tailwind.config.js`

```javascript
export default {
  content: ["./dist/**/*.{js,jsx,ts,tsx}"],
  darkMode: ["selector", "[data-mode='dark']"],
  theme: { extend: {} },
  plugins: [],
}
```

Dark mode is triggered by a `data-mode="dark"` selector, not by `prefers-color-scheme`. This gives the design system explicit control over theme switching.

## ESLint Plugin: kumo-plugin

Source: `lint/kumo-plugin.js`

```javascript
import { noTailwindDarkVariantRule } from "./no-tailwind-dark-variant.js"
import { noPrimitiveColorsRule } from "./no-primitive-colors.js"
import { enforceVariantStandardRule } from "./enforce-variant-standard.js"
import { noCrossPackageImportsRule } from "./no-cross-package-imports.js"

const plugin = {
  meta: { name: "kumo" },
  rules: {
    "no-tailwind-dark-variant": noTailwindDarkVariantRule,
    "no-primitive-colors": noPrimitiveColorsRule,
    "enforce-variant-standard": enforceVariantStandardRule,
    "no-cross-package-imports": noCrossPackageImportsRule,
  },
}
```

4 custom rules, all built on oxlint's `defineRule()` API.

## Rule 1: no-tailwind-dark-variant

Source: `lint/no-tailwind-dark-variant.js` — 120 lines.

Detects `dark:` in className strings and reports them:

```javascript
function hasDarkVariant(str) {
  return /\bdark:[a-z]+[-\w]*/.test(str)
}
```

### String Extraction

The `extractStrings()` function walks the AST to find all string values:

| Node Type | How Strings Are Found |
|-----------|---------------------|
| `Literal` | `node.value` if string |
| `TemplateLiteral` | `quasis[].value.cooked` |
| `BinaryExpression` (operator `+`) | Recurse left + right |
| `ArrayExpression` | Recurse each element |
| `ObjectExpression` | Recurse keys + values |
| `CallExpression` | Recurse arguments (skip spread) |
| `ConditionalExpression` | Recurse consequent + alternate + test |
| `JSXAttribute` (className/class) | Extract from value node |

Aha: The rule avoids false positives on `{ dark: "vesper" }` object keys by checking `isInsideJsxAttribute()` — walking up the parent chain to see if the string is inside a JSX attribute. This allows passing theme objects with `dark` keys while banning `dark:bg-blue-500` in className strings.

### Reporting

```javascript
return {
  JSXAttribute(node) {
    if (node.name.name !== "className" && node.name.name !== "class") return
    const strings = extractStrings(node.value)
    for (const s of strings) {
      if (hasDarkVariant(s)) context.report({ node, messageId: RULE_NAME })
    }
  },
  Literal(node) {
    // Only report if NOT inside JSX attribute (already handled above)
    if (typeof node.value === "string" && hasDarkVariant(node.value) && !isInsideJsxAttribute(node)) {
      context.report({ node, messageId: RULE_NAME })
    }
  },
}
```

## Rule 2: no-primitive-colors

Source: `lint/no-primitive-colors.js` — 300+ lines.

This is the most complex rule. It does two things:
1. **Flag Tailwind primitive colors**: `bg-blue-500`, `text-red-400`
2. **Flag invalid semantic tokens**: `bg-unknown-color` (not in theme CSS)

### Token Regex

```javascript
export const TOKEN_RE = /(?:^|[^a-zA-Z0-9-])(((?:[a-z-]+:)*)?(?:bg|border|text|ring(?:-offset)?|fill|stroke|placeholder|caret|accent|decoration|divide|outline|from|via|to)-([a-z][a-z0-9-]*)(?:-\d{2,3})?(?:\/[0-9]{1,3})?)/gim
```

Matches: `bg-blue-500`, `hover:text-surface/50`, `border-kumo-fill`, `from-blue-500`, etc.

### Semantic Color Parsing

```javascript
function parseKumoSemanticColors() {
  const themeFiles = [
    resolve(__dirname, "../packages/kumo/src/styles/theme-kumo.css"),
    resolve(__dirname, "../packages/kumo/src/styles/theme-fedramp.css"),
  ]

  for (const themePath of themeFiles) {
    const css = readFileSync(themePath, "utf-8")
    // Match --color-<name> custom properties
    const colorPropRe = /--color-([a-z][a-z0-9-]*)(?=\s*:)/gi
    while ((match = colorPropRe.exec(css))) {
      const name = match[1]
      // Skip Tailwind primitives like red-650, blue-400
      if (/^[a-z]+-\d{2,3}$/.test(name)) continue
      colorTokens.add(name)
    }
  }
}
```

Aha: The valid color tokens are parsed from the actual CSS theme files at lint time. This means the lint allowlist stays in sync with the theme — when a new color token is added to `theme-kumo.css`, it's automatically allowed by the linter. No manual allowlist maintenance.

### Two-Pass Validation

```javascript
function reportColorIssues(node, collected) {
  for (const s of collected) {
    // Pass 1: Check for Tailwind primitive colors
    const primitive = findPrimitiveColor(s)
    if (primitive) {
      context.report({ node, messageId: RULE_NAME })
      return
    }
    // Pass 2: Check for invalid/unknown semantic tokens
    const invalid = findInvalidToken(s)
    if (invalid) {
      context.report({
        node, messageId: INVALID_TOKEN_RULE,
        data: { token: invalid.token, tokenName: invalid.tokenName },
      })
      return
    }
  }
}
```

### Token Validation Logic

```javascript
// For text-* utilities: check against --text-color-* tokens
if (TEXT_COLOR_PREFIXES.has(prefix)) {
  if (!VALID_TEXT_COLOR_TOKENS.has(tokenName) && !BUILTIN_COLORS.has(tokenName)) {
    const primitiveFamily = tokenName.replace(/-\d+$/, "")
    if (!TAILWIND_COLOR_FAMILIES.has(primitiveFamily)) {
      return { type: "invalid", token: fullToken, tokenName }
    }
  }
}
// For bg-*, border-*, etc.: check against --color-* tokens
else if (COLOR_PREFIXES.has(prefix)) {
  if (!VALID_COLOR_TOKENS.has(tokenName) && !BUILTIN_COLORS.has(tokenName)) {
    return { type: "invalid", token: fullToken, tokenName }
  }
}
```

### Tailwind Color Families

```javascript
export const TAILWIND_COLOR_FAMILIES = new Set([
  "red", "orange", "amber", "yellow", "lime", "green", "emerald",
  "teal", "cyan", "sky", "blue", "indigo", "violet", "purple",
  "fuchsia", "pink", "slate", "gray", "zinc", "neutral", "stone",
])
```

`black`, `white`, and `transparent` are intentionally excluded from the blocked set — `bg-white`, `text-black`, `ring-transparent` are allowed.

### Non-Color Utility Filtering

```javascript
const NON_COLOR_UTILITIES = new Set([
  // Text utilities
  "xs", "sm", "base", "lg", "xl", "2xl", "3xl", "4xl",
  "left", "center", "right", "justify", "wrap", "nowrap",
  // Border utilities
  "0", "2", "4", "8", "t", "r", "b", "l", "x", "y",
  "solid", "dashed", "dotted", "double", "hidden",
  // etc.
])
```

Prevents false positives like `text-xl` or `border-2` being flagged as color issues.

## Rule 3: enforce-variant-standard

Source: `lint/enforce-variant-standard.js`

Enforces consistent Tailwind variant ordering (e.g., `hover:`, `focus:`, `disabled:`).

## Rule 4: no-cross-package-imports

Source: `lint/no-cross-package-imports.js`

Prevents imports across kumo packages that violate the package boundary contract.

## CI Integration

Source: `lint/lint-astro-colors.js`

Additional rule for Astro files — same color enforcement applies.

## Cross-References

- [Overview](00-overview.md) — What Kumo is
- [Tooling](03-tooling.md) — CI/CD, changesets, visual regression
- [Packages](04-packages.md) — Sub-packages, lint rules

[Next → AI Registry](02-ai-registry.md)
