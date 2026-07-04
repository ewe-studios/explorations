# Core Package: @remote-dom/core Deep Dive

The core protocol for remote DOM communication. Version 1.11.1. This package defines the serialization format, the connection layer, receivers, and custom element infrastructure.

Source: `packages/core/source/` — multiple files.

## The Protocol: Tuple-Based Mutations

Source: `constants.ts`, `types.ts`

The remote-dom protocol uses typed tuples for mutations — not JSON objects, not a binary format. Just positional arrays:

```ts
// constants.ts
export const NODE_TYPE_ELEMENT = 1;
export const NODE_TYPE_TEXT = 3;
export const NODE_TYPE_COMMENT = 8;
export const NODE_TYPE_ROOT = 9;

export const MUTATION_TYPE_INSERT_CHILD = 0;
export const MUTATION_TYPE_REMOVE_CHILD = 1;
export const MUTATION_TYPE_UPDATE_TEXT = 2;
export const MUTATION_TYPE_UPDATE_PROPERTY = 3;

export const UPDATE_PROPERTY_TYPE_PROPERTY = 1;
export const UPDATE_PROPERTY_TYPE_ATTRIBUTE = 2;
export const UPDATE_PROPERTY_TYPE_EVENT_LISTENER = 3;

export const ROOT_ID = '~';
```

### Mutation Record Types

```ts
// types.ts
type RemoteMutationRecordInsertChild = [
  type: typeof MUTATION_TYPE_INSERT_CHILD,
  id: string,           // parent node ID
  child: RemoteTextSerialization | RemoteCommentSerialization | RemoteElementSerialization,
  index: number,        // insertion index
];

type RemoteMutationRecordRemoveChild = [
  type: typeof MUTATION_TYPE_REMOVE_CHILD,
  id: string,           // parent node ID
  index: number,        // child index to remove
];

type RemoteMutationRecordUpdateText = [
  type: typeof MUTATION_TYPE_UPDATE_TEXT,
  id: string,           // text node ID
  text: string,         // new text content
];

type RemoteMutationRecordUpdateProperty = [
  type: typeof MUTATION_TYPE_UPDATE_PROPERTY,
  id: string,           // element ID
  property: string,     // property name
  value: any,           // property value
  type?: typeof UPDATE_PROPERTY_TYPE_PROPERTY
       | typeof UPDATE_PROPERTY_TYPE_ATTRIBUTE
       | typeof UPDATE_PROPERTY_TYPE_EVENT_LISTENER,
];
```

Aha: The tuple format is deliberately minimal. Each mutation is a 3-5 element array. There's no schema, no field names — just positional data. This minimizes serialization overhead for cross-environment communication (postMessage, worker messages). The tradeoff is readability, but since these are internal protocol messages, that's acceptable.

## Node Serialization Types

```ts
interface RemoteTextSerialization {
  id: string;
  type: typeof NODE_TYPE_TEXT;
  data: string;
}

interface RemoteCommentSerialization {
  id: string;
  type: typeof NODE_TYPE_COMMENT;
  data: string;
}

interface RemoteElementSerialization {
  id: string;
  type: typeof NODE_TYPE_ELEMENT;
  tag: string;              // Custom element tag name
  properties: Record<string, any>;
  attributes: Record<string, string>;
  eventListeners: Record<string, boolean>;
  children: RemoteNodeSerialization[];
}
```

## RemoteConnection

Source: `connection.ts`

```ts
export interface RemoteConnection {
  call(method: string, args: any[]): Promise<any>;
  mutate(records: RemoteMutationRecord[]): void;
}
```

Two operations:
- **`call`**: Invoke a method on the remote side (e.g., `element.focus()`)
- **`mutate`**: Apply DOM mutations to the host side

### createRemoteConnection

```ts
export function createRemoteConnection({
  call,
  insertChild,
  removeChild,
  updateText,
  updateProperty,
}: RemoteConnectionHandler): RemoteConnection {
  const handlers = {
    [MUTATION_TYPE_INSERT_CHILD]: insertChild,
    [MUTATION_TYPE_REMOVE_CHILD]: removeChild,
    [MUTATION_TYPE_UPDATE_TEXT]: updateText,
    [MUTATION_TYPE_UPDATE_PROPERTY]: updateProperty,
  };

  return {
    call,
    mutate(records) {
      for (const record of records) {
        const handler = handlers[record[0]];
        if (handler) handler(record[1], record[2], record[3], record[4]);
      }
    },
  };
}
```

Maps mutation type tuples to named handler functions. The `mutate()` method dispatches each record to its handler by type index.

## RemoteReceiver

Source: `receivers/RemoteReceiver.ts`

Maintains a JavaScript representation of the remote DOM tree.

