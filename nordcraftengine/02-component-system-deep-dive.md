# Nordcraft Component System Deep Dive

## Overview

The component system is the foundation of Nordcraft's architecture. Components are self-contained, reusable building blocks that encapsulate structure, styles, and behavior. This deep-dive examines the technical implementation of Nordcraft components.

## Component Data Model

The core component structure is defined by the `Component` interface:

```typescript
interface Component {
  name: string                    // Unique component identifier
  version?: 2                     // Version indicator (legacy)
  page?: string                   // Page route (for page components)
  route?: PageRoute | null        // Route configuration
  
  // Interface definition
  attributes: Record<string, ComponentAttribute>
  
  // Internal state and logic
  variables: Record<string, ComponentVariable>
  formulas?: Record<string, ComponentFormula>
  workflows?: Record<string, ComponentWorkflow>
  contexts?: Record<string, ComponentContext>
  
  // External integrations
  apis: Record<string, ComponentAPI>
  
  // Visual structure
  nodes: Record<string, NodeModel>
  
  // Lifecycle and events
  events?: ComponentEvent[]
  onLoad?: EventModel
  onAttributeChange?: EventModel
  
  // Package export flag
  exported?: boolean
}
```

## Component Anatomy

### Root Element and Node Tree

Every component has a root node that serves as the top-level container:

```typescript
// Node model types
type NodeModel = 
  | TextNodeModel
  | ElementNodeModel
  | ComponentNodeModel
  | SlotNodeModel

// Element node structure
interface ElementNodeModel {
  id: string
  type: 'element'
  tag: string                    // HTML tag name
  attrs: Record<string, Formula> // Dynamic attributes
  style: NodeStyleModel          // CSS properties
  variants?: StyleVariant[]      // Conditional styles
  children: string[]             // Child node IDs
  events: Record<string, EventModel>
  
  // Conditional rendering
  condition?: Formula            // Show/hide formula
  repeat?: Formula               // Repeat formula
  repeatKey?: Formula            // Key for list rendering
  
  // Slot targeting
  slot?: string                  // Parent component slot
  
  // Styling
  classes: Record<string, { formula?: Formula }>
  'style-variables'?: Array<{
    category: StyleTokenCategory
    name: string
    formula: Formula
    unit?: string
  }>
}
```

### Component Instance Model

When a component is used, an instance is created:

```typescript
interface ComponentNodeModel {
  id: string
  type: 'component'
  name: string              // Reference to component definition
  package?: string          // Source package (if external)
  path?: string             // File path (if local)
  
  // Instance-specific data
  attrs: Record<string, Formula>  // Bound attribute values
  style?: NodeStyleModel          // Root style overrides
  variants?: StyleVariant[]       // Style variants for root
  
  // Composition
  children: string[]              // Projected children (slots)
  events: Record<string, EventModel>
  
  // Conditional rendering
  condition?: Formula
  repeat?: Formula
  repeatKey?: Formula
}
```

## Attribute System

### Attribute Definition

Component attributes define the public interface:

```typescript
interface ComponentAttribute {
  name: string
  testValue: unknown        // Editor preview value
}

// Example: Button component attributes
const ButtonAttributes = {
  'label': {
    name: 'label',
    testValue: 'Click me'
  },
  'variant': {
    name: 'variant',
    testValue: 'primary'
  },
  'disabled': {
    name: 'disabled',
    testValue: false
  },
  'onClick': {
    name: 'onClick',
    testValue: null  // Event attribute
  }
}
```

### Attribute Binding

When using a component, attributes bind to formulas:

```typescript
// Component instance with attribute bindings
const componentInstance: ComponentNodeModel = {
  type: 'component',
  name: 'Button',
  attrs: {
    // Static binding
    'label': { type: 'static', value: 'Submit' },
    
    // Dynamic binding from variable
    'disabled': { 
      type: 'formula',
      formula: { 
        op: 'not',
        args: [{ type: 'variable', name: 'isEnabled' }]
      }
    },
    
    // Binding from API response
    'label': {
      type: 'formula',
      formula: {
        op: 'get',
        args: [
          { type: 'api', name: 'userData' },
          { type: 'static', value: 'name' }
        ]
      }
    }
  }
}
```

