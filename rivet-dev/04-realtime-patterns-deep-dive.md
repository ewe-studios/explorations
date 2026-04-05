---
source: /home/darkvoid/Boxxed/@formulas/src.rivet-dev/rivetkit
repository: github.com/rivet-dev/rivetkit
explored_at: 2026-04-05
focus: WebSocket events, SSE, client subscriptions, broadcast patterns, optimistic updates
---

# Deep Dive: Realtime Patterns and Event Broadcasting

## Overview

This deep dive examines RivetKit's realtime capabilities - WebSocket event broadcasting, Server-Sent Events (SSE), client subscription patterns, and optimistic updates for responsive UIs.

## Architecture

```mermaid
sequenceDiagram
    participant Client1 as Client 1
    participant Client2 as Client 2
    participant WS as WebSocket Handler
    participant Actor as Actor Instance
    participant State as State Storage

    Client1->>WS: Connect WebSocket
    WS->>Actor: Subscribe to events
    Actor-->>Client1: Connection established

    Client2->>WS: Connect WebSocket
    WS->>Actor: Subscribe to events
    Actor-->>Client2: Connection established

    Client1->>Actor: action.call("update", data)
    Actor->>State: Persist state
    Actor->>Actor: Broadcast "updated" event
    Actor-->>Client1: Event (sender)
    Actor-->>Client2: Event (receiver)
    
    Client2->>Client2: Update UI
```

## WebSocket Event System

### WebSocket Handler

```typescript
// packages/core/src/realtime/websocket.ts

import { WebSocket } from "ws";
import { Registry } from "../registry";
import { ActorProxy } from "../actor";

export interface WebSocketMessage {
  type: "subscribe" | "unsubscribe" | "event" | "ack";
  actorType: string;
  actorKey: string;
  event?: string;
  data?: any;
  id?: string;
}

export class WebSocketHandler {
  private registry: Registry;
  private connections: Map<WebSocket, Set<string>> = new Map();

  constructor(registry: Registry) {
    this.registry = registry;
  }

  /**
   * Handle WebSocket connection
   */
  handleConnection(ws: WebSocket, request: any): void {
    console.log("Client connected");

    ws.on("message", (data) => {
      this.handleMessage(ws, JSON.parse(data.toString()));
    });

    ws.on("close", () => {
      this.handleDisconnect(ws);
    });

    ws.on("error", (error) => {
      console.error("WebSocket error:", error);
    });
  }

  /**
   * Handle incoming WebSocket message
   */
  private handleMessage(ws: WebSocket, message: WebSocketMessage): void {
    switch (message.type) {
      case "subscribe":
        this.handleSubscribe(ws, message);
        break;

      case "unsubscribe":
        this.handleUnsubscribe(ws, message);
        break;

      case "event":
        this.handleEvent(ws, message);
        break;
    }
  }

  /**
   * Subscribe client to actor events
   */
  private handleSubscribe(ws: WebSocket, message: WebSocketMessage): void {
    const { actorType, actorKey } = message;

    // Get actor proxy
    const actor = this.registry.getOrCreate(actorType, actorKey);

    // Subscribe to events
    const unsubscribe = actor.subscribe((event, data) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(
          JSON.stringify({
            type: "event",
            actorType,
            actorKey,
            event,
            data,
          })
        );
      } else {
        unsubscribe();
      }
    });

    // Track subscription
    const subscriptions = this.connections.get(ws) || new Set();
    subscriptions.add(`${actorType}:${actorKey}`);
    this.connections.set(ws, subscriptions);

    // Send ack
    ws.send(
      JSON.stringify({
        type: "ack",
        id: message.id,
        subscribed: true,
      })
    );
  }

  /**
   * Unsubscribe client from actor events
   */
  private handleUnsubscribe(ws: WebSocket, message: WebSocketMessage): void {
    const subscriptions = this.connections.get(ws);

    if (subscriptions) {
      subscriptions.delete(`${message.actorType}:${message.actorKey}`);
    }
  }

  /**
   * Handle event from client (e.g., action call)
   */
  private handleEvent(ws: WebSocket, message: WebSocketMessage): void {
    // Forward to actor
    // Actor will broadcast to all subscribers
  }

  /**
   * Handle client disconnect
   */
  private handleDisconnect(ws: WebSocket): void {
    // Clean up all subscriptions for this client
    this.connections.delete(ws);
  }
}
```