### Node Types with Version Tracking

```ts
interface RemoteReceiverElement {
  readonly id: string;
  readonly tag: string;
  readonly properties: Record<string, any>;
  readonly attributes: Record<string, string>;
  readonly eventListeners: Record<string, boolean>;
  readonly children: readonly RemoteReceiverNode[];
  readonly version: number;  // Incremented on every change
}
```

Every node has a `version` counter that increments on every mutation. Subscribers can watch for version changes to know when a specific node has been updated.

### Receiver Implementation

```ts
export class RemoteReceiver {
  readonly root: RemoteReceiverRoot = {
    id: ROOT_ID,
    type: NODE_TYPE_ROOT,
    children: [],
    version: 0,
    properties: {},
    attributes: {},
    eventListeners: {},
  };

  readonly connection: RemoteConnection;
  #nodes = new Map<string, RemoteReceiverNodeOrRoot>();

  constructor(options?: RemoteReceiverOptions) {
    this.#nodes.set(ROOT_ID, this.root);

    this.connection = createRemoteConnection({
      call: async (method, id, args) => {
        const node = this.#nodes.get(id);
        if (!node) throw new Error(`Node not found: ${id}`);
        return this.#call(method, node, args);
      },

      insertChild: (parentId, child, index) => {
        const parent = this.#nodes.get(parentId) as RemoteReceiverParent;
        const node = this.#deserializeNode(child);
        parent.children.splice(index, 0, node);
        parent.version++;
        this.#notify(parentId);
      },

      removeChild: (parentId, index) => {
        const parent = this.#nodes.get(parentId) as RemoteReceiverParent;
        const [removed] = parent.children.splice(index, 1);
        parent.version++;
        this.#removeNode(removed.id);
        this.#notify(parentId);
      },

      updateText: (id, text) => {
        const node = this.#nodes.get(id) as RemoteReceiverText;
        (node as Writable<typeof node>).data = text;
        node.version++;
        this.#notify(id);
      },

      updateProperty: (id, property, value, type) => {
        const node = this.#nodes.get(id) as RemoteReceiverElement;
        switch (type) {
          case UPDATE_PROPERTY_TYPE_PROPERTY:
            (node.properties as Writable<typeof node.properties>)[property] = value;
            break;
          case UPDATE_PROPERTY_TYPE_ATTRIBUTE:
            (node.attributes as Writable<typeof node.attributes>)[property] = value;
            break;
          case UPDATE_PROPERTY_TYPE_EVENT_LISTENER:
            (node.eventListeners as Writable<typeof node.eventListeners>)[property] = value != null;
            break;
        }
        node.version++;
        this.#notify(id);
      },
    });
  }
```

### Subscription System

```ts
subscribe(id: string, callback: (node: RemoteReceiverNodeOrRoot) => void): () => void {
  const subscribers = this.#subscribers.get(id) ?? new Set();
  subscribers.add(callback);
  this.#subscribers.set(id, subscribers);

  callback(this.#nodes.get(id)!);

  return () => {
    subscribers.delete(callback);
  };
}

#notify(id: string) {
  const node = this.#nodes.get(id);
  if (!node) return;

  const subscribers = this.#subscribers.get(id);
  if (subscribers) {
    for (const callback of subscribers) {
      callback(node);
    }
  }

  // Also notify parent subscribers
  if (node !== this.root) {
    const parent = this.#findParent(id);
    if (parent) this.#notify(parent.id);
  }
}
```

Aha: Subscribers are notified not just for the mutated node but also for its parent. This ensures that React components watching a parent element re-render when children are added or removed. The parent chain notification propagates up to the root.

## DOMRemoteReceiver

Source: `receivers/DOMRemoteReceiver.ts`

Creates actual DOM elements instead of JS objects.