## Variable System

### Variable Definition

Component variables store internal state:

```typescript
interface ComponentVariable {
  initialValue: Formula
}

// Example: Form component variables
const FormVariables = {
  'formData': {
    initialValue: {
      type: 'static',
      value: { email: '', password: '' }
    }
  },
  'errors': {
    initialValue: {
      type: 'static',
      value: {}
    }
  },
  'isSubmitting': {
    initialValue: {
      type: 'static',
      value: false
    }
  }
}
```

### Variable Scope and Encapsulation

Variables are strictly scoped to their component:

```typescript
// Variable access pattern
class ComponentScope {
  private variables: Map<string, unknown>
  private parentScope: ComponentScope | null
  
  getVariable(name: string): unknown {
    // First check local variables
    if (this.variables.has(name)) {
      return this.variables.get(name)
    }
    
    // Variables do NOT bubble up to parents
    // This is intentional for encapsulation
    throw new Error(`Variable "${name}" not found`)
  }
  
  setVariable(name: string, value: unknown): void {
    // Can only set local variables
    if (!this.variables.has(name)) {
      throw new Error(`Variable "${name}" does not exist`)
    }
    this.variables.set(name, value)
  }
}
```

### Variable Reactivity

Variables use a signal-based reactivity system:

```typescript
class Signal<T> {
  private value: T
  private subscribers: Set<Effect> = new Set()
  
  get(): T {
    // Track dependency if in effect context
    if (currentEffect) {
      this.subscribers.add(currentEffect)
    }
    return this.value
  }
  
  set(newValue: T): void {
    if (this.value !== newValue) {
      this.value = newValue
      // Notify subscribers
      this.subscribers.forEach(effect => effect.run())
    }
  }
}

// Variable update triggers reactivity
function setVariable(name: string, value: unknown): void {
  const signal = componentSignals.get(name)
  if (signal) {
    signal.set(value)
  }
}
```

## Formula System

### Component Formula Structure

Formulas are reusable calculations scoped to the component:

```typescript
interface ComponentFormula {
  name: string
  arguments?: Array<{ 
    name: string
    testValue: any 
  }>
  memoize?: boolean           // Cache result
  exposeInContext?: boolean   // Share via context
  formula: Formula            // Formula AST
}

// Example: Form validation formula
const EmailValidationFormula: ComponentFormula = {
  name: 'isValidEmail',
  arguments: [
    { name: 'email', testValue: 'test@example.com' }
  ],
  memoize: true,
  formula: {
    op: 'regex.test',
    args: [
      { type: 'argument', name: 'email' },
      { 
        type: 'static', 
        value: '^[\\w-\\.]+@([\\w-]+\\.)+[\\w-]{2,4}$'
      }
    ]
  }
}
```

### Formula Evaluation Context

Formulas evaluate with access to component data:

```typescript
interface FormulaContext {
  component: Component
  data: ComponentData
  root: Document | ShadowRoot
  env: ToddleEnv
}

interface ComponentData {
  Attributes: Record<string, unknown>
  Variables: Record<string, unknown>
  Contexts: Record<string, Record<string, unknown>>
  Apis: Record<string, ApiStatus>
  ListItem?: { Item: unknown; Index: number }
  Event?: unknown
}

// Formula evaluation
function evaluateFormula(
  formula: Formula,
  context: FormulaContext
): unknown {
  switch (formula.type) {
    case 'variable':
      return context.data.Variables[formula.name]
    
    case 'attribute':
      return context.data.Attributes[formula.name]
    
    case 'api':
      return context.data.Apis[formula.name]?.response
    
    case 'argument':
      return context.data.Arguments[formula.name]
    
    case 'formula':
      // Evaluate nested formula
      const handler = getFormulaHandler(formula.op)
      const args = formula.args.map(arg => 
        evaluateFormula(arg, context)
      )
      return handler(args, context)
    
    default:
      return formula.value
  }
}
```

