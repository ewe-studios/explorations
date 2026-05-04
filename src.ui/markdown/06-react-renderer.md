# OpenUI -- React Renderer

The React renderer is a `Renderer` component that takes an OpenUI Lang string, a component library, and streams the progressive parse results as React nodes. It uses `useSyncExternalStore` to subscribe to the internal Store and QueryManager, re-rendering automatically when signals change or queries resolve.

**Aha:** The renderer intentionally shows the "last good state" during streaming errors. If the LLM produces malformed markup mid-stream, the renderer doesn't crash or show a blank screen — it shows the last successfully parsed version. This is critical for the streaming experience where the LLM is still generating and the markup is temporarily incomplete. The `ElementErrorBoundary` catches render errors and falls back to the previous render.

Source: `openui/packages/react-lang/src/Renderer.tsx` — main React component
Source: `openui/packages/react-lang/src/hooks/useOpenUIState.ts` — state management hook

## Renderer Component

```tsx
// Renderer.tsx
interface RendererProps {
  response: string | null;       // OpenUI Lang string (streaming), can be null
  library: ComponentLibrary;     // Available components
  isStreaming: boolean;          // Whether LLM is still generating
  onAction?: (plan: ActionPlan) => void;
  onStateUpdate?: (state: Record<string, any>) => void;
  initialState?: Record<string, any>;
  toolProvider?: Record<string, any> | McpClientLike | null;  // MCP client or function map
  queryLoader?: React.ReactNode;  // Data fetching component
  onError?: (errors: OpenUIError[]) => void;
}
```

### Rendering Pipeline

```mermaid
flowchart TD
    A["response string"] --> B["Streaming Parser<br/>createStreamParser()"]
    B --> C["ParseResult<br/>statements + errors"]
    C --> D["Materializer<br/>resolve refs, map props"]
    D --> E["ElementNode tree"]
    E --> F["renderDeep()<br/>recursive rendering"]
    F --> G{"Node type?"}
    G -->|Comp| H["Look up in library,<br/>evaluate props, render"]
    G -->|Str/Num/Bool| I["Render as text"]
    G -->|Arr| J["Render children"]
    G -->|Each| K["Evaluate template<br/>per item"]
    H --> L["React element"]
    I --> L
    J --> L
    K --> L
```

## useOpenUIState Hook

The hook manages the streaming parser, store, and query manager:

```typescript
function useOpenUIState(
  config: {
    response: string | null;
    library: ComponentLibrary;
    isStreaming: boolean;
    initialState?: Record<string, any>;
  },
  renderDeep: boolean
): RenderState {
  // Create streaming parser
  const parserRef = useRef(createStreamParser());
  parserRef.current.push(response);
  const result = parserRef.current.buildResult();

  // Create store
  const storeRef = useRef(new Store());
  if (initialState) {
    storeRef.current.initSafe(initialState);
  }

  // Create query manager
  const queryManagerRef = useRef(new QueryManager(queryLoader));

  // Subscribe to store and query changes
  const storeSnapshot = useSyncExternalStore(
    storeRef.current.subscribe,
    () => storeRef.current.getAll()
  );

  const querySnapshot = useSyncExternalStore(
    queryManagerRef.current.subscribe,
    () => queryManagerRef.current.getSnapshot()
  );

  return { statements: result.statements, errors: result.errors, storeSnapshot, querySnapshot };
}
```

`useSyncExternalStore` is React 18's hook for subscribing to external stores without re-rendering on every state change. The store only notifies React when values actually change (shallow comparison), preventing unnecessary re-renders.

## ElementErrorBoundary

```tsx
<ElementErrorBoundary>
  {renderDeep(elementNode)}
</ElementErrorBoundary>
```

The boundary catches rendering errors and falls back to the last good state:

```typescript
class ElementErrorBoundary extends React.Component {
  state = { hasError: false, lastChildren: null };

  static getDerivedStateFromError(error) {
    return { hasError: true };
  }

  render() {
    if (this.state.hasError) {
      return this.state.lastChildren;  // Show last good state
    }
    return this.props.children;
  }
}
```

**Aha:** The boundary doesn't show an error UI — it silently falls back to the last successful render. This is intentional for streaming: if the LLM writes `<Table data=$itemz>` but the signal is `$items`, the table disappears (last good state had no table) rather than showing an error. When the LLM corrects to `$items`, the table appears.

## Action Execution

```typescript
async function triggerAction(plan: ActionPlan) {
  for (const step of plan.steps) {
    switch (step.type) {
      case 'Set':
        store.set(step.target, step.value);
        break;
      case 'Reset':
        store.reset(step.target);
        break;
      case 'ToAssistant':
        await onMessage(step.value);
        break;
      case 'OpenUrl':
        window.open(step.value, '_blank');
        break;
      case 'Run':
        await executeSteps(step.steps);
        break;
    }
    // Mutation failures halt the action plan
    if (step.type === 'Mutation' && step.failed) {
      break;  // Halt on mutation failure
    }
  }
}
```

Actions execute sequentially. If a mutation step fails (e.g., a query mutation returns an error), the plan halts — remaining steps are not executed.

## QueryManager

Source: `openui/packages/lang-core/src/queryManager.ts`

The QueryManager handles data fetching:

- **Cache**: Stable key ordering via `JSON.stringify` with sorted keys for consistent cache keys
- **Deduplication**: In-flight requests are deduplicated — two queries for the same data share one fetch
- **Refetch on dependency change**: When a signal that a query depends on changes, the query refetches
- **Auto-refresh**: Configurable refresh intervals
- **Loading states**: `__openui_loading`, `__openui_refetching`, `__openui_errors` markers

```typescript
const querySnapshot = {
  data: { users: [...] },
  __openui_loading: { users: false },
  __openui_refetching: { users: true },
  __openui_errors: { users: 'Network error' },
};
```

See [Evaluator](05-evaluator.md) for how expressions produce action plans.
See [Component Library](07-component-library.md) for the available components.
See [OpenClaw Plugin](08-openclaw-plugin.md) for server-side tool integration.