```ts
export class DOMRemoteReceiver {
  readonly root: DocumentFragment | Element;
  readonly connection: RemoteConnection;
  private readonly attached = new Map<string, Node>();

  constructor(options?: RemoteReceiverOptions & { root?: Element; call?; retain?; release?; cache? }) {
    this.root = options?.root ?? document.createDocumentFragment();

    this.connection = createRemoteConnection({
      call: async (method, id, args) => {
        const element = this.attached.get(id);
        if (!element) throw new Error(`Element not found: ${id}`);
        if (options?.call) return options.call(element as Element, method, ...args);
        return (element as any)[method]?.(...args);
      },

      insertChild: (parentId, child, index) => {
        const parentNode = this.attached.get(parentId);
        if (!parentNode) return;

        const childNode = this.#createNode(child);
        const refChild = parentNode.childNodes[index] ?? null;
        parentNode.insertBefore(childNode, refChild);
        this.attached.set(child.id, childNode);
      },

      removeChild: (parentId, index) => {
        const parentNode = this.attached.get(parentId);
        if (!parentNode) return;

        const childNode = parentNode.childNodes[index];
        if (childNode) {
          const id = REMOTE_IDS.get(childNode);
          if (id) this.attached.delete(id);
          parentNode.removeChild(childNode);
        }
      },

      updateText: (id, text) => {
        const node = this.attached.get(id);
        if (node instanceof Text) node.data = text;
      },

      updateProperty: (id, property, value, type) => {
        const element = this.attached.get(id);
        if (!(element instanceof Element)) return;

        switch (type) {
          case UPDATE_PROPERTY_TYPE_PROPERTY:
            (element as any)[property] = value;
            break;
          case UPDATE_PROPERTY_TYPE_ATTRIBUTE:
            if (value == null) element.removeAttribute(property);
            else element.setAttribute(property, value);
            break;
          case UPDATE_PROPERTY_TYPE_EVENT_LISTENER:
            // Event listener management
            break;
        }
      },
    });
  }
```

### #createNode()

```ts
#createNode(serialization: RemoteNodeSerialization): Node {
  switch (serialization.type) {
    case NODE_TYPE_ELEMENT:
      const element = document.createElement(serialization.tag);
      setRemoteId(element, serialization.id);
      // Apply properties, attributes, event listeners
      // Recursively create children
      return element;

    case NODE_TYPE_TEXT:
      const text = document.createTextNode(serialization.data);
      setRemoteId(text, serialization.id);
      return text;

    case NODE_TYPE_COMMENT:
      const comment = document.createComment(serialization.data);
      setRemoteId(comment, serialization.id);
      return comment;
  }
}
```

## RemoteElement

Source: `elements/RemoteElement.ts`

Base class for custom elements in the remote environment.

```ts
export type RemoteElementConstructor<Properties, Methods, Slots, EventListeners> = {
  new (): RemoteElement<Properties, Methods, Slots, EventListeners> & Properties & Methods;

  readonly remoteSlots?: RemoteElementSlotsDefinition<Slots> | readonly (keyof Slots)[];
  readonly remoteProperties?: RemoteElementPropertiesDefinition<Properties> | readonly (keyof Properties)[];
  readonly remoteAttributes?: readonly string[];
  readonly remoteEvents?: RemoteElementEventListenersDefinition<EventListeners> | readonly (keyof EventListeners)[];

  createProperty<Value>(name: string, definition?: RemoteElementPropertyDefinition<Value>): void;
  createAttribute(name: string, definition?: RemoteElementAttributeDefinition): void;
  createEvent(name: string, definition?: RemoteElementEventListenerDefinition): void;
};
```

### Defining Properties

```ts
class ButtonElement extends RemoteElement {
  static remoteProperties = {
    disabled: Boolean,
    variant: String,
    loading: Boolean,
  };

  static remoteSlots = ['icon', 'label'];
  static remoteEvents = ['click', 'hover'];
}
```

Each property definition specifies how the value is serialized between remote and host:

```ts
interface RemoteElementPropertyDefinition<Value> {
  type?: RemoteElementPropertyType<Value>;  // Boolean, String, Number, etc.
  attribute?: string;                        // Map to attribute name
}
```

## RemoteMutationObserver

Source: `elements/RemoteMutationObserver.ts`

Wraps the browser's `MutationObserver` to detect DOM changes and serialize them as remote mutations.

```ts
export class RemoteMutationObserver extends MutationObserver {
  readonly connection: RemoteConnection;
  readonly #observed: Set<Node>;

  constructor(connection: RemoteConnection) {
    super((records) => {
      const addedNodes: Node[] = [];
      const remoteRecords: RemoteMutationRecord[] = [];

      for (const record of records) {
        const targetId = remoteId(record.target);

        if (record.type === 'childList') {
          // Handle removed nodes
          record.removedNodes.forEach((node) => {
            disconnectRemoteNode(node);
            remoteRecords.push([MUTATION_TYPE_REMOVE_CHILD, targetId, position]);
          });

          // Handle added nodes — serialize entire subtree
          record.addedNodes.forEach((node, index) => {
            // Skip if already included in a parent's serialization
            if (addedNodes.some((n) => n.contains(node))) return;

            addedNodes.push(node);
            connectRemoteNode(node, connection);
            remoteRecords.push([MUTATION_TYPE_INSERT_CHILD, targetId, serializeRemoteNode(node), index]);
          });
        }
      }

      if (remoteRecords.length > 0) {
        this.connection.mutate(remoteRecords);
      }
    });
  }
```

