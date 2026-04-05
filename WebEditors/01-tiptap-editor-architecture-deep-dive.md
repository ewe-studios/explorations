# Tiptap Editor Architecture Deep Dive

## Table of Contents

1. [Core Architecture](#1-core-architecture)
   - [Editor Class Structure](#editor-class-structure)
   - [Extension System Foundation](#extension-system-foundation)
   - [Command System](#command-system)
   - [Transaction Pipeline](#transaction-pipeline)
   - [State Management](#state-management)

2. [ProseMirror Integration](#2-prosemirror-integration)
   - [Schema Definition](#schema-definition)
   - [Node Types and Mark Types](#node-types-and-mark-types)
   - [Plugin System](#plugin-system)
   - [View Integration](#view-integration)
   - [Input Rules](#input-rules)

3. [Document Structure](#3-document-structure)
   - [ProseMirror Document Model](#prosemirror-document-model)
   - [Node Structure](#node-structure)
   - [Content Arrays](#content-arrays)
   - [Fragment Operations](#fragment-operations)
   - [Slice Operations](#slice-operations)

4. [Selection System](#4-selection-system)
   - [Text Selections](#text-selections)
   - [Node Selections](#node-selections)
   - [All Selections](#all-selections)
   - [Selection Transactions](#selection-transactions)
   - [Cursor Position](#cursor-position)

5. [Transaction Flow](#5-transaction-flow)
   - [Transaction Creation](#transaction-creation)
   - [Steps and Step Maps](#steps-and-step-maps)
   - [Metadata](#metadata)
   - [Scoping](#scoping)
   - [Batch Transactions](#batch-transaction)

6. [Event System](#6-event-system)
   - [Transaction Events](#transaction-events)
   - [Selection Events](#selection-events)
   - [Focus Events](#focus-events)
   - [Custom Events](#custom-events)
   - [Event Bubbling](#event-bubbling)

7. [Editor View](#7-editor-view)
   - [DOM Rendering](#dom-rendering)
   - [Node Views](#node-views)
   - [Decoration System](#decoration-system)
   - [Input Handling](#input-handling)
   - [Clipboard Handling](#clipboard-handling)

---

## 1. Core Architecture

### Editor Class Structure

The Tiptap `Editor` class is the central orchestrator that wraps ProseMirror's `EditorState` and `EditorView` while providing a higher-level API. Here's the complete structure:

```typescript
// Simplified Editor class structure
class Editor {
  // Core Properties
  extensionManager: ExtensionManager
  state: EditorState
  view: EditorView
  schema: Schema
  
  // Configuration
  options: EditorOptions
  
  // Event Management
  eventHandler: EventEmitter
  
  // Command Interface
  commands: CommandManager
  
  // Storage for extensions
  storage: Record<string, any>
  
  // Lifecycle flags
  isInitialized: boolean
  isDestroyed: boolean
  
  // Core Methods
  constructor(options: Partial<EditorOptions>)
  create(): Promise<void>
  destroy(): void
  
  // State Access
  get state(): EditorState
  get storage(): Record<string, any>
  
  // Command Interface
  chain(): ChainableCommandManager
  commands: { [commandName: string]: (...args) => boolean }
  
  // Event Registration
  on(event: EditorEvents['transaction'], callback: (props) => void): Editor
  on(event: EditorEvents['selection'], callback: (props) => void): Editor
  on(event: EditorEvents['focus'], callback: (props) => void): Editor
  on(event: EditorEvents['blur'], callback: (props) => void): Editor
  
  // Extension Management
  registerExtension(extension: Extension): void
  getExtension<T extends Extension>(name: string): T | undefined
  
  // Content Management
  setContent(content: Content, emitUpdate?: boolean): void
  getContent(options?: GetContentOptions): JSONContent
  getHTML(): string
  getJSON(): JSONContent
  
  // Selection Management
  setSelection(from: number, to?: number): void
  
  // Utility Methods
  isEditable: boolean
  isEmpty: boolean
  isActive(nameOrAttributes?: string | Attributes): boolean
  can(): CanManager
}
```

**Initialization Flow:**

```typescript
// Editor initialization sequence
async create(): Promise<void> {
  // 1. Initialize Extension Manager
  this.extensionManager = new ExtensionManager(this.options.extensions, this);
  
  // 2. Build Schema from extensions
  this.schema = this.extensionManager.schema;
  
  // 3. Create initial ProseMirror state
  this.state = EditorState.create({
    schema: this.schema,
    doc: this.createDocument(this.options.content),
    selection: this.options.initialSelection,
    plugins: this.extensionManager.plugins
  });
  
  // 4. Create Editor View
  this.view = new EditorView(this.options.element, {
    state: this.state,
    dispatchTransaction: this.dispatchTransaction.bind(this),
    ...this.options.editorView
  });
  
  // 5. Initialize storage for each extension
  this.initStorage();
  
  // 6. Fire onCreated event
  this.emit('created', { editor: this });
}
```

**Editor Options Structure:**

```typescript
interface EditorOptions {
  // DOM Element
  element: HTMLElement
  
  // Initial Content
  content: Content | null
  
  // Editor State
  editable: boolean
  autofocus: boolean | 'start' | 'end' | 'all' | number
  injectCSS: boolean
  
  // Extensions
  extensions: Extension[]
  
  // Editor View Configuration
  editorView: EditorViewProps
  
  // Initial Selection
  initialSelection: Selection | null
  
  // Event Callbacks
  onBeforeCreate: (props: { editor: Editor }) => void
  onCreate: (props: { editor: Editor }) => void
  onUpdate: (props: { editor: Editor }) => void
  onSelectionUpdate: (props: { editor: Editor }) => void
  onTransaction: (props: { editor: Editor; transaction: Transaction }) => void
  onFocus: (props: { editor: Editor; event: FocusEvent }) => void
  onBlur: (props: { editor: Editor; event: FocusEvent }) => void
  onDestroy: (props: { editor: Editor }) => void
  
  // Content Parsing
  parseOptions: ParseOptions
}
```

---

### Extension System Foundation

Tiptap's extension system is built on a composable architecture that allows extensions to be combined, extended, and configured.

**Extension Base Class:**

```typescript
interface ExtensionConfig<Options, Storage> {
  name: string
  defaultOptions?: Partial<Options>
  
  // Extension Composition
  parent?: Extension
  extensions?: Extension[]
  
  // Priority (lower = earlier execution)
  priority?: number
  
  // Lifecycle Hooks
  onCreate?(props: { editor: Editor }): void
  onUpdate?(props: { editor: Editor }): void
  onSelectionUpdate?(props: { editor: Editor }): void
  onTransaction?(props: { editor: Editor; transaction: Transaction }): void
  onFocus?(props: { editor: Editor; event: FocusEvent }): void
  onBlur?(props: { editor: Editor; event: FocusEvent }): void
  onDestroy?(props: { editor: Editor }): void
  
  // Schema Definition
  addSchema?(): Partial<SchemaSpec>
  
  // Node/Mark/Extension Definitions
  addNode?(): NodeConfig
  addMark?(): MarkConfig
  addExtension?(): ExtensionConfig
  
  // Commands
  addCommands?(): Partial<Commands<Options>>
  
  // Keyboard Shortcuts
  addKeyboardShortcuts?(): { [key: string]: () => boolean }
  
  // Input Rules
  addInputRules?(): InputRule[]
  
  // Paste Rules
  addPasteRules?(): PasteRule[]
  
  // Node Views
  addNodeView?(): (props: NodeViewProps) => NodeView
  
  // Plugin Registration
  addProseMirrorPlugins?(): Plugin[]
  
  // Global Attributes
  addGlobalAttributes?(): GlobalAttribute[]
  
  // Storage
  addStorage?(): Storage
}

class Extension<Options = any, Storage = any> {
  static name: string = 'extension'
  static defaultOptions: Partial<Options> = {}
  
  config: ExtensionConfig<Options, Storage>
  
  // Extension methods
  configure(options: Partial<Options>): Extension
  extend(overrides: Partial<ExtensionConfig>): Extension
}
```

**Extension Resolution and Merging:**

```typescript
class ExtensionManager {
  editor: Editor
  extensions: Extension[]
  
  constructor(extensions: Extension[], editor: Editor) {
    this.editor = editor
    // Flatten and resolve all extensions
    this.extensions = this.resolveExtensions(extensions)
  }
  
  // Flatten nested extensions and handle priorities
  resolveExtensions(extensions: Extension[]): Extension[] {
    return extensions
      .flatMap(ext => {
        const nested = ext.config.extensions || []
        return [ext, ...this.resolveExtensions(nested)]
      })
      .sort((a, b) => (a.config.priority || 100) - (b.config.priority || 100))
      .filter((ext, index, self) => 
        self.findIndex(e => e.config.name === ext.config.name) === index
      )
  }
  
  // Build schema from all extensions
  get schema(): Schema {
    const nodes = this.getNodeTypes()
    const marks = this.getMarksTypes()
    
    return new Schema({
      nodes: this.mergeNodeSpecs(nodes),
      marks: this.mergeMarkSpecs(marks)
    })
  }
  
  // Collect all plugins from extensions
  get plugins(): Plugin[] {
    return this.extensions.flatMap(ext => 
      ext.config.addProseMirrorPlugins?.({ editor: this.editor }) || []
    )
  }
  
  // Merge global attributes from all extensions
  get globalAttributes(): GlobalAttribute[] {
    return this.extensions.flatMap(ext => 
      ext.config.addGlobalAttributes?.() || []
    )
  }
}
```

**Extension Composition Example:**

```typescript
// Creating a composite extension
const CustomTextFormatting = Extension.create({
  name: 'customTextFormatting',
  
  addExtensions() {
    return [
      Bold.configure({ HTMLAttributes: { class: 'custom-bold' } }),
      Italic.configure({ HTMLAttributes: { class: 'custom-italic' } }),
      Underline.configure({ HTMLAttributes: { class: 'custom-underline' } }),
    ]
  },
})

// Using the extension
const editor = new Editor({
  extensions: [
    StarterKit,
    CustomTextFormatting,
    // Extensions can be configured
    Link.configure({
      openOnClick: false,
      HTMLAttributes: {
        class: 'custom-link'
      }
    })
  ]
})
```

---

### Command System

Tiptap's command system provides a typed, chainable interface for editor operations. Commands are functions that create transactions.

**Command Architecture:**

```typescript
// Command signature type
type Command = (props: CommandProps) => boolean

interface CommandProps {
  editor: Editor
  state: EditorState
  dispatch: (tr?: Transaction) => void
  view: EditorView | null
  commands: SingleCommands
  can: () => CanManager
  chain: () => ChainableCommandManager
  editor: Editor
  tr: Transaction
}

// Command manager that provides the commands interface
class CommandManager {
  editor: Editor
  
  get commands(): { [name: string]: Command } {
    return this.editor.extensionManager.commands
  }
  
  // Execute a single command
  execute(name: string, args?: any): boolean {
    const command = this.commands[name]
    if (!command) return false
    return command(this.createProps())
  }
  
  createProps(): CommandProps {
    return {
      editor: this.editor,
      state: this.editor.state,
      dispatch: (tr) => {
        if (tr) this.editor.view.dispatch(tr)
      },
      view: this.editor.view,
      commands: this.commands,
      can: () => this.can(),
      chain: () => this.chain(),
      editor: this.editor,
      tr: this.editor.state.tr
    }
  }
}
```

**Chainable Commands:**

```typescript
class ChainableCommandManager {
  private chain: Command[] = []
  private tr: Transaction
  
  // Add command to chain
  [commandName: string](...args: any): ChainableCommandManager {
    const command = this.getCommand(commandName)
    this.chain.push((props) => command({ ...props, ...args }))
    return this
  }
  
  // Execute the chain
  run(): boolean {
    const { tr } = this
    
    for (const command of this.chain) {
      const result = command({
        editor: this.editor,
        state: this.editor.state,
        dispatch: (newTr) => {
          if (newTr) this.tr = newTr
        },
        tr: this.tr,
        commands: {},
        can: () => ({}),
        chain: () => this,
        view: this.editor.view
      })
      
      if (!result) return false
    }
    
    this.editor.view.dispatch(this.tr)
    return true
  }
}

// Usage example
editor
  .chain()
  .focus()
  .setTextSelection(10)
  .toggleBold()
  .insertContent('Hello World')
  .run()
```

**Built-in Commands Implementation:**

```typescript
// Example: Toggle Mark Command
const toggleMark = (typeOrName: string | MarkType, attributes?: Attrs) => 
  ({ state, dispatch, tr }) => {
    const { from, to } = state.selection
    
    // Get mark type
    const markType = typeof typeOrName === 'string' 
      ? state.schema.marks[typeOrName]
      : typeOrName
    
    if (!markType) return false
    
    // Check if mark is active in selection
    const { marks } = state.selection.$from
    const mark = marks.find(m => m.type === markType)
    
    if (mark) {
      // Remove mark
      dispatch?.(tr.removeMark(from, to, markType))
    } else {
      // Add mark
      dispatch?.(tr.addMark(from, to, markType.create(attributes)))
    }
    
    return true
  }

// Example: Set Node Attribute Command
const setNodeAttribute = (attribute: string, value: any) =>
  ({ state, dispatch, tr }) => {
    const { $from } = state.selection
    const pos = $from.start($from.depth)
    const node = $from.node($from.depth)
    
    if (node.attrs[attribute] === value) return false
    
    dispatch?.(tr.setNodeAttribute(pos, attribute, value))
    return true
  }

// Example: Insert Content Command
const insertContent = (content: Content, options?: InsertOptions) =>
  ({ state, dispatch, tr, commands }) => {
    const { content: parsed } = parseContent(content, state.schema)
    const { from, to } = state.selection
    
    // Create insert step
    const tr = state.tr.replaceWith(from, to, parsed)
    
    if (options?.focus) {
      const pos = from + parsed.content.size
      tr.setSelection(TextSelection.create(tr.doc, pos))
    }
    
    dispatch?.(tr)
    return true
  }
```

---

### Transaction Pipeline

The transaction pipeline is the core mechanism for state changes in Tiptap/ProseMirror.

```typescript
// Transaction Pipeline Flow
//
// ┌─────────────────┐
// │   Command       │
// │   (User Action) │
// └────────┬────────┘
//          │
//          ▼
// ┌─────────────────┐
// │ Transaction     │
// │ Creation        │
// └────────┬────────┘
//          │
//          ▼
// ┌─────────────────┐
// │ Step            │
// │ Application     │
// └────────┬────────┘
//          │
//          ▼
// ┌─────────────────┐
// │ Plugin Hooks    │
// │ (appendTransaction) │
// └────────┬────────┘
//          │
//          ▼
// ┌─────────────────┐
// │ State Update    │
// └────────┬────────┘
//          │
//          ▼
// ┌─────────────────┐
// │ View Update     │
// │ (dispatchTransaction) │
// └────────┬────────┘
//          │
//          ▼
// ┌─────────────────┐
// │ DOM Rendering   │
// └─────────────────┘
```

**Transaction Processing:**

```typescript
// Editor's dispatchTransaction handles all state changes
dispatchTransaction(transaction: Transaction): void {
  // 1. Get new state
  const state = this.state.apply(transaction)
  this.state = state
  
  // 2. Update view with new state
  this.view.updateState(state)
  
  // 3. Process transaction through extension hooks
  this.extensionManager.extensions.forEach(ext => {
    ext.config.onTransaction?.({
      editor: this,
      transaction
    })
  })
  
  // 4. Handle selection changes
  if (transaction.selectionSet || transaction.docChanged) {
    this.emit('selectionUpdate', { 
      editor: this, 
      transaction 
    })
  }
  
  // 5. Handle focus changes
  if (transaction.getMeta('focus')) {
    this.emit('focus', { 
      editor: this, 
      event: transaction.getMeta('focusEvent')
    })
  }
  
  if (transaction.getMeta('blur')) {
    this.emit('blur', { 
      editor: this, 
      event: transaction.getMeta('blurEvent')
    })
  }
  
  // 6. Emit generic transaction event
  this.emit('transaction', { editor: this, transaction })
  
  // 7. Emit update event for document changes
  if (transaction.docChanged) {
    this.emit('update', { editor: this, transaction })
  }
}
```

---

### State Management

Tiptap manages state through ProseMirror's `EditorState` with additional Tiptap-specific state handling.

```typescript
// State Management Architecture
interface EditorState {
  // Core ProseMirror state
  schema: Schema
  doc: Node
  selection: Selection
  plugins: Plugin[]
  
  // State accessors
  apply(tr: Transaction): EditorState
  applyTransaction(tr: Transaction): { state: EditorState, transactions: Transaction[] }
  
  // Selection helpers
  resolve(pos: number): ResolvedPos
}

// Tiptap State Extensions
interface TiptapState {
  // Computed properties
  isEmpty: boolean
  isEditable: boolean
  
  // Extension storage
  extensionStorage: Record<string, any>
  
  // Active marks at cursor
  activeMarks: Mark[]
  
  // Active node type at cursor
  currentNode: Node | null
}

// State computation helpers
class StateManager {
  editor: Editor
  
  // Check if current selection has a specific mark
  isMarkActive(typeOrName: string | MarkType): boolean {
    const { from, to } = this.editor.state.selection
    const { marks } = this.editor.state
    
    if (to === from) {
      // Check stored marks at cursor position
      return marks.some(mark => mark.type.name === typeOrName)
    }
    
    // Check if mark exists in entire selection
    return this.editor.state.doc.rangeHasMark(from, to, typeOrName)
  }
  
  // Get current node at selection
  getCurrentNode(): Node | null {
    const { $from } = this.editor.state.selection
    return $from.parent
  }
  
  // Get current node type
  getCurrentNodeType(): NodeType | null {
    return this.getCurrentNode()?.type || null
  }
  
  // Check if a specific node type is active
  isNodeActive(typeOrName: string | NodeType, attributes?: Attrs): boolean {
    const node = this.getCurrentNode()
    if (!node) return false
    
    const matches = node.type.name === typeOrName
    
    if (attributes && matches) {
      return Object.entries(attributes).every(
        ([key, value]) => node.attrs[key] === value
      )
    }
    
    return matches
  }
}
```

---

## 2. ProseMirror Integration

### Schema Definition

Tiptap builds its schema by aggregating node and mark specifications from all registered extensions.

```typescript
// Schema Construction from Extensions
interface SchemaSpec {
  nodes: { [name: string]: NodeSpec }
  marks: { [name: string]: MarkSpec }
}

// Tiptap's schema building process
class SchemaBuilder {
  extensions: Extension[]
  
  build(): Schema {
    const nodeSpecs = this.collectNodeSpecs()
    const markSpecs = this.collectMarkSpecs()
    
    // Merge default nodes with extension nodes
    const nodes = this.mergeNodes({
      doc: { content: 'block+' },
      text: { group: 'inline' },
      ...nodeSpecs
    })
    
    // Merge marks
    const marks = this.mergeMarks(markSpecs)
    
    return new Schema({ nodes, marks })
  }
  
  collectNodeSpecs(): Record<string, NodeSpec> {
    const specs: Record<string, NodeSpec> = {}
    
    for (const ext of this.extensions) {
      const node = ext.config.addNode?.()
      if (node) {
        specs[node.name] = this.normalizeNodeSpec(node)
      }
    }
    
    return specs
  }
  
  normalizeNodeSpec(node: NodeConfig): NodeSpec {
    return {
      // Required
      toDOM: node.toDOM || this.defaultToDOM(node),
      
      // Content model
      content: node.content || '',
      group: node.group || '',
      
      // Structural
      isolating: node.isolating || false,
      selectable: node.selectable ?? true,
      atom: node.atom || false,
      
      // Nesting
      marks: node.marks ?? '_',
      code: node.code || false,
      
      // Default attributes
      attrs: this.normalizeAttrs(node.addAttributes?.()),
      
      // Default content
      defaultContent: node.defaultContent,
      
      // Draggable
      draggable: node.draggable ?? false,
      
      // Parse rules
      parseDOM: node.parseDOM,
    }
  }
  
  normalizeAttrs(attributes?: Record<string, Attribute>): Record<string, any> {
    if (!attributes) return {}
    
    return Object.fromEntries(
      Object.entries(attributes).map(([name, attr]) => [
        name,
        {
          default: attr.default,
          validate: attr.validate,
          parseDOM: attr.parseDOM,
          toDOM: attr.toDOM
        }
      ])
    )
  }
}
```

**Node Specification Example:**

```typescript
// Custom node specification
const ImageNode = Node.create({
  name: 'image',
  group: 'block',
  atom: true,
  draggable: true,
  
  addAttributes() {
    return {
      src: {
        default: null,
        parseDOM: (element) => element.getAttribute('src'),
        toDOM: (attrs) => ['src', attrs.src]
      },
      alt: {
        default: null,
        parseDOM: (element) => element.getAttribute('alt'),
        toDOM: (attrs) => ['alt', attrs.alt]
      },
      title: {
        default: null,
        parseDOM: (element) => element.getAttribute('title'),
        toDOM: (attrs) => ['title', attrs.title]
      },
      width: {
        default: null,
        parseDOM: (element) => element.getAttribute('width'),
        toDOM: (attrs) => ['width', attrs.width]
      }
    }
  },
  
  parseHTML() {
    return [
      {
        tag: 'img[src]'
      }
    ]
  },
  
  renderHTML({ HTMLAttributes }) {
    return ['img', mergeAttributes(this.options.HTMLAttributes, HTMLAttributes)]
  },
  
  addNodeView() {
    return ({ node, HTMLAttributes, getPos, editor }) => {
      const wrapper = document.createElement('div')
      wrapper.className = 'image-node-wrapper'
      
      const img = document.createElement('img')
      img.src = node.attrs.src
      img.alt = node.attrs.alt
      img.title = node.attrs.title
      
      wrapper.appendChild(img)
      
      return {
        dom: wrapper,
        contentDOM: null // No editable content for atom nodes
      }
    }
  },
  
  addCommands() {
    return {
      setImage: (attrs) => ({ tr, dispatch }) => {
        const { selection } = tr
        const node = this.type.create(attrs)
        tr.replaceSelectionWith(node)
        
        if (dispatch) {
          dispatch(tr)
        }
        
        return true
      }
    }
  }
})
```

**Mark Specification Example:**

```typescript
// Custom mark specification
const HighlightMark = Mark.create({
  name: 'highlight',
  
  addAttributes() {
    return {
      color: {
        default: null,
        parseDOM: (element) => element.style.backgroundColor,
        toDOM: (attrs) => {
          const style = `background-color: ${attrs.color}`
          return ['style', style]
        }
      }
    }
  },
  
  parseHTML() {
    return [
      {
        tag: 'mark',
        getAttrs: (element) => ({
          color: element.style.backgroundColor
        })
      },
      {
        tag: 'span[data-highlight]',
        getAttrs: (element) => ({
          color: element.dataset.highlight
        })
      }
    ]
  },
  
  renderHTML({ HTMLAttributes }) {
    return ['mark', mergeAttributes(this.options.HTMLAttributes, HTMLAttributes), 0]
  },
  
  addCommands() {
    return {
      setHighlight: (attrs) => ({ tr, state, dispatch }) => {
        const { from, to } = state.selection
        const mark = this.type.create(attrs)
        tr.addMark(from, to, mark)
        
        if (dispatch) {
          dispatch(tr)
        }
        
        return true
      },
      unsetHighlight: () => ({ tr, state, dispatch }) => {
        const { from, to } = state.selection
        tr.removeMark(from, to, this.type)
        
        if (dispatch) {
          dispatch(tr)
        }
        
        return true
      },
      toggleHighlight: (attrs) => ({ tr, state, dispatch, commands }) => {
        if (this.editor.isActive('highlight')) {
          return commands.unsetHighlight()
        }
        return commands.setHighlight(attrs)
      }
    }
  }
})
```

---

### Node Types and Mark Types

**NodeType Structure:**

```typescript
interface NodeType {
  name: string
  schema: Schema
  spec: NodeSpec
  
  // Content model
  content: ContentMatch
  inlineContent: boolean
  isBlock: boolean
  isText: boolean
  
  // Attributes
  attrs: { [key: string]: any }
  
  // Helper methods
  create(attrs?: Attrs, content?: Fragment, marks?: readonly Mark[]): Node
  createChecked(attrs?: Attrs, content?: Fragment, marks?: readonly Mark[]): Node
  isCompatibleWith(other: NodeType): boolean
  
  // Check if mark can apply to this node type
  allowsMark(mark: MarkType): boolean
}

// Creating nodes
const node = nodeType.create(
  { id: 'abc123' },           // Attributes
  Fragment.from(paragraph),   // Content
  [boldMark]                  // Marks
)
```

**MarkType Structure:**

```typescript
interface MarkType {
  name: string
  schema: Schema
  spec: MarkSpec
  
  // Attributes
  attrs: { [key: string]: any }
  
  // Helper methods
  create(attrs?: Attrs): Mark
  
  // Check if mark can apply to node
  isInclusive(node: NodeType): boolean
  excludes(mark: MarkType): boolean
}
```

**Attribute Definition:**

```typescript
interface Attribute {
  default: any
  parseDOM?: (element: HTMLElement) => any
  toDOM?: (attrs: Record<string, any>) => [string, string] | null
  validate?: (value: any) => boolean
  rendered?: boolean  // Whether attribute should be rendered to DOM
}

// Example: ID attribute with validation
const idAttribute: Attribute = {
  default: null,
  parseDOM: (element) => element.getAttribute('data-id'),
  toDOM: (attrs) => attrs.id ? ['data-id', attrs.id] : null,
  validate: (value) => {
    if (!value) return true
    return /^[a-zA-Z][a-zA-Z0-9_-]*$/.test(value)
  }
}
```

---

### Plugin System

Tiptap uses ProseMirror's plugin system extensively. Plugins can observe and modify transactions.

```typescript
// Plugin Architecture
interface PluginSpec<State> {
  // State initialization
  state?: {
    init: (config: any, state: EditorState) => State
    apply: (
      tr: Transaction,
      pluginState: State,
      oldState: EditorState,
      newState: EditorState
    ) => State
  }
  
  // Transaction filtering
  filterTransaction?: (tr: Transaction, state: EditorState) => boolean
  
  // View integration
  view?: {
    update?: (view: EditorView, prevState: EditorState) => void
    destroy?: (state: EditorState) => void
  }
  
  // Append transactions
  appendTransaction?: (
    transactions: Transaction[],
    oldState: EditorState,
    newState: EditorState
  ) => Transaction | undefined
  
  // Props for the view
  props?: EditorProps
}

class Plugin<State = any> {
  spec: PluginSpec<State>
  key: PluginKey
  
  // Access plugin state
  getState(state: EditorState): State
  
  // Create plugin with unique key
  constructor(options: { key: PluginKey; spec: PluginSpec<State> })
}

// Example: Custom Plugin
const searchHighlightPlugin = new Plugin({
  key: new PluginKey('searchHighlight'),
  
  state: {
    init: () => ({ searchTerm: null, matches: [] }),
    
    apply: (tr, pluginState, oldState, newState) => {
      const searchTerm = tr.getMeta('setSearchTerm')
      
      if (searchTerm !== undefined) {
        // Update matches based on new search term
        const matches = findMatches(newState.doc, searchTerm)
        return { searchTerm, matches }
      }
      
      return pluginState
    }
  },
  
  props: {
    decorations(state) {
      const { matches } = this.getState(state)
      return DecorationSet.create(
        state.doc,
        matches.map(match => 
          Decoration.inline(match.from, match.to, { class: 'search-match' })
        )
      )
    }
  },
  
  appendTransaction(transactions, oldState, newState) {
    // Auto-clear highlights on document change
    const docChanged = transactions.some(tr => tr.docChanged)
    if (docChanged) {
      const { searchTerm } = this.getState(oldState)
      if (searchTerm) {
        return newState.tr.setMeta(this.key, { searchTerm: null, matches: [] })
      }
    }
  }
})
```

**Plugin Key Pattern:**

```typescript
// Plugin keys provide type-safe state access
const highlightPluginKey = new PluginKey('highlightPlugin')

// Usage in commands
const setHighlight = (searchTerm: string) => ({ tr, state, dispatch }) => {
  tr.setMeta(highlightPluginKey, { searchTerm })
  dispatch?.(tr)
  return true
}

// Usage in node views
const { searchTerm } = highlightPluginKey.getState(state)
```

---

### View Integration

**EditorView Configuration:**

```typescript
interface EditorViewProps {
  state: EditorState
  dispatchTransaction: (tr: Transaction) => void
  
  // Node view factory
  nodeViews?: {
    [nodeName: string]: (node: Node, view: EditorView, getPos: () => number | null) => NodeView
  }
  
  // Clipboard handling
  clipboardTextParser?: (text: string, context: ResolvedPos, plain: boolean) => Slice
  clipboardTextSerializer?: (slice: Slice) => string
  
  // DOM props
  attributes?: { [name: string]: string }
  handleDOMEvents?: {
    [eventName: string]: (view: EditorView, event: Event) => boolean
  }
  
  // Input handling
  handleTextInput?: (view: EditorView, from: number, to: number, text: string) => boolean
  handleKeyDown?: (view: EditorView, event: KeyboardEvent) => boolean
  handlePaste?: (view: EditorView, event: ClipboardEvent, slice: Slice) => boolean
  handleDrop?: (view: EditorView, event: DragEvent, slice: Slice, moved: boolean) => boolean
  
  // Decoration
  decorations?: (state: EditorState) => DecorationSet
  
  // Transformations
  transformPasted?: (slice: Slice) => Slice
  transformPastedHTML?: (html: string) => string
  transformPastedText?: (text: string) => string
  
  // Node creation
  transformCopied?: (slice: Slice) => Slice
}
```

**DOM Event Handling:**

```typescript
// Tiptap's event handling layer
const domEventHandlers = {
  handleKeyDown: (view, event) => {
    // Check keyboard shortcuts from extensions
    for (const ext of editor.extensionManager.extensions) {
      const shortcuts = ext.config.addKeyboardShortcuts?.()
      const key = event.key.toLowerCase()
      
      if (shortcuts?.[key]) {
        return shortcuts[key]()
      }
    }
    return false
  },
  
  handleTextInput: (view, from, to, text) => {
    // Check input rules from extensions
    for (const ext of editor.extensionManager.extensions) {
      const rules = ext.config.addInputRules?.()
      
      for (const rule of rules) {
        if (rule.find(text)) {
          return rule.handler({ 
            editor, 
            from, 
            to, 
            text, 
            range: { from, to }
          })
        }
      }
    }
    return false
  },
  
  handlePaste: (view, event, slice) => {
    // Check paste rules from extensions
    for (const ext of editor.extensionManager.extensions) {
      const rules = ext.config.addPasteRules?.()
      
      for (const rule of rules) {
        // Apply paste transformation
        slice = rule.transform(slice, { from: view.state.selection.from })
      }
    }
    
    return false // Let default paste handling proceed
  }
}
```

---

### Input Rules

Input rules enable markdown-style input transformations.

```typescript
// Input Rule Architecture
interface InputRule {
  find: RegExp | ((text: string) => RegExpMatchArray | null)
  handler: (props: {
    state: EditorState
    range: { from: number; to: number }
    match: RegExpMatchArray
    commands: Commands
    chain: () => ChainableCommandManager
    can: () => CanManager
  }) => void | boolean
}

// Common input rule patterns
const inputRules: InputRule[] = [
  // Markdown headings
  new InputRule(
    /^(#{1,6})\s$/,
    ({ state, range, match }) => {
      const level = match[1].length
      const { tr } = state
      const pos = range.from
      
      tr.setBlockType(pos - level - 1, pos, state.schema.nodes.heading, { level })
      tr.deleteText(pos - 1, pos)
      
      return tr
    }
  ),
  
  // Bold text
  new InputRule(
    /(\*\*)([^*]+)(\*\*)$/,
    ({ state, range, match }) => {
      const { tr } = state
      const boldType = state.schema.marks.strong
      
      tr.addMark(
        range.from - match[1].length,
        range.to,
        boldType.create()
      )
      tr.deleteText(range.to - 2, range.to)
      tr.deleteText(range.from - 2, range.from)
      
      return tr
    }
  ),
  
  // Bullet list
  new InputRule(
    /^[-*+]\s$/,
    ({ state, range }) => {
      const { tr } = state
      const pos = range.from
      
      tr.setBlockType(pos - 2, pos, state.schema.nodes.bullet_list)
      tr.deleteText(pos - 1, pos)
      
      return tr
    }
  ),
  
  // Code block
  new InputRule(
    /^```(\w*)\s$/,
    ({ state, range, match }) => {
      const { tr } = state
      const pos = range.from
      const language = match[1]
      
      tr.setBlockType(pos - 4 - language.length, pos, state.schema.nodes.code_block, { language })
      tr.deleteText(pos - 3 - language.length, pos)
      
      return tr
    }
  ),
  
  // Blockquote
  new InputRule(
    /^>\s$/,
    ({ state, range }) => {
      const { tr } = state
      const pos = range.from
      
      tr.setBlockType(pos - 2, pos, state.schema.nodes.blockquote)
      tr.deleteText(pos - 1, pos)
      
      return tr
    }
  ),
  
  // Horizontal rule
  new InputRule(
    /^---\s$/,
    ({ state, range }) => {
      const { tr } = state
      const pos = range.from
      
      tr.replaceWith(
        pos - 4,
        pos,
        state.schema.nodes.horizontal_rule.create()
      )
      
      return tr
    }
  )
]
```

---

## 3. Document Structure

### ProseMirror Document Model

The ProseMirror document model is a tree structure where each node can contain child nodes (content) and have marks applied.

```typescript
// Document Structure
//
// Document
// └── Paragraph (Node)
//     ├── Text (Node) + Bold Mark
//     ├── Text (Node) + Italic Mark
//     └── HardBreak (Node)
//
// ┌─────────────────────────────────────────┐
// │  Document (Node)                        │
// │  type: "doc"                            │
// │  content: [Paragraph]                   │
// │  attrs: {}                              │
// │  ┌─────────────────────────────────┐   │
// │  │  Paragraph (Node)               │   │
// │  │  type: "paragraph"              │   │
// │  │  content: [Text, Text, HardBreak]   │
// │  │  attrs: {}                      │   │
// │  │  ┌────────────┐ ┌────────────┐  │   │
// │  │  │  Text      │ │  Text      │  │   │
// │  │  │  "Hello"   │ │  "World"   │  │   │
// │  │  │  marks: [+]│ │  marks: [+]│  │   │
// │  │  └────────────┘ └────────────┘  │   │
// │  └─────────────────────────────────┘   │
// └─────────────────────────────────────────┘
```

**Node Class Structure:**

```typescript
interface Node {
  type: NodeType
  attrs: { [key: string]: any }
  marks: readonly Mark[]
  content: Fragment
  
  // Size and position
  nodeSize: number
  contentSize: number
  childCount: number
  
  // Navigation
  firstChild: Node | null
  lastChild: Node | null
  child(n: number): Node | null
  
  // Content operations
  cut(from: number, to?: number): Node
  copy(content?: Fragment): Node
  replace(from: number, to: number, content: Fragment): Node
  
  // Mark operations
  hasMark(mark: MarkType): boolean
  mark(m: Mark): Node
  withoutMark(mark: MarkType): Node
  
  // Text operations
  isText: boolean
  text: string | null
  isLeaf: boolean
  isInline: boolean
  
  // Serialization
  toJSON(): Object
  toString(): string
  
  // Utility
  eq(other: Node): boolean
  sameMarkup(other: Node): boolean
}
```

---

### Node Structure

```typescript
// Creating a Node
const paragraph = schema.nodes.paragraph.create(
  {},  // Attributes
  [    // Content (Fragment)
    schema.text('Hello', [boldMark]),
    schema.text(' '),
    schema.text('World', [italicMark])
  ]
)

// Node from JSON
const node = Node.fromJSON(schema, {
  type: 'paragraph',
  attrs: { textAlign: 'center' },
  content: [
    { type: 'text', text: 'Hello', marks: [{ type: 'bold' }] },
    { type: 'text', text: ' ' },
    { type: 'text', text: 'World', marks: [{ type: 'italic' }] }
  ]
})

// Node properties
console.log(paragraph.type.name)      // 'paragraph'
console.log(paragraph.contentSize)    // 12 (includes node size)
console.log(paragraph.firstChild)     // Text node "Hello "
console.log(paragraph.textContent)    // "Hello World"
console.log(paragraph.child(0))       // First text node
```

**Text Node Creation:**

```typescript
// Schema.text is a convenience for creating text nodes
schema.text(string: string, marks?: Mark[]): Node

// Examples
const plainText = schema.text('Hello')
const boldText = schema.text('Bold', [schema.marks.strong.create()])
const multiMark = schema.text('Styled', [
  schema.marks.strong.create(),
  schema.marks.em.create(),
  schema.marks.underline.create()
])
```

---

### Content Arrays

Content in ProseMirror is stored as a `Fragment` - an immutable, persistent data structure.

```typescript
// Fragment Structure
interface Fragment {
  content: readonly Node[]
  size: number
  
  // Access
  child(n: number): Node
  firstChild: Node | null
  lastChild: Node | null
  childCount: number
  
  // Iteration
  forEach(f: (node: Node, offset: number, index: number) => void): void
  toArray(): Node[]
  
  // Operations
  cut(from: number, to?: number): Fragment
  replaceChild(index: number, node: Node): Fragment
  append(other: Fragment): Fragment
  addToStart(node: Node): Fragment
  addToEnd(node: Node): Fragment
  
  // Search
  findDiffStart(other: Fragment): number | null
  findDiffEnd(other: Fragment): { a: number; b: number } | null
  
  // Utility
  eq(other: Fragment): boolean
  toJSON(): Object[]
  static from(nodes: readonly Node[]): Fragment
  static fromJSON(schema: Schema, value: any): Fragment
}
```

**Fragment Operations:**

```typescript
// Creating fragments
const fragment = Fragment.from([
  schema.nodes.paragraph.create(),
  schema.nodes.heading.create({ level: 1 }),
  schema.nodes.paragraph.create()
])

// Iterating
fragment.forEach((node, offset, index) => {
  console.log(`Node ${index} at position ${offset}: ${node.type.name}`)
})

// Modifying (creates new fragment - immutable)
const newFragment = fragment
  .addToStart(schema.nodes.paragraph.create())
  .addToEnd(schema.nodes.paragraph.create())

// Cutting portions
const middle = fragment.cut(10, 50)  // Extract nodes between positions 10 and 50

// Finding differences
const diffStart = oldFragment.findDiffStart(newFragment)
const diffEnd = oldFragment.findDiffEnd(newFragment)
```

---

### Fragment Operations

```typescript
// Deep Fragment Operations
class FragmentOperations {
  // Map over all nodes
  mapFragment(fragment: Fragment, fn: (node: Node) => Node): Fragment {
    const nodes = []
    fragment.forEach(node => {
      nodes.push(fn(node))
      // Recursively process children
      if (node.content.size > 0) {
        nodes[nodes.length - 1] = nodes[nodes.length - 1].copy(
          this.mapFragment(node.content, fn)
        )
      }
    })
    return Fragment.from(nodes)
  }
  
  // Filter nodes by type
  filterByType(fragment: Fragment, typeName: string): Node[] {
    const result: Node[] = []
    fragment.forEach(node => {
      if (node.type.name === typeName) {
        result.push(node)
      }
      result.push(...this.filterByType(node.content, typeName))
    })
    return result
  }
  
  // Find node by position
  findNodeAt(fragment: Fragment, pos: number): { node: Node; offset: number } | null {
    let found: { node: Node; offset: number } | null = null
    
    fragment.forEach((node, offset) => {
      if (pos >= offset && pos < offset + node.nodeSize) {
        if (node.content.size > 0) {
          const childResult = this.findNodeAt(node.content, pos - offset - 1)
          if (childResult) {
            found = childResult
          } else {
            found = { node, offset }
          }
        } else {
          found = { node, offset }
        }
      }
    })
    
    return found
  }
  
  // Calculate total text content
  textContent(fragment: Fragment): string {
    let text = ''
    fragment.forEach(node => {
      text += node.textContent
      text += '\n'  // Add separator for block nodes
    })
    return text.trim()
  }
}
```

---

### Slice Operations

Slices represent portions of a document that can be inserted, replacing content.

```typescript
// Slice Structure
interface Slice {
  content: Fragment
  openStart: number   // Depth of open start boundary
  openEnd: number     // Depth of open end boundary
  
  // Size calculations
  size: number
  
  // Insert into document
  insert(from: number, tr: Transaction): void
  
  // JSON serialization
  toJSON(): Object
  
  static fromJSON(schema: Schema, json: Object): Slice
  static maxOpenDepth: number = 10
}

// Creating slices
const slice = new Slice(
  Fragment.from([paragraph1, paragraph2]),
  1,  // openStart - how deeply the start is "open" (not at node boundary)
  1   // openEnd
)

// Slice from selection
const slice = state.selection.content()

// Slice operations
class SliceOperations {
  // Fit slice into parent node
  fitSliceToParent(slice: Slice, parentType: NodeType): Slice {
    const { content, openStart, openEnd } = slice
    
    // Check if content fits
    const fits = parentType.contentMatch.matchFragment(content)
    
    if (!fits) {
      // Wrap content in appropriate parent
      const wrapped = parentType.createAndFill()
      return new Slice(wrapped.content, openStart + 1, openEnd + 1)
    }
    
    return slice
  }
  
  // Normalize slice for insertion
  normalizeSlice(slice: Slice, insertPos: number, state: EditorState): Slice {
    const $pos = state.doc.resolve(insertPos)
    const parent = $pos.parent
    const sliceContent = slice.content
    
    // Check if slice content matches parent's content expectations
    const match = parent.type.contentMatch.matchFragment(sliceContent)
    
    if (!match) {
      // Need to wrap or unwrap content
      return this.adjustSliceDepth(slice, parent.type)
    }
    
    return slice
  }
  
  // Adjust slice depth for proper insertion
  private adjustSliceDepth(slice: Slice, targetType: NodeType): Slice {
    const { content, openStart, openEnd } = slice
    
    if (openStart > 0) {
      // Slice starts inside a node - extract inner content
      const inner = content.firstChild
      if (inner) {
        return new Slice(inner.content, openStart - 1, openEnd)
      }
    }
    
    // Wrap content in target type
    const wrapped = targetType.create(null, content)
    return new Slice(Fragment.from(wrapped), openStart + 1, openEnd)
  }
}
```

---

## 4. Selection System

### Text Selections

TextSelection represents a cursor position or text range selection.

```typescript
// TextSelection Structure
interface TextSelection {
  $anchor: ResolvedPos
  $head: ResolvedPos
  anchor: number
  head: number
  from: number
  to: number
  empty: boolean
  
  // Creation
  static create(doc: Node, from: number, to?: number, $bias?: ResolvedPos): TextSelection
  static atStart(doc: Node): TextSelection
  static atEnd(doc: Node): TextSelection
  static near(pos: ResolvedPos, bias?: number): TextSelection
  
  // Properties
  $from: ResolvedPos
  $to: ResolvedPos
  
  // Content
  content(): Slice
  
  // Mapping
  map(doc: Node, mapping: Mappable): Selection
}

// Creating text selections
const selection = TextSelection.create(doc, 10)        // Cursor at position 10
const range = TextSelection.create(doc, 5, 20)          // Selection from 5 to 20
const atStart = TextSelection.atStart(doc)              // At document start
const atEnd = TextSelection.atEnd(doc)                  // At document end
const near = TextSelection.near($pos)                   // Near position, adjusted for validity
```

**ResolvedPos (Resolved Position):**

```typescript
interface ResolvedPos {
  pos: number              // Absolute position
  depth: number            // Depth in the tree
  parent: Node             // Parent node
  parentOffset: number     // Offset within parent
  
  // Navigation
  before(depth?: number): number
  after(depth?: number): number
  resolve(depth: number): ResolvedPos
  
  // Node access
  node(depth: number): Node
  nodeAtIndex(depth: number, index: number): Node
  
  // Path
  path: readonly number[]
  index(depth: number): number
  indexAfter(depth: number): number
  
  // Parent chain
  parentIndex(depth: number): number
  
  // Marks at position
  marks(): readonly Mark[]
  marksAcross(): readonly Mark[]
  
  // Node types
  nodeType: NodeType
}

// Resolving a position
const $pos = doc.resolve(10)

console.log($pos.pos)           // 10
console.log($pos.depth)         // Depth in tree
console.log($pos.parent)        // Parent node
console.log($pos.parentOffset)  // Offset within parent
console.log($pos.marks())       // Marks stored at this position
```

---

### Node Selections

NodeSelection selects an entire node (block or inline atom).

```typescript
// NodeSelection Structure
interface NodeSelection {
  $from: ResolvedPos
  $to: ResolvedPos
  anchor: number
  head: number
  from: number
  to: number
  
  // Selected node
  node: Node
  
  // Creation
  static create(doc: Node, pos: number): NodeSelection
  
  // Properties
  isBlock: boolean
  isInline: boolean
  
  // Content
  content(): Slice
}

// Creating node selection
const nodeSelection = NodeSelection.create(doc, 10)

// Access selected node
console.log(nodeSelection.node)           // The selected node
console.log(nodeSelection.node.type)      // Node type
console.log(nodeSelection.node.attrs)     // Node attributes

// Check selection type
if (selection instanceof NodeSelection) {
  console.log('Node selected:', selection.node.type.name)
}
```

---

### All Selections

Selection base class and other selection types.

```typescript
// Selection Hierarchy
//
// Selection (abstract)
// ├── TextSelection
// ├── NodeSelection
// └── AllSelection (selects entire document)

// AllSelection - selects the entire document
interface AllSelection {
  $from: ResolvedPos
  $to: ResolvedPos
  anchor: number
  head: number
  from: number
  to: number
  
  // Always selects entire document
  static create(doc: Node): AllSelection
  
  // Content
  content(): Slice
}

// Selection utilities
class SelectionUtils {
  // Get selection type
  static getType(selection: Selection): string {
    if (selection instanceof TextSelection) return 'text'
    if (selection instanceof NodeSelection) return 'node'
    if (selection instanceof AllSelection) return 'all'
    return 'unknown'
  }
  
  // Check if selection is empty (cursor)
  static isEmpty(selection: Selection): boolean {
    return selection.empty
  }
  
  // Get selection range
  static getRange(selection: Selection): { from: number; to: number } {
    return { from: selection.from, to: selection.to }
  }
  
  // Map selection through transaction
  static mapSelection(selection: Selection, tr: Transaction): Selection {
    return selection.map(tr.doc, tr.mapping)
  }
  
  // Create selection for specific position
  static createAt(doc: Node, pos: number): Selection {
    const $pos = doc.resolve(pos)
    
    // Try to create text selection
    try {
      return TextSelection.create(doc, pos)
    } catch {
      // Fall back to node selection if text selection fails
      return NodeSelection.create(doc, pos)
    }
  }
}
```

---

### Selection Transactions

```typescript
// Setting selection in transaction
const tr = state.tr

// Set text selection
tr.setSelection(TextSelection.create(tr.doc, 10))

// Set node selection
tr.setSelection(NodeSelection.create(tr.doc, 5))

// Set selection with metadata
tr.setSelection(TextSelection.create(tr.doc, 20))
  .setMeta('selectionSource', 'keyboard')

// Scroll to selection
tr.setSelection(TextSelection.create(tr.doc, 100))
  .scrollIntoView()

// Selection transaction flow
class SelectionTransactionFlow {
  // Create selection change transaction
  setSelection(from: number, to: number, options?: SelectionOptions) {
    return ({ tr, dispatch, state }) => {
      const selection = TextSelection.create(tr.doc, from, to)
      
      tr.setSelection(selection)
      
      // Add scroll behavior
      if (options?.scrollIntoView) {
        tr.scrollIntoView()
      }
      
      // Add source metadata
      if (options?.source) {
        tr.setMeta('selectionSource', options.source)
      }
      
      // Add focus metadata
      if (options?.focus) {
        tr.setMeta('focus', true)
      }
      
      dispatch?.(tr)
      return true
    }
  }
  
  // Selection with bias (prefer start or end of ambiguous position)
  setSelectionNear(pos: number, bias: 'start' | 'end' = 'start') {
    return ({ tr, dispatch, state }) => {
      const $pos = state.doc.resolve(pos)
      const selection = TextSelection.near($pos, bias === 'start' ? -1 : 1)
      
      tr.setSelection(selection)
      dispatch?.(tr)
      return true
    }
  }
  
  // Restore previous selection
  restoreSelection(previousSelection: Selection) {
    return ({ tr, dispatch }) => {
      const mapped = previousSelection.map(tr.doc, tr.mapping)
      tr.setSelection(mapped)
      dispatch?.(tr)
      return true
    }
  }
}
```

---

### Cursor Position

```typescript
// Cursor Position Management
interface CursorPosition {
  pos: number
  $pos: ResolvedPos
  marks: readonly Mark[]
  node: Node
  parent: Node
  depth: number
  
  // Get current cursor position
  static fromState(state: EditorState): CursorPosition {
    const { selection } = state
    const $pos = selection.$from
    
    return {
      pos: selection.from,
      $pos,
      marks: $pos.marks(),
      node: $pos.parent,
      parent: $pos.node($pos.depth - 1),
      depth: $pos.depth
    }
  }
}

// Cursor position utilities
class CursorManager {
  editor: Editor
  
  // Get cursor position
  getCursorPosition(): CursorPosition {
    return CursorPosition.fromState(this.editor.state)
  }
  
  // Check if cursor is at start of node
  isAtStartOfNode(): boolean {
    const { $from } = this.editor.state.selection
    return $from.parentOffset === 0
  }
  
  // Check if cursor is at end of node
  isAtEndOfNode(): boolean {
    const { $from } = this.editor.state.selection
    return $from.parentOffset === $from.parent.contentSize
  }
  
  // Check if cursor is at start of document
  isAtStartOfDocument(): boolean {
    return this.editor.state.selection.from === 0
  }
  
  // Check if cursor is at end of document
  isAtEndOfDocument(): boolean {
    return this.editor.state.selection.to === this.editor.state.doc.content.size
  }
  
  // Get cursor position relative to current node
  getRelativePosition(): number {
    const { $from } = this.editor.state.selection
    return $from.parentOffset
  }
  
  // Move cursor by offset
  moveCursor(offset: number) {
    return ({ tr, dispatch, state }) => {
      const newPos = state.selection.from + offset
      const clampedPos = Math.max(0, Math.min(newPos, state.doc.content.size))
      
      tr.setSelection(TextSelection.create(tr.doc, clampedPos))
      dispatch?.(tr)
      return true
    }
  }
  
  // Move cursor to start of current node
  moveToNodeStart() {
    return ({ tr, dispatch, state }) => {
      const { $from } = state.selection
      const nodeStart = $from.start($from.depth)
      
      tr.setSelection(TextSelection.create(tr.doc, nodeStart))
      dispatch?.(tr)
      return true
    }
  }
  
  // Move cursor to end of current node
  moveToNodeEnd() {
    return ({ tr, dispatch, state }) => {
      const { $from } = state.selection
      const nodeEnd = $from.end($from.depth)
      
      tr.setSelection(TextSelection.create(tr.doc, nodeEnd))
      dispatch?.(tr)
      return true
    }
  }
}
```

---

## 5. Transaction Flow

### Transaction Creation

```typescript
// Transaction Structure
interface Transaction {
  // Document state
  doc: Node
  before: Node
  
  // Selection
  selection: Selection
  selectionSet: boolean
  
  // Document changes
  docChanged: boolean
  
  // Steps applied
  steps: Step[]
  stepResults: StepResult[]
  
  // Mappings
  mapping: StepMapping
  docs: Node[]
  
  // Metadata storage
  meta: { [key: string]: any }
  
  // Flags
  isGeneric: boolean
  scrollingIntoView: boolean
  
  // Methods
  setMeta(key: string | PluginKey, value: any): Transaction
  getMeta(key: string | PluginKey, defaultValue?: any): any
  
  // Marking
  setStepType(stepType: StepType): Transaction
  setBatching(batching: boolean): Transaction
  
  // Scrolling
  scrollIntoView(): Transaction
  
  // Mark as generic (not user-triggered)
  setGeneric(generic: boolean): Transaction
}

// Transaction Creation Patterns
const createTransaction = {
  // Basic text insertion
  insertText: (state: EditorState, pos: number, text: string) => {
    return state.tr.insertText(pos, text)
  },
  
  // Replace selection
  replaceSelection: (state: EditorState, content: Node | Fragment) => {
    return state.tr.replaceSelection(content)
  },
  
  // Replace range
  replaceRange: (state: EditorState, from: number, to: number, content: Slice) => {
    return state.tr.replace(from, to, content)
  },
  
  // Delete range
  deleteRange: (state: EditorState, from: number, to: number) => {
    return state.tr.delete(from, to)
  },
  
  // Set node type
  setBlockType: (
    state: EditorState,
    from: number,
    to: number,
    nodeType: NodeType,
    attrs?: Attrs
  ) => {
    return state.tr.setBlockType(from, to, nodeType, attrs)
  },
  
  // Toggle mark
  toggleMark: (state: EditorState, from: number, to: number, markType: MarkType) => {
    const { doc } = state
    const hasMark = doc.rangeHasMark(from, to, markType)
    
    if (hasMark) {
      return state.tr.removeMark(from, to, markType)
    } else {
      return state.tr.addMark(from, to, markType.create())
    }
  }
}
```

---

### Steps and Step Maps

Steps are the atomic changes that make up a transaction.

```typescript
// Step Hierarchy
//
// Step (abstract)
// ├── ReplaceStep
// ├── ReplaceAroundStep
// ├── AddMarkStep
// ├── RemoveMarkStep
// ├── AddNodeStep
// ├── RemoveNodeStep
// └── AttributeStep

// Step Interface
interface Step {
  // Apply step to document
  apply(doc: Node): StepResult
  
  // Get affected range
  from: number
  to: number
  
  // Map step through other changes
  map(mapping: Mappable, offset: number): Step | null
  
  // Get step map (how positions change)
  getMap(): StepMap
  
  // JSON serialization
  toJSON(): Object
  
  static fromJSON(schema: Schema, json: Object): Step
}

// StepResult - result of applying a step
interface StepResult {
  doc?: Node       // New document (if successful)
  failed?: boolean // Whether step failed
  stepMap: StepMap // Position mapping
  
  // Create successful result
  static ok(doc: Node, stepMap: StepMap): StepResult
  
  // Create failed result
  static fail(stepMap: StepMap): StepResult
}

// StepMap - how positions change after a step
interface StepMap {
  positions: number[]  // Array of position deltas
  
  // Map a position
  map(pos: number, assoc?: number): number
  
  // Map a range
  mapResult(pos: number, assoc?: number): MapResult
  
  // Create step map
  static create(positions: number[]): StepMap
}

// MapResult - detailed mapping result
interface MapResult {
  pos: number    // Mapped position
  deleted: boolean  // Whether position was deleted
  assoc: number     // Association (-1 or 1)
}

// Example: ReplaceStep
class ReplaceStep extends Step {
  from: number
  to: number
  slice: Slice
  
  constructor(from: number, to: number, slice: Slice) {
    super()
    this.from = from
    this.to = to
    this.slice = slice
  }
  
  apply(doc: Node): StepResult {
    // Validate step can be applied
    if (!this.slice.content.size && !doc.rangeHasMark(this.from, this.to, null)) {
      return StepResult.fail(StepMap.empty)
    }
    
    // Create new document
    const newDoc = doc.replace(this.from, this.to, this.slice)
    
    // Create step map
    const stepMap = this.createStepMap(doc, this.from, this.to, this.slice)
    
    return StepResult.ok(newDoc, stepMap)
  }
  
  getMap(): StepMap {
    return this.createStepMap(null, this.from, this.to, this.slice)
  }
  
  map(mapping: Mappable): ReplaceStep | null {
    const mappedFrom = mapping.map(this.from, -1)
    const mappedTo = mapping.map(this.to, 1)
    
    if (mappedFrom >= mappedTo) {
      return null  // Step maps to empty range
    }
    
    return new ReplaceStep(mappedFrom, mappedTo, this.slice)
  }
}
```

**Step Mapping Visualization:**

```typescript
// Before: "Hello World" (11 characters + 2 for doc nodes = 13 total)
//
// Position mapping after inserting "Beautiful " at position 6:
//
// Before:  H  e  l  l  o     W  o  r  l  d
// Pos:     1  2  3  4  5  6  7  8  9 10 11 12
//
// After:   H  e  l  l  o     B  e  a  u  t  i  f  u  l     W  o  r  l  d
// Pos:     1  2  3  4  5  6  7  8  9 10 11 12 13 14 15 16 17 18 19 20 21 22
//
// StepMap.positions: [0, 0, 0, 0, 0, 10, 10, 10, 10, 10, 10, 10, 10, 10]
// (Each number represents the delta at that position)

// Step Map usage
const stepMap = new StepMap([0, 0, 0, 10, 10, 10])

// Map position 2 (unchanged)
console.log(stepMap.map(2))  // 2

// Map position 5 (after insertion point, shifted by 10)
console.log(stepMap.map(5))  // 15
```

---

### Metadata

```typescript
// Transaction Metadata System
interface TransactionMeta {
  // Built-in metadata keys
  selectionSource?: 'pointer' | 'keyboard' | 'api'
  focus?: boolean
  blur?: boolean
  scrollIntoView?: boolean
  
  // Plugin-specific metadata
  [pluginKey: string]: any
}

// Metadata usage patterns
class MetadataPatterns {
  // Setting metadata for plugin
  setPluginMeta(tr: Transaction, plugin: Plugin, value: any) {
    return tr.setMeta(plugin, value)
  }
  
  // Getting plugin metadata
  getPluginMeta(tr: Transaction, plugin: Plugin, defaultValue?: any) {
    return tr.getMeta(plugin, defaultValue)
  }
  
  // Setting custom metadata
  setCustomMeta(tr: Transaction, key: string, value: any) {
    return tr.setMeta(key, value)
  }
  
  // Common metadata patterns
  markAsUserAction(tr: Transaction) {
    return tr.setMeta('userAction', true)
  }
  
  markAsProgrammatic(tr: Transaction) {
    return tr.setMeta('userAction', false)
  }
  
  preventScroll(tr: Transaction) {
    return tr.setMeta('scrollIntoView', false)
  }
  
  markTransactionType(tr: Transaction, type: string) {
    return tr.setMeta('transactionType', type)
  }
}

// Plugin metadata pattern
const selectionPlugin = new Plugin({
  key: new PluginKey('selectionPlugin'),
  
  state: {
    init: () => null,
    apply: (tr, value) => tr.getMeta(selectionPlugin) || value
  },
  
  appendTransaction: (transactions, oldState, newState) => {
    const lastTr = transactions[transactions.length - 1]
    const meta = lastTr.getMeta(selectionPlugin)
    
    if (meta) {
      console.log('Selection plugin received:', meta)
    }
    
    return undefined
  }
})
```

---

### Scoping

```typescript
// Transaction Scoping
// Scoping allows transactions to be limited to specific document ranges

interface ScopedTransaction {
  // Scope definition
  scope: {
    from: number
    to: number
  }
  
  // Check if position is in scope
  isInScope(pos: number): boolean
  
  // Check if range is in scope
  isRangeInScope(from: number, to: number): boolean
  
  // Adjust position to scope
  clampToScope(pos: number): number
}

// Scoped transaction helpers
class ScopedTransactionManager {
  editor: Editor
  
  // Execute transaction only if changes are within scope
  scopedTransaction(
    scope: { from: number; to: number },
    fn: (tr: Transaction) => void
  ) {
    const { state } = this.editor
    
    this.editor.view.dispatch(
      state.tr.setMeta('scope', scope)
    )
    
    fn(state.tr)
  }
  
  // Create command that only works within scope
  createScopedCommand(
    scope: { from: number; to: number },
    command: Command
  ): Command {
    return (props) => {
      const { state } = props
      const { from, to } = state.selection
      
      // Check if selection is within scope
      if (from < scope.from || to > scope.to) {
        return false
      }
      
      return command(props)
    }
  }
  
  // Isolate changes to specific range
  isolateRange(
    from: number,
    to: number,
    fn: (tr: Transaction, rangeFrom: number, rangeTo: number) => void
  ) {
    return ({ tr, dispatch, state }) => {
      // Store original positions
      const originalFrom = from
      const originalTo = to
      
      // Execute function with isolated range
      fn(tr, originalFrom, originalTo)
      
      // Mark transaction as scoped
      tr.setMeta('isolatedRange', { from: originalFrom, to: originalTo })
      
      if (dispatch) {
        dispatch(tr)
      }
      
      return true
    }
  }
}
```

---

### Batch Transactions

```typescript
// Batch Transaction Pattern
// Batching groups multiple changes into a single undo/redo step

class BatchTransactionManager {
  editor: Editor
  private batchQueue: Transaction[] = []
  private isBatching = false
  
  // Start batching
  startBatch() {
    this.isBatching = true
    this.batchQueue = []
  }
  
  // Execute command and queue transaction
  queueCommand(command: Command): boolean {
    if (!this.isBatching) {
      return command(this.createProps())
    }
    
    // Create a temporary state to capture the transaction
    const tempState = this.editor.state
    let capturedTr: Transaction | null = null
    
    const result = command({
      editor: this.editor,
      state: tempState,
      dispatch: (tr) => {
        capturedTr = tr
      },
      view: this.editor.view,
      commands: {},
      can: () => ({}),
      chain: () => ({} as any),
      tr: tempState.tr
    })
    
    if (capturedTr) {
      this.batchQueue.push(capturedTr)
    }
    
    return result
  }
  
  // End batching and dispatch all as one transaction
  endBatch(): boolean {
    this.isBatching = false
    
    if (this.batchQueue.length === 0) {
      return false
    }
    
    // Create combined transaction
    const combinedTr = this.editor.state.tr
    
    for (const tr of this.batchQueue) {
      // Merge steps from each transaction
      for (let i = 0; i < tr.steps.length; i++) {
        combinedTr.step(tr.steps[i])
      }
      
      // Merge metadata
      Object.entries(tr.getMeta(null) || {}).forEach(([key, value]) => {
        combinedTr.setMeta(key, value)
      })
    }
    
    this.batchQueue = []
    this.editor.view.dispatch(combinedTr)
    
    return true
  }
  
  // Utility: execute multiple commands as batch
  batch(commands: Command[]): boolean {
    this.startBatch()
    
    for (const command of commands) {
      if (!this.queueCommand(command)) {
        // Optional: abort on failure
        // this.endBatch()
        // return false
      }
    }
    
    return this.endBatch()
  }
  
  private createProps(): CommandProps {
    return {
      editor: this.editor,
      state: this.editor.state,
      dispatch: (tr) => this.editor.view.dispatch(tr),
      view: this.editor.view,
      commands: {},
      can: () => ({}),
      chain: () => ({} as any),
      tr: this.editor.state.tr
    }
  }
}

// Usage example
const multiFormatCommand = () => ({ chain, dispatch }) => {
  const result = chain()
    .setTextSelection({ from: 10, to: 20 })
    .toggleBold()
    .setTextSelection({ from: 30, to: 40 })
    .toggleItalic()
    .setTextSelection({ from: 50, to: 60 })
    .toggleUnderline()
    .run()
  
  if (dispatch) {
    // All changes are in a single transaction
  }
  
  return result
}
```

---

## 6. Event System

### Transaction Events

```typescript
// Transaction Event System
interface EditorEvents {
  transaction: {
    editor: Editor
    transaction: Transaction
  }
  
  update: {
    editor: Editor
    transaction: Transaction
  }
  
  selectionUpdate: {
    editor: Editor
    transaction: Transaction
    rawTransaction: Transaction
  }
  
  focus: {
    editor: Editor
    event: FocusEvent
  }
  
  blur: {
    editor: Editor
    event: FocusEvent
  }
  
  beforeCreate: {
    editor: Editor
  }
  
  create: {
    editor: Editor
  }
  
  destroy: {
    editor: Editor
  }
}

// Event Emitter Implementation
class EventEmitter {
  private listeners: Map<string, Set<Function>> = new Map()
  
  // Subscribe to event
  on<K extends keyof EditorEvents>(
    event: K,
    callback: (data: EditorEvents[K]) => void
  ): Editor {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set())
    }
    
    this.listeners.get(event)!.add(callback as Function)
    return this.editor
  }
  
  // Unsubscribe from event
  off<K extends keyof EditorEvents>(
    event: K,
    callback: (data: EditorEvents[K]) => void
  ): Editor {
    this.listeners.get(event)?.delete(callback as Function)
    return this.editor
  }
  
  // Emit event
  emit<K extends keyof EditorEvents>(
    event: K,
    data: EditorEvents[K]
  ): void {
    this.listeners.get(event)?.forEach(callback => callback(data))
  }
  
  // Remove all listeners
  removeAllListeners(): void {
    this.listeners.clear()
  }
}

// Transaction event handling in extensions
const loggingExtension = Extension.create({
  name: 'loggingExtension',
  
  onTransaction({ editor, transaction }) {
    console.log('Transaction occurred:', {
      docChanged: transaction.docChanged,
      selectionSet: transaction.selectionSet,
      steps: transaction.steps.length,
      isGeneric: transaction.isGeneric
    })
  },
  
  onUpdate({ editor }) {
    console.log('Document updated')
  },
  
  onSelectionUpdate({ editor, transaction }) {
    console.log('Selection changed:', {
      from: editor.state.selection.from,
      to: editor.state.selection.to,
      empty: editor.state.selection.empty
    })
  }
})
```

---

### Selection Events

```typescript
// Selection Event Handling
interface SelectionEvent {
  editor: Editor
  transaction: Transaction
  previousSelection: Selection
  currentSelection: Selection
}

// Selection event utilities
class SelectionEventManager {
  editor: Editor
  private previousSelection: Selection | null = null
  
  constructor(editor: Editor) {
    this.editor = editor
    
    editor.on('selectionUpdate', ({ transaction }) => {
      this.handleSelectionChange(transaction)
    })
  }
  
  private handleSelectionChange(transaction: Transaction) {
    const currentSelection = this.editor.state.selection
    
    if (this.previousSelection && !this.selectionsEqual(this.previousSelection, currentSelection)) {
      // Selection changed
      this.emitSelectionChange({
        editor: this.editor,
        transaction,
        previousSelection: this.previousSelection,
        currentSelection
      })
    }
    
    this.previousSelection = currentSelection
  }
  
  private selectionsEqual(a: Selection, b: Selection): boolean {
    return a.from === b.from && a.to === b.to && a.empty === b.empty
  }
  
  private emitSelectionChange(event: SelectionEvent) {
    // Custom selection change handling
    console.log('Selection changed', event)
  }
  
  // Detect selection direction
  getSelectionDirection(): 'forward' | 'backward' | 'none' {
    const { selection } = this.editor.state
    
    if (selection.empty) return 'none'
    
    if (!this.previousSelection) return 'none'
    
    if (selection.anchor > selection.head) return 'backward'
    if (selection.anchor < selection.head) return 'forward'
    
    return 'none'
  }
}

// Selection source detection
const selectionSourcePlugin = new Plugin({
  key: new PluginKey('selectionSource'),
  
  view: (view) => ({
    update: (view, prevState) => {
      const prevSelection = prevState.selection
      const currSelection = view.state.selection
      
      if (!prevSelection.eq(currSelection)) {
        // Selection changed - determine source
        let source: 'pointer' | 'keyboard' | 'api' = 'api'
        
        // Check for pointer events
        if (view.lastDOMSelection) {
          source = 'pointer'
        }
        
        // Check for keyboard-related selection changes
        if (view.lastKeyCode) {
          source = 'keyboard'
        }
        
        // Store source in transaction
        view.dispatch(
          view.state.tr.setMeta('selectionSource', source)
        )
      }
    }
  })
})
```

---

### Focus Events

```typescript
// Focus Event Handling
interface FocusEvent {
  editor: Editor
  event: globalThis.FocusEvent
}

// Focus management
class FocusManager {
  editor: Editor
  
  // Check if editor has focus
  isFocused(): boolean {
    return this.editor.view.hasFocus()
  }
  
  // Focus the editor
  focus(options?: {
    position?: number | 'start' | 'end' | 'all'
    scrollIntoView?: boolean
  }): void {
    const { view } = this.editor
    
    if (!options?.position) {
      view.focus()
      return
    }
    
    let pos: number
    
    if (options.position === 'start') {
      pos = 0
    } else if (options.position === 'end') {
      pos = view.state.doc.content.size
    } else if (options.position === 'all') {
      pos = 0
    } else {
      pos = options.position
    }
    
    const tr = view.state.tr
    tr.setSelection(TextSelection.create(tr.doc, pos))
    tr.setMeta('focus', true)
    tr.scrollIntoView()
    
    view.dispatch(tr)
    view.focus()
  }
  
  // Blur the editor
  blur(): void {
    const { view } = this.editor
    
    if (view.hasFocus()) {
      const tr = view.state.tr
      tr.setMeta('blur', true)
      view.dispatch(tr)
      
      // Actually blur the DOM
      view.dom.blur()
    }
  }
  
  // Focus specific node
  focusNode(pos: number): void {
    const { view } = this.editor
    
    const tr = view.state.tr
    tr.setSelection(NodeSelection.create(tr.doc, pos))
    tr.setMeta('focus', true)
    tr.scrollIntoView()
    
    view.dispatch(tr)
    view.focus()
  }
  
  // Focus and select range
  focusRange(from: number, to: number): void {
    const { view } = this.editor
    
    const tr = view.state.tr
    tr.setSelection(TextSelection.create(tr.doc, from, to))
    tr.setMeta('focus', true)
    tr.scrollIntoView()
    
    view.dispatch(tr)
    view.focus()
  }
}

// Focus event plugin
const focusPlugin = new Plugin({
  key: new PluginKey('focus'),
  
  view: (view) => ({
    update: (view, prevState) => {
      const wasFocused = prevState.selection.$from.pos !== null
      const isFocused = view.hasFocus()
      
      // Detect focus change
      if (wasFocused && !isFocused) {
        // Focus lost
        const tr = view.state.tr
        tr.setMeta('blur', { event: 'click-outside' })
        view.dispatch(tr)
      } else if (!wasFocused && isFocused) {
        // Focus gained
        const tr = view.state.tr
        tr.setMeta('focus', { event: 'click-inside' })
        view.dispatch(tr)
      }
    }
  }),
  
  props: {
    handleFocus: (view, event) => {
      const tr = view.state.tr
      tr.setMeta('focus', { event })
      view.dispatch(tr)
      return false
    },
    
    handleBlur: (view, event) => {
      // Only blur if focus is moving outside editor
      if (!view.dom.contains(event.relatedTarget)) {
        const tr = view.state.tr
        tr.setMeta('blur', { event })
        view.dispatch(tr)
      }
      return false
    }
  }
})
```

---

### Custom Events

```typescript
// Custom Event System
interface CustomEvent<T = any> {
  type: string
  data: T
  bubbles: boolean
  cancelable: boolean
  defaultPrevented: boolean
  target: Editor
  currentTarget: Editor
  
  preventDefault(): void
  stopPropagation(): void
}

// Custom event emitter with typing
class CustomEventEmitter<T extends Record<string, any>> {
  private listeners: Map<keyof T, Set<(data: any) => void>> = new Map()
  
  on<K extends keyof T>(event: K, callback: (data: T[K]) => void): void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set())
    }
    this.listeners.get(event)!.add(callback as (data: any) => void)
  }
  
  off<K extends keyof T>(event: K, callback: (data: T[K]) => void): void {
    this.listeners.get(event)?.delete(callback as (data: any) => void)
  }
  
  emit<K extends keyof T>(event: K, data: T[K]): void {
    this.listeners.get(event)?.forEach(callback => callback(data))
  }
}

// Custom event in extension
const customEventExtension = Extension.create({
  name: 'customEventExtension',
  
  addStorage() {
    return {
      eventEmitter: new CustomEventEmitter<{
        customEvent: { value: string }
        dataLoaded: { data: any }
        contentChanged: { oldContent: any; newContent: any }
      }>()
    }
  },
  
  addCommands() {
    return {
      emitCustomEvent: (data: { value: string }) => ({ editor }) => {
        this.storage.eventEmitter.emit('customEvent', data)
        return true
      }
    }
  },
  
  // Listen to custom events
  onCreate() {
    this.storage.eventEmitter.on('customEvent', (data) => {
      console.log('Custom event received:', data)
    })
  }
})

// Usage
const editor = new Editor({
  extensions: [customEventExtension]
})

// Subscribe to custom event
const extension = editor.getExtension('customEventExtension')
extension.storage.eventEmitter.on('customEvent', (data) => {
  console.log('Received:', data)
})

// Emit custom event
editor.commands.emitCustomEvent({ value: 'test' })
```

---

### Event Bubbling

```typescript
// Event Bubbling System
interface EventBubbleConfig {
  // Stop propagation at this level
  stopPropagation?: boolean
  
  // Event priority (lower = earlier)
  priority?: number
  
  // Capture phase handler
  capture?: boolean
}

// Event bubbling in extensions
class EventBubblingSystem {
  editor: Editor
  private eventStack: Array<{ event: string; handler: Function }> = []
  
  // Register event with bubbling
  registerEvent(event: string, handler: Function, options?: EventBubbleConfig) {
    this.eventStack.push({
      event,
      handler,
      ...options
    })
    
    // Sort by priority
    this.eventStack.sort((a, b) => (a.priority || 100) - (b.priority || 100))
  }
  
  // Emit event with bubbling
  emitWithBubble(event: string, data: any): boolean {
    let stopped = false
    let handled = false
    
    for (const { event: registeredEvent, handler } of this.eventStack) {
      if (registeredEvent === event) {
        const result = handler({
          ...data,
          stopPropagation: () => {
            stopped = true
          }
        })
        
        if (result !== undefined) {
          handled = true
        }
        
        if (stopped) break
      }
    }
    
    return handled
  }
}

// Keyboard event bubbling
const keyboardEventExtension = Extension.create({
  name: 'keyboardEventExtension',
  
  addKeyboardShortcuts() {
    return {
      // Escape key - bubble through handlers
      Escape: () => {
        // First handler - could be a popup close
        if (this.editor.commands.closePopup?.()) {
          return true  // Stop bubbling
        }
        
        // Second handler - could be selection collapse
        if (this.editor.commands.blur?.()) {
          return true
        }
        
        // Default - do nothing
        return false
      }
    }
  }
})

// Command chain with event bubbling
const bubbleCommand = () => ({ commands, state }) => {
  // Try each handler in order
  const handlers = [
    () => commands.closeDialog?.(),
    () => commands.closePopup?.(),
    () => commands.clearSelection?.(),
    () => commands.blur?.()
  ]
  
  for (const handler of handlers) {
    if (handler()) {
      return true  // Stop bubbling
    }
  }
  
  return false  // Let default behavior handle
}
```

---

## 7. Editor View

### DOM Rendering

```typescript
// EditorView DOM Structure
interface EditorViewDOM {
  // Main editor element
  dom: HTMLElement
  
  // Content editable element
  contentDOM: HTMLElement
  
  // DOM methods
  destroy(): void
  updateState(state: EditorState): void
  focus(): void
  blur(): void
  hasFocus(): boolean
  
  // Position methods
  posAtDOM(node: Node, offset: number): number
  posAtCoords(coords: { left: number; top: number }): number | null
  coordsAtPos(pos: number): { left: number; top: number; bottom: number }
  
  // Selection methods
  setSelection(anchor: number, head: number): void
}

// DOM rendering options
interface DOMRenderOptions {
  // Class names
  class?: string
  
  // Attributes
  attributes?: { [name: string]: string }
  
  // Content DOM specific
  contentDOMAttributes?: { [name: string]: string }
  
  // Event handlers
  handleDOMEvents?: {
    [event: string]: (view: EditorView, event: Event) => boolean
  }
}

// DOM rendering pipeline
class DOMRenderer {
  view: EditorView
  
  // Create editor DOM
  createDOM(element: HTMLElement, options: DOMRenderOptions) {
    const wrapper = element
    wrapper.classList.add('ProseMirror', ...(options.class?.split(' ') || []))
    
    // Set attributes
    if (options.attributes) {
      Object.entries(options.attributes).forEach(([key, value]) => {
        wrapper.setAttribute(key, value)
      })
    }
    
    // Create content DOM
    const contentDOM = document.createElement('div')
    contentDOM.className = 'ProseMirror-content'
    contentDOM.contentEditable = 'true'
    
    if (options.contentDOMAttributes) {
      Object.entries(options.contentDOMAttributes).forEach(([key, value]) => {
        contentDOM.setAttribute(key, value)
      })
    }
    
    wrapper.appendChild(contentDOM)
    
    return { dom: wrapper, contentDOM }
  }
  
  // Render node to DOM
  renderNode(node: Node, options: NodeRenderOptions): { dom: HTMLElement } {
    const spec = node.type.spec
    const renderResult = spec.toDOM(node)
    
    let dom: HTMLElement
    
    if (typeof renderResult === 'string') {
      dom = document.createElement(renderResult)
    } else if (Array.isArray(renderResult)) {
      dom = this.createDOMFromSpec(renderResult)
    } else {
      dom = renderResult.dom
    }
    
    return { dom }
  }
  
  // Create DOM from spec array
  createDOMFromSpec(spec: any[]): HTMLElement {
    const [tag, attrsOrContent, ...children] = spec
    
    const element = document.createElement(tag.replace(/[.#].*/, ''))
    
    // Parse class and id from tag
    const classMatch = tag.match(/\.([^.#]+)/g)
    if (classMatch) {
      element.classList.add(...classMatch.map(c => c.slice(1)))
    }
    
    const idMatch = tag.match(/#([^.#]+)/)
    if (idMatch) {
      element.id = idMatch[1]
    }
    
    // Handle attributes
    if (attrsOrContent && typeof attrsOrContent === 'object' && !Array.isArray(attrsOrContent)) {
      Object.entries(attrsOrContent).forEach(([key, value]) => {
        if (key === 'class') {
          element.classList.add(...value.split(' '))
        } else if (key === 'style' && typeof value === 'object') {
          Object.entries(value).forEach(([styleKey, styleValue]) => {
            element.style[styleKey] = styleValue
          })
        } else {
          element.setAttribute(key, value)
        }
      })
    }
    
    return element
  }
}
```

---

### Node Views

```typescript
// NodeView Interface
interface NodeView {
  // DOM elements
  dom: HTMLElement
  contentDOM?: HTMLElement
  
  // Update handling
  update(node: Node): boolean
  
  // Selection
  selectNode(): void
  deselectNode(): void
  
  // Event handling
  stopEvent(event: Event): boolean
  ignoreMutation(mutation: MutationRecord): boolean
  
  // Cleanup
  destroy(): void
  
  // Position
  getPos?(): number
  
  // Decoration
  setAttrs?(attrs: Record<string, any>): void
}

// NodeView Props
interface NodeViewProps {
  editor: Editor
  node: Node
  decorations: Decoration[]
  selected: boolean
  extension: Extension
  HTMLAttributes: Record<string, any>
  getPos: () => number | null
  updateAttributes: (attrs: Record<string, any>) => void
}

// Creating custom NodeView
class CustomNodeView implements NodeView {
  dom: HTMLElement
  contentDOM?: HTMLElement
  
  private node: Node
  private getPos: () => number | null
  
  constructor(props: NodeViewProps) {
    this.node = props.node
    this.getPos = props.getPos
    
    // Create DOM
    this.dom = document.createElement('div')
    this.dom.className = 'custom-node'
    
    // Create content DOM if node has content
    if (this.node.content.size > 0) {
      this.contentDOM = document.createElement('div')
      this.dom.appendChild(this.contentDOM)
    }
  }
  
  // Update node
  update(node: Node): boolean {
    if (node.type !== this.node.type) return false
    this.node = node
    return true
  }
  
  // Node selected
  selectNode(): void {
    this.dom.classList.add('selected')
  }
  
  // Node deselected
  deselectNode(): void {
    this.dom.classList.remove('selected')
  }
  
  // Stop event from reaching ProseMirror
  stopEvent(event: Event): boolean {
    // Handle clicks within the node
    if (event instanceof MouseEvent) {
      return true  // Prevent ProseMirror from handling
    }
    return false
  }
  
  // Ignore specific mutations
  ignoreMutation(mutation: MutationRecord): boolean {
    // Ignore attribute mutations we don't care about
    if (mutation.type === 'attributes') {
      return !['class', 'style'].includes(mutation.attributeName || '')
    }
    return false
  }
  
  // Cleanup
  destroy(): void {
    // Remove event listeners, etc.
  }
}

// React NodeView example
class ReactNodeView implements NodeView {
  dom: HTMLElement
  contentDOM?: HTMLElement
  
  private root: ReactDOM.Root
  private node: Node
  
  constructor(props: NodeViewProps) {
    this.node = props.node
    
    // Create container
    this.dom = document.createElement('div')
    
    // Render React component
    const component = React.createElement(CustomReactComponent, {
      node: props.node,
      selected: props.selected,
      updateAttributes: props.updateAttributes
    })
    
    this.root = ReactDOM.createRoot(this.dom)
    this.root.render(component)
    
    // Setup content DOM for editable children
    if (props.node.content.size > 0) {
      this.contentDOM = this.dom.querySelector('[data-content]') || undefined
    }
  }
  
  update(node: Node): boolean {
    if (node.type !== this.node.type) return false
    this.node = node
    this.root.render(
      React.createElement(CustomReactComponent, {
        node,
        selected: false,  // Will be updated via selectNode/deselectNode
        updateAttributes: /* ... */
      })
    )
    return true
  }
  
  selectNode(): void {
    this.root.render(
      React.createElement(CustomReactComponent, {
        node: this.node,
        selected: true,
        updateAttributes: /* ... */
      })
    )
  }
  
  deselectNode(): void {
    this.root.render(
      React.createElement(CustomReactComponent, {
        node: this.node,
        selected: false,
        updateAttributes: /* ... */
      })
    )
  }
  
  destroy(): void {
    this.root.unmount()
  }
}
```

---

### Decoration System

```typescript
// Decoration Types
type DecorationType = 'node' | 'widget' | 'inline' | 'mark'

// Decoration Interface
interface Decoration {
  type: DecorationType
  from: number
  to: number
  
  // Spec with attributes
  spec: {
    class?: string
    style?: string
    side?: number
    marks?: readonly Mark[]
    inclusiveStart?: boolean
    inclusiveEnd?: boolean
    [key: string]: any
  }
  
  // Equality check
  eq(other: Decoration): boolean
  
  // Map through changes
  map(mapping: Mappable): Decoration | null
  
  // Create methods
  static node(pos: number, attrs: DecorationAttrs): Decoration
  static widget(pos: number, widget: HTMLElement, side?: number): Decoration
  static inline(from: number, to: number, attrs: DecorationAttrs): Decoration
}

// Decoration Attributes
interface DecorationAttrs {
  class?: string
  style?: string
  [key: string]: any
}

// DecorationSet - collection of decorations
interface DecorationSet {
  // Content
  local: Decoration[]
  children: { pos: number; set: DecorationSet }[]
  
  // Find decorations
  find(from?: number, to?: number, predicate?: (d: Decoration) => boolean): Decoration[]
  
  // Map through changes
  map(tr: Transaction): DecorationSet
  
  // Add decorations
  add(doc: Node, decorations: Decoration[]): DecorationSet
  
  // Remove decorations
  remove(decorations: Decoration[]): DecorationSet
  
  // Empty set
  static empty: DecorationSet
  
  // Create from decorations
  static create(doc: Node, decorations: Decoration[]): DecorationSet
}

// Creating decorations
class DecorationFactory {
  // Node decoration - wraps entire node
  static node(pos: number, attrs: DecorationAttrs): Decoration {
    return Decoration.node(pos, attrs)
  }
  
  // Widget decoration - inserts element at position
  static widget(pos: number, element: HTMLElement, side: number = 1): Decoration {
    return Decoration.widget(pos, element, side)
  }
  
  // Inline decoration - applies to text range
  static inline(from: number, to: number, attrs: DecorationAttrs): Decoration {
    return Decoration.inline(from, to, attrs)
  }
  
  // Common decoration patterns
  static highlight(from: number, to: number, color: string): Decoration {
    return Decoration.inline(from, to, {
      class: 'highlight',
      style: `background-color: ${color}`
    })
  }
  
  static error(from: number, to: number): Decoration {
    return Decoration.inline(from, to, {
      class: 'error-underline',
      style: 'text-decoration: underline wavy red'
    })
  }
  
  static selectedNode(pos: number): Decoration {
    return Decoration.node(pos, {
      class: 'node-selected'
    })
  }
}

// Decoration plugin example
const decorationPlugin = new Plugin({
  key: new PluginKey('decorations'),
  
  state: {
    init: () => DecorationSet.empty,
    
    apply: (tr, set, oldState, newState) => {
      // Map existing decorations
      set = set.map(tr)
      
      // Check for decoration updates
      const newDecorations = tr.getMeta('addDecorations')
      if (newDecorations) {
        set = set.add(tr.doc, newDecorations)
      }
      
      const removeDecorations = tr.getMeta('removeDecorations')
      if (removeDecorations) {
        set = set.remove(removeDecorations)
      }
      
      return set
    }
  },
  
  props: {
    decorations(state) {
      return this.getState(state)
    }
  }
})

// Using decorations in commands
const addHighlight = (from: number, to: number, color: string) => 
  ({ tr, dispatch, state }) => {
    const decoration = DecorationFactory.highlight(from, to, color)
    
    tr.setMeta('addDecorations', [decoration])
    
    if (dispatch) {
      dispatch(tr)
    }
    
    return true
  }
```

---

### Input Handling

```typescript
// Input Handling System
interface InputHandlers {
  // Text input
  handleTextInput: (
    view: EditorView,
    from: number,
    to: number,
    text: string
  ) => boolean
  
  // Key events
  handleKeyDown: (
    view: EditorView,
    event: KeyboardEvent
  ) => boolean
  
  handleKeyPress: (
    view: EditorView,
    event: KeyboardEvent
  ) => boolean
  
  handleKeyUp: (
    view: EditorView,
    event: KeyboardEvent
  ) => boolean
  
  // Composition (IME)
  handleCompositionStart: (
    view: EditorView,
    event: CompositionEvent
  ) => boolean
  
  handleCompositionUpdate: (
    view: EditorView,
    event: CompositionEvent
  ) => boolean
  
  handleCompositionEnd: (
    view: EditorView,
    event: CompositionEvent
  ) => boolean
}

// Input rules handler
class InputRulesHandler {
  private rules: InputRule[]
  
  constructor(rules: InputRule[]) {
    this.rules = rules
  }
  
  handleTextInput(view: EditorView, from: number, to: number, text: string): boolean {
    const { state, dispatch } = view
    
    for (const rule of this.rules) {
      const match = this.matchRule(rule, text, state, from, to)
      
      if (match) {
        const result = rule.handler({
          state,
          range: { from, to },
          match,
          commands: {},
          chain: () => ({} as any),
          can: () => ({})
        })
        
        if (result !== false) {
          return true
        }
      }
    }
    
    return false
  }
  
  private matchRule(
    rule: InputRule,
    text: string,
    state: EditorState,
    from: number,
    to: number
  ): RegExpMatchArray | null {
    const { doc } = state
    
    // Get text before cursor
    const beforeText = doc.textBetween(
      Math.max(0, from - 100),
      from
    ) + text
    
    if (typeof rule.find === 'function') {
      return rule.find(beforeText)
    }
    
    return beforeText.match(rule.find)
  }
}

// Keyboard shortcut handler
class KeyboardShortcutHandler {
  private shortcuts: Record<string, () => boolean>
  
  constructor(shortcuts: Record<string, () => boolean>) {
    this.shortcuts = shortcuts
  }
  
  handleKeyDown(view: EditorView, event: KeyboardEvent): boolean {
    const key = this.normalizeKey(event)
    const handler = this.shortcuts[key]
    
    if (handler) {
      return handler()
    }
    
    return false
  }
  
  private normalizeKey(event: KeyboardEvent): string {
    const parts: string[] = []
    
    if (event.ctrlKey || event.metaKey) parts.push('Mod')
    if (event.shiftKey) parts.push('Shift')
    if (event.altKey) parts.push('Alt')
    
    const key = event.key.length === 1 
      ? event.key.toLowerCase()
      : event.key
    
    parts.push(key)
    
    return parts.join('-')
  }
}

// IME Composition handler
class IMECompositionHandler {
  private composing = false
  private compositionText = ''
  
  handleCompositionStart(): boolean {
    this.composing = true
    this.compositionText = ''
    return false
  }
  
  handleCompositionUpdate(event: CompositionEvent): boolean {
    this.compositionText = event.data || ''
    return false
  }
  
  handleCompositionEnd(event: CompositionEvent): boolean {
    this.composing = false
    this.compositionText = ''
    return false
  }
  
  isComposing(): boolean {
    return this.composing
  }
}
```

---

### Clipboard Handling

```typescript
// Clipboard Handling System
interface ClipboardHandlers {
  handlePaste: (
    view: EditorView,
    event: ClipboardEvent,
    slice: Slice
  ) => boolean
  
  handleCut: (
    view: EditorView,
    event: ClipboardEvent
  ) => boolean
  
  handleCopy: (
    view: EditorView,
    event: ClipboardEvent
  ) => boolean
  
  handleDrop: (
    view: EditorView,
    event: DragEvent,
    slice: Slice,
    moved: boolean
  ) => boolean
}

// Clipboard text parser
interface ClipboardTextParser {
  (text: string, context: ResolvedPos, plain: boolean): Slice
}

// Clipboard text serializer
interface ClipboardTextSerializer {
  (slice: Slice): string
}

// Clipboard handler implementation
class ClipboardHandler {
  view: EditorView
  
  constructor(view: EditorView) {
    this.view = view
    this.setupEventListeners()
  }
  
  private setupEventListeners(): void {
    this.view.dom.addEventListener('paste', this.handlePaste.bind(this))
    this.view.dom.addEventListener('cut', this.handleCut.bind(this))
    this.view.dom.addEventListener('copy', this.handleCopy.bind(this))
    this.view.dom.addEventListener('dragover', this.handleDragOver.bind(this))
    this.view.dom.addEventListener('drop', this.handleDrop.bind(this))
  }
  
  handlePaste(event: ClipboardEvent): boolean {
    const { view } = this.view
    const { state, dispatch } = view
    
    // Check for custom handler
    if (view.props.handlePaste?.(view, event, null as any)) {
      return true
    }
    
    // Prevent default
    event.preventDefault()
    
    // Get clipboard data
    const clipboardData = event.clipboardData
    if (!clipboardData) return false
    
    // Try HTML first
    const html = clipboardData.getData('text/html')
    if (html) {
      const slice = this.parseHTML(html, state)
      if (slice) {
        const tr = state.tr.replaceSelection(slice)
        dispatch(tr)
        return true
      }
    }
    
    // Fall back to plain text
    const text = clipboardData.getData('text/plain')
    if (text) {
      const tr = state.tr.insertText(text)
      dispatch(tr)
      return true
    }
    
    return false
  }
  
  handleCut(event: ClipboardEvent): boolean {
    const { view } = this.view
    const { state, dispatch } = view
    
    // Check for custom handler
    if (view.props.handleCut?.(view, event)) {
      return true
    }
    
    // Get selection content
    const slice = state.selection.content()
    
    // Serialize to clipboard
    this.setClipboardData(event, slice)
    
    // Delete selection
    dispatch(state.tr.deleteSelection())
    
    return true
  }
  
  handleCopy(event: ClipboardEvent): boolean {
    const { view } = this.view
    const { state } = view
    
    // Check for custom handler
    if (view.props.handleCopy?.(view, event)) {
      return true
    }
    
    // Get selection content
    const slice = state.selection.content()
    
    // Serialize to clipboard
    this.setClipboardData(event, slice)
    
    return true
  }
  
  handleDrop(event: DragEvent): boolean {
    const { view } = this.view
    const { state, dispatch } = view
    
    // Check for custom handler
    if (view.props.handleDrop?.(view, event, null as any, false)) {
      return true
    }
    
    event.preventDefault()
    
    // Get drop position
    const pos = view.posAtCoords({
      left: event.clientX,
      top: event.clientY
    })
    
    if (!pos) return false
    
    // Check if this is a move from within the editor
    const draggedSelection = (view as any).draggedSelection
    if (draggedSelection) {
      const slice = draggedSelection.content()
      const tr = state.tr
        .delete(draggedSelection.from, draggedSelection.to)
        .insert(pos.pos, slice)
      
      dispatch(tr)
      return true
    }
    
    // Handle external drop
    const html = event.dataTransfer?.getData('text/html')
    if (html) {
      const slice = this.parseHTML(html, state)
      if (slice) {
        const tr = state.tr.replaceSelection(slice)
        dispatch(tr)
        return true
      }
    }
    
    return false
  }
  
  private handleDragOver(event: DragEvent): boolean {
    event.preventDefault()
    return false
  }
  
  private parseHTML(html: string, state: EditorState): Slice | null {
    // Parse HTML to DOM
    const parser = new DOMParser()
    const doc = parser.parseFromString(html, 'text/html')
    
    // Use ProseMirror's DOM parser
    const { schema } = state
    const slice = DOMParser.fromSchema(schema).parseSlice(doc.body)
    
    return slice
  }
  
  private setClipboardData(event: ClipboardEvent, slice: Slice): void {
    const clipboardData = event.clipboardData
    if (!clipboardData) return
    
    // Serialize to HTML
    const html = this.serializeHTML(slice)
    clipboardData.setData('text/html', html)
    
    // Serialize to plain text
    const text = this.serializeText(slice)
    clipboardData.setData('text/plain', text)
  }
  
  private serializeHTML(slice: Slice): string {
    const serializer = DOMSerializer.fromSchema(this.view.state.schema)
    const fragment = slice.content
    const container = document.createElement('div')
    
    fragment.forEach(node => {
      container.appendChild(serializer.serializeNode(node))
    })
    
    return container.innerHTML
  }
  
  private serializeText(slice: Slice): string {
    return slice.content.content.map(node => node.textContent).join('\n')
  }
}

// Custom clipboard handling plugin
const clipboardPlugin = new Plugin({
  key: new PluginKey('clipboard'),
  
  props: {
    // Custom clipboard text parser
    clipboardTextParser: (text, context, plain) => {
      // Parse plain text into ProseMirror content
      const nodes: Node[] = []
      const { schema } = context.doc.type.schema
      
      text.split('\n').forEach(line => {
        nodes.push(schema.nodes.paragraph.create(null, schema.text(line)))
      })
      
      return new Slice(Fragment.from(nodes), 0, 0)
    },
    
    // Custom clipboard text serializer
    clipboardTextSerializer: (slice) => {
      // Customize text serialization
      return slice.content.content.map(node => {
        if (node.type.name === 'paragraph') {
          return node.textContent
        }
        return node.textContent
      }).join('\n')
    },
    
    // Transform pasted content
    transformPasted: (slice, view) => {
      // Example: remove all marks from pasted content
      const nodes: Node[] = []
      slice.content.forEach(node => {
        nodes.push(node.mark([]))
      })
      return new Slice(Fragment.from(nodes), slice.openStart, slice.openEnd)
    },
    
    // Transform pasted HTML
    transformPastedHTML: (html) => {
      // Example: remove all style attributes
      return html.replace(/style="[^"]*"/g, '')
    }
  }
})
```

---

## Architecture Diagrams

### Complete Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              TIPTAP EDITOR                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                           Editor Class                                  │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │ │
│  │  │ extension   │  │  command    │  │    state    │  │    event    │   │ │
│  │  │  manager    │  │  manager    │  │  manager    │  │   handler   │   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                      │                                       │
│                                      ▼                                       │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                        Extension Manager                                │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │ │
│  │  │   Schema    │  │   Plugins   │  │  Commands   │  │   Storage   │   │ │
│  │  │   Builder   │  │   Collector │  │  Collector  │  │  Manager    │   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘   │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                      │                                       │
│                                      ▼                                       │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                        ProseMirror Core                                 │ │
│  │  ┌─────────────────────────┐         ┌─────────────────────────┐       │ │
│  │  │      EditorState        │         │       EditorView        │       │ │
│  │  │  ┌───────────────────┐  │         │  ┌───────────────────┐  │       │ │
│  │  │  │   Document (Node) │  │◄───────►│  │   DOM Rendering   │  │       │ │
│  │  │  │   Selection       │  │         │  │   Event Handling  │  │       │ │
│  │  │  │   Plugins         │  │         │  │   Input/Output    │  │       │ │
│  │  │  │   Transaction     │  │         │  │   Node Views      │  │       │ │
│  │  │  └───────────────────┘  │         │  └───────────────────┘  │       │ │
│  │  └─────────────────────────┘         └─────────────────────────┘       │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Transaction Flow

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                           TRANSACTION FLOW                                    │
└──────────────────────────────────────────────────────────────────────────────┘

    ┌─────────────────┐
    │   User Action   │ (Typing, Click, Command)
    │   or Command    │
    └────────┬────────┘
             │
             ▼
    ┌─────────────────┐
    │  Command        │
    │  Handler        │
    └────────┬────────┘
             │ Creates
             ▼
    ┌─────────────────┐
    │  Transaction    │
    │  (state.tr)     │
    └────────┬────────┘
             │
             │ Add Steps
             ▼
    ┌─────────────────┐
    │  Steps          │ (ReplaceStep, AddMarkStep, etc.)
    │  ┌───────────┐  │
    │  │ Step 1    │  │
    │  │ Step 2    │  │
    │  │ Step 3    │  │
    │  └───────────┘  │
    └────────┬────────┘
             │
             │ Set Metadata
             ▼
    ┌─────────────────┐
    │  Metadata       │ (Plugin keys, custom data)
    │  ┌───────────┐  │
    │  │ focus: T  │  │
    │  │ source:X  │  │
    │  └───────────┘  │
    └────────┬────────┘
             │
             │ Dispatch
             ▼
    ┌─────────────────────────────────────────────────────────────────┐
    │                    dispatchTransaction()                         │
    ├─────────────────────────────────────────────────────────────────┤
    │  1. Apply transaction to state                                  │
    │     state = state.apply(transaction)                            │
    │                                                                 │
    │  2. Update view with new state                                  │
    │     view.updateState(state)                                     │
    │                                                                 │
    │  3. Run plugin hooks (appendTransaction)                        │
    │     plugins may add additional transactions                     │
    │                                                                 │
    │  4. Emit events                                                 │
    │     - transaction                                               │
    │     - selectionUpdate (if selection changed)                    │
    │     - focus/blur (if focus changed)                             │
    │     - update (if document changed)                              │
    └─────────────────────────────────────────────────────────────────┘
             │
             ▼
    ┌─────────────────┐
    │    DOM Update   │
    │  - Recalculate  │
    │    decorations  │
    │  - Diff DOM     │
    │  - Apply changes│
    └─────────────────┘
```

### Extension Resolution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                    EXTENSION RESOLUTION                          │
└─────────────────────────────────────────────────────────────────┘

    Input Extensions: [StarterKit, CustomExtension, Link.configure()]
    
    ┌─────────────────┐
    │  Flatten        │
    │  (expand nested │
    │   extensions)   │
    └────────┬────────┘
             │
             ▼
    ┌─────────────────────────────────────────────────────────────┐
    │  [Document, Paragraph, Heading, Bold, Italic,              │
    │   History, Dropcursor, Gapcursor, Clipboard,                │
    │   Text, StarterKitExtension, CustomExtension, Link]         │
    └─────────────────────────────────────────────────────────────┘
             │
             │ Sort by Priority
             ▼
    ┌─────────────────────────────────────────────────────────────┐
    │  [Document(p:1), Paragraph(p:1), ..., Link(p:100)]          │
    └─────────────────────────────────────────────────────────────┘
             │
             │ Deduplicate
             ▼
    ┌─────────────────────────────────────────────────────────────┐
    │  (Remove duplicates, keep first occurrence)                 │
    └─────────────────────────────────────────────────────────────┘
             │
             │ Build Components
             ▼
    ┌─────────────────────────────────────────────────────────────┐
    │  Schema:   { nodes: {...}, marks: {...} }                   │
    │  Plugins:  [keyPlugin, historyPlugin, clipboardPlugin...]   │
    │  Commands: { toggleBold, setHeading, insertContent... }     │
    │  Storage:  { history: {...}, clipboard: {...} }             │
    └─────────────────────────────────────────────────────────────┘
```

### Document Position Model

```
┌─────────────────────────────────────────────────────────────────┐
│                    DOCUMENT POSITION MODEL                       │
└─────────────────────────────────────────────────────────────────┘

    Document Structure:
    
    Doc (0)
    ├── Paragraph (1)
    │   ├── Text "Hello" (2-6)
    │   └── Text " World" (7-13)
    └── Paragraph (14)
        └── Text "Second" (15-21)
    
    Position Mapping:
    
    0 ─┬─ Doc Start
       │
    1 ─┼─ Paragraph Start
       │
    2 ─┼─ Text "Hello" Start
       │ H
    3 ─┤ e
    4 ─┤ l
    5 ─┤ l
    6 ─┤ o
       │
    7 ─┼─ Text " World" Start
       │ W
    8 ─┤ o
    9 ─┤ r
    10─┤ l
    11─┤ d
       │
    12─┼─ Paragraph End
       │
    13─┼─ (between paragraphs)
       │
    14─┼─ Paragraph Start
       │
    15─┼─ Text "Second" Start
       │ S
    16─┤ e
    17─┤ c
    18─┤ o
    19─┤ n
    20─┤ d
       │
    21─┼─ Paragraph End
       │
    22─┴─ Doc End
    
    ResolvedPos Example ($pos at position 5):
    
    $pos.pos = 5
    $pos.depth = 2        (Doc → Paragraph → Text)
    $pos.parent = Text    (Current parent is Text node)
    $pos.parentOffset = 3 (Offset within Text node)
    $pos.node(0) = Doc
    $pos.node(1) = Paragraph
    $pos.node(2) = Text
    $pos.start(1) = 1     (Start of Paragraph)
    $pos.end(1) = 12      (End of Paragraph)
    $pos.before(1) = 0    (Position before Paragraph)
    $pos.after(1) = 13    (Position after Paragraph)
```

---

## Appendix: Key Interfaces Reference

```typescript
// Complete Editor Options Interface
interface EditorOptions {
  element: HTMLElement | null
  content: Content | null
  extensions: Extension[]
  editable: boolean
  autofocus: boolean | 'start' | 'end' | 'all' | number
  initialSelection: Selection | null
  
  // Editor View
  editorView: EditorViewProps | null
  
  // Parse Options
  parseOptions: ParseOptions
  
  // Core Callbacks
  onBeforeCreate: (props: { editor: Editor }) => void
  onCreate: (props: { editor: Editor }) => void
  onUpdate: (props: { editor: Editor; transaction: Transaction }) => void
  onSelectionUpdate: (props: { editor: Editor; transaction: Transaction }) => void
  onTransaction: (props: { editor: Editor; transaction: Transaction }) => void
  onFocus: (props: { editor: Editor; event: FocusEvent }) => void
  onBlur: (props: { editor: Editor; event: FocusEvent }) => void
  onDestroy: (props: { editor: Editor }) => void
}

// Extension Configuration
interface ExtensionConfig<Options = any, Storage = any> {
  name: string
  defaultOptions?: Partial<Options>
  priority?: number
  
  // Composition
  parent?: Extension
  extensions?: Extension[]
  
  // Lifecycle
  onCreate?: (props: { editor: Editor }) => void
  onUpdate?: (props: { editor: Editor }) => void
  onSelectionUpdate?: (props: { editor: Editor }) => void
  onTransaction?: (props: { editor: Editor; transaction: Transaction }) => void
  onFocus?: (props: { editor: Editor; event: FocusEvent }) => void
  onBlur?: (props: { editor: Editor; event: FocusEvent }) => void
  onDestroy?: (props: { editor: Editor }) => void
  
  // Schema
  addSchema?: () => Partial<SchemaSpec>
  addNode?: () => NodeConfig
  addMark?: () => MarkConfig
  addExtension?: () => ExtensionConfig
  
  // Features
  addCommands?: () => Partial<Commands<Options>>
  addKeyboardShortcuts?: () => { [key: string]: () => boolean }
  addInputRules?: () => InputRule[]
  addPasteRules?: () => PasteRule[]
  addNodeView?: () => (props: NodeViewProps) => NodeView
  addProseMirrorPlugins?: () => Plugin[]
  addGlobalAttributes?: () => GlobalAttribute[]
  addStorage?: () => Storage
}

// Content Types
type Content = string | JSONContent | JSONContent[] | Node | Fragment | null

interface JSONContent {
  type?: string
  attrs?: Record<string, any>
  content?: JSONContent[]
  marks?: {
    type: string
    attrs?: Record<string, any>
  }[]
  text?: string
}
```

---

## Sources

This document is based on the Tiptap open-source project architecture. For the most up-to-date information, refer to:

- [Tiptap Official Documentation](https://tiptap.dev/docs)
- [Tiptap GitHub Repository](https://github.com/ueberdosis/tiptap)
- [ProseMirror Documentation](https://prosemirror.net/docs/)
- [ProseMirror GitHub Repository](https://github.com/ProseMirror/prosemirror)
