# kumo Tooling: CI/CD, Changesets, Visual Regression

Kumo's CI/CD infrastructure: changeset-based versioning, beta releases, visual regression testing, and PR reporting.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/kumo/`

## Changeset Workflow

```bash
pnpm changeset     # Create changeset
pnpm version       # Consume changesets, bump versions
pnpm build         # Build all packages
pnpm release       # Publish to npm
```

### Pending Changesets

| Changeset | Description |
|-----------|-------------|
| `blue-red-button-rings` | Button ring style update |
| `bright-toasts-close` | Toast close behavior fix |
| `fix-button-firefox-turnstile` | Firefox Turnstile compatibility |
| `fix-dialog-size-width` | Dialog size width correction |
| `flow-vertical-orientation` | Flow component vertical orientation |
| `fresh-otters-draw` | Pending change |
| `spicy-maps-glow` | Pending change |

## GitHub Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `preview.yml` | PR with kumo changes | Build + publish beta |
| `release.yml` | Release tag | Production release |
| `validate-pr-description.yml` | PR opened | PR description validation |

## Visual Regression

**Source:** `ci/visual-regression/`

```
ci/
├── reporters/
│   ├── visual-regression.ts   # Visual diff reporter
│   ├── npm-release.ts         # npm release reporter
│   └── kumo-docs-preview.ts   # Docs preview reporter
├── scripts/
│   ├── validate-kumo-changeset.ts
│   └── post-pr-report.ts
└── utils/
    ├── github-api.ts
    └── pr-reporter.ts
```

## Build Scripts

```bash
pnpm run build              # Full production build
pnpm run dev                # Vite watch mode (~400ms rebuilds)
pnpm run codegen:primitives # Regenerate Base UI wrappers
pnpm run codegen:registry   # Regenerate AI registry
pnpm test:unit              # Unit tests
pnpm test:browser           # Browser tests (Playwright)
```

## Next

Return to the [README](README.md) for the full index.