Aha: When a node is added, the observer serializes the **entire subtree**, not just the single node. This handles the case where React creates a whole tree of elements at once — rather than sending one mutation per element, it sends one mutation with the full tree serialization. The `addedNodes.some((n) => n.contains(node))` check prevents duplicate mutations when a parent and its children are both in the mutation record list.

## Element Internals

Source: `elements/internals.ts`

### Remote ID Management

```ts
const IDS = new WeakMap<Node, string>();
let nextId = 1;

export function remoteId(node: Node): string | undefined {
  return IDS.get(node);
}

export function setRemoteId(node: Node, id?: string): string {
  if (!id) id = String(nextId++);
  IDS.set(node, id);
  return id;
}
```

Each DOM node gets a unique string ID stored in a `WeakMap`. The weak reference ensures nodes are garbage collected when no longer referenced.

### connectRemoteNode / disconnectRemoteNode

```ts
export function connectRemoteNode(node: Node, connection: RemoteConnection): void {
  // Store connection on node for method calls
  // Set up property/attribute/event synchronization
}

export function disconnectRemoteNode(node: Node): void {
  // Remove connection reference
  // Clean up any observers or listeners
  IDS.delete(node);
}
```

### serializeRemoteNode

```ts
export function serializeRemoteNode(node: Node): RemoteNodeSerialization {
  if (node instanceof Text) {
    return { id: remoteId(node)!, type: NODE_TYPE_TEXT, data: node.data };
  }
  if (node instanceof Comment) {
    return { id: remoteId(node)!, type: NODE_TYPE_COMMENT, data: node.data };
  }
  if (node instanceof Element) {
    return {
      id: remoteId(node)!,
      type: NODE_TYPE_ELEMENT,
      tag: node.tagName.toLowerCase(),
      properties: getRemoteProperties(node),
      attributes: Object.fromEntries([...node.attributes].map(a => [a.name, a.value])),
      eventListeners: getRemoteEventListeners(node),
      children: [...node.childNodes].map(serializeRemoteNode),
    };
  }
}
```

Recursively serializes an element and all its children into the protocol format.

## BatchingRemoteConnection

Source: `elements/connection.ts`

```ts
export class BatchingRemoteConnection implements RemoteConnection {
  #batch: RemoteMutationRecord[] = [];
  #scheduled = false;
  #inner: RemoteConnection;

  constructor(inner: RemoteConnection) {
    this.#inner = inner;
  }

  mutate(records: RemoteMutationRecord[]): void {
    this.#batch.push(...records);

    if (!this.#scheduled) {
      this.#scheduled = true;
      queueMicrotask(() => {
        this.#scheduled = false;
        const batch = this.#batch;
        this.#batch = [];
        this.#inner.mutate(batch);
      });
    }
  }

  call(method: string, args: any[]): Promise<any> {
    return this.#inner.call(method, args);
  }
}
```

Aha: Mutations are batched using `queueMicrotask()`. Multiple DOM changes within the same microtask queue into a single `mutate()` call. This reduces the number of cross-environment messages — instead of sending one message per DOM change, all changes in a single render cycle are sent together.

## Package Structure

```
packages/core/source/
├── index.ts                    # Public API
├── connection.ts               # RemoteConnection, createRemoteConnection
├── constants.ts                # Node types, mutation types, ROOT_ID
├── types.ts                    # Serialization interfaces
├── receivers/
│   ├── RemoteReceiver.ts       # JS object tree representation
│   ├── DOMRemoteReceiver.ts    # Creates actual DOM elements
│   └── shared.ts               # RemoteReceiverOptions
├── elements/
│   ├── RemoteElement.ts        # Base class for custom elements
│   ├── RemoteFragmentElement.ts # Fragment wrapper for slots
│   ├── RemoteRootElement.ts    # Root element
│   ├── RemoteReceiverElement.ts # Receiver-side element
│   ├── RemoteEvent.ts          # Custom event class
│   ├── RemoteMutationObserver.ts # DOM change detection + serialization
│   ├── internals.ts            # remoteId, connectRemoteNode, serializeRemoteNode
│   ├── connection.ts           # BatchingRemoteConnection
│   ├── decorators/
│   │   ├── remote-slots.ts
│   │   ├── remote-properties.ts
│   │   ├── remote-property.ts
│   │   └── custom-element.ts
│   └── property-types/
│       └── BooleanOrString.ts
└── html.ts                     # HTM/React HTML template
```

## Cross-References

- [Overview](00-overview.md) — Package summary
- [React Package](02-react-package.md) — React component bridge
- [Polyfill Package](03-polyfill-package.md) — DOM API emulation
- [Architecture](07-architecture.md) — Full host/remote flow

[Next → React Package](02-react-package.md)
