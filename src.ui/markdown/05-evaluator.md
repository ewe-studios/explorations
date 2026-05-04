# OpenUI -- Evaluator (AST Interpreter)

The evaluator is a framework-agnostic AST interpreter that executes OpenUI Lang expressions. It handles literals, references, operators, ternary conditionals, member/index access, built-in functions, and action expressions. It is not a compiler — it walks the AST at runtime and evaluates each node.

**Aha:** The `Each` builtin is lazy — it controls its own evaluation. Unlike `Sum($items)` which eagerly evaluates all items then computes, `Each($items, "item", <Text>{item.name}</Text>)` evaluates the template once per item with a scoped variable substitution. This is analogous to how `if` is a special form in Lisp — it doesn't evaluate all branches, only the one that's taken. The evaluator has a special case for lazy builtins that bypasses the normal eager evaluation of all arguments.

Source: `openui/packages/lang-core/src/runtime/evaluator.ts` — AST interpreter
Source: `openui/packages/lang-core/src/parser/builtins.ts` — 13 data functions + 5 action steps

## Evaluation Model

```typescript
function evaluate(node: ElementNode, ctx: EvalContext): Value {
  switch (node.k) {
    case 'Str': return node.value;
    case 'Num': return node.value;
    case 'Bool': return node.value;
    case 'Null': return null;
    case 'StateRef': return ctx.store.get(node.name)();
    case 'Ref': return evaluate(resolveRef(node), ctx);
    case 'BinOp': return evalBinOp(node.op, evaluate(node.left, ctx), evaluate(node.right, ctx));
    case 'Ternary': return evaluate(node.cond, ctx) ? evaluate(node.then, ctx) : evaluate(node.else, ctx);
    // ... etc
  }
}
```

The evaluator is a recursive function that pattern-matches on the AST node type and evaluates accordingly.

## Built-in Functions (Data)

Source: `openui/packages/lang-core/src/builtins.ts`

| Builtin | Arguments | Returns | Purpose |
|---------|-----------|---------|---------|
| `Count(arr)` | Array | number | Length of array |
| `First(arr)` | Array | any | First element |
| `Last(arr)` | Array | any | Last element |
| `Sum(arr)` | Array<number> | number | Sum of values |
| `Avg(arr)` | Array<number> | number | Average of values |
| `Min(arr)` | Array<number> | number | Minimum value |
| `Max(arr)` | Array<number> | number | Maximum value |
| `Sort(arr, field?)` | Array, string? | Array | Sort by field or value |
| `Filter(arr, pred)` | Array, predicate | Array | Filter elements |
| `Round(n, digits?)` | number, number? | number | Round to digits |
| `Abs(n)` | number | number | Absolute value |
| `Floor(n)` | number | number | Floor |
| `Ceil(n)` | number | number | Ceiling |

## Lazy Builtin: Each

```
Each($items, "item", <Card title={item.name} />)
```

The `Each` builtin:
1. Evaluates the array expression (`$items`)
2. For each item, creates a scoped variable binding (`item → current value`)
3. Evaluates the template with the scoped binding
4. Returns an array of evaluated template results

```typescript
function evalEach(node: BuiltinCall, ctx: EvalContext): Value[] {
  const [arrExpr, varName, template] = node.args;
  const arr = evaluate(arrExpr, ctx);
  return arr.map((item: Value) => {
    const scopedCtx = { ...ctx, scope: { [varName]: item } };
    return evaluate(template, scopedCtx);
  });
}
```

**Aha:** The template in `Each` is evaluated with a scoped variable binding that shadows any outer variable of the same name. This is lexical scoping — the inner `item` shadows the outer `item` for the duration of the template evaluation, then the outer `item` is restored. The `substituteRef()` function handles this variable capture.

## Action Expressions

| Action | Purpose |
|--------|---------|
| `Run` | Execute a sequence of action steps |
| `ToAssistant` | Send a message to the LLM |
| `OpenUrl` | Open a URL in the browser |
| `Set` | Set a state variable |
| `Reset` | Reset a state variable to its initial value |

Actions are executed by the `triggerAction()` function in the React renderer, not by the evaluator. The evaluator produces an `ActionPlan` — a list of steps to execute:

```typescript
interface ActionPlan {
  steps: ActionStep[];
}

interface ActionStep {
  type: 'Set' | 'Reset' | 'ToAssistant' | 'OpenUrl';
  target?: string;
  value?: Value;
}
```

## ReactiveAssign Marker

The evaluator marks state variable assignments for two-way binding:

```typescript
interface ReactiveAssign {
  target: string;
  value: Value;
  __openui_reactive: true;
}
```

When the renderer encounters a `ReactiveAssign`, it updates the store and triggers re-rendering of all expressions that depend on that variable.

## Store Integration

Source: `openui/packages/lang-core/src/store.ts`

The evaluator reads state from a reactive store:

```typescript
class Store {
  private data = new Map<string, any>();
  private listeners = new Map<string, Set<() => void>>();

  get(name: string) {
    // Returns a signal-like function that tracks dependencies
    return () => this.data.get(name);
  }

  set(name: string, value: any) {
    if (this.data.get(name) !== value) {
      this.data.set(name, value);
      this.notify(name);
    }
  }

  subscribe(name: string, listener: () => void) {
    this.listeners.get(name)?.add(listener);
  }

  private notify(name: string) {
    this.listeners.get(name)?.forEach(l => l());
  }
}
```

The store uses shallow comparison for objects — if `newVal !== oldVal` (reference comparison), listeners are notified. This avoids unnecessary re-renders when the object reference hasn't changed.

**Aha:** The store's safe initialization preserves user state during streaming. When the LLM is still generating, the store might have partial state. The initializer only sets a signal if it hasn't been set by the user — it doesn't overwrite user-provided initial state with LLM-generated defaults.

See [Materializer](04-materializer.md) for how expressions reach the evaluator.
See [React Renderer](06-react-renderer.md) for how the evaluator integrates with React.
See [Lang Core](02-lang-core.md) for the AST structure.