## Workflow System

### Workflow Definition

Workflows handle side-effectful logic:

```typescript
interface ComponentWorkflow {
  name: string
  parameters: Array<{ 
    name: string
    testValue: any 
  }>
  actions: ActionModel[]
  exposeInContext?: boolean
}

// Example: Form submission workflow
const SubmitFormWorkflow: ComponentWorkflow = {
  name: 'submitForm',
  parameters: [
    { name: 'formData', testValue: { email: '', password: '' } }
  ],
  actions: [
    // Set isSubmitting to true
    {
      type: 'SetVariable',
      variable: 'isSubmitting',
      data: { type: 'static', value: true }
    },
    
    // Call API
    {
      type: 'Fetch',
      api: 'loginApi',
      inputs: {
        body: {
          formula: { type: 'argument', name: 'formData' }
        }
      },
      onSuccess: {
        actions: [
          {
            type: 'TriggerEvent',
            event: 'onLoginSuccess',
            data: { type: 'api', name: 'loginApi.response' }
          }
        ]
      },
      onError: {
        actions: [
          {
            type: 'SetVariable',
            variable: 'errors',
            data: { type: 'api', name: 'loginApi.error' }
          }
        ]
      }
    },
    
    // Set isSubmitting back to false
    {
      type: 'SetVariable',
      variable: 'isSubmitting',
      data: { type: 'static', value: false }
    }
  ]
}
```

### Action Types

Workflows consist of action nodes:

```typescript
type ActionModel =
  | VariableActionModel        // SetVariable
  | EventActionModel           // TriggerEvent
  | SwitchActionModel          // Switch/Case
  | FetchActionModel           // API call
  | CustomActionModel          // Custom action
  | SetURLParameterAction      // Update URL
  | WorkflowActionModel        // Call workflow

// Switch action for conditional logic
interface SwitchActionModel {
  type: 'Switch'
  data: Formula
  cases: Array<{
    condition: Formula
    actions: ActionModel[]
  }>
  default: { actions: ActionModel[] }
}

// Example: User role-based routing
const RoleSwitchAction: SwitchActionModel = {
  type: 'Switch',
  data: { type: 'variable', name: 'userRole' },
  cases: [
    {
      condition: { 
        op: 'eq', 
        args: [{ type: 'switch.value' }, { type: 'static', value: 'admin' }]
      },
      actions: [
        {
          type: 'Navigate',
          url: { type: 'static', value: '/admin/dashboard' }
        }
      ]
    },
    {
      condition: {
        op: 'eq',
        args: [{ type: 'switch.value' }, { type: 'static', value: 'user' }]
      },
      actions: [
        {
          type: 'Navigate',
          url: { type: 'static', value: '/user/home' }
        }
      ]
    }
  ],
  default: {
    actions: [
      {
        type: 'Navigate',
        url: { type: 'static', value: '/login' }
      }
    ]
  }
}
```

### Sequential Execution Model

Workflows execute actions sequentially:

```typescript
async function executeWorkflow(
  workflow: ComponentWorkflow,
  parameters: Record<string, unknown>,
  context: WorkflowContext
): Promise<void> {
  const localContext = {
    ...context,
    parameters
  }
  
  for (const action of workflow.actions) {
    const result = await executeAction(action, localContext)
    
    // Handle async callbacks
    if (action.type === 'Fetch') {
      // API calls are async - callbacks execute on events
      // The workflow continues without waiting
      if (action.onSuccess) {
        registerCallback('success', action.onSuccess)
      }
      if (action.onError) {
        registerCallback('error', action.onError)
      }
    }
  }
}
```

## Event System

### Component Events

Components communicate upward through custom events:

```typescript
interface ComponentEvent {
  name: string
  dummyEvent: any  // Test event data
}

// Event definition in component
const CardEvents: ComponentEvent[] = [
  {
    name: 'onClick',
    dummyEvent: { type: 'click', target: {} }
  },
  {
    name: 'onDelete',
    dummyEvent: { id: '123' }
  }
]

// Event triggering
function triggerComponentEvent(
  component: ComponentNodeModel,
  eventName: string,
  data: unknown
): void {
  const eventModel = component.events[eventName]
  if (!eventModel) return
  
  // Execute the bound workflow
  if (eventModel.trigger === 'workflow') {
    executeWorkflow(eventModel.workflow, { event: data })
  }
  
  // Dispatch native DOM event for external handlers
  const domEvent = new CustomEvent(eventName, {
    bubbles: true,
    detail: data
  })
  element.dispatchEvent(domEvent)
}
```

### Event Binding

Parent components bind to child events:

```typescript
// Parent component with event handlers
const parentComponent: Component = {
  nodes: {
    'root': {
      type: 'element',
      tag: 'div',
      children: ['card1', 'card2']
    },
    'card1': {
      type: 'component',
      name: 'Card',
      events: {
        'onClick': {
          trigger: 'workflow',
          actions: [
            {
              type: 'SetVariable',
              variable: 'selectedCard',
              data: { type: 'static', value: 'card1' }
            }
          ]
        }
      }
    }
  }
}
```

## Context System

### Context Definition

Contexts expose formulas and workflows to descendant components:

```typescript
interface ComponentContext {
  formulas: string[]      // Exposed formula names
  workflows: string[]     // Exposed workflow names
  componentName?: string
  package?: string
}

// Example: Form context
const FormContext: ComponentContext = {
  formulas: ['validateField', 'getFormData'],
  workflows: ['submitForm', 'resetForm'],
  componentName: 'FormProvider'
}
```

### Context Subscription

Descendant components consume context:

```typescript
interface ContextConsumer {
  contexts: Record<string, {
    provider: string      // Provider component name
    formulas: string[]    // Subscribed formulas
    workflows: string[]   // Subscribed workflows
  }>
}

// Using context in a child component
const childComponent: Component = {
  name: 'FormField',
  contexts: {
    'Form': {
      provider: 'FormProvider',
      formulas: ['validateField'],
      workflows: ['submitForm']
    }
  },
  formulas: {
    'validateAndSubmit': {
      formula: {
        op: 'context.call',
        args: [
          { type: 'static', value: 'Form.validateField' },
          { type: 'argument', name: 'value' }
        ]
      }
    }
  }
}
```

## Lifecycle Hooks

### On Load

Executes when component mounts:

```typescript
interface Component {
  onLoad?: EventModel
}

// Example: Fetch data on load
const UserProfileComponent: Component = {
  name: 'UserProfile',
  onLoad: {
    trigger: 'load',
    actions: [
      {
        type: 'Fetch',
        api: 'userApi',
        inputs: {
          userId: {
            formula: { type: 'urlParam', name: 'id' }
          }
        },
        onSuccess: {
          actions: [
            {
              type: 'SetVariable',
              variable: 'userData',
              data: { type: 'api', name: 'userApi.response' }
            }
          ]
        }
      }
    ]
  }
}
```

### On Attribute Change

Executes when component attributes update:

```typescript
interface Component {
  onAttributeChange?: EventModel
}

// Example: React to attribute changes
const SearchComponent: Component = {
  name: 'Search',
  attributes: {
    'query': { name: 'query', testValue: '' }
  },
  onAttributeChange: {
    trigger: 'attributeChange',
    actions: [
      // Debounced search
      {
        type: 'TriggerWorkflow',
        workflow: 'debouncedSearch',
        parameters: {
          'query': { formula: { type: 'attribute', name: 'query' } }
        }
      }
    ]
  }
}
```

## Component Extraction

### Extract from Elements

The "Extract as Component" feature:

```typescript
function extractAsComponent(
  selectedNodeId: string,
  componentName: string
): Component {
  const selectedNode = nodes[selectedNodeId]
  
  // Create new component with selected structure
  const newComponent: Component = {
    name: componentName,
    attributes: {},
    variables: {},
    formulas: {},
    workflows: {},
    apis: {},
    nodes: extractSubtree(selectedNode),
    events: []
  }
  
  // Analyze dependencies
  const dependencies = findDependencies(selectedNode)
  
  // Externalize dependencies as attributes
  for (const dep of dependencies.externalVariables) {
    newComponent.attributes[dep.name] = {
      name: dep.name,
      testValue: dep.defaultValue
    }
    // Replace variable references with attribute references
    replaceVariableWithAttribute(newComponent, dep.nodeId, dep.variableName)
  }
  
  return newComponent
}

// Find external dependencies
function findDependencies(node: NodeModel): DependencyAnalysis {
  const dependencies = {
    internalVariables: new Set<string>(),
    externalVariables: new Set<{ name: string; nodeId: string }>(),
    formulas: new Set<string>()
  }
  
  // Traverse node tree
  traverseNodes(node, (child) => {
    // Check condition formulas
    if (child.condition) {
      const refs = findVariableReferences(child.condition)
      refs.forEach(ref => {
        if (isInternalVariable(ref)) {
          dependencies.internalVariables.add(ref)
        } else {
          dependencies.externalVariables.add({
            name: ref,
            nodeId: child.id
          })
        }
      })
    }
  })
  
  return dependencies
}
```

## Component Composition Patterns

### Slot-Based Composition

Components support content projection through slots:

```typescript
interface SlotNodeModel {
  type: 'slot'
  name?: string             // Named slot
  children: string[]        // Default/fallback content
}

// Container component with slots
const CardComponent: Component = {
  name: 'Card',
  nodes: {
    'root': {
      type: 'element',
      tag: 'div',
      class: 'card',
      children: ['header', 'content', 'footer']
    },
    'header': {
      type: 'slot',
      name: 'header',
      children: [
        // Default header content
        {
          type: 'element',
          tag: 'h3',
          children: ['defaultTitle']
        }
      ]
    },
    'content': {
      type: 'slot',
      children: []  // No default content
    },
    'footer': {
      type: 'slot',
      name: 'footer',
      children: []
    }
  }
}

// Using the component with projected content
const cardInstance: ComponentNodeModel = {
  type: 'component',
  name: 'Card',
  children: [
    // Projected content goes into default slot
    {
      type: 'element',
      tag: 'p',
      children: ['textContent']
    }
  ]
}
```

### Component Style Overrides

Styling component instances from the outside:

```typescript
// Instance-specific style overrides
const styledCardInstance: ComponentNodeModel = {
  type: 'component',
  name: 'Card',
  style: {
    'background-color': '#f0f0f0',
    'border-radius': '12px'
  },
  variants: [
    {
      hover: true,
      style: {
        'box-shadow': '0 4px 12px rgba(0,0,0,0.15)'
      }
    }
  ]
}

// Style override application (runtime)
function applyComponentStyles(
  component: Component,
  instanceStyles: NodeStyleModel | undefined
): void {
  const rootElement = getRootElement(component)
  
  // Apply instance-specific styles to root
  if (instanceStyles) {
    for (const [property, value] of Object.entries(instanceStyles)) {
      rootElement.style.setProperty(property, value)
    }
  }
  
  // Apply variants
  if (component.variants) {
    applyVariants(rootElement, component.variants)
  }
}
```

## Summary

The Nordcraft component system provides:

1. **Encapsulation**: Variables, formulas, and workflows are scoped to the component
2. **Composability**: Components nest through slots and content projection
3. **Reactivity**: Signal-based updates when variables change
4. **Communication**: Events bubble up, contexts provide downward communication
5. **Reusability**: Attributes define the public interface for customization
6. **Lifecycle Hooks**: On load and on attribute change handlers
7. **Extraction**: Convert existing elements into reusable components

This architecture enables building complex applications from simple, reusable building blocks while maintaining clean separation of concerns.
