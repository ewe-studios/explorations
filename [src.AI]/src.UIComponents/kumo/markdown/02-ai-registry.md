# Kumo AI Registry: Component Metadata for AI Tools

Kumo's component registry provides structured metadata for AI-assisted component generation. The registry discovers components, extracts props, generates schemas, and produces markdown documentation.

Source: `packages/kumo/scripts/component-registry/` — 10 files.

## Component Registry Architecture

```
scripts/component-registry/
├── index.ts              # Main entry: discovery → metadata → markdown
├── discovery.ts          # Find components in packages/kumo/src/
├── metadata.ts           # Extract props, variants, examples
├── schema-generator.ts   # Generate JSON Schema for AI tools
├── props-filter.ts       # Filter props to public API
├── sub-components.ts     # Handle sub-component patterns
├── markdown-generator.ts # Generate markdown docs from metadata
├── cache.ts              # Cache component data between runs
├── example-cleanup.ts    # Clean up example code
└── index.test.ts         # Tests
```

## Discovery

Source: `discovery.ts`

Scans `packages/kumo/src/components/` for component files, finding all `.tsx` files that export React components. Sub-directories map to component groups.

## Metadata Extraction

Source: `metadata.ts`

For each discovered component:

```typescript
interface ComponentMetadata {
  name: string
  path: string
  props: PropDefinition[]
  variants: VariantDefinition[]
  examples: string[]
  description: string
  subComponents: string[]
}
```

## Props Filter

Source: `props-filter.ts`

Filters extracted props to only include the public API — excludes internal props (prefixed `_`), inherited HTML element props (unless explicitly defined), generic type parameters, and `React.Ref` attributes.

Aha: The props filter ensures AI tools only see the intentional public API, not implementation details or inherited props that users shouldn't use.

## Schema Generator

Source: `schema-generator.ts`

Generates JSON Schema for each component, mapping each prop to a schema property with type, description, enum (for variants), and required status. Used by AI tools to validate component usage and suggest props.

## Markdown Generator

Source: `markdown-generator.ts`

Generates markdown documentation with props table (name, type, default, description), variants, and examples for each component.

## Cache

Source: `cache.ts`

Caches component metadata between registry runs using source file hashes to detect changes. Speeds up subsequent runs by skipping unchanged components.

## Cross-References

- [Overview](00-overview.md) — What Kumo is
- [Theming](01-theming.md) — Theme system, tokens, lint rules
- [Tooling](03-tooling.md) — CI/CD, changesets, visual regression
- [Packages](04-packages.md) — Sub-packages, CI structure

[Next → Tooling](03-tooling.md)
