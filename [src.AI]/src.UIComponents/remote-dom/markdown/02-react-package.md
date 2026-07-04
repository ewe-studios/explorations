# React Package: @remote-dom/react

React bridge for remote DOM elements. Version 1.2.2.

Source: `packages/react/source/` — 6 files.

## createRemoteComponent

Source: `packages/react/source/component.tsx`

```tsx
export function createRemoteComponent(tagName, ElementConstructor, options?) {
  const Component = forwardRef((props, ref) => {
    const elementRef = useRef(null);
    useLayoutEffect(() => {
      // Create the remote element and connect it
    }, []);
    return createElement(tagName, { ref: elementRef }, /* children */);
  });
  return Component;
}
```

### Options

```ts
interface RemoteComponentOptions {
  slotProps?: boolean | { wrapper?: boolean | string };
  eventProps?: Record<string, { event: string }>;
}
```

| Option | Default | Purpose |
|--------|---------|---------|
| `slotProps` | `true` | Map React props with React element values to `remote-fragment` slots |
| `slotProps.wrapper` | `'remote-fragment'` | Wrapper element for slotted content |
| `eventProps` | `{}` | Map React prop names to custom element event names |

### Example

```tsx
const Button = createRemoteComponent('ui-button', ButtonElement, {
  eventProps: {
    onClick: { event: 'click' },
  },
});

// Usage:
<Button onClick={() => console.log('clicked')}>
  Click me
</Button>
```

## Types

Source: `packages/react/source/types.ts`

| Type | Purpose |
|------|---------|
| `RemoteComponentProps` | Props for a remote component |
| `RemoteComponentTypeFromElementConstructor` | Infer component type from element |
| `RemoteComponentPropsFromElementConstructor` | Infer props from element constructor |

## Other Exports

| File | Exports |
|------|---------|
| `host.ts` | Host-side React integration |
| `html.ts` | HTML template support |
| `polyfill.ts` | Polyfill setup for React |
| `types.ts` | Type definitions |

## Cross-References

- [Overview](00-overview.md) — Package summary
- [Core Package](01-core-package.md) — Remote elements
- [Examples](06-examples.md) — React usage in kitchen-sink
