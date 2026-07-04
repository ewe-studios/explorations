# Core Package: @remote-dom/core

The core protocol for remote DOM communication. Version 1.11.1.

Source: `packages/core/source/` — multiple files.

## Exports

```ts
// packages/core/source/index.ts
export { createRemoteConnection, type RemoteConnectionHandler } from './connection.ts';
export * from './types.ts';
export { ROOT_ID, NODE_TYPE_COMMENT, NODE_TYPE_ELEMENT, NODE_TYPE_ROOT, NODE_TYPE_TEXT,
  MUTATION_TYPE_INSERT_CHILD, MUTATION_TYPE_REMOVE_CHILD, MUTATION_TYPE_UPDATE_TEXT,
  MUTATION_TYPE_UPDATE_PROPERTY, ... } from './constants.ts';
```

## RemoteConnection Protocol

Source: `packages/core/source/connection.ts`

The core protocol is a tuple-based mutation stream:

```ts
type RemoteMutationRecordInsertChild = [type, parentId, child, index];
type RemoteMutationRecordRemoveChild = [type, parentId, index];
type RemoteMutationRecordUpdateText = [type, textNodeId, text];
type RemoteMutationRecordUpdateProperty = [type, elementId, property, value, propertyType];
```

`createRemoteConnection` wraps this into named handlers:

```ts
const conn = createRemoteConnection({
  call: (method, args) => { /* method invocation */ },
  insertChild: (id, child, index) => { /* handle insert */ },
  removeChild: (id, index) => { /* handle remove */ },
  updateText: (id, text) => { /* handle text update */ },
  updateProperty: (id, property, value, type) => { /* handle property update */ },
});
```

## Receivers

Source: `packages/core/source/receivers.ts`

| Receiver | Purpose |
|----------|---------|
| `RemoteReceiver` | Pure JS tree representation |
| `DOMRemoteReceiver` | Creates actual DOM elements |

### RemoteReceiver Types

| Type | Description |
|------|-------------|
| `RemoteReceiverElement` | Element node with properties, attributes, children |
| `RemoteReceiverText` | Text node with `data` property |
| `RemoteReceiverComment` | Comment node with `data` property |
| `RemoteReceiverRoot` | Root of the tree |
| `RemoteReceiverNode` | Union of all node types |
| `RemoteReceiverParent` | Union of parent-capable nodes |

## Elements

Source: `packages/core/source/elements.ts`

| Export | Purpose |
|--------|---------|
| `RemoteElement` | Base class for remote custom elements |
| `RemoteFragmentElement` | Fragment wrapper for slotted content |
| `RemoteRootElement` | Root element of a remote tree |
| `RemoteReceiverElement` | Receiver-side element |
| `RemoteEvent` | Custom event class |
| `RemoteMutationObserver` | Observes and serializes mutations |

### Decorators

| Decorator | Purpose |
|-----------|---------|
| `@remoteSlots()` | Define slotted content regions |
| `@remoteProperties()` | Define remote properties |
| `@remoteProperty()` | Single property definition |
| `@customElement()` | Register custom element |

### Internals

| Function | Purpose |
|----------|---------|
| `remoteId(node)` | Get/set unique ID for a node |
| `connectRemoteNode(node, connection)` | Connect node to remote connection |
| `disconnectRemoteNode(node)` | Disconnect node |
| `serializeRemoteNode(node)` | Serialize node for transport |
| `updateRemoteElementProperty(...)` | Update property on remote element |
| `callRemoteElementMethod(...)` | Invoke method on remote element |

### Connection

| Export | Purpose |
|--------|---------|
| `BatchingRemoteConnection` | Batches mutations for efficiency |

## Property Types

| Type | Purpose |
|------|---------|
| `BooleanOrString` | Property type that accepts boolean or string |

## Cross-References

- [Overview](00-overview.md) — Package summary
- [React Package](02-react-package.md) — React bridge
- [Architecture](07-architecture.md) — Full flow