### Event Broadcasting

```typescript
// packages/core/src/realtime/broadcast.ts

export interface BroadcastEvent {
  event: string;
  data: any;
  timestamp: number;
  senderId?: string;
}

export class BroadcastManager {
  private subscribers: Map<string, Set<(event: BroadcastEvent) => void>> =
    new Map();

  /**
   * Subscribe to events for an actor
   */
  subscribe(
    actorId: string,
    handler: (event: BroadcastEvent) => void
  ): () => void {
    const actorSubscribers = this.subscribers.get(actorId) || new Set();
    actorSubscribers.add(handler);
    this.subscribers.set(actorId, actorSubscribers);

    // Return unsubscribe function
    return () => {
      actorSubscribers.delete(handler);
      if (actorSubscribers.size === 0) {
        this.subscribers.delete(actorId);
      }
    };
  }

  /**
   * Broadcast event to all subscribers
   */
  broadcast(
    actorId: string,
    event: string,
    data: any,
    senderId?: string
  ): void {
    const actorSubscribers = this.subscribers.get(actorId);

    if (!actorSubscribers) {
      return;
    }

    const broadcastEvent: BroadcastEvent = {
      event,
      data,
      timestamp: Date.now(),
      senderId,
    };

    for (const handler of actorSubscribers) {
      try {
        handler(broadcastEvent);
      } catch (error) {
        console.error("Error in event handler:", error);
      }
    }
  }

  /**
   * Get subscriber count for monitoring
   */
  getSubscriberCount(actorId: string): number {
    return this.subscribers.get(actorId)?.size || 0;
  }
}
```

## Server-Sent Events (SSE)

### SSE Handler

```typescript
// packages/core/src/realtime/sse.ts

import { Response } from "hono";

export interface SSEOptions {
  pingInterval?: number;
  retryTimeout?: number;
}

export class SSEHandler {
  private pingInterval: number;

  constructor(options?: SSEOptions) {
    this.pingInterval = options?.pingInterval || 30000;
  }

  /**
   * Create SSE response for actor events
   */
  createStream(registry: any, actorType: string, actorKey: string): Response {
    const encoder = new TextEncoder();
    let unsubscribe: (() => void) | null = null;

    const stream = new ReadableStream({
      start(controller) {
        // Send initial connection message
        controller.enqueue(
          encoder.encode(`data: ${JSON.stringify({ type: "connected" })}\n\n`)
        );

        // Get actor and subscribe to events
        const actor = registry.getOrCreate(actorType, actorKey);

        unsubscribe = actor.subscribe((event, data) => {
          const message = {
            type: "event",
            event,
            data,
            timestamp: Date.now(),
          };

          controller.enqueue(
            encoder.encode(`data: ${JSON.stringify(message)}\n\n`)
          );
        });

        // Send ping to keep connection alive
        const pingInterval = setInterval(() => {
          controller.enqueue(encoder.encode(": ping\n\n"));
        }, 30000);

        // Cleanup on close
        controller.enqueue({
          cancel() {
            if (unsubscribe) {
              unsubscribe();
            }
            clearInterval(pingInterval);
          },
        });
      },
    });

    return new Response(stream, {
      headers: {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
      },
    });
  }
}

// Usage with Hono
import { Hono } from "hono";
import { registry } from "./registry";

const app = new Hono();

app.get("/events/:type/:key", (c) => {
  const type = c.req.param("type");
  const key = c.req.param("key");

  const handler = new SSEHandler();
  return handler.createStream(registry, type, key);
});
```

### Client-Side SSE

```typescript
// packages/react/src/useSSE.ts

import { useEffect, useState } from "react";

export function useSSE<T>(
  url: string,
  options?: {
    enabled?: boolean;
    onEvent?: (event: string, data: any) => void;
  }
) {
  const [data, setData] = useState<T | null>(null);
  const [connected, setConnected] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (!options?.enabled) {
      return;
    }

    const eventSource = new EventSource(url);

    eventSource.onopen = () => {
      setConnected(true);
      setError(null);
    };

    eventSource.onerror = (err) => {
      setConnected(false);
      setError(err as Error);
      eventSource.close();
    };

    eventSource.onmessage = (event) => {
      const message = JSON.parse(event.data);

      if (message.type === "connected") {
        return;
      }

      if (message.type === "event") {
        setData(message.data);

        if (options?.onEvent) {
          options.onEvent(message.event, message.data);
        }
      }
    };

    return () => {
      eventSource.close();
    };
  }, [url, options?.enabled]);

  return { data, connected, error };
}
```

