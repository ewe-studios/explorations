# Nordcraft Data Bindings and Reactive Flow Deep Dive

## Overview

Nordcraft uses a reactive data model built on signals and formulas. This deep-dive examines how variables, formulas, workflows, and data bindings work together to create dynamic, reactive applications.

## Data Model Architecture

### Component Data Structure

Each component maintains a structured data store:

```typescript
interface ComponentData {
  // Browser location (pages only)
  Location?: {
    page?: string
    path: string
    params: Record<string, string | null>    // Combined path + query
    query: Record<string, string | null>
    hash: string
  }
  
  // Component interface
  Attributes: Record<string, unknown>
  
  // Internal state
  Variables?: Record<string, unknown>
  
  // Context subscriptions
  Contexts?: Record<string, Record<string, unknown>>
  
  // URL parameters (legacy)
  'URL parameters'?: Record<string, string | null>
  
  // Route parameters
  'Route parameters'?: {
    path: Record<string, string | null>
    query: Record<string, string | null>
  }
  
  // API responses
  Apis?: Record<string, ApiStatus>
  
  // Component instance data
  Args?: unknown
  Parameters?: Record<string, unknown>
  
  // Event data (during event handling)
  Event?: unknown
  
  // List iteration context
  ListItem?: {
    Item: unknown
    Index: number
    Parent?: ListItem
  }
}

interface ApiStatus {
  status: 'loading' | 'success' | 'error' | 'idle'
  response?: unknown
  error?: unknown
  inputs?: Record<string, unknown>
}
```

### Data Access Pattern

Formulas and workflows access data through a unified interface:

```typescript
interface FormulaContext {
  component: Component
  data: ComponentData
  root: Document | ShadowRoot
  env: ToddleEnv
}

// Data resolution
function resolveDataPath(
  path: string,
  context: FormulaContext
): unknown {
  const [category, ...rest] = path.split('.')
  const key = rest.join('.')
  
  switch (category) {
    case 'Attributes':
      return getNestedValue(context.data.Attributes, key)
    
    case 'Variables':
      return getNestedValue(context.data.Variables, key)
    
    case 'Apis':
      const [apiName, ...responsePath] = key.split('.')
      const api = context.data.Apis?.[apiName]
      return responsePath.length 
        ? getNestedValue(api?.response, responsePath.join('.'))
        : api?.response
    
    case 'Contexts':
      return getNestedValue(context.data.Contexts, key)
    
    default:
      return undefined
  }
}

function getNestedValue(
  obj: Record<string, unknown>,
  path: string
): unknown {
  return path.split('.').reduce((acc, key) => acc?.[key], obj)
}
```

## Variable System

### Variable Definition and Initialization

```typescript
interface ComponentVariable {
  initialValue: Formula
}

// Example variables
const componentVariables = {
  // Simple value
  'count': {
    initialValue: { type: 'static', value: 0 }
  },
  
  // Object value
  'formData': {
    initialValue: {
      type: 'static',
      value: {
        email: '',
        password: '',
        rememberMe: false
      }
    }
  },
  
  // Computed initial value
  'userId': {
    initialValue: {
      type: 'formula',
      op: 'get',
      args: [
        { type: 'location', key: 'params.userId' },
        { type: 'static', value: null }
      ]
    }
  },
  
  // From API response
  'user': {
    initialValue: {
      type: 'api',
      name: 'userApi',
      path: 'data'
    }
  }
}
```

### Variable Reactivity with Signals

Variables use a signal-based reactivity system:

