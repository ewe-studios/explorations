# kumo AI Registry: Component Metadata for AI Tools

Machine-readable component metadata generated from TypeScript types, consumed by the CLI and AI tools.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/kumo/packages/kumo/`

## Registry Location

```
ai/
├── component-registry.json     ← Full component metadata
└── schemas/                    ← Zod schemas for AI validation
```

## CLI Commands

```bash
npx @cloudflare/kumo ls          # List all components
npx @cloudflare/kumo doc Button  # Get component documentation
npx @cloudflare/kumo docs        # Get all component documentation
```

**Source:** `src/command-line/`

## Registry Structure

```json
{
  "components": [
    {
      "name": "Button",
      "description": "Interactive button component",
      "props": [...],
      "variants": {
        "shape": ["base", "square", "circle"],
        "size": ["xs", "sm", "base", "lg"],
        "variant": ["primary", "secondary", "ghost", "destructive", "outline"]
      },
      "examples": [...]
    }
  ]
}
```

## Generation

**Source:** `scripts/component-registry/index.ts`

Generated from TypeScript types + demo examples. Always stays in sync with actual component API.

```bash
pnpm run codegen:registry
```

## Next

Continue to [03-tooling](03-tooling.md) for CI/CD and build tooling.