## React Integration

### useActor Hook

```typescript
// packages/react/src/useActor.ts

import { useState, useEffect, useCallback, useRef } from "react";
import { createClient, ActorClient } from "rivetkit/client";

export interface UseActorOptions<TState> {
  /**
   * Initial state for optimistic updates
   */
  initialState?: TState;

  /**
   * Enable realtime subscriptions
   */
  realtime?: boolean;

  /**
   * WebSocket server URL
   */
  wsUrl?: string;

  /**
   * Retry connection on error
   */
  retry?: boolean;

  /**
   * Retry delay in milliseconds
   */
  retryDelay?: number;
}

export function useActor<TActor extends ActorClient<any>>(
  type: string,
  key: string,
  options?: UseActorOptions<any>
) {
  const [state, setState] = useState(options?.initialState || {});
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [actions, setActions] = useState<any>({});
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    // Create client
    const client = createClient(window.RIVET_URL);
    const actor = client[type].get(key) as any;

    // Load initial state
    actor
      .getState()
      .then((s: any) => {
        setState(s);
        setIsLoading(false);
      })
      .catch((err: Error) => {
        setError(err);
        setIsLoading(false);
      });

    // Create action proxies
    const actionProxies: any = {};

    for (const actionName of actor.getActions()) {
      actionProxies[actionName] = async (...args: any[]) => {
        try {
          return await actor[actionName](...args);
        } catch (err) {
          setError(err as Error);
          throw err;
        }
      };
    }

    setActions(actionProxies);

    // Setup realtime connection
    if (options?.realtime && options?.wsUrl) {
      connectRealtime(options.wsUrl, type, key, actor, setState);
    }

    return () => {
      // Cleanup
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [type, key, options?.wsUrl]);

  return { state, actions, isLoading, error };
}

function connectRealtime(
  wsUrl: string,
  type: string,
  key: string,
  actor: any,
  setState: any
) {
  const ws = new WebSocket(wsUrl);

  ws.onopen = () => {
    // Subscribe to actor events
    ws.send(
      JSON.stringify({
        type: "subscribe",
        actorType: type,
        actorKey: key,
      })
    );
  };

  ws.onmessage = (event) => {
    const message = JSON.parse(event.data);

    if (message.type === "event") {
      setState((prev: any) => ({
        ...prev,
        ...message.data,
      }));
    }
  };

  ws.onerror = (error) => {
    console.error("WebSocket error:", error);
  };
}
```

### Optimistic Updates

```typescript
// packages/react/src/optimistic.ts

import { useState, useCallback } from "react";

export interface OptimisticUpdate<T> {
  /**
   * Optimistic state
   */
  optimisticState: T;

  /**
   * Apply update optimistically
   */
  applyOptimistic: (updater: (state: T) => T) => void;

  /**
   * Revert to previous state
   */
  revert: () => void;

  /**
   * Confirm update (sync with server)
   */
  confirm: (serverState: T) => void;
}

export function useOptimistic<T>(
  initialState: T
): OptimisticUpdate<T> & { state: T } {
  const [state, setState] = useState(initialState);
  const [optimisticState, setOptimisticState] = useState(initialState);
  const [pending, setPending] = useState(false);
  const previousState = useRef<T>(initialState);

  const applyOptimistic = useCallback(
    (updater: (state: T) => T) => {
      previousState.current = state;
      const updated = updater(state);
      setOptimisticState(updated);
      setPending(true);
    },
    [state]
  );

  const revert = useCallback(() => {
    setOptimisticState(previousState.current);
    setPending(false);
  }, []);

  const confirm = useCallback((serverState: T) => {
    setState(serverState);
    setOptimisticState(serverState);
    setPending(false);
  }, []);

  return {
    state: pending ? optimisticState : state,
    optimisticState,
    applyOptimistic,
    revert,
    confirm,
  };
}

// Usage example
function ChatMessage({ actor }: { actor: any }) {
  const { state, applyOptimistic, revert, confirm } = useOptimistic({
    messages: [],
  });

  const sendMessage = useCallback(
    async (text: string) => {
      const newMessage = {
        id: crypto.randomUUID(),
        text,
        timestamp: Date.now(),
        pending: true,
      };

      // Apply optimistic update
      applyOptimistic((s) => ({
        ...s,
        messages: [...s.messages, newMessage],
      }));

      try {
        // Call server
        const result = await actor.sendMessage(text);

        // Confirm with server response
        confirm(result);
      } catch (error) {
        // Revert on error
        revert();
        throw error;
      }
    },
    [applyOptimistic, revert, confirm]
  );

  return (
    <div>
      {state.messages.map((msg) => (
        <div key={msg.id} className={msg.pending ? "pending" : ""}>
          {msg.text}
        </div>
      ))}
      <button onClick={() => sendMessage("Hello")}>Send</button>
    </div>
  );
}
```

