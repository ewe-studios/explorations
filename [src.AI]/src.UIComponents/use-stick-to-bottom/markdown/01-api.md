# API Reference

## useStickToBottom()

```typescript
function useStickToBottom(options?: StickToBottomOptions): {
  containerRef: RefObject<HTMLDivElement>;
  contentRef: RefObject<HTMLDivElement>;
  isSticky: boolean;
}
```

### Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `smooth` | `boolean` | `true` | Use smooth scrolling |
| `threshold` | `number` | `50` | Pixel threshold for sticky detection |

### Returns

| Property | Type | Description |
|----------|------|-------------|
| `containerRef` | `RefObject` | Ref for scroll container |
| `contentRef` | `RefObject` | Ref for scroll content |
| `isSticky` | `boolean` | Whether currently at bottom |

## StickToBottom Component

```tsx
<StickToBottom smooth threshold={50}>
  {/* children */}
</StickToBottom>
```
