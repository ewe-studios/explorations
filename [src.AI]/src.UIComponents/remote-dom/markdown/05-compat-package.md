# Compat Package: @remote-dom/compat

Legacy adapter for adapting to older remote-dom channel formats. Version 1.0.2.

Source: `packages/compat/source/index.ts`

## Exports

```ts
export { adaptToLegacyRemoteChannel } from './adapter/host';
```

### adaptToLegacyRemoteChannel

Adapts the current remote connection format to a legacy format expected by older host implementations.

Use this when:
- The host app uses an older version of remote-dom
- The remote context produces mutations in the new format
- You need to bridge between the two

## Cross-References

- [Overview](00-overview.md) — Package summary
- [Architecture](07-architecture.md) — Where compat fits
