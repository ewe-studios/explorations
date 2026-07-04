# Signals Package: @remote-dom/signals

Preact signals-based receiver for reactive remote tree rendering. Version 2.1.1.

Source: `packages/signals/source/SignalRemoteReceiver.ts`

## SignalRemoteReceiver

Wraps the remote tree in Preact signals for fine-grained reactivity:

```ts
interface SignalRemoteReceiverElement {
  readonly properties: ReadonlySignal<...>;
  readonly attributes: ReadonlySignal<...>;
  readonly eventListeners: ReadonlySignal<...>;
  readonly children: ReadonlySignal<readonly SignalRemoteReceiverNode[]>;
}
```

Each node type wraps its mutable data in signals:
- **Text/Comment nodes**: `data` wrapped in `ReadonlySignal`
- **Element nodes**: `properties`, `attributes`, `eventListeners`, `children` all signals
- **Root node**: Same as element, with `id = ROOT_ID` and `type = NODE_TYPE_ROOT`

Aha: Signals enable fine-grained reactivity — only the parts of the tree that actually change trigger re-renders. Without signals, the entire tree would re-render on every mutation. This is especially important for large remote trees where mutations are frequent.

## Integration

Uses `@preact/signals-core` as the signal implementation, making it framework-agnostic (works with Preact, React via preact/compat, or any signals-compatible framework).

## Cross-References

- [Overview](00-overview.md) — Package summary
- [Core Package](01-core-package.md) — RemoteConnection protocol