```typescript
// Signal implementation
class Signal<T> {
  private _value: T
  private _version: number = 0
  private _subscribers: Set<Effect> = new Set()
  private _computed?: Computed<T>
  
  constructor(initialValue: T) {
    this._value = initialValue
  }
  
  get(): T {
    // Track dependency if reading within an effect
    if (currentTrackingEffect) {
      currentTrackingEffect.addDependency(this)
    }
    return this._value
  }
  
  set(newValue: T): void {
    if (this._value !== newValue) {
      this._value = newValue
      this._version++
      this._notifySubscribers()
    }
  }
  
  private _notifySubscribers(): void {
    // Schedule effects for re-execution
    for (const effect of this._subscribers) {
      scheduleEffect(effect)
    }
  }
  
  subscribe(effect: Effect): () => void {
    this._subscribers.add(effect)
    return () => this._subscribers.delete(effect)
  }
}

// Variable signals in component
class ComponentInstance {
  private signals: Map<string, Signal<unknown>> = new Map()
  
  setVariable(name: string, value: unknown): void {
    const signal = this.signals.get(name)
    if (!signal) {
      throw new Error(`Variable "${name}" not found`)
    }
    signal.set(value)
  }
  
  getVariable(name: string): unknown {
    const signal = this.signals.get(name)
    if (!signal) {
      throw new Error(`Variable "${name}" not found`)
    }
    return signal.get()
  }
}
```

### SetVariable Action

The primary way to update variables:

```typescript
interface VariableActionModel {
  type: 'SetVariable'
  variable: string
  data: Formula
}

// Example: Increment counter
const incrementAction: VariableActionModel = {
  type: 'SetVariable',
  variable: 'count',
  data: {
    op: 'add',
    args: [
      { type: 'variable', name: 'count' },
      { type: 'static', value: 1 }
    ]
  }
}

// Example: Update form field
const updateEmailAction: VariableActionModel = {
  type: 'SetVariable',
  variable: 'formData',
  data: {
    op: 'object.set',
    args: [
      { type: 'variable', name: 'formData' },
      { type: 'static', value: 'email' },
      { type: 'argument', name: 'value' }
    ]
  }
}

// Action execution
async function executeSetVariable(
  action: VariableActionModel,
  context: ActionContext
): Promise<void> {
  const value = evaluateFormula(action.data, context)
  context.component.setVariable(action.variable, value)
}
```

## Formula System

### Formula AST Structure

Formulas are represented as abstract syntax trees:

```typescript
type Formula = 
  | StaticFormula
  | VariableFormula
  | ArgumentFormula
  | ApiFormula
  | LocationFormula
  | OperationFormula

interface StaticFormula {
  type: 'static'
  value: unknown
}

interface VariableFormula {
  type: 'variable'
  name: string
}

interface ArgumentFormula {
  type: 'argument'
  name: string
}

interface ApiFormula {
  type: 'api'
  name: string
  path?: string
}

interface LocationFormula {
  type: 'location'
  key: string  // 'params.userId', 'query.search', etc.
}

interface OperationFormula {
  type: 'formula'
  op: string
  args: Formula[]
}

// Example: Complex formula
const complexFormula: OperationFormula = {
  type: 'formula',
  op: 'if',
  args: [
    // Condition: user is logged in
    {
      op: 'not',
      args: [
        { type: 'variable', name: 'user' }
      ]
    },
    // Then: show login prompt
    { type: 'static', value: 'Please log in' },
    // Else: show greeting
    {
      op: 'concat',
      args: [
        { type: 'static', value: 'Hello, ' },
        {
          type: 'api',
          name: 'userApi',
          path: 'data.name'
        }
      ]
    }
  ]
}
```

### Formula Evaluation

Recursive formula evaluation:

```typescript
function evaluateFormula(
  formula: Formula,
  context: FormulaContext
): unknown {
  switch (formula.type) {
    case 'static':
      return formula.value
    
    case 'variable':
      return context.data.Variables?.[formula.name]
    
    case 'argument':
      return context.data.Arguments?.[formula.name]
    
    case 'api':
      const api = context.data.Apis?.[formula.name]
      if (!api || api.status !== 'success') return undefined
      return formula.path 
        ? getNestedValue(api.response, formula.path)
        : api.response
    
    case 'location':
      return getLocationValue(context.data.Location, formula.key)
    
    case 'formula':
      return evaluateOperation(formula.op, formula.args, context)
  }
}

function evaluateOperation(
  op: string,
  args: Formula[],
  context: FormulaContext
): unknown {
  const evaluatedArgs = args.map(arg => evaluateFormula(arg, context))
  
  // Get handler for operation
  const handler = getFormulaHandler(op)
  if (!handler) {
    throw new Error(`Unknown formula operation: ${op}`)
  }
  
  return handler(evaluatedArgs, context)
}

// Example formula handlers
const FORMULA_HANDLERS: Record<string, FormulaHandler> = {
  'add': (args) => args.reduce((sum, n) => sum + (Number(n) || 0), 0),
  
  'subtract': (args) => args[0] - args[1],
  
  'multiply': (args) => args.reduce((prod, n) => prod * (Number(n) || 1), 1),
  
  'concat': (args) => args.join(''),
  
  'eq': (args) => args[0] === args[1],
  
  'not': (args) => !args[0],
  
  'and': (args) => args.every(arg => Boolean(arg)),
  
  'or': (args) => args.some(arg => Boolean(arg)),
  
  'if': (args) => args[0] ? args[1] : args[2],
  
  'get': (args) => {
    const [obj, key, defaultValue] = args
    return obj?.[key as string] ?? defaultValue
  },
  
  'array.map': (args, context) => {
    const [array, formula] = args
    if (!Array.isArray(array)) return []
    return array.map((item, index) => {
      const itemContext = { ...context, data: { ...context.data, ListItem: { Item: item, Index: index } } }
      return evaluateFormula(formula as Formula, itemContext)
    })
  },
  
  'array.filter': (args, context) => {
    const [array, formula] = args
    if (!Array.isArray(array)) return []
    return array.filter((item, index) => {
      const itemContext = { ...context, data: { ...context.data, ListItem: { Item: item, Index: index } } }
      return Boolean(evaluateFormula(formula as Formula, itemContext))
    })
  }
}
```

### Memoized Formulas

Component formulas can be memoized:

```typescript
interface ComponentFormula {
  name: string
  memoize?: boolean
  formula: Formula
}

class MemoizedFormula {
  private formula: Formula
  private _cachedValue: unknown = undefined
  private _dependencies: Set<Signal> = new Set()
  private _version: number = 0
  
  constructor(formula: Formula) {
    this.formula = formula
  }
  
  evaluate(context: FormulaContext): unknown {
    // Check if dependencies changed
    if (this._dependenciesChanged()) {
      this._cachedValue = this._evaluateWithTracking(context)
      this._version++
    }
    
    return this._cachedValue
  }
  
  private _evaluateWithTracking(context: FormulaContext): unknown {
    const oldTracking = currentTrackingEffect
    const newTracking = new EffectTracker()
    currentTrackingEffect = newTracking
    
    try {
      return evaluateFormula(this.formula, context)
    } finally {
      currentTrackingEffect = oldTracking
      this._updateDependencies(newTracking.dependencies)
    }
  }
  
  private _dependenciesChanged(): boolean {
    for (const dep of this._dependencies) {
      if (dep.hasChanged()) return true
    }
    return false
  }
}
```

## Workflow System

### Workflow Execution Context

```typescript
interface WorkflowContext extends FormulaContext {
  parameters: Record<string, unknown>
  triggerWorkflow: (name: string, params?: Record<string, unknown>) => Promise<void>
  triggerAction: (name: string, args?: Record<string, unknown>) => void
}

// Workflow execution
async function executeWorkflow(
  workflow: ComponentWorkflow,
  parameters: Record<string, unknown>,
  context: WorkflowContext
): Promise<void> {
  const localContext: WorkflowContext = {
    ...context,
    parameters,
    triggerWorkflow: createWorkflowTrigger(context),
    triggerAction: createActionTrigger(context)
  }
  
  // Execute actions sequentially
  for (const action of workflow.actions) {
    await executeAction(action, localContext)
  }
}
```

### Action Types