## Event Patterns

### Debounced Broadcasting

```typescript
// Debounce events to avoid flooding clients

const debouncedActor = actor({
  state: { value: "" },

  actions: {
    update: (ctx, value: string) => {
      ctx.state.value = value;

      // Debounce broadcast
      ctx.debounceBroadcast("updated", { value }, 300);
    },
  },
});

// Implementation
class ActorContext {
  private debounceTimers: Map<string, NodeJS.Timeout> = new Map();

  debounceBroadcast(event: string, data: any, delay: number): void {
    // Clear existing timer
    const existingTimer = this.debounceTimers.get(event);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }

    // Set new timer
    const timer = setTimeout(() => {
      this.broadcast(event, data);
      this.debounceTimers.delete(event);
    }, delay);

    this.debounceTimers.set(event, timer);
  }
}
```

### Throttled Broadcasting

```typescript
// Throttle to max N broadcasts per second

const throttledActor = actor({
  state: { items: [] },

  actions: {
    addItem: (ctx, item: any) => {
      ctx.state.items.push(item);

      // Throttle broadcast
      ctx.throttleBroadcast("itemsUpdated", { items: ctx.state.items }, 1000);
    },
  },
});

// Implementation
class ActorContext {
  private lastBroadcast: Map<string, number> = new Map();
  private throttleInterval = 1000; // 1 second

  throttleBroadcast(event: string, data: any, interval: number): void {
    const now = Date.now();
    const lastTime = this.lastBroadcast.get(event) || 0;

    if (now - lastTime >= interval) {
      this.broadcast(event, data);
      this.lastBroadcast.set(event, now);
    } else {
      // Queue for later
      setTimeout(() => {
        this.broadcast(event, data);
        this.lastBroadcast.set(event, Date.now());
      }, interval - (now - lastTime));
    }
  }
}
```

### Event Batching

```typescript
// Batch multiple events into single broadcast

const batchedActor = actor({
  state: { updates: [] },

  actions: {
    update1: (ctx, data: any) => {
      ctx.state.updates.push({ type: "update1", data });
      ctx.batchBroadcast(100); // Batch for 100ms
    },

    update2: (ctx, data: any) => {
      ctx.state.updates.push({ type: "update2", data });
      ctx.batchBroadcast(100);
    },
  },
});

// Implementation
class ActorContext {
  private batchTimer: NodeJS.Timeout | null = null;
  private pendingEvents: any[] = [];

  batchBroadcast(delay: number): void {
    if (this.batchTimer) {
      clearTimeout(this.batchTimer);
    }

    this.batchTimer = setTimeout(() => {
      this.broadcast("batch", { updates: this.pendingEvents });
      this.pendingEvents = [];
      this.batchTimer = null;
    }, delay);
  }
}
```

## Conclusion

RivetKit's realtime system provides:

1. **WebSocket Events**: Bidirectional realtime communication
2. **SSE Support**: Simpler unidirectional streaming
3. **React Integration**: useActor hook with automatic subscriptions
4. **Optimistic Updates**: Responsive UI before server confirmation
5. **Event Throttling**: Prevent client flooding
6. **Event Batching**: Efficient update delivery
7. **Connection Management**: Automatic cleanup on disconnect
