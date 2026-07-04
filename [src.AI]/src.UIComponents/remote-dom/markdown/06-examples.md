# Examples: Kitchen-Sink

The kitchen-sink example demonstrates all supported frameworks and transport mechanisms.

Source: `examples/kitchen-sink/app/`

## Host App

Source: `app/host.tsx`, `app/host/components.tsx`, `app/host/state.ts`

The host:
1. Creates a `RemoteReceiver` to receive mutations
2. Renders received nodes into DOM elements
3. Manages state for which example is active

## Remote Context

Source: `app/remote/render.ts`, `app/remote/elements.ts`

The remote context:
1. Defines remote elements (custom elements)
2. Renders UI using the chosen framework
3. Sends mutations to the host

## Framework Examples

| File | Framework | Description |
|------|-----------|-------------|
| `react.tsx` | React | Standard React remote UI |
| `react-remote-ui.tsx` | React | Remote UI pattern with custom elements |
| `preact.tsx` | Preact | Lightweight React alternative |
| `vue.ts` | Vue | Vue.js integration |
| `svelte.ts` | Svelte | Svelte integration |
| `vanilla.ts` | Vanilla JS | No framework, direct element creation |
| `htm.ts` | HTM | Hyperscript Tagged Markup |

## Transport Sandboxes

| Sandbox | Transport | Description |
|---------|-----------|-------------|
| `iframe/sandbox.ts` | `postMessage` | Remote runs in iframe, communicates via postMessage |
| `worker/sandbox.ts` | `postMessage` | Remote runs in Web Worker, communicates via postMessage |

Aha: The sandbox pattern isolates the remote context completely. The iframe sandbox provides DOM access but isolated JS context. The worker sandbox provides isolated JS but no DOM (requires the polyfill). Both use `postMessage` for the transport layer.

## Cross-References

- [Overview](00-overview.md) — Package summary
- [Architecture](07-architecture.md) — How host and remote communicate
