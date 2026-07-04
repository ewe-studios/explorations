---
title: Configuration & Build
---

# Configuration & Build Setup

This document covers the build system, TypeScript configuration, Tailwind setup, and all package configuration for use-stick-to-bottom.

## Package Configuration

Source: `package.json`

```json
{
  "name": "use-stick-to-bottom",
  "version": "1.1.6",
  "type": "module",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "sideEffects": false,
  "license": "MIT",
  "peerDependencies": {
    "react": "^16.8.0 || ^17.0.0 || ^18.0.0 || ^19.0.0"
  }
}
```

### Key Fields

| Field | Value | Significance |
|-------|-------|-------------|
| `type: "module"` | ESM | Package uses `.js` extensions, `import` syntax |
| `sideEffects: false` | Tree-shakeable | Bundlers can drop unused exports |
| `main` | `dist/index.js` | Library output entry point |
| `types` | `dist/index.d.ts` | TypeScript declaration file |
| `peerDependencies` | React 16.8-19 | Must be provided by the consumer |

### Build Scripts

```json
{
  "scripts": {
    "dev": "vite",
    "build": "rm -rf dist > /dev/null 2>&1; tsc -b",
    "prepublishOnly": "pnpm build",
    "lint": "biome check",
    "lint:fix": "pnpm lint --write"
  }
}
```

- **`dev`** — Starts Vite dev server for the demo app
- **`build`** — Cleans `dist/`, runs `tsc -b` (TypeScript project references build)
- **`prepublishOnly`** — Automatically builds before npm publish
- **`lint`** — Biome static analysis
- **`lint:fix`** — Biome auto-fix

## TypeScript Configuration

### Root tsconfig.json

```json
// tsconfig.json
{
  "files": [],
  "references": [
    { "path": "./tsconfig.app.json" },
    { "path": "./tsconfig.demo.json" },
    { "path": "./tsconfig.node.json" }
  ]
}
```

Uses TypeScript project references (`tsc -b`) to build three sub-projects independently.

### tsconfig.app.json (Library)

```json
// tsconfig.app.json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "strict": true,
    "isolatedDeclarations": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "dist",
    "jsx": "react-jsx",
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "include": ["src/**/*.ts", "src/**/*.tsx"]
}
```

Key settings:
- **`isolatedDeclarations: true`** — Forces explicit return types on all exported functions. This is a strict TypeScript 5.5+ feature that ensures consumers get full type information without the compiler needing to infer across module boundaries.
- **`NodeNext`** module resolution — Matches the `"type": "module"` in package.json.
- **`declarationMap: true`** — Enables "Go to Definition" in editors to jump to source files, not `.d.ts` files.

### tsconfig.demo.json (Demo App)

```json
// tsconfig.demo.json
{
  "compilerOptions": {
    "target": "ESNext",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true
  },
  "include": ["demo/**/*.ts", "demo/**/*.tsx"]
}
```

No emission — the demo is type-checked but built by Vite, not `tsc`.

### tsconfig.node.json (Node Tooling)

```json
// tsconfig.node.json
{
  "compilerOptions": {
    "target": "ES2020",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "noEmit": true
  },
  "include": ["vite.config.ts"]
}
```

Type-checks the Vite config itself.

## Vite Configuration

```ts
// vite.config.ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
});
```

Minimal — only the React plugin. No library build config because the library is compiled by `tsc -b`, not Vite. Vite only serves the demo app.

**Aha:** The build uses `tsc` for the library (not Vite's library mode). This means the output is pure TypeScript compilation: `.ts` → `.js` + `.d.ts`. No bundling, no minification, no tree-shaking at build time. The consumer's bundler handles tree-shaking thanks to `"sideEffects": false`. This is a deliberate "source distribution" approach — the published package is already-compiled TypeScript, not a bundled artifact.

## Tailwind Configuration

```js
// tailwind.config.js
const { iconsPlugin } = require('@egoist/tailwindcss-icons');

export default {
  content: ['./index.html', './demo/**/*.{ts,tsx}'],
  plugins: [require('@tailwindcss/typography'), iconsPlugin()],
};
```

The content glob scans `index.html` and all demo files — NOT the library source (`src/`). This is correct because the library doesn't ship with Tailwind classes; consumers apply their own styling.

### Plugins

| Plugin | Purpose |
|--------|---------|
| `@tailwindcss/typography` | `prose` class for formatted text in the demo |
| `@egoist/tailwindcss-icons` + `@iconify/ph` | Phosphor icon utility classes (`i-ph-*`) |

### PostCSS

```js
// postcss.config.js
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {},
  },
};
```

Standard Tailwind + Autoprefixer pipeline.

## Linting: Biome

```json
// biome.json
{
  "formatter": {
    "enabled": true
  },
  "linter": {
    "enabled": true
  }
}
```

Biome (the successor to ESLint + Prettier) handles both formatting and linting. The existing code has `// biome-ignore lint/correctness/useExhaustiveDependencies:` comments in two places in `useStickToBottom.ts` where the deps array is intentionally incomplete.

## Version Compatibility

The package supports React 16.8 through React 19. This wide range works because:

1. **No React 18+ specific APIs** — The hook uses `useState`, `useCallback`, `useMemo`, `useRef`, `useEffect`, `useLayoutEffect`, `useImperativeHandle`, `useContext` — all available since React 16.8.
2. **SSR guard on `useLayoutEffect`** — `useIsomorphicLayoutEffect` falls back to `useEffect` when `window` is undefined.
3. **Optional chaining on `globalThis.document`** — Doesn't crash when `document` is absent during SSR.

### DevDependencies

| Package | Version | Purpose |
|---------|---------|---------|
| `react` | 18.x | Development rendering |
| `react-dom` | 18.x | Demo app DOM mounting |
| `@vitejs/plugin-react` | 4.3.1 | Vite React support |
| `vite` | 5.4.1 | Dev server |
| `vite-plugin-dts` | 4.2.1 | (Unused — types from tsc) |
| `tailwindcss` | 3.4.10 | Demo styling |
| `@biomejs/biome` | 1.9.4 | Linting |
| `lorem-ipsum` | 2.0.8 | Fake message generation |
| `typescript` | 5.5.3 | Type checking |

## Project Directory Structure

```
use-stick-to-bottom/
├── src/
│   ├── index.ts              # Re-exports (2 lines)
│   ├── useStickToBottom.ts   # Core hook (666 lines)
│   └── StickToBottom.tsx     # Component wrapper (189 lines)
├── demo/
│   ├── index.tsx             # React 18 entry point (8 lines)
│   ├── index.css             # Tailwind imports (3 lines)
│   ├── Demo.tsx              # Demo app with speed slider (86 lines)
│   └── useFakeMessages.tsx   # Fake lorem generator (47 lines)
├── package.json              # Package metadata
├── vite.config.ts            # Vite + React plugin
├── tailwind.config.js        # Tailwind + typography + icons
├── postcss.config.js         # Tailwind + Autoprefixer
├── biome.json                # Biome lint/format config
├── tsconfig.json             # Project references root
├── tsconfig.app.json         # Library: strict, isolatedDeclarations
├── tsconfig.demo.json        # Demo: noEmit, Bundler resolution
└── tsconfig.node.json        # Tooling: Vite config type-check
```

## Cross-References

- [Overview](00-overview.md) — Package summary
- [Core Hook](01-core-hook.md) — useStickToBottom implementation
- [Component API](02-component.md) — StickToBottom component
- [Demo App](03-demo.md) — Demo usage patterns
