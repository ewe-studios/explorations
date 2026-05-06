# OpenUI -- Evaluator (AST Interpreter)

The evaluator is a framework-agnostic AST interpreter that executes OpenUI Lang expressions. It handles literals, references, operators, ternary conditionals, member/index access, built-in functions, and action expressions. It is not a compiler — it walks the AST at runtime and evaluates each node.

**Aha:** The `Each` builtin is lazy — it controls its own evaluation. Unlike `Sum($items)` which eagerly evaluates all items then computes, `Each($items, "item", <Text>{item.name}</Text>)` evaluates the template once per item with a scoped variable substitution. This is analogous to how `if` is a special form in Lisp — it doesn't evaluate all branches, only the one that's taken. The evaluator has a special case for lazy builtins that bypasses the normal eager evaluation of all arguments.

Source: `openui/packages/lang-core/src/runtime/evaluator.ts` — AST interpreter
Source: `openui/packages/lang-core/src/parser/builtins.ts` — 13 data functions + 5 action steps

## Evaluation Model

```typescript
interface EvaluationContext {
  getState(name: string): unknown;         // Read $variable from the store
  resolveRef(name: string): unknown;       // Resolve a reference to another declaration
  extraScope?: Record<string, unknown>;    // Scoped variables (e.g. Each iterator)
}

function evaluate(node: ASTNode, context: EvaluationContext, schemaCtx?: SchemaContext): unknown {
  switch (node.k) {
    case "Str": return node.v;
    case "Num": return node.v;
    case "Bool": return node.v;
    case "Null": case "Ph": return null;
    case "StateRef": return context.extraScope?.[node.n] ?? context.getState(node.n);
    case "Ref": case "RuntimeRef": return context.resolveRef(node.n);
    case "Arr": return node.els.map(el => evaluate(el, context));
    case "Obj": return Object.fromEntries(node.entries.map(([k, v]) => [k, evaluate(v, context)]));
    case "BinOp": /* short-circuit for && and ||, then operator dispatch */
    case "Ternary": return evaluate(node.cond, context) ? evaluate(node.then, context) : evaluate(node.else, context);
    case "Comp": /* dispatch to LAZY_BUILTINS, BUILTINS, ACTION_NAMES, or mappedProps */
  }
}
```

The evaluator is a recursive function that pattern-matches on the AST `k` discriminant. Division by zero returns 0 (not Infinity). String `+` concatenation treats null as `""`. Loose equality (`==`) is used for `==`/`!=` operators.

## Built-in Functions (Data)

Source: `openui/packages/lang-core/src/parser/builtins.ts`

| Builtin | Signature | Purpose |
|---------|-----------|---------|
| `Count` | `Count(array) → number` | Array length |
| `First` | `First(array) → element` | First element |
| `Last` | `Last(array) → element` | Last element |
| `Sum` | `Sum(numbers[]) → number` | Sum of numeric array |
| `Avg` | `Avg(numbers[]) → number` | Average of numeric array |
| `Min` | `Min(numbers[]) → number` | Minimum value |
| `Max` | `Max(numbers[]) → number` | Maximum value |
| `Sort` | `Sort(array, field, direction?) → sorted array` | Sort by field ("asc" or "desc") |
| `Filter` | `Filter(array, field, operator, value) → filtered array` | Filter by field + operator ("==", "!=", ">", "<", ">=", "<=", "contains") |
| `Round` | `Round(number, decimals?) → number` | Round to N decimal places |
| `Abs` | `Abs(number) → number` | Absolute value |
| `Floor` | `Floor(number) → number` | Round down |
| `Ceil` | `Ceil(number) → number` | Round up |

All builtins are defined in `BUILTINS: Record<string, BuiltinDef>` with a `fn` field that the evaluator calls eagerly after evaluating arguments.

## Lazy Builtin: Each

```
Each($items, "item", Card(item.name, item.description))
```

`Each` is the only lazy builtin (`LAZY_BUILTINS: Set<string> = new Set(["Each"])`). It receives AST nodes directly instead of pre-evaluated values:

1. Evaluates the first arg (the array expression)
2. For each element, creates a scoped `extraScope` binding (`varName → current element`)
3. Evaluates the template AST with the scoped binding
4. Returns an array of results

