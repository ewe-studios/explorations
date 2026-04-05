# Collaboration in WebEditors: Yjs and Hocuspocus Deep Dive

**Explored at:** 2026-04-05  
**Focus:** Real-time collaboration infrastructure for Tiptap and tldraw

---

## Table of Contents

1. [Part 1: Yjs Fundamentals](#part-1-yjs-fundamentals)
   - [1.1 CRDT Basics](#11-crdt-basics)
   - [1.2 Yjs Data Types](#12-yjs-data-types)
   - [1.3 Awareness System](#13-awareness-system)
   - [1.4 Providers](#14-providers)
2. [Part 2: Tiptap Collaboration (Hocuspocus)](#part-2-tiptap-collaboration-hocuspocus)
   - [2.1 Hocuspocus Server](#21-hocuspocus-server)
   - [2.2 Tiptap Collaboration](#22-tiptap-collaboration)
   - [2.3 Sync Protocol](#23-sync-protocol)
3. [Part 3: tldraw Collaboration](#part-3-tldraw-collaboration)
   - [3.1 tldraw Sync](#31-tldraw-sync)
   - [3.2 Custom Sync Implementation](#32-custom-sync-implementation)
4. [Part 4: Production Patterns](#part-4-production-patterns)
   - [4.1 Scaling Collaboration](#41-scaling-collaboration)
   - [4.2 Security](#42-security)

---

## Part 1: Yjs Fundamentals

### 1.1 CRDT Basics

#### What are CRDTs?

**CRDTs (Conflict-Free Replicated Data Types)** are data structures that can be replicated across multiple nodes without requiring coordination between them. They guarantee eventual consistency while allowing concurrent updates.

```
┌─────────────────────────────────────────────────────────────────┐
│                     CRDT Architecture                            │
│                                                                  │
│  ┌──────────┐         ┌──────────┐         ┌──────────┐        │
│  │  Node A  │◄───────►│  Node B  │◄───────►│  Node C  │        │
│  │  (Doc)   │  Sync   │  (Doc)   │  Sync   │  (Doc)   │        │
│  └────┬─────┘         └────┬─────┘         └────┬─────┘        │
│       │                    │                    │               │
│       ▼                    ▼                    ▼               │
│  ┌──────────┐         ┌──────────┐         ┌──────────┐        │
│  │ State:   │         │ State:   │         │ State:   │        │
│  │ "Hello"  │         │ "Hello"  │         │ "Hello"  │        │
│  │ + " World"│        │ + "!"   │          │ + " World"│       │
│  └──────────┘         └──────────┘         └──────────┘        │
│       │                    │                    │               │
│       └────────────────────┼────────────────────┘               │
│                            ▼                                    │
│                    ┌──────────────┐                             │
│                    │ Converged:   │                             │
│                    │ "Hello World!"│                            │
│                    └──────────────┘                             │
└─────────────────────────────────────────────────────────────────┘
```

#### Key CRDT Properties

**1. Commutativity:** Operations can be applied in any order
```typescript
// These produce the same result regardless of order
op1.then(op2) === op2.then(op1)
```

**2. Associativity:** Grouping of operations doesn't matter
```typescript
(op1 + op2) + op3 === op1 + (op2 + op3)
```

**3. Idempotency:** Applying the same operation multiple times has no additional effect
```typescript
apply(op, apply(op, state)) === apply(op, state)
```

#### Conflict-Free Resolution

Yjs uses a **Lamport timestamp-based ordering** combined with **unique client IDs** to deterministically resolve conflicts:

```typescript
// Internal Yjs conflict resolution logic
interface Operation {
  id: [clientId: number, clock: number];  // Unique identifier
  lamport: number;                         // Logical timestamp
  origin: [clientId: number, clock: number]; // Causality tracking
  content: ContentType;
}

// When two operations conflict (same position):
// 1. Higher Lamport timestamp wins
// 2. If equal, higher client ID wins
function resolveConflict(op1: Operation, op2: Operation): number {
  if (op1.lamport !== op2.lamport) {
    return op1.lamport - op2.lamport;
  }
  return op1.id[0] - op2.id[0]; // Client ID tiebreaker
}
```

#### Eventual Consistency Guarantee

```
┌─────────────────────────────────────────────────────────────────┐
│              Eventual Consistency Flow                           │
│                                                                  │
│  Time │  Client A    │  Client B    │  Server      │  Client C  │
│  ─────┼──────────────┼──────────────┼──────────────┼────────────│
│   T0  │  "Hello"     │  "Hello"     │  "Hello"     │  "Hello"   │
│   T1  │  insert(6,"X")│              │              │            │
│   T2  │  "HelloX"    │  insert(6,"Y")│              │            │
│   T3  │──────────────►│  Sync        │              │            │
│   T4  │              │──────────────►│  Merge       │            │
│   T5  │              │              │  "HelloXY"   │            │
│   T6  │              │              │──────────────►│  Sync      │
│   T7  │              │              │              │  "HelloXY" │
│   T8  │◄─────────────┤              │              │            │
│   T9  │  "HelloXY"   │  "HelloXY"   │  "HelloXY"   │  "HelloXY" │
│                                                                  │
│  All nodes converge to identical state without central coord.   │
└─────────────────────────────────────────────────────────────────┘
```

#### Yjs Data Types Overview

Yjs provides specialized CRDT types for different use cases:

| Type | Use Case | Tiptap | tldraw |
|------|----------|--------|--------|
| `Y.Text` | Rich text content | ✅ | ❌ |
| `Y.XmlFragment` | Structured documents | ✅ | ❌ |
| `Y.Map` | Key-value state | ⚠️ | ✅ |
| `Y.Array` | Ordered collections | ⚠️ | ✅ |
| `Y.Doc` | Document container | ✅ | ✅ |

---

### 1.2 Yjs Data Types

#### Y.Doc - The Root Container

Every Yjs document starts with a `Y.Doc`:

```typescript
import * as Y from 'yjs';

const doc = new Y.Doc({
  // Unique identifier for this client
  clientId: Math.floor(Math.random() * 1000000),
  
  // GC settings
  gc: true,           // Enable garbage collection
  gcFilter: () => true, // Custom GC filter
  
  // Meta information
  meta: new Map(),    // Arbitrary metadata
});

// Get or create a top-level type
const yxml = doc.getXmlFragment('document');
const ymap = doc.getMap('metadata');
const yarray = doc.getArray('history');
```

**Y.Doc Internal Structure:**

```typescript
interface YDoc {
  clientID: number;
  guid: string;
  
  // Shared types
  _xml: Map<string, Y.XmlFragment>;
  _maps: Map<string, Y.Map<any>>;
  _arrays: Map<string, Y.Array<any>>;
  _texts: Map<string, Y.Text>;
  
  // Operation tracking
  _store: struct.Store;
  _pendingStructs: Array<any>;
  _missing: Array<string>;
  
  // Event system
  _observers: Map<string, Function>;
  
  // Awareness (optional)
  awareness?: Awareness;
}
```

#### Y.XmlFragment - For Tiptap

Tiptap uses `Y.XmlFragment` to represent the document structure:

```typescript
const yxml = doc.getXmlFragment('document');

// XML elements have attributes and children
const paragraph = new Y.XmlElement('paragraph');
paragraph.setAttribute('class', 'text-lg');

const text = new Y.XmlText('Hello World');
text.insert(0, 'Bold', { bold: true });

paragraph.insert(0, [text]);
yxml.insert(0, [paragraph]);

// Tree structure:
// <paragraph class="text-lg">
//   <text bold="true">Hello World</text>
// </paragraph>
```

**Tiptap-Specific Structure:**

```typescript
// Typical Tiptap + Yjs document structure
const yxml = doc.getXmlFragment('document');

// Document structure mirrors ProseMirror nodes:
// Y.XmlElement('doc')
//   └─ Y.XmlElement('paragraph')
//      ├─ Y.XmlText("Hello ")
//      └─ Y.XmlText("World") [bold=true]

interface TiptapYjsNode {
  name: string;           // Node type (paragraph, heading, etc.)
  attrs: Record<string, any>; // Node attributes
  content: Y.XmlFragment[];   // Children
}
```

**Deep Dive: Y.XmlFragment Operations**

```typescript
// Insert operations
yxml.insert(index: number, items: Array<Y.XmlElement | Y.XmlText>);

// Delete operations
yxml.delete(index: number, length: number);

// Attribute operations
element.setAttribute(key: string, value: any);
element.removeAttribute(key: string);
element.getAttribute(key: string): any;

// Tree traversal
element.firstChild: Y.XmlElement | null;
element.nextSibling: Y.XmlElement | null;
element.parentNode: Y.XmlElement | null;
element.toArray(): Array<Y.XmlElement | Y.XmlText>;

// Events
yxml.observe(event => {
  event.changes.keys.forEach((change, key) => {
    console.log(change.action); // 'add', 'update', 'delete'
  });
  event.changes.delta.forEach(change => {
    console.log(change.insert); // Inserted elements
    console.log(change.delete); // Deleted count
  });
});
```

#### Y.Map - For tldraw State

tldraw uses `Y.Map` extensively for document state:

```typescript
const document = doc.getMap('document');

// Nested maps for shapes
const shapes = doc.getMap('shapes');
const shape1 = new Y.Map();
shape1.set('id', 'shape-1');
shape1.set('type', 'rectangle');
shape1.set('x', 100);
shape1.set('y', 200);
shape1.set('width', 300);
shape1.set('height', 200);

shapes.set('shape-1', shape1);

// Map operations
map.set(key, value);
map.get(key);
map.delete(key);
map.has(key);
map.keys();
map.values();
map.entries();
map.clear();
```

**tldraw Document Structure:**

```typescript
interface TldrawYDoc {
  document: Y.Map<{
    name: string;
    gridSize: number;
    state: Y.Map<{
      currentPageId: string;
      editingShapeId: string | null;
    }>;
  }>;
  shapes: Y.Map<Y.Map<{
    id: string;
    type: string;
    x: number;
    y: number;
    rotation: number;
    opacity: number;
    props: Y.Map<any>;
  }>>;
  pages: Y.Map<Y.Map<{
    id: string;
    name: string;
    shapeIds: Y.Array<string>;
  }>>;
  assets: Y.Map<Y.Map<{
    id: string;
    type: 'image' | 'video' | 'bookmark';
    src: string;
  }>>;
}
```

#### Y.Array - Ordered Collections

```typescript
const yarray = doc.getArray('items');

// Array operations
yarray.insert(index: number, elements: any[]);
yarray.delete(index: number, length: number);
yarray.push(elements: any[]);
yarray.unshift(elements: any[]);
yarray.get(index: number): any;
yarray.toArray(): any[];

// Observe changes
yarray.observe(event => {
  event.changes.delta.forEach(change => {
    if (change.insert) {
      console.log('Inserted:', change.insert);
    }
    if (change.delete) {
      console.log('Deleted:', change.delete, 'items');
    }
  });
});
```

#### Y.Text - Rich Text Content

```typescript
const ytext = doc.getText('content');

// Text operations
ytext.insert(index: number, content: string, attributes?: object);
ytext.delete(index: number, length: number);
ytext.toString(): string;
ytext.toDelta(): Array<{
  insert: string;
  attributes?: { bold: true; italic: true; link: string };
}>;

// Apply delta (Quill-compatible format)
ytext.applyDelta([
  { insert: 'Hello ' },
  { insert: 'World', attributes: { bold: true } },
  { insert: '!' }
]);

// Observe changes
ytext.observe(event => {
  event.changes.delta.forEach(change => {
    console.log('Text changed:', change);
  });
});
```

**Y.Text Internal Delta Format:**

```typescript
interface TextDelta {
  insert: string;
  attributes?: Record<string, any>;
  retain?: number;
  delete?: number;
}

// Example delta representing "Hello [bold]World[/bold]!"
const delta: TextDelta[] = [
  { insert: 'Hello ' },
  { insert: 'World', attributes: { bold: true } },
  { insert: '!' }
];
```

---

### 1.3 Awareness System

The Awareness system is a **separate layer** on top of Yjs that handles **ephemeral state** like cursor positions, selections, and user presence.

#### Awareness Protocol Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Awareness System                              │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Awareness Class                       │    │
│  │  ┌─────────────────────────────────────────────────────┐│    │
│  │  │  states: Map<clientId, ClientState>                 ││    │
│  │  │  meta: Map<clientId, { timestamp, user }>           ││    │
│  │  └─────────────────────────────────────────────────────┘│    │
│  └─────────────────────────────────────────────────────────┘    │
│                            │                                     │
│         ┌──────────────────┼──────────────────┐                 │
│         ▼                  ▼                  ▼                 │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐           │
│  │  Cursor     │   │  Selection  │   │  Presence   │           │
│  │  Position   │   │  Range      │   │  Status     │           │
│  └─────────────┘   └─────────────┘   └─────────────┘           │
│                                                                  │
│  Ephemeral updates (not persisted in Y.Doc)                     │
└─────────────────────────────────────────────────────────────────┘
```

#### Awareness Implementation

```typescript
import { Awareness } from 'y-protocols/awareness';

const awareness = new Awareness(doc);

// Set local state
awareness.setLocalState({
  user: {
    id: 'user-123',
    name: 'John Doe',
    color: '#ff0000',
    avatar: 'https://example.com/avatar.jpg'
  },
  cursor: {
    path: ['doc', 'paragraph', 0],
    offset: 5
  },
  selection: {
    anchor: { path: ['doc', 'paragraph', 0], offset: 5 },
    head: { path: ['doc', 'paragraph', 0], offset: 10 }
  },
  status: 'active' // 'active' | 'idle' | 'offline'
});

// Observe remote states
awareness.on('change', ({ added, updated, removed }) => {
  added.forEach(clientId => {
    const state = awareness.get(clientId);
    console.log('User joined:', state.user.name);
  });
  
  updated.forEach(clientId => {
    const state = awareness.get(clientId);
    console.log('User updated:', state);
    // Update cursor position, selection, etc.
  });
  
  removed.forEach(clientId => {
    console.log('User left:', clientId);
  });
});

// Get all states
awareness.getStates(): Map<number, any>;

// Get specific client state
awareness.get(clientId): any | null;

// Remove local state
awareness.setLocalState(null);
```

#### Cursor Positions

**Cursor rendering in Tiptap:**

```typescript
import { yCursorPlugin } from 'y-prosemirror';

const cursorPlugin = yCursorPlugin(awareness, {
  // Cursor rendering
  cursorBuilder: (cursor) => {
    const cursorElement = document.createElement('span');
    cursorElement.classList.add('yjs-cursor');
    cursorElement.style.position = 'absolute';
    cursorElement.style.borderLeft = `1px solid ${cursor.color}`;
    cursorElement.style.height = '1em';
    cursorElement.style.pointerEvents = 'none';
    
    // User label
    const label = document.createElement('div');
    label.classList.add('yjs-cursor-label');
    label.textContent = cursor.name;
    label.style.backgroundColor = cursor.color;
    label.style.color = 'white';
    label.style.padding = '2px 4px';
    label.style.borderRadius = '2px';
    label.style.fontSize = '12px';
    
    cursorElement.appendChild(label);
    return cursorElement;
  },
  
  // Selection rendering
  selectionBackground: 'rgba(0, 0, 255, 0.2)',
  
  // Throttling
  updateInterval: 100
});
```

**Custom cursor implementation:**

```typescript
interface CursorState {
  userId: string;
  userName: string;
  color: string;
  // Path through document tree
  path: Array<string | number>;
  // Character offset within text node
  offset: number;
  // Selection range (optional)
  selection?: {
    anchorOffset: number;
    headOffset: number;
  };
}

class CursorManager {
  private cursors: Map<string, CursorElement> = new Map();
  
  updateCursor(clientId: number, state: CursorState) {
    if (!this.cursors.has(clientId.toString())) {
      this.createCursor(clientId, state);
    }
    
    const cursor = this.cursors.get(clientId.toString())!;
    cursor.updatePosition(state);
  }
  
  private createCursor(clientId: number, state: CursorState) {
    const container = document.createElement('div');
    container.className = 'remote-cursor';
    container.innerHTML = `
      <div class="cursor-caret" style="border-color: ${state.color}"></div>
      <div class="cursor-label" style="background-color: ${state.color}">
        ${state.userName}
      </div>
    `;
    
    document.querySelector('.editor-container')?.appendChild(container);
    this.cursors.set(clientId.toString(), { element: container, state });
  }
}
```

#### User Presence

```typescript
interface PresenceState {
  user: {
    id: string;
    name: string;
    color: string;
    avatar?: string;
  };
  status: 'active' | 'idle' | 'away' | 'offline';
  lastActive: number;
  currentRoom: string;
  typing?: boolean;
}

class PresenceManager {
  private awareness: Awareness;
  private idleTimeout: NodeJS.Timeout;
  private readonly IDLE_THRESHOLD = 60000; // 1 minute
  
  constructor(awareness: Awareness) {
    this.awareness = awareness;
    this.setupIdleDetection();
  }
  
  setPresence(presence: Partial<PresenceState>) {
    const currentState = this.awareness.getLocalState() || {};
    this.awareness.setLocalState({
      ...currentState,
      ...presence,
      lastActive: Date.now()
    });
  }
  
  setTyping(isTyping: boolean) {
    this.setPresence({ typing: isTyping });
    
    // Auto-clear typing status after 2 seconds
    if (isTyping) {
      setTimeout(() => {
        this.setPresence({ typing: false });
      }, 2000);
    }
  }
  
  private setupIdleDetection() {
    const updateStatus = () => {
      const timeSinceActive = Date.now() - (this.awareness.getLocalState()?.lastActive || 0);
      const status = timeSinceActive > this.IDLE_THRESHOLD ? 'idle' : 'active';
      this.setPresence({ status });
    };
    
    document.addEventListener('mousemove', updateStatus);
    document.addEventListener('keydown', updateStatus);
    
    // Check every 30 seconds
    this.idleTimeout = setInterval(updateStatus, 30000);
  }
  
  getRemoteUsers(): PresenceState[] {
    const states = this.awareness.getStates();
    const users: PresenceState[] = [];
    
    states.forEach((state, clientId) => {
      if (clientId !== this.awareness.clientID && state?.user) {
        users.push(state);
      }
    });
    
    return users;
  }
}
```

#### Selection Sharing

```typescript
interface SharedSelection {
  userId: string;
  roomId: string;
  ranges: Array<{
    anchor: { path: string[]; offset: number };
    head: { path: string[]; offset: number };
  }>;
  timestamp: number;
}

class SelectionSync {
  private awareness: Awareness;
  private editor: Editor;
  
  constructor(awareness: Awareness, editor: Editor) {
    this.awareness = awareness;
    this.editor = editor;
    
    // Listen to local selection changes
    editor.on('selectionUpdate', ({ editor }) => {
      const { from, to } = editor.state.selection;
      this.broadcastSelection(from, to);
    });
    
    // Listen to remote selections
    awareness.on('change', ({ updated }) => {
      updated.forEach(clientId => {
        const state = awareness.get(clientId);
        if (state?.selection) {
          this.renderRemoteSelection(clientId, state.selection);
        }
      });
    });
  }
  
  private broadcastSelection(from: number, to: number) {
    const selection: SharedSelection = {
      userId: this.awareness.clientID.toString(),
      roomId: this.editor.options.editorProps.roomId,
      ranges: [{
        anchor: { path: this.getPath(from), offset: from },
        head: { path: this.getPath(to), offset: to }
      }],
      timestamp: Date.now()
    };
    
    this.awareness.setLocalStateField('selection', selection);
  }
  
  private renderRemoteSelection(clientId: number, selection: SharedSelection) {
    // Remove old selection highlights
    this.removeRemoteSelection(clientId);
    
    // Create new highlight
    const { anchor, head } = selection.ranges[0];
    const from = anchor.offset;
    const to = head.offset;
    
    const userState = this.awareness.get(clientId);
    const color = userState?.user?.color || '#0000ff';
    
    const decoration = Decoration.inline(from, to, {
      class: 'remote-selection',
      style: `background-color: ${this.hexToRgba(color, 0.3)}`
    });
    
    // Add to decoration set
    this.remoteDecorations.set(clientId.toString(), decoration);
    this.editor.view.dispatch(this.editor.view.state.tr);
  }
}
```

---

### 1.4 Providers

Providers are the **transport layer** for Yjs documents, handling synchronization between clients and persistence.

#### Provider Interface

```typescript
interface Provider {
  // Document reference
  doc: Y.Doc;
  
  // Awareness (optional)
  awareness?: Awareness;
  
  // Connection state
  on: (event: string, cb: Function) => void;
  off: (event: string, cb: Function) => void;
  
  // Connection management
  connect: () => void;
  disconnect: () => void;
  
  // Sync control
  sync: () => void;
  clearCache: () => void;
}

interface ProviderEvents {
  // Connection events
  'status': ({ status: 'connecting' | 'connected' | 'disconnected' }) => void;
  'connection-error': ({ error: Error }) => void;
  
  // Sync events
  'sync': (isSynced: boolean) => void;
  'synced': () => void;
  
  // Document events
  'update': (update: Uint8Array, origin: any) => void;
  'subdocs': (docs: Set<Y.Doc>) => void;
}
```

#### WebSocket Provider

The standard WebSocket provider for real-time sync:

```typescript
import WebsocketProvider from 'y-websocket';

const provider = new WebsocketProvider(
  'wss://yjs-demo.hocuspocus.dev',  // WebSocket server URL
  'my-room-id',                       // Room name for document
  doc,                               // Y.Doc instance
  {
    connect: true,                   // Auto-connect
    maxBufferTime: 0,                // No batching
    disableBc: false,                // Enable broadcast channel
    awareness: awareness             // Optional awareness instance
  }
);

// Connection events
provider.on('status', ({ status }) => {
  console.log('Connection status:', status);
});

provider.on('connection-error', ({ error }) => {
  console.error('Connection error:', error);
});

// Sync events
provider.on('sync', (isSynced) => {
  console.log('Synced:', isSynced);
});

// Manual connection control
provider.connect();
provider.disconnect();

// Force sync
provider.sync();
```

**WebSocket Provider Internals:**

```typescript
class WebsocketProvider {
  private ws: WebSocket | null = null;
  private reconnectInterval: number = 2000;
  private maxReconnectAttempts: number = 10;
  private reconnectAttempts: number = 0;
  
  // Sync protocol
  private syncStep1Sent: boolean = false;
  private syncStep2Received: boolean = false;
  
  connect() {
    this.ws = new WebSocket(this.url);
    
    this.ws.binaryType = 'arraybuffer';
    
    this.ws.onopen = () => {
      this.reconnectAttempts = 0;
      this.emit('status', { status: 'connected' });
      this.sync();
    };
    
    this.ws.onmessage = (event) => {
      this.handleMessage(new Uint8Array(event.data));
    };
    
    this.ws.onclose = () => {
      this.emit('status', { status: 'disconnected' });
      this.scheduleReconnect();
    };
  }
  
  private handleMessage(data: Uint8Array) {
    const messageType = data[0];
    
    switch (messageType) {
      case message.SyncStep1:
        // Server requests our state
        this.sendSyncStep2();
        break;
      case message.SyncStep2:
        // Server sends their state
        this.applyUpdate(data.slice(1));
        this.syncStep2Received = true;
        this.emit('synced');
        break;
      case message.Update:
        // Incremental update
        this.applyUpdate(data.slice(1));
        break;
      case message.Awareness:
        // Awareness update
        this.awareness.applyUpdate(data.slice(1));
        break;
    }
  }
}
```

#### IndexedDB Provider

Offline-first persistence:

```typescript
import IndexeddbPersistence from 'y-indexeddb';

const persistence = new IndexeddbPersistence('my-room-id', doc);

// Persistence events
persistence.on('synced', () => {
  console.log('Loaded from IndexedDB');
});

persistence.on('error', (error) => {
  console.error('IndexedDB error:', error);
});

// Combine with WebSocket provider for offline support
const wsProvider = new WebsocketProvider(wsUrl, roomId, doc);
const idbProvider = new IndexeddbPersistence(roomId, doc);

// When WebSocket connects, it syncs with server
// When offline, IndexedDB provides persistence
// When reconnected, changes sync bidirectionally
```

**IndexedDB Schema:**

```typescript
const DB_NAME = 'yjs-indexeddb';
const DB_VERSION = 1;

interface YjsStore {
  doc: Uint8Array;
  meta: {
    createdAt: number;
    updatedAt: number;
    version: number;
  };
}

class IndexeddbPersistence {
  private db: IDBDatabase | null = null;
  
  constructor(roomId: string, doc: Y.Doc) {
    this.openDB(roomId, doc);
  }
  
  private async openDB(roomId: string, doc: Y.Doc) {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(DB_NAME, DB_VERSION);
      
      request.onupgradeneeded = (event) => {
        const db = (event.target as IDBOpenDBRequest).result;
        if (!db.objectStoreNames.contains('rooms')) {
          db.createObjectStore('rooms', { keyPath: 'roomId' });
        }
      };
      
      request.onsuccess = async (event) => {
        this.db = (event.target as IDBOpenDBRequest).result;
        await this.loadFromDB(roomId, doc);
        resolve(this.db);
      };
      
      request.onerror = () => reject(request.error);
    });
  }
  
  private async loadFromDB(roomId: string, doc: Y.Doc) {
    const transaction = this.db!.transaction('rooms', 'readonly');
    const store = transaction.objectStore('rooms');
    const request = store.get(roomId);
    
    request.onsuccess = () => {
      if (request.result) {
        Y.applyUpdate(doc, request.result.doc);
        this.emit('synced');
      }
    };
  }
  
  private async saveToDB(roomId: string, update: Uint8Array) {
    const transaction = this.db!.transaction('rooms', 'readwrite');
    const store = transaction.objectStore('rooms');
    
    store.put({
      roomId,
      doc: update,
      meta: {
        updatedAt: Date.now()
      }
    });
  }
}
```

#### Custom Providers

Creating a custom provider for specialized use cases:

```typescript
class CustomProvider implements Provider {
  public doc: Y.Doc;
  public awareness?: Awareness;
  
  private subscribers: Map<string, Set<Function>> = new Map();
  private connected: boolean = false;
  
  constructor(roomId: string, doc: Y.Doc, options: CustomOptions) {
    this.doc = doc;
    this.roomId = roomId;
    
    // Listen to document updates
    doc.on('update', (update: Uint8Array, origin: any) => {
      this.broadcastUpdate(update, origin);
    });
  }
  
  // Provider interface
  connect() {
    if (this.connected) return;
    this.establishConnection();
    this.connected = true;
    this.emit('status', { status: 'connected' });
  }
  
  disconnect() {
    this.closeConnection();
    this.connected = false;
    this.emit('status', { status: 'disconnected' });
  }
  
  sync() {
    this.requestFullSync();
  }
  
  // Event emitter
  on(event: string, cb: Function) {
    if (!this.subscribers.has(event)) {
      this.subscribers.set(event, new Set());
    }
    this.subscribers.get(event)!.add(cb);
  }
  
  off(event: string, cb: Function) {
    this.subscribers.get(event)?.delete(cb);
  }
  
  private emit(event: string, data: any) {
    this.subscribers.get(event)?.forEach(cb => cb(data));
  }
  
  // Custom implementation
  private async establishConnection() {
    // Your connection logic here
  }
  
  private async broadcastUpdate(update: Uint8Array, origin: any) {
    // Your broadcast logic
  }
  
  private applyUpdate(update: Uint8Array) {
    Y.applyUpdate(this.doc, update);
  }
}
```

---

## Part 2: Tiptap Collaboration (Hocuspocus)

### 2.1 Hocuspocus Server

Hocuspocus is a **WebSocket server** specifically designed for Yjs document synchronization.

#### Server Setup

```typescript
// server.ts
import { Hocuspocus } from '@hocuspocus/server';
import { Logger } from '@hocuspocus/logger';
import { Database } from '@hocuspocus/database';

const server = new Hocuspocus({
  port: 4001,
  
  // Load document on first connection
  async onLoadDocument({ documentName, request }) {
    console.log(`Loading document: ${documentName}`);
    
    // Load from database
    const doc = await Database.fetch(documentName);
    return doc;
  },
  
  // Persist document changes
  async onStoreDocument({ documentName, document }) {
    console.log(`Storing document: ${documentName}`);
    
    // Save to database
    await Database.save(documentName, document);
  },
  
  // Debounce document storage (prevent excessive writes)
  debounce: 1000,
  
  // Maximum delay before forcing storage
  maxDebounce: 10000,
  
  // Enable debug logging
  quiet: false,
});

server.listen();
```

**Complete Production Server:**

```typescript
import { Hocuspocus, IncomingMessage, Connection, Document } from '@hocuspocus/server';
import { Redis } from 'ioredis';
import { PostgreSQL } from '@hocuspocus/database-postgresql';
import { Logger } from '@hocuspocus/logger';
import jwt from 'jsonwebtoken';

interface User {
  id: string;
  name: string;
  email: string;
  color: string;
}

interface TokenPayload {
  userId: string;
  documentId: string;
}

const redis = new Redis(process.env.REDIS_URL);
const database = new PostgreSQL({
  host: process.env.DB_HOST,
  port: parseInt(process.env.DB_PORT || '5432'),
  database: process.env.DB_NAME,
  user: process.env.DB_USER,
  password: process.env.DB_PASSWORD,
});

const server = new Hocuspocus({
  port: parseInt(process.env.HOCUSPOCUS_PORT || '4001'),
  
  // Load document from database
  async onLoadDocument({ documentName, request }) {
    const token = request?.documentName?.split('/')[0];
    
    try {
      // Verify JWT token
      const decoded = jwt.verify(token, process.env.JWT_SECRET) as TokenPayload;
      
      // Check user has access to document
      const hasAccess = await database.userHasAccess(
        decoded.userId,
        decoded.documentId
      );
      
      if (!hasAccess) {
        throw new Error('Access denied');
      }
      
      // Load or create document
      let doc = await database.getDocument(decoded.documentId);
      
      if (!doc) {
        doc = new Document(documentName);
      }
      
      return doc;
    } catch (error) {
      console.error('Failed to load document:', error);
      throw error;
    }
  },
  
  // Store document to database
  async onStoreDocument({ documentName, document, clientsCount }) {
    const token = documentName?.split('/')[0];
    
    try {
      const decoded = jwt.verify(token, process.env.JWT_SECRET) as TokenPayload;
      await database.saveDocument(decoded.documentId, document);
      console.log(`Document ${decoded.documentId} stored (${clientsCount} clients)`);
    } catch (error) {
      console.error('Failed to store document:', error);
    }
  },
  
  // Called when a client connects
  async onConnect({ connection, document }) {
    console.log(`Client connected to ${document.name}`);
    
    // Store connection in Redis for cluster awareness
    await redis.sadd(`document:${document.name}:clients`, connection.connectionId);
  },
  
  // Called when a client disconnects
  async onDisconnect({ connection, document }) {
    console.log(`Client disconnected from ${document.name}`);
    
    // Remove from Redis
    await redis.srem(`document:${document.name}:clients`, connection.connectionId);
  },
  
  // Handle awareness updates
  async onAwarenessUpdate({ document, awareness }) {
    // Broadcast awareness to other services if needed
  },
  
  // Handle document updates
  async onChange({ document, clientsCount }) {
    // Real-time analytics, webhooks, etc.
  },
  
  // Debounce settings
  debounce: 2000,
  maxDebounce: 15000,
});

// Add logging
server.use(Logger);

server.listen();
```

#### Authentication

```typescript
// Authentication middleware
import { IncomingMessage } from '@hocuspocus/server';
import jwt from 'jsonwebtoken';

const authenticate = async (connection: Connection) => {
  const token = connection.connectionContext?.token;
  
  if (!token) {
    throw new Error('Authentication required');
  }
  
  try {
    const user = jwt.verify(token, process.env.JWT_SECRET) as User;
    connection.context.user = user;
    return true;
  } catch (error) {
    throw new Error('Invalid token');
  }
};

// Usage in Hocuspocus
const server = new Hocuspocus({
  async onConnect({ connection, document }) {
    await authenticate(connection);
    
    // Verify user has access to this document
    const hasAccess = await checkDocumentAccess(
      connection.context.user.id,
      document.name
    );
    
    if (!hasAccess) {
      connection.disconnect();
      throw new Error('Access denied');
    }
  },
});
```

**Token-based Authentication Flow:**

```
┌─────────────────────────────────────────────────────────────────┐
│              Authentication Flow                                 │
│                                                                  │
│  ┌──────────┐                              ┌──────────┐         │
│  │  Client  │                              │  Server  │         │
│  └────┬─────┘                              └────┬─────┘         │
│       │                                         │               │
│       │  1. Request access token                │               │
│       │────────────────────────────────────────►│               │
│       │     (from your auth server)             │               │
│       │                                         │               │
│       │  2. JWT with document permissions       │               │
│       │◄────────────────────────────────────────│               │
│       │                                         │               │
│       │  3. WebSocket connection with token     │               │
│       │────────────────────────────────────────►│               │
│       │     ws://server/token/documentId        │               │
│       │                                         │               │
│       │                                    Verify token         │
│       │                                    Check permissions    │
│       │                                         │               │
│       │  4. Connection accepted/rejected        │               │
│       │◄────────────────────────────────────────│               │
│       │                                         │               │
│       │  5. Document sync begins                │               │
│       │◄───────────────────────────────────────►│               │
│       │                                         │               │
└─────────────────────────────────────────────────────────────────┘
```

#### Database Integration

```typescript
// PostgreSQL database adapter
import { Client } from 'pg';

class PostgreSQLAdapter {
  private client: Client;
  
  constructor(config: any) {
    this.client = new Client(config);
    this.client.connect();
  }
  
  async getDocument(documentId: string): Promise<Uint8Array | null> {
    const result = await this.client.query(
      'SELECT data FROM documents WHERE id = $1',
      [documentId]
    );
    
    if (result.rows.length === 0) {
      return null;
    }
    
    return result.rows[0].data;
  }
  
  async saveDocument(documentId: string, data: Uint8Array): Promise<void> {
    await this.client.query(
      `INSERT INTO documents (id, data, updated_at)
       VALUES ($1, $2, NOW())
       ON CONFLICT (id) DO UPDATE SET
         data = $2,
         updated_at = NOW()`,
      [documentId, data]
    );
  }
  
  async getDocumentSnapshot(documentId: string, version: number): Promise<Uint8Array> {
    const result = await this.client.query(
      'SELECT data FROM document_snapshots WHERE document_id = $1 AND version = $2',
      [documentId, version]
    );
    
    return result.rows[0]?.data;
  }
  
  async createSnapshot(documentId: string, data: Uint8Array, version: number): Promise<void> {
    await this.client.query(
      `INSERT INTO document_snapshots (document_id, data, version, created_at)
       VALUES ($1, $2, $3, NOW())`,
      [documentId, data, version]
    );
  }
  
  async getUserDocumentAccess(userId: string, documentId: string): Promise<boolean> {
    const result = await this.client.query(
      `SELECT 1 FROM document_permissions
       WHERE user_id = $1 AND document_id = $2`,
      [userId, documentId]
    );
    
    return result.rows.length > 0;
  }
}

// Redis adapter for clustering
import { Redis } from 'ioredis';

class RedisAdapter {
  private redis: Redis;
  private pubsub: Redis;
  
  constructor(url: string) {
    this.redis = new Redis(url);
    this.pubsub = new Redis(url);
  }
  
  async broadcastUpdate(documentId: string, update: Uint8Array) {
    await this.redis.publish(
      `yjs:${documentId}`,
      JSON.stringify({ update: Buffer.from(update).toString('base64') })
    );
  }
  
  subscribe(documentId: string, callback: (update: Uint8Array) => void) {
    this.pubsub.subscribe(`yjs:${documentId}`);
    
    this.pubsub.on('message', (channel, message) => {
      if (channel === `yjs:${documentId}`) {
        const { update } = JSON.parse(message);
        callback(Buffer.from(update, 'base64'));
      }
    });
  }
}
```

#### Hooks System

Hocuspocus provides lifecycle hooks for custom logic:

```typescript
import { Hocuspocus } from '@hocuspocus/server';

const server = new Hocuspocus({
  // Before document is loaded
  async beforeLoadDocument({ documentName, request }) {
    console.log(`Before load: ${documentName}`);
    // Can throw to prevent loading
  },
  
  // After document is loaded
  async afterLoadDocument({ document, request }) {
    console.log(`After load: ${document.name}`);
    // Initialize document metadata
  },
  
  // Before storing document
  async beforeStoreDocument({ documentName, document }) {
    console.log(`Before store: ${documentName}`);
    // Can throw to prevent storing
    // Validate document state
  },
  
  // After storing document
  async afterStoreDocument({ documentName, document }) {
    console.log(`After store: ${documentName}`);
    // Trigger webhooks, notifications, etc.
  },
  
  // On client connection
  async onConnect({ connection, document, clientsCount }) {
    console.log(`Connect: ${document.name} (${clientsCount} clients)`);
  },
  
  // On client disconnect
  async onDisconnect({ connection, document, clientsCount }) {
    console.log(`Disconnect: ${document.name} (${clientsCount} clients)`);
    
    // Last client disconnected - could trigger cleanup
    if (clientsCount === 0) {
      console.log('Last client disconnected');
    }
  },
  
  // On awareness state change
  async onAwarenessUpdate({ document, awareness }) {
    const states = awareness.getStates();
    // Track active users, cursors, etc.
  },
  
  // On any document change
  async onChange({ document, clientsCount }) {
    // Real-time analytics
    // Trigger downstream processes
  },
  
  // On request received
  async onRequest({ request, response }) {
    // Custom HTTP handling
  },
});
```

**Custom Hook for Version Control:**

```typescript
const server = new Hocuspocus({
  async onStoreDocument({ documentName, document }) {
    const state = Y.encodeStateAsUpdate(document);
    
    // Get current version
    const currentVersion = await this.getVersion(documentName);
    const newVersion = currentVersion + 1;
    
    // Store snapshot every 10 versions
    if (newVersion % 10 === 0) {
      await this.createSnapshot(documentName, state, newVersion);
    }
    
    // Store incremental update
    await this.storeUpdate(documentName, state, newVersion);
  },
  
  async beforeStoreDocument({ documentName, document }) {
    // Validate document isn't corrupted
    try {
      const state = Y.encodeStateAsUpdate(document);
      const testDoc = new Y.Doc();
      Y.applyUpdate(testDoc, state);
      return true;
    } catch (error) {
      console.error('Invalid document state:', error);
      throw new Error('Document validation failed');
    }
  },
});
```

---

### 2.2 Tiptap Collaboration

The Tiptap collaboration extension integrates Yjs with the editor.

#### Collaboration Extension Setup

```typescript
import { Editor } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import Collaboration from '@tiptap/extension-collaboration';
import CollaborationCursor from '@tiptap/extension-collaboration-cursor';
import * as Y from 'yjs';
import { WebsocketProvider } from 'y-websocket';
import { Awareness } from 'y-protocols/awareness';

// Create Y.js document
const ydoc = new Y.Doc();

// Create awareness instance
const awareness = new Awareness(ydoc);

// Set user info
awareness.setLocalStateField('user', {
  name: 'John Doe',
  color: '#ff0000',
});

// Create WebSocket provider
const provider = new WebsocketProvider(
  'wss://yjs-demo.hocuspocus.dev',
  'my-room-id',
  ydoc,
  { awareness }
);

// Create Tiptap editor
const editor = new Editor({
  element: document.querySelector('#editor'),
  extensions: [
    StarterKit.configure({
      // Disable history since we're using collaborative editing
      history: false,
    }),
    Collaboration.configure({
      document: ydoc,
      // Optional: specify the Y.XmlFragment field name
      field: 'default',
    }),
    CollaborationCursor.configure({
      provider: provider,
      user: {
        name: 'John Doe',
        color: '#ff0000',
      },
      render(user) {
        const cursor = document.createElement('span');
        cursor.classList.add('collaboration-cursor__caret');
        cursor.setAttribute('style', `border-color: ${user.color}`);
        
        const label = document.createElement('div');
        label.classList.add('collaboration-cursor__label');
        label.setAttribute('style', `background-color: ${user.color}`);
        label.textContent = user.name;
        
        cursor.appendChild(label);
        return cursor;
      },
    }),
  ],
});
```

#### Provider Configuration

```typescript
// Advanced provider configuration
interface ProviderConfig {
  // Connection settings
  maxReconnectAttempts?: number;
  reconnectInterval?: number;
  
  // Sync settings
  syncInterval?: number;
  
  // Awareness settings
  awareness?: Awareness;
  
  // WebSocket options
  WebSocketPolyfill?: any;
  
  // Parameters for URL
  params?: Record<string, string>;
}

const provider = new WebsocketProvider(
  'wss://collaboration.example.com',
  'room-id',
  ydoc,
  {
    // Auto-connect on creation
    connect: true,
    
    // Attach awareness
    awareness: awareness,
    
    // Disable broadcast channel (for cross-tab sync)
    disableBc: false,
    
    // Custom WebSocket implementation
    // WebSocketPolyfill: WebSocket,
    
    // URL parameters
    params: {
      token: 'auth-token-here',
      version: '1.0'
    },
    
    // Maximum buffer time for batching updates
    maxBufferTime: 0,
  }
);

// Listen to connection status
provider.on('status', ({ status }) => {
  console.log('Connection status:', status);
  
  // Update UI based on connection state
  switch (status) {
    case 'connecting':
      showConnectionIndicator('connecting');
      break;
    case 'connected':
      showConnectionIndicator('connected');
      break;
    case 'disconnected':
      showConnectionIndicator('disconnected');
      break;
  }
});

// Handle connection errors
provider.on('connection-error', ({ error }) => {
  console.error('Connection error:', error);
  showErrorNotification('Connection lost. Reconnecting...');
});

// Sync status
provider.on('sync', (isSynced) => {
  if (isSynced) {
    console.log('Document is fully synced');
  }
});
```

#### User Awareness

```typescript
// Complete awareness setup
import { Awareness } from 'y-protocols/awareness';

interface User {
  id: string;
  name: string;
  color: string;
  avatar?: string;
}

class CollaborationAwareness {
  private awareness: Awareness;
  private ydoc: Y.Doc;
  
  constructor(ydoc: Y.Doc) {
    this.ydoc = ydoc;
    this.awareness = new Awareness(ydoc);
    
    this.setupEventListeners();
  }
  
  private setupEventListeners() {
    // Handle awareness changes
    this.awareness.on('change', ({ added, updated, removed }) => {
      // New users joined
      added.forEach((clientId: number) => {
        const state = this.awareness.get(clientId);
        this.onUserJoined(clientId, state);
      });
      
      // Users updated (cursor, selection, etc.)
      updated.forEach((clientId: number) => {
        const state = this.awareness.get(clientId);
        this.onUserUpdated(clientId, state);
      });
      
      // Users left
      removed.forEach((clientId: number) => {
        this.onUserLeft(clientId);
      });
    });
  }
  
  setUser(user: User) {
    this.awareness.setLocalStateField('user', user);
  }
  
  setCursor(position: { line: number; column: number }) {
    this.awareness.setLocalStateField('cursor', position);
  }
  
  setSelection(range: { from: number; to: number }) {
    this.awareness.setLocalStateField('selection', range);
  }
  
  setStatus(status: 'active' | 'idle' | 'away') {
    this.awareness.setLocalStateField('status', status);
  }
  
  getRemoteUsers(): Array<{ clientId: number; user: User }> {
    const users: Array<{ clientId: number; user: User }> = [];
    
    this.awareness.getStates().forEach((state, clientId) => {
      if (clientId !== this.awareness.clientID && state?.user) {
        users.push({ clientId, user: state.user });
      }
    });
    
    return users;
  }
  
  private onUserJoined(clientId: number, state: any) {
    console.log(`User joined: ${state.user?.name}`);
    // Show notification, add to user list, etc.
  }
  
  private onUserUpdated(clientId: number, state: any) {
    // Update cursor position, selection, etc.
  }
  
  private onUserLeft(clientId: number) {
    console.log(`User left: ${clientId}`);
    // Remove cursor, notification, etc.
  }
}
```

#### Cursor Colors

```typescript
// Generate consistent colors for users
function generateUserColor(userId: string): string {
  const colors = [
    '#ff0000', // Red
    '#00ff00', // Green
    '#0000ff', // Blue
    '#ffff00', // Yellow
    '#00ffff', // Cyan
    '#ff00ff', // Magenta
    '#ff8000', // Orange
    '#8000ff', // Purple
    '#00ff80', // Mint
    '#ff0080', // Rose
  ];
  
  // Hash user ID to get consistent index
  let hash = 0;
  for (let i = 0; i < userId.length; i++) {
    hash = userId.charCodeAt(i) + ((hash << 5) - hash);
  }
  
  return colors[Math.abs(hash) % colors.length];
}

// Custom cursor rendering with colors
const CollaborationCursorExtension = CollaborationCursor.configure({
  provider: provider,
  
  // User info with generated color
  user: {
    name: 'John Doe',
    color: generateUserColor('user-123'),
  },
  
  // Custom rendering
  render(user) {
    const container = document.createElement('span');
    container.className = 'collaboration-cursor';
    
    // Caret element
    const caret = document.createElement('span');
    caret.className = 'collaboration-cursor__caret';
    caret.style.cssText = `
      position: absolute;
      border-left: 2px solid ${user.color};
      height: 1.2em;
      pointer-events: none;
      z-index: 10;
    `;
    
    // Label element
    const label = document.createElement('div');
    label.className = 'collaboration-cursor__label';
    label.textContent = user.name;
    label.style.cssText = `
      position: absolute;
      top: -1.5em;
      left: -2px;
      padding: 2px 6px;
      font-size: 12px;
      font-weight: 500;
      font-family: sans-serif;
      background-color: ${user.color};
      color: white;
      border-radius: 4px;
      white-space: nowrap;
      box-shadow: 0 1px 3px rgba(0,0,0,0.2);
    `;
    
    container.appendChild(caret);
    container.appendChild(label);
    
    return container;
  },
});
```

#### Presence Indicators

```typescript
// User list component with presence
class UserList {
  private container: HTMLElement;
  private awareness: Awareness;
  private users: Map<number, UserState> = new Map();
  
  constructor(container: HTMLElement, awareness: Awareness) {
    this.container = container;
    this.awareness = awareness;
    
    this.setupListeners();
    this.render();
  }
  
  private setupListeners() {
    this.awareness.on('change', ({ added, updated, removed }) => {
      added.forEach(id => this.addUser(id));
      updated.forEach(id => this.updateUser(id));
      removed.forEach(id => this.removeUser(id));
      this.render();
    });
  }
  
  private addUser(clientId: number) {
    const state = this.awareness.get(clientId);
    this.users.set(clientId, {
      clientId,
      user: state.user,
      status: state.status || 'active',
      cursor: state.cursor,
    });
  }
  
  private updateUser(clientId: number) {
    const state = this.awareness.get(clientId);
    const existing = this.users.get(clientId);
    
    if (existing) {
      this.users.set(clientId, {
        ...existing,
        status: state.status || 'active',
        cursor: state.cursor,
      });
    }
  }
  
  private removeUser(clientId: number) {
    this.users.delete(clientId);
  }
  
  private render() {
    this.container.innerHTML = '';
    
    this.users.forEach((userState) => {
      const userElement = document.createElement('div');
      userElement.className = `user-item user-item--${userState.status}`;
      
      // Avatar
      const avatar = document.createElement('div');
      avatar.className = 'user-avatar';
      avatar.style.backgroundColor = userState.user?.color || '#ccc';
      avatar.textContent = userState.user?.name?.[0]?.toUpperCase() || '?';
      
      // Name
      const name = document.createElement('span');
      name.className = 'user-name';
      name.textContent = userState.user?.name || 'Anonymous';
      
      // Status indicator
      const status = document.createElement('span');
      status.className = `user-status user-status--${userState.status}`;
      status.title = userState.status;
      
      userElement.appendChild(avatar);
      userElement.appendChild(name);
      userElement.appendChild(status);
      
      this.container.appendChild(userElement);
    });
  }
}

// CSS for presence indicators
const presenceStyles = `
.user-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px;
  border-radius: 6px;
  margin-bottom: 4px;
}

.user-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-weight: bold;
}

.user-status {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  margin-left: auto;
}

.user-status--active {
  background-color: #22c55e;
}

.user-status--idle {
  background-color: #eab308;
}

.user-status--away {
  background-color: #ef4444;
}
`;
```

---

### 2.3 Sync Protocol

Yjs uses a state-based synchronization protocol that ensures all clients converge to the same state.

#### Initial Sync

```
┌─────────────────────────────────────────────────────────────────┐
│              Initial Sync Protocol                               │
│                                                                  │
│  Client                                    Server                │
│    │                                         │                  │
│    │  1. Connect (Sync Step 1)               │                  │
│    │────────────────────────────────────────►│                  │
│    │  - State vector (what client has)       │                  │
│    │                                         │                  │
│    │  2. Sync Step 2 (Full state)            │                  │
│    │◄────────────────────────────────────────│                  │
│    │  - Missing operations                   │                  │
│    │  - Complete document state              │                  │
│    │                                         │                  │
│    │  3. Acknowledge                         │                  │
│    │────────────────────────────────────────►│                  │
│    │                                         │                  │
│    │  4. Synced ✓                            │                  │
│    │◄────────────────────────────────────────│                  │
│    │                                         │                  │
└─────────────────────────────────────────────────────────────────┘
```

**Sync Implementation:**

```typescript
// Sync message types
const message = {
  SyncStep1: 0,      // State vector
  SyncStep2: 1,      // Missing updates
  Update: 2,         // Incremental update
  Awareness: 3,      // Awareness state
  Auth: 4,           // Authentication
  AuthReply: 5,      // Authentication response
};

class SyncProtocol {
  private doc: Y.Doc;
  
  sendSyncStep1(ws: WebSocket) {
    const encoder = encoding.createEncoder();
    encoding.writeVarUint(encoder, message.SyncStep1);
    
    // Encode state vector (what we have)
    Y.writeStateVector(encoder, this.doc);
    
    ws.send(encoding.toUint8Array(encoder));
  }
  
  handleSyncStep1(data: Uint8Array, ws: WebSocket) {
    // Decode remote state vector
    const decoder = encoding.createDecoder(data);
    encoding.readVarUint(decoder); // Skip message type
    
    const stateVector = Y.readStateVector(decoder);
    
    // Create SyncStep2 with missing updates
    const encoder = encoding.createEncoder();
    encoding.writeVarUint(encoder, message.SyncStep2);
    
    Y.writeMissingUpdates(encoder, this.doc, stateVector);
    
    ws.send(encoding.toUint8Array(encoder));
    
    // Emit synced event
    this.emit('synced');
  }
  
  handleSyncStep2(data: Uint8Array) {
    const decoder = encoding.createDecoder(data);
    encoding.readVarUint(decoder); // Skip message type
    
    // Apply missing updates
    const updates = Y.readMissingUpdates(decoder);
    updates.forEach(update => {
      Y.applyUpdate(this.doc, update);
    });
  }
}
```

#### Updates Propagation

```typescript
// Incremental update propagation
class UpdatePropagation {
  private doc: Y.Doc;
  private peers: Set<WebSocket> = new Set();
  
  constructor(doc: Y.Doc) {
    this.doc = doc;
    
    // Listen for local updates
    this.doc.on('update', (update: Uint8Array, origin: any) => {
      // Don't broadcast update back to its origin
      this.broadcast(update, origin);
    });
  }
  
  private broadcast(update: Uint8Array, origin: WebSocket | null) {
    const encoder = encoding.createEncoder();
    encoding.writeVarUint(encoder, message.Update);
    encoding.writeVarUint8Array(encoder, update);
    
    const data = encoding.toUint8Array(encoder);
    
    this.peers.forEach(peer => {
      if (peer !== origin && peer.readyState === WebSocket.OPEN) {
        peer.send(data);
      }
    });
  }
  
  handleUpdate(data: Uint8Array, origin: WebSocket) {
    const decoder = encoding.createDecoder(data);
    encoding.readVarUint(decoder); // Skip message type
    
    const update = encoding.readVarUint8Array(decoder);
    
    // Apply update to document
    Y.applyUpdate(this.doc, update, origin);
    
    // Broadcast to other peers
    this.broadcast(update, origin);
  }
}
```

**Update Batching:**

```typescript
class BatchedUpdates {
  private doc: Y.Doc;
  private buffer: Uint8Array[] = [];
  private flushTimer: NodeJS.Timeout | null = null;
  private readonly MAX_BATCH_TIME = 100; // ms
  
  constructor(doc: Y.Doc) {
    this.doc = doc;
    
    this.doc.on('update', (update: Uint8Array) => {
      this.buffer.push(update);
      
      if (!this.flushTimer) {
        this.flushTimer = setTimeout(() => this.flush(), this.MAX_BATCH_TIME);
      }
      
      // Force flush if buffer gets too large
      if (this.buffer.length > 100) {
        this.flush();
      }
    });
  }
  
  private flush() {
    if (this.flushTimer) {
      clearTimeout(this.flushTimer);
      this.flushTimer = null;
    }
    
    if (this.buffer.length === 0) return;
    
    // Merge all buffered updates
    const merged = Y.mergeUpdates(this.buffer);
    
    // Send merged update
    this.send(merged);
    
    this.buffer = [];
  }
}
```

#### Conflict Resolution

```typescript
// How Yjs handles conflicts
interface ConflictResolution {
  // Example: Two users insert at the same position
  // User A: inserts "Hello " at position 0
  // User B: inserts "Hi " at position 0
  
  // Result is deterministic based on:
  // 1. Lamport timestamp (logical clock)
  // 2. Client ID (tiebreaker)
  
  scenario: {
    userA: {
      operation: 'insert',
      position: 0,
      content: 'Hello ',
      timestamp: 1000,
      clientId: 123,
    },
    userB: {
      operation: 'insert',
      position: 0,
      content: 'Hi ',
      timestamp: 1000,  // Same timestamp
      clientId: 456,   // Higher client ID wins
    },
  };
  
  // Resolution:
  // Since timestamps are equal, client ID is the tiebreaker
  // User B (456 > 123) wins, so "Hi " comes first
  // Final result: "Hi Hello "
}

// Custom conflict handling in application code
class ConflictHandler {
  private doc: Y.Doc;
  
  constructor(doc: Y.Doc) {
    this.doc = doc;
    
    // Observe conflicts at application level
    this.doc.on('update', (update, origin) => {
      // Check for potential conflicts
      this.detectConflicts(update);
    });
  }
  
  private detectConflicts(update: Uint8Array) {
    // Parse update to identify conflicting operations
    // This is application-specific logic
    
    // For example, if two users edit the same text range
    // within a short time window, flag for review
  }
  
  // Manual conflict resolution (application-level)
  resolveConflict(conflictId: string, resolution: 'accept-local' | 'accept-remote') {
    // Implementation depends on application requirements
  }
}
```

#### Offline Support

```typescript
// Offline-first architecture
class OfflineFirstProvider {
  private doc: Y.Doc;
  private wsProvider: WebsocketProvider;
  private idbProvider: IndexeddbPersistence;
  private offlineUpdates: Uint8Array[] = [];
  private isOnline: boolean = true;
  
  constructor(roomId: string, doc: Y.Doc) {
    this.doc = doc;
    
    // IndexedDB for persistence
    this.idbProvider = new IndexeddbPersistence(roomId, doc);
    
    // WebSocket for real-time sync
    this.wsProvider = new WebsocketProvider(wsUrl, roomId, doc);
    
    // Handle online/offline transitions
    window.addEventListener('online', () => this.goOnline());
    window.addEventListener('offline', () => this.goOffline());
    
    this.setupSync();
  }
  
  private goOffline() {
    this.isOnline = false;
    this.wsProvider.disconnect();
    
    console.log('Going offline - updates will be queued');
  }
  
  private goOnline() {
    this.isOnline = true;
    this.wsProvider.connect();
    
    // Sync any offline updates
    if (this.offlineUpdates.length > 0) {
      this.syncOfflineUpdates();
    }
  }
  
  private setupSync() {
    this.doc.on('update', (update, origin) => {
      if (!this.isOnline && origin !== 'idb') {
        // Queue update for later sync
        this.offlineUpdates.push(update);
        this.storeOfflineUpdate(update);
      }
    });
  }
  
  private async storeOfflineUpdate(update: Uint8Array) {
    // Store in IndexedDB or local storage
    const key = `offline-update-${Date.now()}`;
    localStorage.setItem(key, JSON.stringify(Array.from(update)));
  }
  
  private async syncOfflineUpdates() {
    console.log(`Syncing ${this.offlineUpdates.length} offline updates`);
    
    for (const update of this.offlineUpdates) {
      // Updates will be automatically sent by Yjs
      // when the connection is established
    }
    
    this.offlineUpdates = [];
    
    // Clear stored offline updates
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith('offline-update-')) {
        localStorage.removeItem(key);
      }
    }
  }
  
  getSyncStatus(): { isSynced: boolean; pendingUpdates: number } {
    return {
      isSynced: this.isOnline && this.offlineUpdates.length === 0,
      pendingUpdates: this.offlineUpdates.length,
    };
  }
}
```

---

## Part 3: tldraw Collaboration

### 3.1 tldraw Sync

tldraw uses a custom synchronization system built on top of Yjs.

#### Store Synchronization

```typescript
// tldraw store structure
import { createPresenceStateDerivation } from '@tldraw/presence';
import { createTLStore } from '@tldraw/tlschema';
import * as Y from 'yjs';

interface TldrawStore {
  // Document metadata
  document: Y.Map<{
    id: string;
    name: string;
    gridSize: number;
  }>;
  
  // All shapes in the document
  shapes: Y.Map<Y.Map<{
    id: string;
    type: string;
    parentId: string;
    x: number;
    y: number;
    rotation: number;
    opacity: number;
    props: Y.Map<any>;
  }>>;
  
  // Pages in the document
  pages: Y.Map<Y.Map<{
    id: string;
    name: string;
    index: string;
  }>>;
  
  // Current page reference
  currentPageId: Y.Map<{ id: string }>;
  
  // Assets (images, videos, bookmarks)
  assets: Y.Map<Y.Map<{
    id: string;
    type: 'image' | 'video' | 'bookmark';
    src: string;
    w: number;
    h: number;
  }>>;
  
  // Camera state
  camera: Y.Map<{
    x: number;
    y: number;
    z: number;
  }>;
}

// Creating a tldraw Yjs store
function createTldrawYjsStore(roomId: string) {
  const doc = new Y.Doc();
  
  // Create top-level types
  const document = doc.getMap('document');
  const shapes = doc.getMap('shapes');
  const pages = doc.getMap('pages');
  const assets = doc.getMap('assets');
  const camera = doc.getMap('camera');
  
  // Initialize defaults
  document.set('id', roomId);
  document.set('name', 'Untitled');
  document.set('gridSize', 10);
  
  return { doc, document, shapes, pages, assets, camera };
}
```

#### Document Merge

```typescript
// Merging tldraw documents
class TldrawMerge {
  private doc: Y.Doc;
  private shapes: Y.Map<Y.Map<any>>;
  
  constructor(doc: Y.Doc) {
    this.doc = doc;
    this.shapes = doc.getMap('shapes');
  }
  
  // Merge incoming shape updates
  mergeShape(shapeId: string, shapeData: any) {
    const existingShape = this.shapes.get(shapeId);
    
    if (!existingShape) {
      // New shape - just add it
      const yShape = new Y.Map();
      Object.entries(shapeData).forEach(([key, value]) => {
        yShape.set(key, value);
      });
      this.shapes.set(shapeId, yShape);
    } else {
      // Existing shape - merge fields
      Object.entries(shapeData).forEach(([key, value]) => {
        const currentValue = existingShape.get(key);
        
        // Handle nested maps (props)
        if (value instanceof Y.Map) {
          if (!(currentValue instanceof Y.Map)) {
            existingShape.set(key, value);
          } else {
            // Merge nested maps
            this.mergeMaps(currentValue, value);
          }
        } else if (this.shouldUpdate(currentValue, value)) {
          existingShape.set(key, value);
        }
      });
    }
  }
  
  // Last-write-wins with timestamp comparison
  private shouldUpdate(current: any, incoming: any): boolean {
    // Simple approach: incoming always wins for primitive values
    // This works because Yjs already handles ordering via CRDTs
    return true;
  }
  
  private mergeMaps(target: Y.Map<any>, source: Y.Map<any>) {
    source.forEach((value, key) => {
      target.set(key, value);
    });
  }
  
  // Handle deletions
  deleteShape(shapeId: string) {
    this.shapes.delete(shapeId);
  }
}
```

#### Presence System

```typescript
// tldraw presence implementation
import { Presence } from '@tldraw/sync';

interface TldrawPresence {
  userId: string;
  userName: string;
  color: string;
  avatar?: string;
  
  // Current shape being edited
  editingShapeId?: string;
  
  // Cursor position in canvas coordinates
  cursor: {
    x: number;
    y: number;
  } | null;
  
  // Selected shape IDs
  selection?: string[];
  
  // Viewport/camera
  viewport?: {
    x: number;
    y: number;
    w: number;
    h: number;
  };
  
  // Typing indicator
  isTyping?: boolean;
  
  // Last activity timestamp
  lastActiveTimestamp: number;
}

class TldrawPresenceManager {
  private presence: Presence<TldrawPresence>;
  private ydoc: Y.Doc;
  
  constructor(ydoc: Y.Doc, userId: string) {
    this.ydoc = ydoc;
    
    this.presence = new Presence({
      document: ydoc,
      userId: userId,
      presence: {
        userId: userId,
        userName: 'Anonymous',
        color: '#ff0000',
        cursor: null,
        lastActiveTimestamp: Date.now(),
      },
    });
    
    this.setupListeners();
  }
  
  private setupListeners() {
    // Listen to presence changes
    this.presence.subscribe((presences) => {
      this.onPresenceChange(presences);
    });
  }
  
  updateCursor(x: number, y: number) {
    this.presence.update({
      cursor: { x, y },
      lastActiveTimestamp: Date.now(),
    });
  }
  
  updateSelection(shapeIds: string[]) {
    this.presence.update({
      selection: shapeIds,
      lastActiveTimestamp: Date.now(),
    });
  }
  
  updateEditingShape(shapeId: string | undefined) {
    this.presence.update({
      editingShapeId: shapeId,
      lastActiveTimestamp: Date.now(),
    });
  }
  
  setTyping(isTyping: boolean) {
    this.presence.update({
      isTyping,
      lastActiveTimestamp: Date.now(),
    });
  }
  
  getRemotePresences(): TldrawPresence[] {
    const presences: TldrawPresence[] = [];
    
    this.presence.presences.forEach((presence, userId) => {
      if (userId !== this.presence.userId) {
        presences.push(presence);
      }
    });
    
    return presences;
  }
  
  private onPresenceChange(presences: Map<string, TldrawPresence>) {
    // Update UI with remote cursors, selections, etc.
    this.renderRemoteCursors();
    this.renderRemoteSelections();
  }
  
  private renderRemoteCursors() {
    // Clear existing cursors
    document.querySelectorAll('.remote-cursor').forEach(el => el.remove());
    
    this.getRemotePresences().forEach(presence => {
      if (presence.cursor) {
        this.createRemoteCursor(presence);
      }
    });
  }
  
  private createRemoteCursor(presence: TldrawPresence) {
    const cursor = document.createElement('div');
    cursor.className = 'remote-cursor';
    cursor.style.cssText = `
      position: absolute;
      left: ${presence.cursor!.x}px;
      top: ${presence.cursor!.y}px;
      pointer-events: none;
      z-index: 1000;
    `;
    
    cursor.innerHTML = `
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none">
        <path d="M5.625 1.5L5.625 17.2222L9.88889 12.9583L13.5833 17.0833L14.8611 15.9444L11.1667 11.8194L16.875 11.8194L5.625 1.5Z" 
              fill="${presence.color}" stroke="white" strokeWidth="1.5"/>
      </svg>
      <div style="
        position: absolute;
        top: 20px;
        left: 10px;
        background-color: ${presence.color};
        color: white;
        padding: 2px 6px;
        border-radius: 4px;
        font-size: 12px;
        font-weight: 500;
      ">
        ${presence.userName}
      </div>
    `;
    
    document.querySelector('.canvas-container')?.appendChild(cursor);
  }
}
```

#### Cursors and Selections

```typescript
// Multiplayer cursor and selection rendering
class MultiplayerRenderer {
  private container: HTMLElement;
  private cursors: Map<string, HTMLElement> = new Map();
  private selections: Map<string, HTMLElement[]> = new Map();
  
  constructor(container: HTMLElement) {
    this.container = container;
  }
  
  updateCursor(userId: string, presence: TldrawPresence) {
    let cursor = this.cursors.get(userId);
    
    if (!cursor) {
      cursor = this.createCursor(userId);
      this.cursors.set(userId, cursor);
    }
    
    if (presence.cursor) {
      cursor.style.display = 'block';
      cursor.style.transform = `translate(${presence.cursor.x}px, ${presence.cursor.y}px)`;
    } else {
      cursor.style.display = 'none';
    }
  }
  
  updateSelection(userId: string, presence: TldrawPresence) {
    // Remove old selection
    this.removeSelection(userId);
    
    if (!presence.selection || presence.selection.length === 0) {
      return;
    }
    
    // Create selection highlights
    const elements: HTMLElement[] = [];
    
    presence.selection.forEach(shapeId => {
      const highlight = this.createSelectionHighlight(shapeId, presence.color);
      if (highlight) {
        elements.push(highlight);
      }
    });
    
    this.selections.set(userId, elements);
  }
  
  private createCursor(userId: string): HTMLElement {
    const cursor = document.createElement('div');
    cursor.className = 'remote-cursor';
    cursor.style.cssText = `
      position: absolute;
      pointer-events: none;
      z-index: 10000;
      transition: transform 0.1s ease;
    `;
    this.container.appendChild(cursor);
    return cursor;
  }
  
  private createSelectionHighlight(shapeId: string, color: string): HTMLElement | null {
    const element = document.querySelector(`[data-shape-id="${shapeId}"]`);
    if (!element) return null;
    
    const rect = element.getBoundingClientRect();
    const highlight = document.createElement('div');
    highlight.className = 'remote-selection-highlight';
    highlight.style.cssText = `
      position: absolute;
      left: ${rect.left}px;
      top: ${rect.top}px;
      width: ${rect.width}px;
      height: ${rect.height}px;
      border: 2px solid ${color};
      background-color: ${this.hexToRgba(color, 0.1)};
      pointer-events: none;
      z-index: 100;
      border-radius: 4px;
    `;
    
    this.container.appendChild(highlight);
    return highlight;
  }
  
  removeSelection(userId: string) {
    const elements = this.selections.get(userId);
    if (elements) {
      elements.forEach(el => el.remove());
      this.selections.delete(userId);
    }
  }
  
  removeUser(userId: string) {
    const cursor = this.cursors.get(userId);
    if (cursor) {
        cursor.remove();
        this.cursors.delete(userId);
    }
    this.removeSelection(userId);
  }
}
```

---

### 3.2 Custom Sync Implementation

#### SyncProvider Interface

```typescript
// Custom sync provider for tldraw
interface SyncProvider {
  // Connection management
  connect(): void;
  disconnect(): void;
  
  // Document operations
  getDocument(): Promise<any>;
  pushDocument(document: any): Promise<void>;
  
  // Real-time updates
  subscribe(callback: (update: any) => void): () => void;
  
  // Presence
  getPresences(): Promise<Map<string, any>>;
  updatePresence(presence: any): void;
  
  // Events
  on(event: 'connect' | 'disconnect' | 'error' | 'update', callback: Function): void;
}

// Implementation example
class CustomSyncProvider implements SyncProvider {
  private ws: WebSocket | null = null;
  private documentId: string;
  private subscribers: Set<(update: any) => void> = new Set();
  private reconnectAttempts: number = 0;
  
  constructor(documentId: string, options: { token: string; url: string }) {
    this.documentId = documentId;
    this.options = options;
  }
  
  connect() {
    this.ws = new WebSocket(
      `${this.options.url}/${this.documentId}?token=${this.options.token}`
    );
    
    this.ws.binaryType = 'arraybuffer';
    
    this.ws.onopen = () => {
      this.reconnectAttempts = 0;
      this.emit('connect');
    };
    
    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      this.handleMessage(message);
    };
    
    this.ws.onclose = () => {
      this.emit('disconnect');
      this.scheduleReconnect();
    };
    
    this.ws.onerror = (error) => {
      this.emit('error', error);
    };
  }
  
  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
  
  async getDocument(): Promise<any> {
    const response = await fetch(`/api/documents/${this.documentId}`, {
      headers: {
        Authorization: `Bearer ${this.options.token}`,
      },
    });
    return response.json();
  }
  
  async pushDocument(document: any): Promise<void> {
    await fetch(`/api/documents/${this.documentId}`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${this.options.token}`,
      },
      body: JSON.stringify(document),
    });
  }
  
  subscribe(callback: (update: any) => void): () => void {
    this.subscribers.add(callback);
    
    return () => {
      this.subscribers.delete(callback);
    };
  }
  
  async getPresences(): Promise<Map<string, any>> {
    const response = await fetch(`/api/documents/${this.documentId}/presence`);
    const data = await response.json();
    return new Map(Object.entries(data.presences));
  }
  
  updatePresence(presence: any) {
    this.ws?.send(JSON.stringify({
      type: 'presence',
      payload: presence,
    }));
  }
  
  private handleMessage(message: any) {
    switch (message.type) {
      case 'update':
        this.subscribers.forEach(cb => cb(message.payload));
        break;
      case 'presence':
        // Handle presence update
        break;
    }
  }
  
  private scheduleReconnect() {
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
    this.reconnectAttempts++;
    
    setTimeout(() => {
      if (!this.ws) {
        this.connect();
      }
    }, delay);
  }
  
  private emit(event: string, data?: any) {
    // Emit to registered listeners
  }
}
```

#### Message Handling

```typescript
// Message protocol for custom sync
interface Message {
  type: MessageType;
  payload: any;
  timestamp: number;
  clientId: string;
}

type MessageType = 
  | 'sync-request'
  | 'sync-response'
  | 'update'
  | 'presence'
  | 'cursor'
  | 'selection'
  | 'ack';

class MessageHandler {
  private ws: WebSocket;
  private pendingAcks: Map<string, (ack: Message) => void> = new Map();
  
  constructor(ws: WebSocket) {
    this.ws = ws;
    this.setupHeartbeat();
  }
  
  send(type: MessageType, payload: any, requireAck: boolean = false): Promise<void> {
    const message: Message = {
      type,
      payload,
      timestamp: Date.now(),
      clientId: this.clientId,
    };
    
    this.ws.send(JSON.stringify(message));
    
    if (requireAck) {
      return this.waitForAck(message);
    }
    
    return Promise.resolve();
  }
  
  private async waitForAck(message: Message): Promise<void> {
    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingAcks.delete(message.type);
        reject(new Error('Ack timeout'));
      }, 5000);
      
      this.pendingAcks.set(message.type, (ack) => {
        clearTimeout(timeout);
        resolve();
      });
    });
  }
  
  handleIncoming(data: string) {
    const message: Message = JSON.parse(data);
    
    switch (message.type) {
      case 'sync-request':
        this.handleSyncRequest(message);
        break;
      case 'sync-response':
        this.handleSyncResponse(message);
        break;
      case 'update':
        this.handleUpdate(message);
        break;
      case 'presence':
        this.handlePresence(message);
        break;
      case 'ack':
        this.handleAck(message);
        break;
    }
  }
  
  private handleSyncRequest(message: Message) {
    // Send full document state
    this.send('sync-response', {
      document: this.getDocument(),
      clock: this.clock,
    });
  }
  
  private handleUpdate(message: Message) {
    // Apply update and broadcast to others
    this.applyUpdate(message.payload);
    
    // Don't broadcast back to sender
    if (message.clientId !== this.clientId) {
      this.broadcast(message);
    }
  }
  
  private setupHeartbeat() {
    setInterval(() => {
      this.send('presence', {
        type: 'heartbeat',
        timestamp: Date.now(),
      });
    }, 30000);
  }
}
```

#### State Reconciliation

```typescript
// Reconciliation for concurrent edits
interface StateVector {
  clientId: string;
  clock: number;
}

class StateReconciliation {
  private localClock: number = 0;
  private remoteClocks: Map<string, number> = new Map();
  private pendingUpdates: Array<{ update: any; clock: StateVector }> = [];
  
  // Apply update with conflict detection
  applyUpdate(update: any, remoteClock: StateVector) {
    const localClock = this.remoteClocks.get(remoteClock.clientId) || 0;
    
    if (remoteClock.clock <= localClock) {
      // Stale update - already processed
      return;
    }
    
    // Check for conflicts
    const conflicts = this.detectConflicts(update);
    
    if (conflicts.length > 0) {
      // Resolve conflicts
      update = this.resolveConflicts(update, conflicts);
    }
    
    // Apply update
    this.apply(update);
    
    // Update clock
    this.remoteClocks.set(remoteClock.clientId, remoteClock.clock);
    this.localClock++;
  }
  
  private detectConflicts(update: any): Conflict[] {
    const conflicts: Conflict[] = [];
    
    // Check if update modifies same entities as recent local updates
    const affectedIds = this.getAffectedIds(update);
    
    for (const localUpdate of this.pendingUpdates) {
      const localIds = this.getAffectedIds(localUpdate.update);
      
      if (this.hasIntersection(affectedIds, localIds)) {
        conflicts.push({
          remote: update,
          local: localUpdate.update,
          affectedIds: this.getIntersection(affectedIds, localIds),
        });
      }
    }
    
    return conflicts;
  }
  
  private resolveConflicts(update: any, conflicts: Conflict[]): any {
    for (const conflict of conflicts) {
      // Last-write-wins based on clock
      if (conflict.remote.clock > conflict.local.clock) {
        // Remote wins - no change needed
      } else {
        // Local wins - revert remote changes to conflicting fields
        update = this.revertConflictingFields(update, conflict);
      }
    }
    
    return update;
  }
  
  getPendingUpdates(): Array<{ update: any; clock: StateVector }> {
    return this.pendingUpdates;
  }
  
  getMissingUpdates(since: StateVector[]): Array<StateVector> {
    const missing: StateVector[] = [];
    
    since.forEach(clock => {
      const localClock = this.remoteClocks.get(clock.clientId) || 0;
      if (clock.clock > localClock) {
        missing.push(clock);
      }
    });
    
    return missing;
  }
}
```

#### Network Optimization

```typescript
// Optimizations for network efficiency
class NetworkOptimizer {
  private updateBuffer: Uint8Array[] = [];
  private flushTimer: NodeJS.Timeout | null = null;
  private readonly FLUSH_INTERVAL = 50; // ms
  private readonly MAX_BUFFER_SIZE = 4096; // bytes
  
  // Compression
  async compressUpdate(update: Uint8Array): Promise<Uint8Array> {
    // Use compression for large updates
    if (update.length > 1024) {
      const compressed = await this.compress(update);
      return compressed;
    }
    return update;
  }
  
  async decompressUpdate(data: Uint8Array): Promise<Uint8Array> {
    if (this.isCompressed(data)) {
      return await this.decompress(data);
    }
    return data;
  }
  
  // Batching
  bufferUpdate(update: Uint8Array) {
    this.updateBuffer.push(update);
    
    const totalSize = this.updateBuffer.reduce((acc, u) => acc + u.length, 0);
    
    // Flush if buffer is too large
    if (totalSize > this.MAX_BUFFER_SIZE) {
      this.flush();
      return;
    }
    
    // Schedule flush
    if (!this.flushTimer) {
      this.flushTimer = setTimeout(() => this.flush(), this.FLUSH_INTERVAL);
    }
  }
  
  private flush() {
    if (this.flushTimer) {
      clearTimeout(this.flushTimer);
      this.flushTimer = null;
    }
    
    if (this.updateBuffer.length === 0) return;
    
    // Merge updates using Yjs
    const merged = Y.mergeUpdates(this.updateBuffer);
    
    // Send merged update
    this.send(merged);
    
    this.updateBuffer = [];
  }
  
  // Differential sync
  async sendDifferential(localState: StateVector, remoteState: StateVector) {
    const missing = this.getMissingUpdates(localState, remoteState);
    
    if (missing.length === 0) {
      // States are in sync
      return;
    }
    
    // Only send missing updates
    const updates = await this.getUpdates(missing);
    for (const update of updates) {
      this.send(update);
    }
  }
  
  // Throttle cursor updates
  private lastCursorSend: number = 0;
  private readonly CURSOR_THROTTLE = 100; // ms
  
  throttledCursorUpdate(cursor: CursorPosition) {
    const now = Date.now();
    
    if (now - this.lastCursorSend < this.CURSOR_THROTTLE) {
      // Skip - throttled
      return;
    }
    
    this.lastCursorSend = now;
    this.sendCursor(cursor);
  }
}
```

---

## Part 4: Production Patterns

### 4.1 Scaling Collaboration

#### Room Management

```typescript
// Scalable room architecture
interface RoomManager {
  createRoom(roomId: string, options: RoomOptions): Promise<Room>;
  getRoom(roomId: string): Promise<Room | null>;
  deleteRoom(roomId: string): Promise<void>;
  getRoomStats(roomId: string): Promise<RoomStats>;
}

class DistributedRoomManager implements RoomManager {
  private rooms: Map<string, Room> = new Map();
  private redis: Redis;
  private nodeId: string;
  
  constructor(redis: Redis, nodeId: string) {
    this.redis = redis;
    this.nodeId = nodeId;
    
    this.setupDiscovery();
  }
  
  async createRoom(roomId: string, options: RoomOptions): Promise<Room> {
    // Check if room exists on another node
    const existingNode = await this.findRoomNode(roomId);
    
    if (existingNode && existingNode !== this.nodeId) {
      // Room is on another node - proxy to it
      return this.proxyToNode(existingNode, roomId);
    }
    
    // Create room locally
    const room = new Room(roomId, options);
    this.rooms.set(roomId, room);
    
    // Register in Redis
    await this.redis.set(`room:${roomId}:node`, this.nodeId);
    await this.redis.expire(`room:${roomId}:node`, 3600); // TTL
    
    return room;
  }
  
  async getRoom(roomId: string): Promise<Room | null> {
    // Check locally first
    const localRoom = this.rooms.get(roomId);
    if (localRoom) {
      return localRoom;
    }
    
    // Check Redis for room location
    const node = await this.redis.get(`room:${roomId}:node`);
    
    if (node) {
      if (node === this.nodeId) {
        return this.rooms.get(roomId) || null;
      } else {
        // Proxy to remote node
        return this.proxyToNode(node, roomId);
      }
    }
    
    return null;
  }
  
  private async setupDiscovery() {
    // Heartbeat for this node
    setInterval(async () => {
      await this.redis.set(
        `node:${this.nodeId}:heartbeat`,
        JSON.stringify({
          timestamp: Date.now(),
          roomCount: this.rooms.size,
        })
      );
      await this.redis.expire(`node:${this.nodeId}:heartbeat`, 30);
    }, 10000);
    
    // Cleanup stale rooms
    setInterval(async () => {
      for (const [roomId, room] of this.rooms) {
        const node = await this.redis.get(`room:${roomId}:node`);
        if (node !== this.nodeId) {
          this.rooms.delete(roomId);
        }
      }
    }, 60000);
  }
}
```

#### Load Balancing

```
┌─────────────────────────────────────────────────────────────────┐
│              Load Balancing Architecture                         │
│                                                                  │
│                         ┌─────────────┐                         │
│                         │   Load      │                         │
│                         │  Balancer   │                         │
│                         │  (nginx/    │                         │
│                         │   HAProxy)  │                         │
│                         └──────┬──────┘                         │
│                                │                                 │
│           ┌────────────────────┼────────────────────┐           │
│           │                    │                    │           │
│           ▼                    ▼                    ▼           │
│  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐   │
│  │    Node A       │ │    Node B       │ │    Node C       │   │
│  │  ┌───────────┐  │ │  ┌───────────┐  │ │  ┌───────────┐  │   │
│  │  │ Room 1    │  │ │  │ Room 2    │  │ │  │ Room 3    │  │   │
│  │  │ Room 4    │  │ │  │ Room 5    │  │ │  │ Room 6    │  │   │
│  │  └───────────┘  │ │  └───────────┘  │ │  └───────────┘  │   │
│  └────────┬────────┘ └────────┬────────┘ └────────┬────────┘   │
│           │                    │                    │           │
│           └────────────────────┼────────────────────┘           │
│                                │                                 │
│                                ▼                                 │
│                       ┌─────────────────┐                       │
│                       │      Redis      │                       │
│                       │   (Room->Node   │                       │
│                       │    mapping)     │                       │
│                       └─────────────────┘                       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Sticky Sessions with Redis:**

```typescript
// Load balancer configuration for sticky sessions
// nginx.conf example:
/*
upstream hocuspocus {
    ip_hash;  # Sticky sessions based on client IP
    server node1:4001;
    server node2:4001;
    server node3:4001;
}

server {
    listen 80;
    
    location / {
        proxy_pass http://hocuspocus;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
*/

// Alternative: Room-based routing
class RoomBasedRouter {
  private redis: Redis;
  private consistentHash: ConsistentHash;
  
  constructor(redis: Redis, nodes: string[]) {
    this.redis = redis;
    this.consistentHash = new ConsistentHash(nodes);
  }
  
  async getRoomNode(roomId: string): Promise<string> {
    // Check existing mapping
    const cached = await this.redis.get(`room:${roomId}:node`);
    if (cached) {
      return cached;
    }
    
    // Use consistent hashing for distribution
    const node = this.consistentHash.getNode(roomId);
    
    // Cache the mapping
    await this.redis.set(`room:${roomId}:node`, node, 'EX', 3600);
    
    return node;
  }
  
  async rebalance() {
    // Called when nodes are added/removed
    const nodes = await this.getActiveNodes();
    this.consistentHash.updateNodes(nodes);
    
    // Migrate rooms as needed
    await this.migrateRooms();
  }
}
```

#### Persistence Strategies

```typescript
// Multi-tier persistence
class PersistenceManager {
  private redis: Redis;
  private postgres: PostgreSQL;
  private s3: S3Client;
  
  // Write-behind caching
  async storeDocument(documentId: string, doc: Y.Doc) {
    const state = Y.encodeStateAsUpdate(doc);
    
    // 1. Immediate Redis write (fast)
    await this.redis.set(
      `doc:${documentId}:state`,
      Buffer.from(state).toString('base64'),
      'EX', 3600 // 1 hour TTL
    );
    
    // 2. Async PostgreSQL write (durable)
    this.queueDatabaseWrite(documentId, state);
    
    // 3. Periodic S3 snapshot (backup)
    if (this.shouldSnapshot(documentId)) {
      this.queueSnapshot(documentId, state);
    }
  }
  
  async loadDocument(documentId: string): Promise<Y.Doc | null> {
    // 1. Try Redis first (fastest)
    const redisState = await this.redis.get(`doc:${documentId}:state`);
    if (redisState) {
      const doc = new Y.Doc();
      Y.applyUpdate(doc, Buffer.from(redisState, 'base64'));
      return doc;
    }
    
    // 2. Try PostgreSQL (durable)
    const dbState = await this.postgres.getDocument(documentId);
    if (dbState) {
      const doc = new Y.Doc();
      Y.applyUpdate(doc, dbState);
      
      // Warm Redis cache
      await this.redis.set(
        `doc:${documentId}:state`,
        Buffer.from(dbState).toString('base64'),
        'EX', 3600
      );
      
      return doc;
    }
    
    // 3. Try S3 (coldest)
    const s3State = await this.s3.getSnapshot(documentId);
    if (s3State) {
      const doc = new Y.Doc();
      Y.applyUpdate(doc, s3State);
      return doc;
    }
    
    return null;
  }
  
  private dbWriteQueue: Array<{ documentId: string; state: Uint8Array }> = [];
  private async queueDatabaseWrite(documentId: string, state: Uint8Array) {
    this.dbWriteQueue.push({ documentId, state });
    
    // Batch writes
    if (this.dbWriteQueue.length >= 100) {
      this.flushDatabaseWrites();
    }
  }
  
  private async flushDatabaseWrites() {
    const batch = this.dbWriteQueue.splice(0, 100);
    
    await this.postgres.transaction(async (tx) => {
      for (const { documentId, state } of batch) {
        await tx.query(
          `INSERT INTO documents (id, data, updated_at)
           VALUES ($1, $2, NOW())
           ON CONFLICT (id) DO UPDATE SET
             data = $2,
             updated_at = NOW()`,
          [documentId, state]
        );
      }
    });
  }
  
  private async queueSnapshot(documentId: string, state: Uint8Array) {
    const version = await this.getNextSnapshotVersion(documentId);
    
    await this.s3.putObject({
      Bucket: 'yjs-snapshots',
      Key: `${documentId}/snapshots/${version}.yjs`,
      Body: state,
    });
  }
  
  private snapshotIntervals: Map<string, number> = new Map();
  private shouldSnapshot(documentId: string): boolean {
    const lastSnapshot = this.snapshotIntervals.get(documentId) || 0;
    const now = Date.now();
    
    // Snapshot every 5 minutes
    if (now - lastSnapshot > 300000) {
      this.snapshotIntervals.set(documentId, now);
      return true;
    }
    
    return false;
  }
}
```

#### Backup and Recovery

```typescript
// Backup and recovery system
class BackupRecovery {
  private s3: S3Client;
  private postgres: PostgreSQL;
  
  // Create point-in-time snapshot
  async createSnapshot(documentId: string): Promise<string> {
    const doc = await this.loadDocument(documentId);
    if (!doc) throw new Error('Document not found');
    
    const state = Y.encodeStateAsUpdate(doc);
    const timestamp = new Date().toISOString();
    const version = await this.getNextVersion(documentId);
    
    // Store in S3
    const key = `${documentId}/snapshots/${version}-${timestamp}.yjs`;
    await this.s3.putObject({
      Bucket: 'yjs-backups',
      Key: key,
      Body: state,
      Metadata: {
        documentId,
        version: version.toString(),
        timestamp,
      },
    });
    
    // Record in database
    await this.postgres.query(
      `INSERT INTO snapshots (document_id, version, s3_key, created_at)
       VALUES ($1, $2, $3, NOW())`,
      [documentId, version, key]
    );
    
    return version.toString();
  }
  
  // Restore from snapshot
  async restoreFromSnapshot(documentId: string, version: string): Promise<void> {
    const snapshot = await this.postgres.query(
      `SELECT s3_key FROM snapshots
       WHERE document_id = $1 AND version = $2`,
      [documentId, version]
    );
    
    if (snapshot.rows.length === 0) {
      throw new Error('Snapshot not found');
    }
    
    const s3Key = snapshot.rows[0].s3_key;
    const state = await this.s3.getObject({
      Bucket: 'yjs-backups',
      Key: s3Key,
    });
    
    const body = await state.Body?.transformToByteArray();
    if (!body) throw new Error('Failed to read snapshot');
    
    // Apply to document
    const doc = new Y.Doc();
    Y.applyUpdate(doc, body);
    
    // Save as current state
    await this.saveDocument(documentId, doc);
  }
  
  // List available snapshots
  async listSnapshots(documentId: string): Promise<Array<{
    version: string;
    timestamp: Date;
    size: number;
  }>> {
    const result = await this.postgres.query(
      `SELECT version, created_at as timestamp
       FROM snapshots
       WHERE document_id = $1
       ORDER BY version DESC`,
      [documentId]
    );
    
    return result.rows.map(row => ({
      version: row.version.toString(),
      timestamp: row.timestamp,
      size: 0, // Would need to fetch from S3 for actual size
    }));
  }
  
  // Point-in-time recovery using operation log
  async recoverToPointInTime(
    documentId: string,
    targetTime: Date
  ): Promise<Y.Doc> {
    // Get snapshot before target time
    const snapshot = await this.getLatestSnapshotBefore(documentId, targetTime);
    
    let doc: Y.Doc;
    if (snapshot) {
      const state = await this.loadSnapshot(snapshot.s3_key);
      doc = new Y.Doc();
      Y.applyUpdate(doc, state);
    } else {
      doc = new Y.Doc();
    }
    
    // Replay operations up to target time
    const operations = await this.getOperations(documentId, snapshot?.timestamp, targetTime);
    
    for (const op of operations) {
      Y.applyUpdate(doc, op.update);
    }
    
    return doc;
  }
  
  // Automatic backup scheduling
  setupAutomaticBackups(documentId: string, intervalMinutes: number = 60) {
    setInterval(async () => {
      try {
        await this.createSnapshot(documentId);
        console.log(`Backup created for ${documentId}`);
      } catch (error) {
        console.error(`Backup failed for ${documentId}:`, error);
      }
    }, intervalMinutes * 60000);
  }
}
```

---

### 4.2 Security

#### Authentication

```typescript
// JWT-based authentication
import jwt from 'jsonwebtoken';
import { IncomingMessage } from '@hocuspocus/server';

interface AuthPayload {
  userId: string;
  email: string;
  permissions: string[];
}

class AuthMiddleware {
  private secret: string;
  
  constructor(secret: string) {
    this.secret = secret;
  }
  
  async authenticate(connection: Connection): Promise<AuthPayload> {
    const token = this.extractToken(connection);
    
    if (!token) {
      throw new Error('Authentication required');
    }
    
    try {
      const payload = jwt.verify(token, this.secret) as AuthPayload;
      return payload;
    } catch (error) {
      throw new Error('Invalid token');
    }
  }
  
  private extractToken(connection: Connection): string | null {
    // Check URL parameters
    const url = new URL(connection.url, 'http://localhost');
    const urlToken = url.searchParams.get('token');
    if (urlToken) return urlToken;
    
    // Check Authorization header
    const authHeader = connection.requestHeaders?.authorization;
    if (authHeader?.startsWith('Bearer ')) {
      return authHeader.substring(7);
    }
    
    return null;
  }
}

// Usage in Hocuspocus
const auth = new AuthMiddleware(process.env.JWT_SECRET!);

const server = new Hocuspocus({
  async onConnect({ connection, document }) {
    const user = await auth.authenticate(connection);
    
    // Attach user to connection context
    connection.context.user = user;
    
    // Verify document access
    const hasAccess = await checkDocumentAccess(user.userId, document.name);
    if (!hasAccess) {
      throw new Error('Access denied');
    }
    
    return true;
  },
});
```

#### Authorization

```typescript
// Fine-grained authorization
enum Permission {
  READ = 'read',
  WRITE = 'write',
  ADMIN = 'admin',
}

interface DocumentACL {
  documentId: string;
  entries: Array<{
    userId: string;
    permissions: Permission[];
  }>;
}

class AuthorizationManager {
  private redis: Redis;
  
  async checkPermission(
    userId: string,
    documentId: string,
    requiredPermission: Permission
  ): Promise<boolean> {
    const acl = await this.getDocumentACL(documentId);
    
    const entry = acl.entries.find(e => e.userId === userId);
    if (!entry) return false;
    
    // Check if user has required permission or higher
    if (entry.permissions.includes(Permission.ADMIN)) {
      return true;
    }
    
    if (requiredPermission === Permission.READ) {
      return entry.permissions.includes(Permission.READ) ||
             entry.permissions.includes(Permission.WRITE);
    }
    
    if (requiredPermission === Permission.WRITE) {
      return entry.permissions.includes(Permission.WRITE);
    }
    
    return entry.permissions.includes(requiredPermission);
  }
  
  async grantPermission(
    userId: string,
    documentId: string,
    permission: Permission,
    grantedBy: string // Admin who granted
  ): Promise<void> {
    const acl = await this.getDocumentACL(documentId);
    
    const entry = acl.entries.find(e => e.userId === userId);
    if (entry) {
      if (!entry.permissions.includes(permission)) {
        entry.permissions.push(permission);
      }
    } else {
      acl.entries.push({
        userId,
        permissions: [permission],
      });
    }
    
    await this.saveDocumentACL(documentId, acl);
  }
  
  async revokePermission(
    userId: string,
    documentId: string,
    permission: Permission
  ): Promise<void> {
    const acl = await this.getDocumentACL(documentId);
    
    const entry = acl.entries.find(e => e.userId === userId);
    if (entry) {
      entry.permissions = entry.permissions.filter(p => p !== permission);
      
      // Remove entry if no permissions left
      if (entry.permissions.length === 0) {
        acl.entries = acl.entries.filter(e => e.userId !== userId);
      }
    }
    
    await this.saveDocumentACL(documentId, acl);
  }
  
  // Role-based access control
  async checkRole(userId: string, role: string): Promise<boolean> {
    const userRoles = await this.getUserRoles(userId);
    return userRoles.includes(role);
  }
}
```

#### Rate Limiting

```typescript
// Rate limiting for WebSocket connections
import { RateLimiterRedis } from 'rate-limiter-flexible';

class RateLimiter {
  private limiter: RateLimiterRedis;
  
  constructor(redis: Redis) {
    this.limiter = new RateLimiterRedis({
      storeClient: redis,
      keyPrefix: 'ratelimit',
      points: 100, // 100 requests
      duration: 60, // per 60 seconds
    });
  }
  
  async checkLimit(clientId: string): Promise<void> {
    try {
      await this.limiter.consume(clientId);
    } catch (error) {
      throw new Error('Rate limit exceeded');
    }
  }
  
  // Different limits for different operations
  async checkOperationLimit(
    clientId: string,
    operation: string
  ): Promise<void> {
    const limits: Record<string, number> = {
      'document:update': 1000, // High limit for updates
      'document:read': 100,
      'presence:update': 60,
      'connection:new': 10, // Low limit for new connections
    };
    
    const limiter = new RateLimiterRedis({
      storeClient: this.limiter.storeClient,
      keyPrefix: `ratelimit:${operation}`,
      points: limits[operation] || 100,
      duration: 60,
    });
    
    await limiter.consume(clientId);
  }
}

// Usage in Hocuspocus
const rateLimiter = new RateLimiter(redis);

const server = new Hocuspocus({
  async onConnect({ connection }) {
    const clientId = connection.connectionId;
    
    // Check connection rate limit
    await rateLimiter.checkOperationLimit(clientId, 'connection:new');
    
    // Check message rate limit
    connection.on('message', async () => {
      await rateLimiter.checkOperationLimit(clientId, 'document:update');
    });
  },
});
```

#### Data Validation

```typescript
// Document validation
import { Document } from '@hocuspocus/server';
import * as Y from 'yjs';

class DocumentValidator {
  // Validate document structure
  validateDocument(document: Document): ValidationResult {
    const errors: string[] = [];
    
    try {
      // Encode and decode to verify integrity
      const state = Y.encodeStateAsUpdate(document);
      const testDoc = new Y.Doc();
      Y.applyUpdate(testDoc, state);
      
      // Check document size
      const size = state.length;
      const maxSize = 10 * 1024 * 1024; // 10MB
      if (size > maxSize) {
        errors.push(`Document too large: ${size} bytes (max: ${maxSize})`);
      }
      
      // Validate structure
      this.validateStructure(testDoc, errors);
      
      // Check for corrupted data
      this.checkCorruption(testDoc, errors);
      
    } catch (error) {
      errors.push(`Document parsing failed: ${error.message}`);
    }
    
    return {
      valid: errors.length === 0,
      errors,
    };
  }
  
  private validateStructure(doc: Y.Doc, errors: string[]) {
    // Check for expected fields
    const expectedFields = ['document', 'shapes', 'pages'];
    
    for (const field of expectedFields) {
      try {
        const type = doc.get(field);
        if (!type) {
          errors.push(`Missing required field: ${field}`);
        }
      } catch {
        errors.push(`Invalid field type: ${field}`);
      }
    }
  }
  
  private checkCorruption(doc: Y.Doc, errors: string[]) {
    // Check for signs of data corruption
    const state = Y.encodeStateAsUpdate(doc);
    
    // Check for null bytes
    if (state.some(b => b === 0)) {
      errors.push('Document contains null bytes');
    }
    
    // Check for suspiciously high operation counts
    const operationCount = this.countOperations(doc);
    if (operationCount > 100000) {
      errors.push(`Suspicious operation count: ${operationCount}`);
    }
  }
  
  // Sanitize incoming updates
  sanitizeUpdate(update: Uint8Array): Uint8Array {
    // Remove potentially dangerous content
    // This is application-specific
    
    // For example, strip script tags from text content
    const decoder = new TextDecoder();
    const text = decoder.decode(update);
    const sanitized = text
      .replace(/<script\b[^<]*(?:(?!<\/script>)<[^<]*)*<\/script>/gi, '')
      .replace(/javascript:/gi, '');
    
    return new TextEncoder().encode(sanitized);
  }
}

interface ValidationResult {
  valid: boolean;
  errors: string[];
}

// Usage in Hocuspocus
const validator = new DocumentValidator();

const server = new Hocuspocus({
  async beforeStoreDocument({ document }) {
    const result = validator.validateDocument(document);
    
    if (!result.valid) {
      console.error('Document validation failed:', result.errors);
      throw new Error('Document validation failed');
    }
    
    return true;
  },
});
```

---

## Appendix: Complete Implementation Example

### Full-Stack Collaboration Setup

```typescript
// ============== SERVER SIDE ==============
// server/index.ts

import { Hocuspocus } from '@hocuspocus/server';
import { Logger } from '@hocuspocus/logger';
import { PostgreSQL } from '@hocuspocus/database-postgresql';
import { Redis } from 'ioredis';
import jwt from 'jsonwebtoken';

const redis = new Redis(process.env.REDIS_URL);
const database = new PostgreSQL({
  host: process.env.DB_HOST,
  database: process.env.DB_NAME,
  user: process.env.DB_USER,
  password: process.env.DB_PASSWORD,
});

const server = new Hocuspocus({
  port: parseInt(process.env.PORT || '4001'),
  
  async onLoadDocument({ documentName, request }) {
    const token = request?.documentName?.split('/')[0];
    
    // Verify authentication
    const user = jwt.verify(token, process.env.JWT_SECRET) as User;
    
    // Check authorization
    const hasAccess = await database.userHasAccess(user.id, documentName);
    if (!hasAccess) {
      throw new Error('Access denied');
    }
    
    // Load or create document
    let doc = await database.getDocument(documentName);
    if (!doc) {
      doc = new Document(documentName);
    }
    
    return doc;
  },
  
  async onStoreDocument({ documentName, document }) {
    await database.saveDocument(documentName, document);
  },
  
  async onConnect({ connection, document }) {
    console.log(`Client connected to ${document.name}`);
    await redis.sadd(`room:${document.name}:clients`, connection.connectionId);
  },
  
  async onDisconnect({ connection, document }) {
    console.log(`Client disconnected from ${document.name}`);
    await redis.srem(`room:${document.name}:clients`, connection.connectionId);
  },
  
  debounce: 2000,
  maxDebounce: 15000,
});

server.use(Logger);
server.listen();

// ============== CLIENT SIDE ==============
// client/collaboration.ts

import { Editor } from '@tiptap/core';
import * as Y from 'yjs';
import { WebsocketProvider } from 'y-websocket';
import { Awareness } from 'y-protocols/awareness';
import Collaboration from '@tiptap/extension-collaboration';
import CollaborationCursor from '@tiptap/extension-collaboration-cursor';

export class CollaborationManager {
  private ydoc: Y.Doc;
  private awareness: Awareness;
  private provider: WebsocketProvider;
  private editor: Editor | null = null;
  
  constructor(roomId: string, token: string) {
    this.ydoc = new Y.Doc();
    this.awareness = new Awareness(this.ydoc);
    
    // Set user info
    this.awareness.setLocalStateField('user', {
      name: this.getUserName(),
      color: this.getUserColor(),
    });
    
    // Create provider
    this.provider = new WebsocketProvider(
      process.env.COLLABORATION_URL,
      `${token}/${roomId}`,
      this.ydoc,
      { awareness: this.awareness }
    );
    
    this.setupEventListeners();
  }
  
  setupEventListeners() {
    // Connection status
    this.provider.on('status', ({ status }) => {
      console.log('Connection status:', status);
    });
    
    // Sync status
    this.provider.on('sync', (isSynced) => {
      console.log('Sync status:', isSynced);
    });
    
    // Awareness changes
    this.awareness.on('change', ({ added, updated, removed }) => {
      added.forEach(id => this.handleUserJoined(id));
      updated.forEach(id => this.handleUserUpdated(id));
      removed.forEach(id => this.handleUserLeft(id));
    });
  }
  
  createEditor(element: HTMLElement): Editor {
    this.editor = new Editor({
      element,
      extensions: [
        StarterKit.configure({ history: false }),
        Collaboration.configure({ document: this.ydoc }),
        CollaborationCursor.configure({ provider: this.provider }),
      ],
    });
    
    return this.editor;
  }
  
  disconnect() {
    this.provider.disconnect();
    this.ydoc.destroy();
  }
}

// ============== REACT INTEGRATION ==============
// client/CollaborationEditor.tsx

import React, { useEffect, useRef, useState } from 'react';
import { CollaborationManager } from './collaboration';

interface CollaborationEditorProps {
  roomId: string;
  token: string;
  userId: string;
}

export const CollaborationEditor: React.FC<CollaborationEditorProps> = ({
  roomId,
  token,
  userId,
}) => {
  const editorRef = useRef<HTMLDivElement>(null);
  const [collaboration, setCollaboration] = useState<CollaborationManager | null>(null);
  const [isSynced, setIsSynced] = useState(false);
  const [connectedUsers, setConnectedUsers] = useState<any[]>([]);
  
  useEffect(() => {
    const collab = new CollaborationManager(roomId, token);
    setCollaboration(collab);
    
    const editor = collab.createEditor(editorRef.current!);
    
    const unsubscribe = collab.provider.on('sync', (synced) => {
      setIsSynced(synced);
    });
    
    return () => {
      unsubscribe();
      collab.disconnect();
      editor.destroy();
    };
  }, [roomId, token]);
  
  return (
    <div className="collaboration-editor">
      <div className="status-bar">
        <span className={`sync-status ${isSynced ? 'synced' : 'syncing'}`}>
          {isSynced ? '✓ Synced' : '⋯ Syncing...'}
        </span>
        <div className="connected-users">
          {connectedUsers.map(user => (
            <div key={user.id} className="user-avatar" style={{ backgroundColor: user.color }}>
              {user.name[0]}
            </div>
          ))}
        </div>
      </div>
      <div ref={editorRef} />
    </div>
  );
};
```

---

## Summary

This deep dive covered:

1. **Yjs Fundamentals** - CRDTs, data types, awareness protocol, and providers
2. **Hocuspocus Server** - Setup, authentication, database integration, hooks
3. **Tiptap Collaboration** - Extension setup, cursors, presence indicators
4. **Sync Protocol** - Initial sync, update propagation, conflict resolution, offline support
5. **tldraw Collaboration** - Store sync, presence, cursors, custom implementations
6. **Production Patterns** - Scaling, load balancing, persistence, backup/recovery
7. **Security** - Authentication, authorization, rate limiting, data validation

The code examples provide production-ready patterns for implementing real-time collaboration in web editors using Yjs and Hocuspocus.
