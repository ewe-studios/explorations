# Polyfill Package: @remote-dom/polyfill

A polyfill for the browser APIs used by Remote DOM. Version 1.5.1.

Source: `packages/polyfill/source/` — multiple files providing DOM-like APIs for environments without a real DOM (like Web Workers).

## Polyfilled APIs

| Class/API | Source | Purpose |
|-----------|--------|---------|
| `Attr` | `Attr.ts` | Attribute node representation |
| `CharacterData` | `CharacterData.ts` | Base class for text/comment nodes |
| `ChildNode` | `ChildNode.ts` | Mixin for node child operations |
| `ClipboardEvent` | `ClipboardEvent.ts` | Clipboard event polyfill |
| `Comment` | `Comment.ts` | Comment node |
| `CustomElementRegistry` | `CustomElementRegistry.ts` | `customElements.define()` support |
| `Document` | `Document.ts` | Document polyfill |
| `DocumentFragment` | `DocumentFragment.ts` | Fragment node |
| `Element` | `Element.ts` | Element base class |
| `Event` | `Event.ts` | Event class |
| `EventTarget` | `EventTarget.ts` | Event target base |
| `HTMLElement` | `HTMLElement.ts` | HTML element |
| `MutationObserver` | `MutationObserver.ts` | Mutation observation |
| `Node` | `Node.ts` | Node base class |
| `Text` | `Text.ts` | Text node |
| `Window` | `Window.ts` | Window polyfill |

Aha: The polyfill is essential for running remote elements in Web Workers. Without it, code like `document.createElement()` or `new MutationObserver()` would fail because workers don't have a DOM. The polyfill provides just enough API surface for the remote element protocol to work — it doesn't try to be a full jsdom replacement.

## Constants

Source: `packages/polyfill/source/constants.ts`

```ts
export const NS, NEXT, VALUE, OWNER_ELEMENT, NAME, NamespaceURI, NodeType, HOOKS;
```

Symbol-based private property keys used across all polyfill classes.

## Cross-References

- [Overview](00-overview.md) — Package summary
- [Core Package](01-core-package.md) — Remote elements that need the polyfill
- [Architecture](07-architecture.md) — Where the polyfill fits