```typescript
type ActionModel =
  | SetVariableAction
  | TriggerEventAction
  | FetchAction
  | SwitchAction
  | TriggerWorkflowAction
  | SetURLParameterAction
  | CustomAction

// Fetch action with callbacks
interface FetchAction {
  type: 'Fetch'
  api: string
  inputs?: Record<string, { formula: Formula }>
  onSuccess: { actions: ActionModel[] }
  onError: { actions: ActionModel[] }
  onMessage?: { actions: ActionModel[] }  // For SSE/WebSocket
}

// Switch action for conditional logic
interface SwitchAction {
  type: 'Switch'
  data: Formula
  cases: Array<{
    condition: Formula
    actions: ActionModel[]
  }>
  default: { actions: ActionModel[] }
}

// Trigger workflow action
interface TriggerWorkflowAction {
  type: 'TriggerWorkflow'
  workflow: string
  parameters: Record<string, { formula: Formula }>
  contextProvider?: string
}

// Execute action based on type
async function executeAction(
  action: ActionModel,
  context: WorkflowContext
): Promise<void> {
  switch (action.type) {
    case 'SetVariable':
      const value = evaluateFormula(action.data, context)
      context.component.setVariable(action.variable, value)
      break
    
    case 'Fetch':
      await executeFetchAction(action, context)
      break
    
    case 'Switch':
      await executeSwitchAction(action, context)
      break
    
    case 'TriggerWorkflow':
      const params = evaluateParameters(action.parameters, context)
      await context.triggerWorkflow(action.workflow, params)
      break
    
    case 'TriggerEvent':
      const eventData = evaluateFormula(action.data, context)
      context.component.triggerEvent(action.event, eventData)
      break
    
    case 'SetURLParameter':
      const paramValue = evaluateFormula(action.data, context)
      setURLParameter(action.parameter, paramValue, action.historyMode)
      break
  }
}
```

### Async Action Handling

Fetch actions execute asynchronously with callback handling:

```typescript
async function executeFetchAction(
  action: FetchAction,
  context: WorkflowContext
): Promise<void> {
  // Evaluate inputs
  const inputs: Record<string, unknown> = {}
  for (const [key, { formula }] of Object.entries(action.inputs || {})) {
    inputs[key] = evaluateFormula(formula, context)
  }
  
  // Execute fetch
  try {
    const response = await context.env.fetch(action.api, inputs)
    
    // Update API state
    context.component.setApiState(action.api, {
      status: 'success',
      response,
      inputs
    })
    
    // Execute success callbacks
    for (const callbackAction of action.onSuccess.actions) {
      await executeAction(callbackAction, context)
    }
  } catch (error) {
    // Update API state with error
    context.component.setApiState(action.api, {
      status: 'error',
      error,
      inputs
    })
    
    // Execute error callbacks
    for (const callbackAction of action.onError.actions) {
      await executeAction(callbackAction, context)
    }
  }
}
```

## Attribute Bindings

### Dynamic Attribute Binding

Element and component attributes bind to formulas:

```typescript
interface ElementNodeModel {
  attrs: Record<string, Formula>
}

// Example: Dynamic href
const linkElement: ElementNodeModel = {
  type: 'element',
  tag: 'a',
  attrs: {
    'href': {
      type: 'formula',
      op: 'concat',
      args: [
        { type: 'static', value: '/users/' },
        { type: 'api', name: 'userApi', path: 'data.id' }
      ]
    },
    'target': { type: 'static', value: '_blank' },
    'rel': { type: 'static', value: 'noopener noreferrer' }
  }
}

// Example: Component attribute binding
const userCardInstance: ComponentNodeModel = {
  type: 'component',
  name: 'UserCard',
  attrs: {
    'user': {
      type: 'api',
      name: 'userApi',
      path: 'data'
    },
    'showAvatar': { type: 'static', value: true },
    'onCardClick': {
      type: 'workflow',
      name: 'handleCardClick'
    }
  }
}

// Attribute evaluation
function evaluateAttributes(
  attrs: Record<string, Formula>,
  context: FormulaContext
): Record<string, unknown> {
  const result: Record<string, unknown> = {}
  
  for (const [name, formula] of Object.entries(attrs)) {
    result[name] = evaluateFormula(formula, context)
  }
  
  return result
}
```

## Conditional Rendering

### Show Formula

Elements conditionally render based on formulas:

```typescript
interface ElementNodeModel {
  condition?: Formula  // Show/hide formula
}

// Example: Show only for admins
const adminPanel: ElementNodeModel = {
  type: 'element',
  tag: 'div',
  condition: {
    type: 'formula',
    op: 'eq',
    args: [
      { type: 'api', name: 'userApi', path: 'data.role' },
      { type: 'static', value: 'admin' }
    ]
  },
  children: ['adminContent']
}

// Example: Show loading state
const loadingSpinner: ElementNodeModel = {
  type: 'element',
  tag: 'div',
  condition: {
    type: 'formula',
    op: 'eq',
    args: [
      { type: 'api', name: 'dataApi', path: 'status' },
      { type: 'static', value: 'loading' }
    ]
  },
  children: ['spinner']
}

// Condition evaluation
function evaluateCondition(
  condition: Formula | undefined,
  context: FormulaContext
): boolean {
  if (!condition) return true
  
  const result = evaluateFormula(condition, context)
  return Boolean(result)
}
```

### Repeat Formula

List rendering with repeat formulas:

```typescript
interface ElementNodeModel {
  repeat?: Formula      // Array formula
  repeatKey?: Formula   // Key formula for list identity
}

// Example: Render list of items
const itemList: ElementNodeModel = {
  type: 'element',
  tag: 'ul',
  children: ['listItem']
}

const listItem: ElementNodeModel = {
  type: 'element',
  tag: 'li',
  repeat: {
    type: 'api',
    name: 'itemsApi',
    path: 'data'
  },
  repeatKey: {
    type: 'formula',
    op: 'get',
    args: [
      { type: 'listItem', key: 'Item' },
      { type: 'static', value: 'id' }
    ]
  },
  children: ['itemName', 'itemDescription']
}

const itemName: TextNodeModel = {
  type: 'text',
  value: {
    type: 'formula',
    op: 'get',
    args: [
      { type: 'listItem', key: 'Item' },
      { type: 'static', value: 'name' }
    ]
  }
}

// Repeat evaluation
function evaluateRepeat(
  repeat: Formula | undefined,
  context: FormulaContext
): Array<{ item: unknown; index: number; key: string }> {
  if (!repeat) return []
  
  const array = evaluateFormula(repeat, context)
  if (!Array.isArray(array)) return []
  
  return array.map((item, index) => ({
    item,
    index,
    key: String(index)  // Default key
  }))
}

// With custom key formula
function evaluateRepeatWithKey(
  repeat: Formula,
  repeatKey: Formula,
  context: FormulaContext
): Array<{ item: unknown; index: number; key: string }> {
  const array = evaluateFormula(repeat, context)
  if (!Array.isArray(array)) return []
  
  return array.map((item, index) => {
    const itemContext = {
      ...context,
      data: {
        ...context.data,
        ListItem: { Item: item, Index: index }
      }
    }
    
    return {
      item,
      index,
      key: String(evaluateFormula(repeatKey, itemContext))
    }
  })
}
```

## Reactive Update System

### Dependency Tracking

Effects track their dependencies for efficient re-execution:

```typescript
class EffectTracker {
  dependencies: Set<Signal> = new Set()
  
  addDependency(signal: Signal): void {
    this.dependencies.add(signal)
  }
}

let currentTrackingEffect: EffectTracker | null = null

class Effect {
  private fn: () => void
  dependencies: Set<Signal> = new Set()
  
  constructor(fn: () => void) {
    this.fn = fn
  }
  
  run(): void {
    // Clear old dependencies
    this._cleanup()
    
    // Track new dependencies
    const tracker = new EffectTracker()
    const oldTracking = currentTrackingEffect
    currentTrackingEffect = tracker
    
    try {
      this.fn()
    } finally {
      currentTrackingEffect = oldTracking
      this._updateDependencies(tracker.dependencies)
    }
  }
  
  private _cleanup(): void {
    for (const signal of this.dependencies) {
      signal.unsubscribe(this)
    }
    this.dependencies.clear()
  }
  
  private _updateDependencies(newDeps: Set<Signal>): void {
    this.dependencies = newDeps
    for (const signal of newDeps) {
      signal.subscribe(this)
    }
  }
}
```

