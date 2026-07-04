# Web Components Deep Dive

`@remix-run/component` (v0.1.0) provides a lightweight web component system with JSX support, virtual DOM diffing, and a stylesheet API.

Source: `packages/component/src/` — 17 source files.

## Component Base Class

Source: `lib/component.ts`

```ts
export class Component<P = {}, S = {}> {
  props: P
  state: S
  context: Record<string, unknown> = {}

  constructor(props: P) {
    this.props = props
    this.state = {} as S
  }

  setState(partial: Partial<S> | ((state: S) => Partial<S>)): void {
    const update = typeof partial === 'function'
      ? (partial as Function)(this.state)
      : partial
    this.state = { ...this.state, ...update }
    this.render()
  }

  render(): void {
    // Override in subclass
  }
}
```

## Virtual DOM

Source: `lib/vdom.ts`

```ts
export interface VNode {
  type: string | Function
  props: Record<string, unknown>
  children: VNode[]
  key?: string
  ref?: Ref
  dom?: Node
}
```

### createElement

Source: `lib/create-element.ts`

```ts
export function createElement(type, props, ...children) {
  const normalizedChildren = children.flat().filter(Boolean)
  return { type, props: { ...props, children: normalizedChildren }, key: props?.key }
}
```

### JSX Runtime

Source: `lib/jsx.ts`, `jsx-runtime.ts`, `jsx-dev-runtime.ts`

```ts
// jsx-runtime.ts
export { jsx, jsxs, Fragment } from './lib/jsx.ts'
```

Supports the modern JSX transform (`"jsx": "react-jsx"` in tsconfig).

### DOM Rendering

Source: `lib/dom.ts`

```ts
export function render(vnode: VNode, container: Element): void {
  const oldNode = container._vnode
  const newNode = diff(oldNode, vnode)
  container._vnode = newNode
}
```

### Diffing Algorithm

The diff function compares old and new VNodes:

```ts
function diff(oldVNode: VNode | null, newVNode: VNode): VNode {
  if (oldVNode === null) {
    return createDom(newVNode)
  }
  if (oldVNode.type !== newVNode.type) {
    replaceDom(oldVNode, newVNode)
    return newVNode
  }
  // Same type: diff props and children
  diffProps(oldVNode, newVNode)
  diffChildren(oldVNode, newVNode)
  return newVNode
}
```

Aha: The diffing is a simplified version of React's reconciliation — no fiber tree, no concurrent rendering. It's O(n) in the tree size with key-based child matching for lists.

## Style System

Source: `lib/style/`

### Style

Source: `lib/style/lib/style.ts`

```ts
export class Style {
  #rules: Map<string, string> = new Map()

  set(property: string, value: string): this {
    this.#rules.set(property, value)
    return this
  }

  toString(): string {
    return Array.from(this.#rules)
      .map(([k, v]) => `${k}: ${v}`)
      .join('; ')
  }
}
```

### Stylesheet

Source: `lib/style/lib/stylesheet.ts`

```ts
export class Stylesheet {
  #sheet: CSSStyleSheet

  constructor() {
    this.#sheet = new CSSStyleSheet()
    document.adoptedStyleSheets.push(this.#sheet)
  }

  insertRule(selector: string, properties: Record<string, string>): void {
    const rule = `${selector} { ${styleToString(properties)} }`
    this.#sheet.insertRule(rule)
  }
}
```

Uses the Constructable Stylesheets API (`CSSStyleSheet`) for dynamic style injection.

## Remix Types

Source: `lib/remix-types.ts`

Type definitions for Remix-specific component interfaces.

## Invariant

Source: `lib/invariant.ts`

```ts
export function invariant(condition: boolean, message: string): asserts condition {
  if (!condition) throw new Error(message)
}
```

Runtime assertion utility.

## Package Files

```
component/src/
├── index.ts                  # Public API
├── jsx-runtime.ts            # JSX runtime (production)
├── jsx-dev-runtime.ts        # JSX dev runtime
├── lib/
│   ├── component.ts          # Base Component class
│   ├── create-element.ts     # createElement
│   ├── vdom.ts               # Virtual DOM types and diffing
│   ├── jsx.ts                # JSX implementation
│   ├── dom.ts                # DOM rendering
│   ├── remix-types.ts        # Remix type definitions
│   ├── invariant.ts          # Assertion utility
│   └── style/
│       ├── index.ts          # Style API
│       ├── lib/
│       │   ├── style.ts      # Style class
│       │   └── stylesheet.ts # Stylesheet with Constructable Stylesheets
```

## Cross-References

- [Overview](00-overview.md) — What Remix is
- [Response](07-response.md) — Response helpers
- [Route Pattern](03-architecture.md) — URL pattern matching