```typescript
// From evaluator.ts — evaluateLazyBuiltin (simplified)
function evaluateLazyBuiltin(name: string, args: ASTNode[], context: EvaluationContext): unknown {
  if (name === "Each") {
    const arr = evaluate(args[0], context);  // Evaluate the array
    const varName = /* extract from args[1] */;
    const template = args[2];                 // NOT evaluated yet
    return arr.map((item) => {
      const scopedCtx = { ...context, extraScope: { ...context.extraScope, [varName]: item } };
      return evaluate(template, scopedCtx);  // Evaluate template per item
    });
  }
}
```

**Aha:** The `Each` template uses `extraScope` to inject the loop variable. The evaluator's `StateRef` case checks `context.extraScope?.[node.n]` before `context.getState(node.n)`, so the loop variable shadows any same-named state variable for the duration of that iteration. The materializer also handles `Each` specially — it scopes the iterator variable during materialization so template refs resolve correctly.

## Action Expressions

Source: `openui/packages/lang-core/src/parser/builtins.ts` — `ACTION_STEPS`

| Action | Runtime Type | Purpose |
|--------|-------------|---------|
| `Action` | (container) | Wraps a sequence of action steps |
| `Run` | `"run"` | Execute a query or mutation by reference |
| `ToAssistant` | `"continue_conversation"` | Send a message to the LLM |
| `OpenUrl` | `"open_url"` | Open a URL in the browser |
| `Set` | `"set"` | Set a state variable to a new value |
| `Reset` | `"reset"` | Reset state variables to initial values |

The evaluator produces an `ActionPlan` — an ordered list of typed steps:

```typescript
interface ActionPlan { steps: ActionStep[] }

type ActionStep =
  | { type: "run"; statementId: string; refType: "query" | "mutation" }
  | { type: "continue_conversation"; message: string; context?: string }
  | { type: "open_url"; url: string }
  | { type: "set"; target: string; valueAST: ASTNode }
  | { type: "reset"; targets: string[] };
```

`ACTION_NAMES` includes all step names plus `"Action"` itself. The React renderer's `triggerAction()` function interprets the plan.

## ReactiveAssign Marker

The evaluator emits `ReactiveAssign` markers for two-way bindings when a `StateRef` is passed to a component prop that has a reactive schema:

```typescript
interface ReactiveAssign {
  __reactive: "assign";     // Discriminant (not __openui_reactive)
  target: string;           // State variable name (e.g. "count")
  expr: ASTNode;            // Expression to evaluate with $value (the new value from the component)
}
```

This only fires when the evaluator has a `SchemaContext` and the prop's schema is marked reactive (`isReactiveSchema()`). The renderer handles the marker by creating a two-way binding: component writes → store update → re-render.

## Store Integration

Source: `openui/packages/lang-core/src/runtime/store.ts`

The store is a factory function (`createStore()`) returning a `Store` interface:

```typescript
interface Store {
  get(name: string): unknown;
  set(name: string, value: unknown): void;
  subscribe(listener: () => void): () => void;  // Global listener, not per-key
  getSnapshot(): Record<string, unknown>;        // Snapshot for React useSyncExternalStore
  initialize(defaults: Record<string, unknown>, persisted: Record<string, unknown>): void;
  dispose(): void;
}
```

Internally, it uses `Object.is()` for primitives and shallow key-by-key comparison for plain objects (form data). This prevents unnecessary re-renders when form state objects have the same shape and values but a new reference.

The `initialize()` method applies persisted values first, then defaults for new keys only — never overwriting existing user-modified state:

```typescript
function initialize(defaults: Record<string, unknown>, persisted: Record<string, unknown>): void {
  for (const key of Object.keys(persisted)) state.set(key, persisted[key]);
  for (const key of Object.keys(defaults)) {
    if (!state.has(key)) state.set(key, defaults[key]);  // Only new keys
  }
  rebuildSnapshot();
  notify();
}
```

**Aha:** The store's safe initialization preserves user state during streaming. When the LLM is still generating and the parser re-runs, `initialize()` is called again with possibly-changed defaults. But existing user-modified values are never overwritten — the `!state.has(key)` guard protects them. This prevents the jarring UX of a user typing into a form field and having their input replaced by a re-parsed default.

See [Materializer](04-materializer.md) for how expressions reach the evaluator.
See [React Renderer](06-react-renderer.md) for how the evaluator integrates with React.
See [Lang Core](02-lang-core.md) for the AST structure.
