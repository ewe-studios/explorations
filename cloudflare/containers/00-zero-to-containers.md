# Zero to Cloudflare Containers: Complete Guide

**Last Updated:** 2026-04-05

---

## Table of Contents

1. [Introduction](#introduction)
2. [What are Cloudflare Containers?](#what-are-cloudflare-containers)
3. [Architecture](#architecture)
4. [Installation](#installation)
5. [Quick Start](#quick-start)
6. [Container Class API](#container-class-api)
7. [Lifecycle Management](#lifecycle-management)
8. [Outbound Interception](#outbound-interception)
9. [Load Balancing](#load-balancing)
10. [Advanced Patterns](#advanced-patterns)

---

## Introduction

Cloudflare Containers is a **container management library for Cloudflare Workers** that provides HTTP/WebSocket proxying, lifecycle hooks, outbound interception, and load balancing across container instances.

```bash
npm install @cloudflare/containers
```

### Key Features

| Feature | Description |
|---------|-------------|
| **HTTP/WebSocket Proxy** | Forward requests and bidirectional WebSocket streams |
| **Lifecycle Hooks** | onStart, onStop, onError, onActivityExpired callbacks |
| **Outbound Interception** | Control and proxy container outbound requests |
| **Activity Timeout** | Auto-shutdown after configurable inactivity |
| **Load Balancing** | getRandom() for distributing across N instances |
| **Port Management** | Default ports, multi-port routing, startAndWaitForPorts |

---

## What are Cloudflare Containers?

### The Problem

Running containers in serverless environments requires:

1. **Lifecycle Management** - Start/stop containers efficiently
2. **Request Routing** - Forward HTTP/WebSocket to correct container
3. **Resource Optimization** - Shut down idle containers
4. **Outbound Control** - Intercept/proxy container egress traffic
5. **Load Distribution** - Balance requests across multiple instances

### The Containers Solution

```
┌─────────────────────────────────────────────────────────────┐
│                    Cloudflare Workers                        │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Container Class (Durable Object)        │   │
│  │                                                      │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌───────────┐ │   │
│  │  │ HTTP Proxy   │  │ WebSocket    │  │ Outbound  │ │   │
│  │  │ Forwarding   │  │ Bidirectional│  │ Intercept │ │   │
│  │  └──────────────┘  └──────────────┘  └───────────┘ │   │
│  │                                                      │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌───────────┐ │   │
│  │  │ onStart()    │  │ Activity     │  │ Load      │ │   │
│  │  │ onStop()     │  │ Timeout      │  │ Balancing │ │   │
│  │  │ onError()    │  │ (sleepAfter) │  │           │ │   │
│  │  └──────────────┘  └──────────────┘  └───────────┘ │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                │
│                            ▼                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              Container Instance                      │   │
│  │         (Docker/OCI container at edge)               │   │
│  │                                                      │   │
│  │  Port 80 (Web)  Port 8080 (API)  Port 9000 (WS)     │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Container States

```typescript
type State = {
  lastChange: number;
} & (
  | {
      // Container starting or transitioning to healthy
      status: 'running' | 'stopping' | 'stopped' | 'healthy';
    }
  | {
      // Container exited with code
      status: 'stopped_with_code';
      exitCode?: number;
    }
);
```

---

## Architecture

### Container Class Hierarchy

```
Container (extends Durable Object)
├── Lifecycle Hooks
│   ├── onStart() - Called on start
│   ├── onStop() - Called on stop
│   ├── onError(error) - Called on errors
│   └── onActivityExpired() - Called when idle timeout fires
├── Fetch Methods
│   ├── fetch(request) - Forward HTTP/WebSocket
│   └── containerFetch(request, port?) - HTTP only
├── Lifecycle Methods
│   ├── start() - Start container
│   ├── startAndWaitForPorts() - Start + wait for readiness
│   ├── stop(signal) - Graceful shutdown
│   └── destroy() - Force kill (SIGKILL)
├── State Management
│   ├── getState() - Get current state
│   └── sleepAfter - Idle timeout configuration
└── Outbound Control
    ├── setOutboundByHost() - Route specific hosts
    ├── setOutboundHandler() - Set catch-all handler
    └── static outboundHandlers - Named handler registry
```

---

## Installation

### npm/yarn

```bash
npm install @cloudflare/containers
# or
yarn add @cloudflare/containers
# or
pnpm add @cloudflare/containers
```

### Development Setup

```bash
# Clone repository
git clone https://github.com/cloudflare/workers-sdk.git
cd workers-sdk/packages/containers

# Install dependencies
pnpm install

# Build
pnpm run build

# Test
pnpm run test
```

---

## Quick Start

### Basic Container

```typescript
import { Container, getContainer } from '@cloudflare/containers';

export class MyContainer extends Container {
  // Default port for container communication
  defaultPort = 8080;
  
  // Shutdown after 1 minute of inactivity
  sleepAfter = '1m';
}

export default {
  async fetch(request, env) {
    const pathname = new URL(request.url).pathname;
    
    // Route to specific container by name
    const container = env.MY_CONTAINER.getByName(pathname);
    return await container.fetch(request);
  },
};
```

### Load Balanced Containers

```typescript
import { Container, getRandom } from '@cloudflare/containers';

export class MyContainer extends Container {
  defaultPort = 8080;
  sleepAfter = '5m';
}

export default {
  async fetch(request, env) {
    const pathname = new URL(request.url).pathname;
    
    // Load balance across 5 container instances
    if (pathname.startsWith('/api/')) {
      const container = await getRandom(env.MY_CONTAINER, 5);
      return await container.fetch(request);
    }
    
    // Route to specific container
    const container = env.MY_CONTAINER.getByName('specific-id');
    return await container.fetch(request);
  },
};
```

### WebSocket Support

```typescript
import { Container, switchPort } from '@cloudflare/containers';

export class WebSocketContainer extends Container {
  defaultPort = 8080;
  sleepAfter = '30m';
}

// WebSocket connections are automatically proxied
// Use switchPort to target specific port
const response = await container.fetch(switchPort(request, 9000));
```

---

## Container Class API

### Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `defaultPort` | `number` | `undefined` | Default port for container communication |
| `requiredPorts` | `number[]` | `undefined` | Ports to check during startup |
| `sleepAfter` | `string \| number` | `'10m'` | Inactivity timeout before shutdown |
| `envVars` | `Record<string, string>` | `{}` | Environment variables for container |
| `entrypoint` | `string[]` | `undefined` | Override container entrypoint |
| `enableInternet` | `boolean` | `true` | Allow outbound internet access |
| `pingEndpoint` | `string` | `'ping'` | Health check endpoint for startup |

### Lifecycle Hooks

Override these methods to hook into container lifecycle:

```typescript
export class MyContainer extends Container {
  defaultPort = 8080;
  sleepAfter = '10m';

  // Called when container starts successfully
  override onStart(): void {
    console.log('Container started!');
    // Can restart if needed: this.startAndWaitForPorts();
  }

  // Called when container shuts down
  override onStop(): void {
    console.log('Container stopped!');
  }

  // Called on errors (default logs and throws)
  override onError(error: unknown): void {
    console.error('Container error:', error);
    throw error;
  }

  // Called when activity timeout expires
  override onActivityExpired(): void {
    console.log('Container activity expired');
    this.destroy(); // Stop container
  }
}
```

### Fetch Methods

#### fetch() - HTTP and WebSocket

```typescript
// Forward request to container (uses defaultPort)
const response = await container.fetch(request);

// Target specific port
import { switchPort } from '@cloudflare/containers';
const response = await container.fetch(switchPort(request, 8080));
```

**Important:** Use `fetch()` (not `containerFetch`) for WebSocket support.

#### containerFetch() - HTTP Only

```typescript
// Traditional fetch signature
const response = await container.containerFetch(request, 8080);

// Standard fetch-like signature
const response = await container.containerFetch('/api/data', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ query: 'example' }),
}, 8080);

// URL with port
const response = await container.containerFetch(
  'https://example.com/admin',
  { method: 'GET' },
  3000
);
```

**Note:** `containerFetch` does NOT support WebSockets.

### Lifecycle Methods

#### startAndWaitForPorts()

Start container and wait for ports to be ready:

```typescript
interface StartAndWaitForPortsOptions {
  startOptions?: {
    envVars?: Record<string, string>;
    entrypoint?: string[];
    enableInternet?: boolean;
  };
  ports?: number | number[];
  cancellationOptions?: {
    abort?: AbortSignal;
    instanceGetTimeoutMS?: number;
    portReadyTimeoutMS?: number;
    waitInterval?: number;
  };
}

// Start with default ports
await container.startAndWaitForPorts();

// Start with specific ports
await container.startAndWaitForPorts({
  ports: [8080, 9000],
  startOptions: {
    envVars: { NODE_ENV: 'production' },
    enableInternet: true,
  },
  cancellationOptions: {
    portReadyTimeoutMS: 30000,
    waitInterval: 100,
  }
});
```

#### start()

Start container without waiting:

```typescript
interface ContainerStartConfigOptions {
  envVars?: Record<string, string>;
  entrypoint?: string[];
  enableInternet?: boolean;
}

interface WaitOptions {
  portToCheck: number;
  signal?: AbortSignal;
  retries?: number;
  waitInterval?: number;
}

// Start without waiting
await container.start({
  envVars: { LOG_LEVEL: 'debug' },
  enableInternet: false,
});
```

#### stop() and destroy()

```typescript
// Graceful shutdown (sends SIGTERM)
await container.stop();

// Graceful shutdown with custom signal
await container.stop('SIGINT');

// Force kill (sends SIGKILL)
await container.destroy();
```

#### getState()

```typescript
const state = await container.getState();

// State types:
// { status: 'running', lastChange: timestamp }
// { status: 'healthy', lastChange: timestamp }
// { status: 'stopping', lastChange: timestamp }
// { status: 'stopped', lastChange: timestamp }
// { status: 'stopped_with_code', exitCode: 0, lastChange: timestamp }
```

#### renewActivityTimeout()

Manually extend container lifetime:

```typescript
async performBackgroundTask(): Promise<void> {
  // Do work...
  
  // Extend activity timeout
  await this.renewActivityTimeout();
  console.log('Container activity extended');
}
```

---

## Lifecycle Management

### Activity Timeout

The `sleepAfter` property controls how long a container stays alive without activity:

```typescript
export class TimeoutContainer extends Container {
  defaultPort = 8080;
  
  // Supported formats:
  sleepAfter = '30m';    // 30 minutes
  sleepAfter = '2h';     // 2 hours
  sleepAfter = '30s';    // 30 seconds
  sleepAfter = 60;       // 60 seconds (number)
  
  // Activity is automatically renewed on:
  // - fetch() calls
  // - containerFetch() calls
  // - WebSocket messages
  // - Manual renewActivityTimeout() calls
  
  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    
    // Example: trigger background task
    if (url.pathname === '/task') {
      await this.performBackgroundTask();
      return new Response(JSON.stringify({
        success: true,
        message: 'Background task executed',
        nextStop: `Container will shut down after ${this.sleepAfter} of inactivity`,
      }), { headers: { 'Content-Type': 'application/json' } });
    }
    
    // For all other requests, forward to container
    // This automatically renews activity timeout
    return this.containerFetch(request);
  }
}
```

### Multi-Port Container

```typescript
import { Container } from '@cloudflare/containers';

export class MultiPortContainer extends Container {
  // No defaultPort - route manually based on path
  
  async fetch(request: Request): Promise<Response> {
    const url = new URL(request.url);
    
    try {
      if (url.pathname.startsWith('/api')) {
        // API server on port 3000
        return await this.containerFetch(request, 3000);
      } else if (url.pathname.startsWith('/admin')) {
        // Admin interface on port 8080
        return await this.containerFetch(request, 8080);
      } else {
        // Public website on port 80
        return await this.containerFetch(request, 80);
      }
    } catch (error) {
      return new Response(
        `Error: ${error instanceof Error ? error.message : String(error)}`,
        { status: 500 }
      );
    }
  }
}
```

### Configuration Example

```typescript
import { Container } from '@cloudflare/containers';

export class ConfiguredContainer extends Container {
  // Network configuration
  defaultPort = 9000;
  sleepAfter = '2h';
  enableInternet = true;
  
  // Container configuration
  envVars = {
    NODE_ENV: 'production',
    LOG_LEVEL: 'info',
    APP_PORT: '9000',
  };
  
  entrypoint = ['node', 'server.js', '--config', 'production.json'];
  
  // Custom health check
  pingEndpoint = 'container/health';
}
```

---

## Outbound Interception

Intercept and control outbound requests from containers:

### Static Handlers

```typescript
import { Container, OutboundHandlerContext } from '@cloudflare/containers';

export class MyContainer extends Container {
  defaultPort = 8080;
  enableInternet = false; // Block by default
  
  // Catch-all handler
  static outbound = (req: Request) => {
    return new Response(`Hi ${req.url}, I can't handle you`);
  };
  
  // Per-host handlers (exact hostname match)
  static outboundByHost = {
    'google.com': (_req: Request, _env: unknown, ctx: OutboundHandlerContext) => {
      return new Response('hi ' + ctx.containerId + ' i am google');
    },
  };
  
  // Named handlers (selectable at runtime)
  static outboundHandlers = {
    async github(_req: Request, _env: unknown, _ctx: OutboundHandlerContext) {
      return new Response('i am github');
    },
  };
  
  // Route specific host to named handler
  async routeGithubThroughHandler(): Promise<void> {
    await this.setOutboundByHost('github.com', 'github');
  }
  
  // Set catch-all to named handler
  async makeEverythingUseGithubHandler(): Promise<void> {
    await this.setOutboundHandler('github');
  }
}
```

### Matching Order

```
1. Runtime setOutboundByHost() override
2. Static outboundByHost
3. Runtime setOutboundHandler() catch-all
4. Static outbound
5. Normal outbound (if enableInternet = true)
6. Blocked (if enableInternet = false)
```

### Runtime Overrides

```typescript
// Set host-specific override
await container.setOutboundByHost('api.example.com', 'customHandler');

// Replace all runtime host overrides
await container.setOutboundByHosts({
  'api.example.com': 'customHandler',
  'cdn.example.com': 'cdnHandler',
});

// Remove runtime host override
await container.removeOutboundByHost('api.example.com');

// Set catch-all handler
await container.setOutboundHandler('globalHandler');
```

### Use Cases

**Block external access:**
```typescript
export class IsolatedContainer extends Container {
  enableInternet = false;
  
  // Only allow specific hosts
  static outboundByHost = {
    'api.trusted-service.com': (req) => forwardToTrustedService(req),
  };
  
  // Block everything else
  static outbound = () => new Response('Blocked', { status: 403 });
}
```

**Proxy through Worker:**
```typescript
export class ProxiedContainer extends Container {
  enableInternet = false;
  
  static outbound = async (req: Request, env: Env) => {
    // Add authentication headers
    const headers = new Headers(req.headers);
    headers.set('X-Container-ID', env.CONTAINER_ID);
    headers.set('Authorization', `Bearer ${env.API_KEY}`);
    
    // Forward through Worker
    return await fetch(req.url, {
      method: req.method,
      headers,
      body: req.body,
    });
  };
}
```

---

## Load Balancing

### getRandom() - Load Balance Across Instances

```typescript
import { Container, getRandom } from '@cloudflare/containers';

export class MyContainer extends Container {
  defaultPort = 8080;
}

export default {
  async fetch(request: Request, env: any) {
    const url = new URL(request.url);
    
    // Load balance across 5 container instances
    if (url.pathname === '/api') {
      const containerInstance = await getRandom(env.MY_CONTAINER, 5);
      return containerInstance.fetch(request);
    }
    
    // Direct to specific container
    if (url.pathname.startsWith('/specific/')) {
      const id = url.pathname.split('/')[2] || 'default';
      const containerInstance = env.MY_CONTAINER.getByName(id);
      return containerInstance.fetch(request);
    }
    
    return new Response('Not found', { status: 404 });
  },
};
```

### getContainer() - Specific Instance

```typescript
import { getContainer } from '@cloudflare/containers';

// Get specific container by name
const container = getContainer(env.CONTAINER, 'unique-id');

// Get default singleton (name = "cf-singleton-container")
const container = getContainer(env.CONTAINER);
```

---

## Advanced Patterns

### Scheduling Tasks

```typescript
export class ScheduledContainer extends Container {
  defaultPort = 8080;
  sleepAfter = '1h';
  
  async performScheduledWork(): Promise<void> {
    // Schedule a task for 5 minutes from now
    await this.schedule(
      new Date(Date.now() + 5 * 60 * 1000),
      'cleanupTask',
      { type: 'cleanup' }
    );
    
    // Or schedule with delay in seconds
    await this.schedule(
      300, // 300 seconds = 5 minutes
      'backupTask',
      { type: 'backup' }
    );
  }
  
  // Scheduled task callback
  async cleanupTask(payload: { type: string }): Promise<void> {
    console.log('Running cleanup:', payload);
    // Cleanup logic...
  }
  
  async backupTask(payload: { type: string }): Promise<void> {
    console.log('Running backup:', payload);
    // Backup logic...
  }
}
```

### Standard Fetch API Syntax

```typescript
import { Container } from '@cloudflare/containers';

export class FetchStyleContainer extends Container {
  defaultPort = 8080;
  
  async customHandler(): Promise<Response> {
    try {
      // Standard fetch syntax
      const response = await this.containerFetch('/api/data', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ query: 'example' }),
      });
      
      // With explicit port
      const adminResponse = await this.containerFetch(
        'https://example.com/admin',
        { method: 'GET' },
        3000 // port
      );
      
      return response;
    } catch (error) {
      return new Response(
        `Error: ${error instanceof Error ? error.message : String(error)}`,
        { status: 500 }
      );
    }
  }
}
```

### Error Handling

```typescript
export class ResilientContainer extends Container {
  defaultPort = 8080;
  sleepAfter = '10m';
  
  override onError(error: unknown): void {
    console.error('Container error:', error);
    
    // Log to external service
    this.logError(error);
    
    // Auto-restart on certain errors
    if (this.isRestartableError(error)) {
      this.startAndWaitForPorts();
    } else {
      throw error;
    }
  }
  
  private isRestartableError(error: unknown): boolean {
    // Custom logic to determine if error is restartable
    return true;
  }
  
  private async logError(error: unknown): Promise<void> {
    // Log to external service
  }
}
```

---

## Production Deployment

### wrangler Configuration

```toml
# wrangler.toml
name = "my-container-app"
main = "src/index.ts"
compatibility_date = "2026-01-28"

[[durable_objects.bindings]]
name = "MY_CONTAINER"
class_name = "MyContainer"

[[migrations]]
tag = "v1"
new_classes = ["MyContainer"]
```

### Worker Entry Point

```typescript
import { MyContainer } from './container';

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const pathname = new URL(request.url).pathname;
    
    // Route to container
    const container = env.MY_CONTAINER.getByName(
      pathname.split('/')[1] || 'default'
    );
    
    return await container.fetch(request);
  },
};

export { MyContainer };
```

### Observability

```typescript
export class ObservableContainer extends Container {
  defaultPort = 8080;
  sleepAfter = '15m';
  
  override onStart(): void {
    console.log('Container started', {
      containerId: this.containerId,
      timestamp: Date.now(),
    });
  }
  
  override onStop(): void {
    console.log('Container stopped', {
      containerId: this.containerId,
      timestamp: Date.now(),
    });
  }
  
  override onError(error: unknown): void {
    console.error('Container error', {
      containerId: this.containerId,
      error: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined,
    });
    throw error;
  }
}
```

---

## Browser Support

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+
- Cloudflare Workers (all runtimes)

---

## Related Documents

- [Deep Dive: Container Internals](./01-container-internals-deep-dive.md)
- [Deep Dive: Outbound Interception Patterns](./02-outbound-interception-deep-dive.md)
- [Deep Dive: Load Balancing Strategies](./03-load-balancing-deep-dive.md)
- [Rust Revision](./rust-revision.md)
- [Production Guide](./production-grade.md)
