# Build System

Two Vite configs: one for the library, one for the docs site.

## Library Build: `vite.lib.config.ts`

Builds the component library for distribution:

```ts
export default defineConfig({
  build: {
    lib: {
      entry: 'src/index.ts',
      formats: ['es', 'cjs'],
    },
    rollupOptions: {
      external: ['react', 'react-dom'],
    },
  },
})
```

Outputs ESM and CJS bundles with React as external peer dependency.

## Docs Build: `vite.docs.config.ts`

Builds the documentation site:

```ts
export default defineConfig({
  root: 'docs',
  plugins: [react()],
})
```

Uses `docs/` as the root directory with React plugin.

## OG Image Generation

Source: `scripts/gen-og.mjs`

Generates the social share card (`docs/public/og.png`) for the docs site.

## Vercel Deployment

Source: `vercel.json`

```json
{
  "buildCommand": "npm run build",
  "outputDirectory": "dist",
  "framework": "vite"
}
```

Deploys the docs site on Vercel.

## Cross-References

- [Overview](00-overview.md) — All components
- [Research Methodology](06-research-methodology.md) — Sourcing