### Effect Scheduling

Effects batch and schedule for efficient updates:

```typescript
const scheduledEffects: Set<Effect> = new Set()
let isFlushing = false

function scheduleEffect(effect: Effect): void {
  scheduledEffects.add(effect)
  
  if (!isFlushing) {
    isFlushing = true
    requestAnimationFrame(flushEffects)
  }
}

function flushEffects(): void {
  const effects = Array.from(scheduledEffects)
  scheduledEffects.clear()
  
  for (const effect of effects) {
    effect.run()
  }
  
  isFlushing = false
  
  // Process any newly scheduled effects
  if (scheduledEffects.size > 0) {
    flushEffects()
  }
}
```

### DOM Updates via Effects

Reactive DOM updates:

```typescript
// Create reactive text binding
function createTextBinding(
  element: Text,
  formula: Formula,
  context: FormulaContext
): () => void {
  const effect = new Effect(() => {
    const value = evaluateFormula(formula, context)
    element.textContent = String(value ?? '')
  })
  
  effect.run()
  
  return () => {
    // Cleanup function
  }
}

// Create reactive attribute binding
function createAttributeBinding(
  element: HTMLElement,
  attrName: string,
  formula: Formula,
  context: FormulaContext
): () => void {
  const effect = new Effect(() => {
    const value = evaluateFormula(formula, context)
    
    if (value === null || value === undefined || value === false) {
      element.removeAttribute(attrName)
    } else if (value === true) {
      element.setAttribute(attrName, '')
    } else {
      element.setAttribute(attrName, String(value))
    }
  })
  
  effect.run()
  return () => {}
}

// Create reactive class binding
function createClassBinding(
  element: HTMLElement,
  className: string,
  formula: Formula,
  context: FormulaContext
): () => void {
  const effect = new Effect(() => {
    const shouldApply = evaluateFormula(formula, context)
    
    if (shouldApply) {
      element.classList.add(className)
    } else {
      element.classList.remove(className)
    }
  })
  
  effect.run()
  return () => {}
}
```

## API Integration

### API Response Handling

```typescript
interface ComponentAPI {
  name: string
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH'
  url: Formula
  headers?: Record<string, Formula>
  body?: Formula
  autoFetch?: boolean
  inputs?: Record<string, { formula: Formula }>
}

// API state management
class ApiManager {
  private state: Map<string, Signal<ApiStatus>> = new Map()
  
  setApiState(apiName: string, state: Partial<ApiStatus>): void {
    const signal = this.state.get(apiName)
    if (signal) {
      signal.set({ ...signal.get(), ...state })
    }
  }
  
  getApiState(apiName: string): ApiStatus {
    const signal = this.state.get(apiName)
    return signal?.get() || { status: 'idle' }
  }
}

// Auto-fetch behavior
function setupAutoFetch(
  api: ComponentAPI,
  context: FormulaContext
): () => void {
  if (!api.autoFetch) return
  
  const effect = new Effect(() => {
    // Evaluate URL and inputs
    const url = evaluateFormula(api.url, context)
    const inputs = evaluateParameters(api.inputs, context)
    
    // Trigger fetch if URL is valid
    if (url) {
      fetchApi(api.name, url, inputs)
    }
  })
  
  effect.run()
  return () => {}
}
```

## Summary

Nordcraft's data binding and reactive flow system provides:

1. **Signal-based Reactivity**: Variables use signals for efficient dependency tracking
2. **Formula AST**: Formulas are composable trees of operations
3. **Workflow Actions**: Side-effectful operations execute sequentially
4. **Async Handling**: Fetch actions with callback-based success/error handling
5. **Conditional Rendering**: Show/hide elements with formulas
6. **List Rendering**: Repeat elements with key-based identity
7. **Attribute Bindings**: Dynamic element and component attributes
8. **Dependency Tracking**: Effects automatically track and re-execute on changes
9. **Batched Updates**: Scheduled effect flushing for performance
10. **API Integration**: Auto-fetch with reactive response handling

This reactive architecture ensures the UI stays synchronized with underlying data while maintaining efficient update patterns.
