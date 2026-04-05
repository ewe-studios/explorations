# Network Simulation Deep Dive

**Source:** `/home/darkvoid/Boxxed/@dev/repo-expolorations/opencontainer`

**Related Documents:**
- [Zero to OpenContainer Developer](00-zero-to-opencontainer-developer.md) - Overview of network simulation features
- [Process Management Deep Dive](02-process-management-deep-dive.md) - Process executor architecture
- [Shell Engine Deep Dive](03-shell-engine-deep-dive.md) - Shell command implementation

---

## Table of Contents

1. [Overview](#overview)
2. [Network Architecture](#network-architecture)
3. [HTTP Interceptor](#http-interceptor)
4. [Network Module Mocking](#network-module-mocking)
5. [Fetch Mocking](#fetch-mocking)
6. [HTTP Server Emulation](#http-server-emulation)
7. [WebSocket Simulation](#websocket-simulation)
8. [Network Types and Interfaces](#network-types-and-interfaces)
9. [Use Cases](#use-cases)
10. [Production Patterns](#production-patterns)

---

## 1. Overview

OpenContainer's network simulation layer provides a comprehensive mocking infrastructure that enables:

- **Complete network isolation** - No real network requests leave the browser sandbox
- **Deterministic testing** - Reproducible network conditions for testing
- **Offline development** - Full functionality without network connectivity
- **API mocking** - Flexible request/response interception and mocking
- **Network condition simulation** - Latency, throttling, and failure injection

### Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                    Application Code (Sandbox)                    │
│                                                                  │
│   fetch('/api/users')  │  http.request()  │  new WebSocket()    │
│           │            │         │         │         │           │
│           ▼            │         ▼         │         ▼           │
├─────────────────────────────────────────────────────────────────┤
│                   Network Simulation Layer                       │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │    Fetch     │  │   HTTP/HTTPS │  │  WebSocket   │          │
│  │   Interceptor│  │   Mock       │  │  Emulator    │          │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘          │
│         │                 │                  │                   │
│         └─────────────────┼──────────────────┘                   │
│                           ▼                                      │
│                ┌────────────────────┐                            │
│                │  Route Registry    │                            │
│                │  + Handler Pool    │                            │
│                └────────────────────┘                            │
└─────────────────────────────────────────────────────────────────┘
                            │
                            ▼
              (No real network - all mocked)
```

---

## 2. Network Architecture

### 2.1 Network Interceptor Design

The network interceptor is the core component that intercepts all network-related operations and routes them to appropriate mock handlers.

#### Core Design Principles

1. **Transparency** - Application code cannot detect it's running in a mocked environment
2. **Flexibility** - Support multiple mocking strategies (static, dynamic, passthrough)
3. **Performance** - Minimal overhead on request/response processing
4. **Isolation** - Each container has its own network mock context

#### Interceptor Architecture

```typescript
// packages/core/src/network/interceptor.ts

/**
 * NetworkInterceptor - Core request interception engine
 * 
 * Intercepts all network requests and routes them to registered handlers.
 * Supports pattern matching, async handlers, and response transformation.
 */
export class NetworkInterceptor {
  private enabled: boolean = false;
  private routes: RouteEntry[] = [];
  private defaultHandler?: NetworkHandler;
  private globalContext: InterceptorContext;

  constructor(context?: InterceptorContext) {
    this.globalContext = context || new InterceptorContext();
  }

  /**
   * Register a route handler
   */
  intercept(
    method: HttpMethod | HttpMethod[] | '*',
    pathPattern: string | RegExp,
    handler: NetworkHandler
  ): RouteRegistration {
    const patterns = Array.isArray(method) ? method : [method];
    
    const entry: RouteEntry = {
      id: generateRouteId(),
      methods: patterns,
      pattern: normalizePathPattern(pathPattern),
      handler,
      priority: this.routes.length,
      createdAt: Date.now(),
    };

    this.routes.push(entry);
    
    // Sort by priority (lower number = higher priority)
    this.routes.sort((a, b) => a.priority - b.priority);

    return {
      unregister: () => this.removeRoute(entry.id),
      entry,
    };
  }

  /**
   * Process an incoming request
   */
  async processRequest(request: NetworkRequest): Promise<NetworkResponse> {
    if (!this.enabled) {
      return this.passthrough(request);
    }

    // Find matching route
    const route = this.findMatchingRoute(request);
    
    if (route) {
      try {
        const context = this.createContext(request, route);
        return await route.handler(request, context);
      } catch (error) {
        return this.createErrorResponse(request, error);
      }
    }

    // Use default handler if registered
    if (this.defaultHandler) {
      return await this.defaultHandler(request, this.globalContext);
    }

    // No route matched - return 404
    return this.createNotFoundResponse(request);
  }

  /**
   * Find the first matching route for a request
   */
  private findMatchingRoute(request: NetworkRequest): RouteEntry | null {
    for (const route of this.routes) {
      if (this.routeMatches(route, request)) {
        return route;
      }
    }
    return null;
  }

  /**
   * Check if a route matches a request
   */
  private routeMatches(route: RouteEntry, request: NetworkRequest): boolean {
    // Check method
    const methodMatch = route.methods.includes('*') || 
                        route.methods.includes(request.method);
    
    if (!methodMatch) return false;

    // Check path pattern
    const url = new URL(request.url, 'http://localhost');
    const path = url.pathname + url.search;
    
    if (route.pattern instanceof RegExp) {
      return route.pattern.test(path);
    } else {
      return route.pattern === path || this.pathMatches(route.pattern, path);
    }
  }

  /**
   * Enable interception
   */
  enable(): void {
    this.enabled = true;
    this.installGlobalMocks();
  }

  /**
   * Disable interception
   */
  disable(): void {
    this.enabled = false;
    this.restoreGlobalMocks();
  }

  /**
   * Install global fetch/XMLHttpRequest mocks
   */
  private installGlobalMocks(): void {
    // Store original implementations
    const originalFetch = globalThis.fetch;
    const originalXHROpen = XMLHttpRequest.prototype.open;
    const originalXHRSend = XMLHttpRequest.prototype.send;

    // Mock fetch
    globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      const request = this.normalizeFetchRequest(input, init);
      const response = await this.processRequest(request);
      return this.toFetchResponse(response);
    };

    // Mock XMLHttpRequest
    this.mockXMLHttpRequest(originalXHROpen, originalXHRSend);
  }

  /**
   * Restore original global implementations
   */
  private restoreGlobalMocks(): void {
    if (this.globalContext.originalFetch) {
      globalThis.fetch = this.globalContext.originalFetch;
    }
    // Restore XHR...
  }
}
```

### 2.2 Request/Response Flow

```
┌────────────────────────────────────────────────────────────────────┐
│                        Request Flow                                 │
└────────────────────────────────────────────────────────────────────┘

┌─────────────┐     ┌──────────────┐     ┌───────────────────────┐
│ Application │────▶│ Global Mock  │────▶│ NetworkInterceptor    │
│   Code      │     │  (fetch)     │     │   .processRequest()   │
└─────────────┘     └──────────────┘     └───────────┬───────────┘
                                                     │
                    ┌────────────────────────────────┼────────────────────────────────┐
                    │                                │                                │
                    ▼                                ▼                                │
          ┌─────────────────┐              ┌─────────────────┐                        │
          │ Route Matching  │─────────────▶│ Handler Found   │                        │
          │                 │    YES        │ Execute Handler │                        │
          └────────┬────────┘              └────────┬────────┘                        │
                   │ NO                             │                                 │
                   ▼                                │                                 │
          ┌─────────────────┐              ┌────────▼────────┐                        │
          │ Default Handler │              │ Transform Result│                        │
          │    or 404       │              │ to Response     │                        │
          └────────┬────────┘              └────────┬────────┘                        │
                   │                                │                                 │
                   └────────────────┬───────────────┘                                 │
                                    │                                                 │
                                    ▼                                                 │
                          ┌─────────────────┐                                        │
                          │ NetworkResponse │                                        │
                          └────────┬────────┘                                        │
                                   │                                                  │
                                   ▼                                                  │
┌─────────────┐     ┌──────────────┴──────┐     ┌─────────────────┐                 │
│ Application │◀────│  Response Adapter   │◀────│ Response Builder│                 │
│   Code      │     │  (fetch Response)   │     │                 │                 │
└─────────────┘     └─────────────────────┘     └─────────────────┘                 │
```

### 2.3 Sandboxing Strategy

Each container gets its own isolated network context:

```typescript
// packages/core/src/network/sandbox.ts

export class NetworkSandbox {
  private interceptor: NetworkInterceptor;
  private context: SandboxContext;
  private requestLog: RequestLogEntry[] = [];

  constructor(containerId: string) {
    this.context = new SandboxContext(containerId);
    this.interceptor = new NetworkInterceptor(this.context);
  }

  /**
   * Create isolated mocking scope
   */
  createScope(options: SandboxOptions): NetworkScope {
    const scope = new NetworkScope(this.context, options);
    
    // Scope inherits parent routes but can add overrides
    scope.interceptor = this.interceptor;
    
    return scope;
  }

  /**
   * Record request for inspection
   */
  recordRequest(entry: RequestLogEntry): void {
    this.requestLog.push({
      ...entry,
      timestamp: Date.now(),
      containerId: this.context.containerId,
    });
  }

  /**
   * Get all recorded requests
   */
  getRequests(filter?: RequestFilter): RequestLogEntry[] {
    if (!filter) return this.requestLog;
    
    return this.requestLog.filter(entry => {
      if (filter.method && entry.method !== filter.method) return false;
      if (filter.urlPattern && !filter.urlPattern.test(entry.url)) return false;
      if (filter.startTime && entry.timestamp < filter.startTime) return false;
      if (filter.endTime && entry.timestamp > filter.endTime) return false;
      return true;
    });
  }

  /**
   * Clear request log
   */
  clearLog(): void {
    this.requestLog = [];
  }
}
```

### 2.4 Thread Isolation (Web Worker)

Network interception happens in the Web Worker where the sandboxed code executes:

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Main Thread                                  │
│                                                                      │
│   ┌─────────────────┐                                               │
│   │  ContainerAPI   │                                               │
│   └────────┬────────┘                                               │
│            │ postMessage                                             │
└────────────┼─────────────────────────────────────────────────────────┘
             │
             │ { type: 'NETWORK_REQUEST', request: {...} }
             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                         Web Worker                                   │
│                                                                      │
│   ┌─────────────────┐     ┌──────────────────┐                      │
│   │ NetworkSandbox  │────▶│ Interceptor      │                      │
│   └─────────────────┘     └──────────────────┘                      │
│                                    │                                 │
│   ┌─────────────────┐              │                                 │
│   │ Application     │──────────────┘                                 │
│   │ Code (Sandbox)  │                                                 │
│   └─────────────────┘                                                 │
└─────────────────────────────────────────────────────────────────────┘
```

Worker bridge implementation:

```typescript
// packages/core/src/worker/network-bridge.ts

export class NetworkBridge {
  private sandbox: NetworkSandbox;
  private messageHandlers: Map<string, MessageHandler> = new Map();

  constructor(sandbox: NetworkSandbox) {
    this.sandbox = sandbox;
    this.setupMessageHandlers();
  }

  private setupMessageHandlers(): void {
    // Handle network configuration from main thread
    this.messageHandlers.set('NETWORK_CONFIG', (data) => {
      const { routes, options } = data;
      
      for (const route of routes) {
        this.sandbox.interceptor.intercept(
          route.method,
          route.pattern,
          createHandler(route.handlerId)
        );
      }
      
      if (options.autoEnable) {
        this.sandbox.interceptor.enable();
      }
    });

    // Handle request inspection from main thread
    this.messageHandlers.set('GET_REQUESTS', (data) => {
      const requests = this.sandbox.getRequests(data.filter);
      self.postMessage({
        type: 'REQUESTS_RESULT',
        requests,
        requestId: data.requestId,
      });
    });

    // Handle mock updates
    this.messageHandlers.set('UPDATE_MOCK', (data) => {
      const { routeId, handler } = data;
      this.sandbox.interceptor.updateHandler(routeId, handler);
    });
  }
}
```

---

## 3. HTTP Interceptor

### 3.1 HttpInterceptor Class

The `HttpInterceptor` is the primary class for intercepting and mocking HTTP requests.

```typescript
// packages/core/src/network/interceptor.ts

import type { 
  NetworkRequest, 
  NetworkResponse, 
  NetworkHandler,
  HttpMethod,
  RouteEntry,
  RouteRegistration,
  InterceptorContext,
} from './types';

export class HttpInterceptor {
  private enabled: boolean = false;
  private routes: Map<string, RouteEntry> = new Map();
  private routeOrder: string[] = [];
  private defaultHandler?: NetworkHandler;
  private context: InterceptorContext;
  private requestLog: RequestLog[] = [];
  private maxLogSize: number = 1000;

  constructor(context?: Partial<InterceptorContext>) {
    this.context = {
      containerId: context?.containerId || 'default',
      baseUrl: context?.baseUrl || 'http://localhost',
      timeout: context?.timeout || 30000,
      originalFetch: globalThis.fetch,
      originalXHR: {
        open: XMLHttpRequest.prototype.open,
        send: XMLHttpRequest.prototype.send,
      },
    };
  }

  // ==================== Route Registration ====================

  /**
   * Register a GET route
   */
  get(path: string | RegExp, handler: NetworkHandler): RouteRegistration {
    return this.intercept('GET', path, handler);
  }

  /**
   * Register a POST route
   */
  post(path: string | RegExp, handler: NetworkHandler): RouteRegistration {
    return this.intercept('POST', path, handler);
  }

  /**
   * Register a PUT route
   */
  put(path: string | RegExp, handler: NetworkHandler): RouteRegistration {
    return this.intercept('PUT', path, handler);
  }

  /**
   * Register a DELETE route
   */
  delete(path: string | RegExp, handler: NetworkHandler): RouteRegistration {
    return this.intercept('DELETE', path, handler);
  }

  /**
   * Register a route with any method
   */
  intercept(
    method: HttpMethod | HttpMethod[] | '*',
    pathPattern: string | RegExp,
    handler: NetworkHandler
  ): RouteRegistration {
    const id = this.generateRouteId(method, pathPattern);
    
    const entry: RouteEntry = {
      id,
      methods: this.normalizeMethods(method),
      pattern: this.normalizePattern(pathPattern),
      handler,
      priority: this.routeOrder.length,
      createdAt: Date.now(),
      hitCount: 0,
    };

    this.routes.set(id, entry);
    this.routeOrder.push(id);

    // Re-enable if currently enabled to pick up new route
    if (this.enabled) {
      this.installMocks();
    }

    return {
      id,
      entry,
      unregister: () => this.removeRoute(id),
      update: (newHandler: NetworkHandler) => {
        entry.handler = newHandler;
      },
    };
  }

  /**
   * Remove a route by ID
   */
  removeRoute(id: string): boolean {
    const existed = this.routes.delete(id);
    if (existed) {
      const index = this.routeOrder.indexOf(id);
      if (index > -1) {
        this.routeOrder.splice(index, 1);
      }
    }
    return existed;
  }

  /**
   * Clear all routes
   */
  clearRoutes(): void {
    this.routes.clear();
    this.routeOrder = [];
  }

  /**
   * Set default handler for unmatched requests
   */
  setDefaultHandler(handler: NetworkHandler): void {
    this.defaultHandler = handler;
  }

  // ==================== Request Processing ====================

  /**
   * Process an incoming request
   */
  async handleRequest(request: NetworkRequest): Promise<NetworkResponse> {
    // Log the request
    this.logRequest(request);

    if (!this.enabled) {
      return this.passthrough(request);
    }

    // Find matching route
    const match = this.findRoute(request);
    
    if (match) {
      const { entry } = match;
      entry.hitCount++;

      try {
        const context = this.createContext(request, entry);
        const response = await entry.handler(request, context);
        return this.normalizeResponse(response, request);
      } catch (error) {
        return this.createErrorResponse(request, error);
      }
    }

    // Use default handler
    if (this.defaultHandler) {
      return await this.defaultHandler(request, this.context);
    }

    // No match - return 404
    return this.createNotFoundResponse(request);
  }

  /**
   * Find a matching route
   */
  private findRoute(request: NetworkRequest): { entry: RouteEntry; params: Record<string, string> } | null {
    for (const id of this.routeOrder) {
      const entry = this.routes.get(id);
      if (!entry) continue;

      const match = this.matchRoute(entry, request);
      if (match) {
        return { entry, params: match.params };
      }
    }
    return null;
  }

  /**
   * Match a route against a request
   */
  private matchRoute(entry: RouteEntry, request: NetworkRequest): { params: Record<string, string> } | null {
    // Check method
    if (!entry.methods.includes('*') && !entry.methods.includes(request.method)) {
      return null;
    }

    // Parse URL
    const url = new URL(request.url, this.context.baseUrl);
    const path = url.pathname + url.search;

    // Check pattern
    if (entry.pattern instanceof RegExp) {
      const matches = entry.pattern.exec(path);
      if (matches) {
        return { params: matches.groups || {} };
      }
      return null;
    }

    // String pattern with path parameters
    const paramMatch = this.matchPathPattern(entry.pattern, path);
    if (paramMatch) {
      return { params: paramMatch };
    }

    return null;
  }

  /**
   * Match path pattern with parameters (e.g., /users/:id)
   */
  private matchPathPattern(pattern: string, path: string): Record<string, string> | null {
    const patternParts = pattern.split('/');
    const pathParts = path.split('/');

    if (patternParts.length !== pathParts.length) {
      return null;
    }

    const params: Record<string, string> = {};

    for (let i = 0; i < patternParts.length; i++) {
      const patternPart = patternParts[i];
      const pathPart = pathParts[i];

      if (patternPart.startsWith(':')) {
        // Path parameter
        params[patternPart.slice(1)] = pathPart;
      } else if (patternPart !== pathPart) {
        return null;
      }
    }

    return params;
  }

  // ==================== Response Creation ====================

  /**
   * Create a normalised response
   */
  private normalizeResponse(
    response: NetworkResponse | Response | string | any,
    request: NetworkRequest
  ): NetworkResponse {
    // Handle Response object
    if (response instanceof Response) {
      return {
        status: response.status,
        statusText: response.statusText,
        headers: Object.fromEntries(response.headers.entries()),
        body: response.body,
        url: request.url,
      };
    }

    // Handle plain object
    if (typeof response === 'object' && response !== null) {
      return {
        status: response.status || 200,
        statusText: response.statusText || 'OK',
        headers: response.headers || {},
        body: this.normalizeBody(response.body),
        url: request.url,
      };
    }

    // Handle string (assume JSON)
    if (typeof response === 'string') {
      return {
        status: 200,
        statusText: 'OK',
        headers: { 'Content-Type': 'application/json' },
        body: this.stringToStream(response),
        url: request.url,
      };
    }

    throw new Error('Invalid response type');
  }

  /**
   * Create error response
   */
  private createErrorResponse(request: NetworkRequest, error: any): NetworkResponse {
    const status = error.status || 500;
    const body = JSON.stringify({
      error: error.message || 'Internal Server Error',
      stack: error.stack,
    });

    return {
      status,
      statusText: this.getStatusText(status),
      headers: { 'Content-Type': 'application/json' },
      body: this.stringToStream(body),
      url: request.url,
    };
  }

  /**
   * Create 404 response
   */
  private createNotFoundResponse(request: NetworkRequest): NetworkResponse {
    return {
      status: 404,
      statusText: 'Not Found',
      headers: { 'Content-Type': 'application/json' },
      body: this.stringToStream(JSON.stringify({
        error: 'No mock handler found for request',
        method: request.method,
        url: request.url,
      })),
      url: request.url,
    };
  }

  // ==================== Global Mock Installation ====================

  /**
   * Enable interception by installing global mocks
   */
  enable(): void {
    if (this.enabled) return;
    
    this.enabled = true;
    this.installMocks();
  }

  /**
   * Disable interception and restore originals
   */
  disable(): void {
    if (!this.enabled) return;
    
    this.enabled = false;
    this.restoreMocks();
  }

  /**
   * Install global fetch and XHR mocks
   */
  private installMocks(): void {
    // Mock fetch
    const originalFetch = this.context.originalFetch;
    const self = this;

    globalThis.fetch = async function(input: RequestInfo | URL, init?: RequestInit) {
      const request = self.createFetchRequest(input, init);
      const response = await self.handleRequest(request);
      return self.toFetchResponse(response);
    };

    // Mock XMLHttpRequest
    this.installXHRMock();
  }

  /**
   * Restore original implementations
   */
  private restoreMocks(): void {
    if (this.context.originalFetch) {
      globalThis.fetch = this.context.originalFetch;
    }
    this.restoreXHRMock();
  }

  // ==================== Utility Methods ====================

  /**
   * Generate unique route ID
   */
  private generateRouteId(method: HttpMethod | HttpMethod[] | '*', pattern: string | RegExp): string {
    const methods = this.normalizeMethods(method).join('-');
    const patternStr = pattern instanceof RegExp ? pattern.source : pattern;
    return `${methods}:${patternStr}`;
  }

  /**
   * Normalize method to array
   */
  private normalizeMethods(method: HttpMethod | HttpMethod[] | '*'): HttpMethod[] {
    if (method === '*') return ['*'];
    return Array.isArray(method) ? method : [method];
  }

  /**
   * Normalize pattern
   */
  private normalizePattern(pattern: string | RegExp): RegExp | string {
    if (pattern instanceof RegExp) return pattern;
    
    // Convert string pattern to handle query params
    if (pattern.includes('*')) {
      return new RegExp(`^${pattern.replace(/\*/g, '.*')}$`);
    }
    
    return pattern;
  }

  /**
   * Log request for inspection
   */
  private logRequest(request: NetworkRequest): void {
    this.requestLog.push({
      ...request,
      timestamp: Date.now(),
    });

    // Trim log
    if (this.requestLog.length > this.maxLogSize) {
      this.requestLog.shift();
    }
  }

  /**
   * Get request log
   */
  getLog(): RequestLog[] {
    return [...this.requestLog];
  }

  /**
   * Clear request log
   */
  clearLog(): void {
    this.requestLog = [];
  }
}
```

### 3.2 Request Pattern Matching

The interceptor supports flexible pattern matching:

```typescript
// packages/core/src/network/pattern-matcher.ts

export class PatternMatcher {
  /**
   * Match URL against pattern
   */
  static match(pattern: string | RegExp, url: string): MatchResult {
    if (pattern instanceof RegExp) {
      return this.matchRegex(pattern, url);
    }
    
    if (pattern.includes('*')) {
      return this.matchWildcard(pattern, url);
    }
    
    if (pattern.includes(':')) {
      return this.matchPathParams(pattern, url);
    }
    
    return this.matchExact(pattern, url);
  }

  /**
   * Exact match
   */
  static matchExact(pattern: string, url: string): MatchResult {
    const parsedPattern = this.parseUrl(pattern);
    const parsedUrl = this.parseUrl(url);
    
    const matched = 
      parsedPattern.pathname === parsedUrl.pathname &&
      parsedPattern.search === parsedUrl.search;

    return { matched, params: {} };
  }

  /**
   * Wildcard match (e.g., /api/*)
   */
  static matchWildcard(pattern: string, url: string): MatchResult {
    const regexPattern = pattern
      .replace(/[.+?^${}()|[\]\\]/g, '\\$&')
      .replace(/\*/g, '.*');
    
    const regex = new RegExp(`^${regexPattern}$`);
    const matched = regex.test(url);
    
    return { matched, params: {} };
  }

  /**
   * Path parameter match (e.g., /users/:id)
   */
  static matchPathParams(pattern: string, url: string): MatchResult {
    const patternParts = pattern.split('/');
    const urlParts = url.split('/')[0]; // Just pathname
    
    if (patternParts.length !== urlParts.length) {
      return { matched: false, params: {} };
    }

    const params: Record<string, string> = {};

    for (let i = 0; i < patternParts.length; i++) {
      const patternPart = patternParts[i];
      const urlPart = urlParts[i];

      if (patternPart.startsWith(':')) {
        const paramName = patternPart.slice(1);
        params[paramName] = decodeURIComponent(urlPart);
      } else if (patternPart !== urlPart) {
        return { matched: false, params: {} };
      }
    }

    return { matched: true, params };
  }

  /**
   * Regex match with named groups
   */
  static matchRegex(pattern: RegExp, url: string): MatchResult {
    const match = pattern.exec(url);
    
    if (!match) {
      return { matched: false, params: {} };
    }

    return {
      matched: true,
      params: match.groups || {},
    };
  }

  /**
   * Parse URL into components
   */
  static parseUrl(url: string): { pathname: string; search: string; hash: string } {
    try {
      const parsed = new URL(url, 'http://localhost');
      return {
        pathname: parsed.pathname,
        search: parsed.search,
        hash: parsed.hash,
      };
    } catch {
      return { pathname: url, search: '', hash: '' };
    }
  }
}

interface MatchResult {
  matched: boolean;
  params: Record<string, string>;
}
```

### 3.3 Route Registration

```typescript
// packages/core/src/network/route-registry.ts

export class RouteRegistry {
  private routes: Map<string, RouteEntry> = new Map();
  private priorityQueue: string[] = [];

  /**
   * Register a route
   */
  register(entry: RouteEntry): RouteRegistration {
    // Check for duplicate
    const existing = this.routes.get(entry.id);
    if (existing) {
      throw new Error(`Route already registered: ${entry.id}`);
    }

    this.routes.set(entry.id, entry);
    this.priorityQueue.push(entry.id);
    this.priorityQueue.sort((a, b) => {
      const routeA = this.routes.get(a)!;
      const routeB = this.routes.get(b)!;
      return routeA.priority - routeB.priority;
    });

    return {
      id: entry.id,
      entry,
      unregister: () => this.unregister(entry.id),
      update: (handler: NetworkHandler) => {
        entry.handler = handler;
      },
    };
  }

  /**
   * Unregister a route
   */
  unregister(id: string): boolean {
    const existed = this.routes.delete(id);
    if (existed) {
      const index = this.priorityQueue.indexOf(id);
      if (index > -1) {
        this.priorityQueue.splice(index, 1);
      }
    }
    return existed;
  }

  /**
   * Find matching route
   */
  find(request: NetworkRequest): RouteEntry | null {
    for (const id of this.priorityQueue) {
      const route = this.routes.get(id);
      if (!route) continue;

      if (this.matches(route, request)) {
        return route;
      }
    }
    return null;
  }

  /**
   * Check if route matches request
   */
  private matches(route: RouteEntry, request: NetworkRequest): boolean {
    // Method check
    const methodMatch = route.methods.includes('*') || 
                        route.methods.includes(request.method);
    if (!methodMatch) return false;

    // Pattern check
    const url = new URL(request.url, 'http://localhost');
    const path = url.pathname + url.search;

    if (route.pattern instanceof RegExp) {
      return route.pattern.test(path);
    }

    return route.pattern === path;
  }

  /**
   * Get all routes
   */
  getAll(): RouteEntry[] {
    return Array.from(this.routes.values());
  }

  /**
   * Clear all routes
   */
  clear(): void {
    this.routes.clear();
    this.priorityQueue = [];
  }
}
```

### 3.4 Handler Functions

Handler functions process matched requests and return responses:

```typescript
// packages/core/src/network/handlers.ts

import type { NetworkRequest, NetworkResponse, NetworkHandler, InterceptorContext } from './types';

/**
 * Create a JSON response handler
 */
export function jsonHandler(
  data: any | ((request: NetworkRequest) => any),
  options?: { status?: number; headers?: Record<string, string> }
): NetworkHandler {
  return async (request: NetworkRequest): Promise<NetworkResponse> => {
    const body = typeof data === 'function' ? data(request) : data;
    
    return {
      status: options?.status || 200,
      statusText: 'OK',
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
      body: stringToStream(JSON.stringify(body)),
      url: request.url,
    };
  };
}

/**
 * Create a delayed response handler
 */
export function delayHandler(
  handler: NetworkHandler,
  delayMs: number
): NetworkHandler {
  return async (request: NetworkRequest): Promise<NetworkResponse> => {
    await new Promise(resolve => setTimeout(resolve, delayMs));
    return handler(request);
  };
}

/**
 * Create a conditional handler
 */
export function conditionalHandler(
  predicate: (request: NetworkRequest) => boolean,
  trueHandler: NetworkHandler,
  falseHandler: NetworkHandler
): NetworkHandler {
  return async (request: NetworkRequest): Promise<NetworkResponse> => {
    if (predicate(request)) {
      return trueHandler(request);
    }
    return falseHandler(request);
  };
}

/**
 * Create a handler that modifies the request
 */
export function modifyRequestHandler(
  modifier: (request: NetworkRequest) => NetworkRequest,
  handler: NetworkHandler
): NetworkHandler {
  return async (request: NetworkRequest): Promise<NetworkResponse> => {
    const modifiedRequest = modifier(request);
    return handler(modifiedRequest);
  };
}

/**
 * Create a handler that logs requests
 */
export function loggingHandler(
  handler: NetworkHandler,
  logger: (request: NetworkRequest, response: NetworkResponse) => void
): NetworkHandler {
  return async (request: NetworkRequest): Promise<NetworkResponse> => {
    const response = await handler(request);
    logger(request, response);
    return response;
  };
}

/**
 * Create a passthrough handler (makes real request)
 * Note: Only works in environments with real network access
 */
export function passthroughHandler(): NetworkHandler {
  return async (request: NetworkRequest): Promise<NetworkResponse> => {
    const fetchInit: RequestInit = {
      method: request.method,
      headers: request.headers,
      body: request.body,
    };

    const response = await fetch(request.url, fetchInit);
    
    return {
      status: response.status,
      statusText: response.statusText,
      headers: Object.fromEntries(response.headers.entries()),
      body: response.body,
      url: request.url,
    };
  };
}

/**
 * Create a handler chain
 */
export function chainHandlers(
  ...handlers: Array<(next: NetworkHandler) => NetworkHandler>
): NetworkHandler {
  return handlers.reduceRight(
    (next, wrapper) => wrapper(next),
    async (request: NetworkRequest): Promise<NetworkResponse> => {
      throw new Error('No terminal handler in chain');
    }
  );
}
```

### 3.5 Response Generation

```typescript
// packages/core/src/network/response-builder.ts

export class ResponseBuilder {
  private status: number = 200;
  private statusText: string = 'OK';
  private headers: Record<string, string> = {};
  private body: any = null;
  private delayMs: number = 0;

  /**
   * Set response status
   */
  setStatus(status: number, statusText?: string): this {
    this.status = status;
    if (statusText) {
      this.statusText = statusText;
    }
    return this;
  }

  /**
   * Set response headers
   */
  setHeaders(headers: Record<string, string>): this {
    this.headers = { ...this.headers, ...headers };
    return this;
  }

  /**
   * Add a header
   */
  addHeader(key: string, value: string): this {
    this.headers[key] = value;
    return this;
  }

  /**
   * Set JSON body
   */
  json(data: any): this {
    this.body = JSON.stringify(data);
    this.headers['Content-Type'] = 'application/json';
    return this;
  }

  /**
   * Set text body
   */
  text(content: string, contentType: string = 'text/plain'): this {
    this.body = content;
    this.headers['Content-Type'] = contentType;
    return this;
  }

  /**
   * Set HTML body
   */
  html(content: string): this {
    this.body = content;
    this.headers['Content-Type'] = 'text/html';
    return this;
  }

  /**
   * Set binary body
   */
  blob(data: Uint8Array, contentType: string): this {
    this.body = data;
    this.headers['Content-Type'] = contentType;
    return this;
  }

  /**
   * Set response delay
   */
  delay(ms: number): this {
    this.delayMs = ms;
    return this;
  }

  /**
   * Build the response
   */
  build(request: NetworkRequest): NetworkResponse {
    return {
      status: this.status,
      statusText: this.statusText,
      headers: this.headers,
      body: this.normalizeBody(this.body),
      url: request.url,
    };
  }

  /**
   * Build with delay
   */
  async buildDelayed(request: NetworkRequest): Promise<NetworkResponse> {
    if (this.delayMs > 0) {
      await new Promise(resolve => setTimeout(resolve, this.delayMs));
    }
    return this.build(request);
  }

  /**
   * Create builder from existing response
   */
  static fromResponse(response: Partial<NetworkResponse>): ResponseBuilder {
    const builder = new ResponseBuilder();
    if (response.status) builder.status = response.status;
    if (response.statusText) builder.statusText = response.statusText;
    if (response.headers) builder.headers = response.headers;
    if (response.body) builder.body = response.body;
    return builder;
  }

  private normalizeBody(body: any): ReadableStream<Uint8Array> | null {
    if (body instanceof ReadableStream) {
      return body;
    }
    if (body instanceof Uint8Array) {
      return streamFromBuffer(body);
    }
    if (typeof body === 'string') {
      return streamFromBuffer(new TextEncoder().encode(body));
    }
    return null;
  }
}

// Helper: Create stream from buffer
function streamFromBuffer(buffer: Uint8Array): ReadableStream<Uint8Array> {
  return new ReadableStream({
    start(controller) {
      controller.enqueue(buffer);
      controller.close();
    },
  });
}

// Helper: Create stream from string
function stringToStream(str: string): ReadableStream<Uint8Array> {
  return streamFromBuffer(new TextEncoder().encode(str));
}
```

---

## 4. Network Module Mocking

### 4.1 net Module Implementation

```typescript
// packages/core/src/process/executors/node/modules/net.ts

import type { Socket, Server, ConnectionListener } from './net-types';

/**
 * Mock implementation of Node.js 'net' module
 */
export class NetModule {
  private sockets: Map<number, MockSocket> = new Map();
  private servers: Map<number, MockServer> = new Map();
  private nextSocketId: number = 1;

  /**
   * Create a new socket
   */
  createSocket(options?: any): Socket {
    const socket = new MockSocket(this.nextSocketId++, options);
    this.sockets.set(socket.id, socket);
    
    socket.on('close', () => {
      this.sockets.delete(socket.id);
    });

    return socket;
  }

  /**
   * Create a connection
   */
  createConnection(options: any | number | string, connectionListener?: ConnectionListener): Socket {
    const socket = this.createSocket(options);
    
    // Parse connection options
    const normalizedOptions = this.normalizeConnectionOptions(options);
    
    // Connect (simulated - just emit 'connect' event)
    setTimeout(() => {
      socket.emit('connect');
      socket.emit('ready');
      if (connectionListener) {
        connectionListener();
      }
    }, 0);

    return socket;
  }

  /**
   * Create a server
   */
  createServer(connectionListener?: ConnectionListener): Server {
    const server = new MockServer(connectionListener);
    const serverId = this.servers.size + 1;
    this.servers.set(serverId, server);
    
    return server;
  }

  /**
   * Normalize connection options
   */
  private normalizeConnectionOptions(options: any | number | string): any {
    if (typeof options === 'number') {
      return { port: options };
    }
    if (typeof options === 'string') {
      return { path: options };
    }
    return options || {};
  }

  /**
   * Get socket by ID
   */
  getSocket(id: number): MockSocket | undefined {
    return this.sockets.get(id);
  }

  /**
   * Get server by ID
   */
  getServer(id: number): MockServer | undefined {
    return this.servers.get(id);
  }

  /**
   * Shutdown all sockets and servers
   */
  shutdown(): void {
    for (const socket of this.sockets.values()) {
      socket.destroy();
    }
    for (const server of this.servers.values()) {
      server.close();
    }
    this.sockets.clear();
    this.servers.clear();
  }
}

/**
 * Mock Socket implementation
 */
export class MockSocket implements Socket {
  readonly id: number;
  readonly readable: boolean = true;
  readonly writable: boolean = true;
  
  private buffer: Uint8Array[] = [];
  private connected: boolean = false;
  private destroyed: boolean = false;
  private listeners: Map<string, Set<Function>> = new Map();
  
  // Socket properties
  bytesRead: number = 0;
  bytesWritten: number = 0;
  bufferSize: number = 0;
  localPort?: number;
  remotePort?: number;
  localAddress?: string;
  remoteAddress?: string;

  constructor(id: number, options?: any) {
    this.id = id;
    
    if (options?.port) {
      this.remotePort = options.port;
      this.remoteAddress = options.host || '127.0.0.1';
    }
  }

  /**
   * Connect to remote
   */
  connect(port: number, host?: string, callback?: () => void): void;
  connect(options: any, callback?: () => void): void;
  connect(...args: any[]): void {
    const callback = typeof args[args.length - 1] === 'function' 
      ? args.pop() 
      : undefined;

    const options = typeof args[0] === 'number'
      ? { port: args[0], host: args[1] }
      : args[0];

    this.remotePort = options.port;
    this.remoteAddress = options.host || '127.0.0.1';
    this.localPort = Math.floor(Math.random() * 10000) + 49152; // Ephemeral port
    this.localAddress = '127.0.0.1';

    setTimeout(() => {
      this.connected = true;
      this.emit('connect');
      this.emit('ready');
      callback?.();
    }, 0);
  }

  /**
   * Write data
   */
  write(data: Uint8Array | string, encoding?: string, callback?: () => void): boolean {
    if (this.destroyed || !this.connected) {
      return false;
    }

    const buffer = typeof data === 'string' 
      ? new TextEncoder().encode(data)
      : data;

    this.bytesWritten += buffer.length;
    this.buffer.push(buffer);

    // Simulate async write
    setTimeout(() => {
      this.emit('drain');
      callback?.();
    }, 0);

    return this.bufferSize < 16384; // Return false if buffer is full
  }

  /**
   * Read data (internal use)
   */
  pushData(data: Uint8Array): void {
    this.bytesRead += data.length;
    this.emit('data', data);
  }

  /**
   * End connection
   */
  end(data?: Uint8Array | string, callback?: () => void): void {
    if (data) {
      this.write(data, undefined, callback);
    }
    this.emit('end');
    this.close();
  }

  /**
   * Destroy connection
   */
  destroy(error?: Error): void {
    if (this.destroyed) return;
    
    this.destroyed = true;
    this.connected = false;
    this.buffer = [];
    
    this.emit('close');
    this.emit('end');
    
    if (error) {
      this.emit('error', error);
    }
  }

  /**
   * Close connection
   */
  close(callback?: () => void): void {
    this.destroyed = true;
    this.connected = false;
    
    setTimeout(() => {
      this.emit('close');
      callback?.();
    }, 0);
  }

  /**
   * Set socket options
   */
  setNoDelay(noDelay?: boolean): this {
    return this;
  }

  setKeepAlive(enable?: boolean, initialDelay?: number): this {
    return this;
  }

  /**
   * Event handling
   */
  on(event: string, listener: Function): this {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(listener);
    return this;
  }

  once(event: string, listener: Function): this {
    const onceListener = (...args: any[]) => {
      listener(...args);
      this.off(event, onceListener);
    };
    return this.on(event, onceListener);
  }

  off(event: string, listener: Function): this {
    this.listeners.get(event)?.delete(listener);
    return this;
  }

  addListener(event: string, listener: Function): this {
    return this.on(event, listener);
  }

  removeListener(event: string, listener: Function): this {
    return this.off(event, listener);
  }

  protected emit(event: string, ...args: any[]): boolean {
    const listeners = this.listeners.get(event);
    if (!listeners || listeners.size === 0) return false;
    
    for (const listener of listeners) {
      listener(...args);
    }
    return true;
  }
}

/**
 * Mock Server implementation
 */
export class MockServer implements Server {
  private listening: boolean = false;
  private connections: Set<MockSocket> = new Set();
  private listeners: Map<string, Set<Function>> = new Map();
  private addressInfo: any = null;
  private maxConnections: number = Infinity;

  constructor(private connectionListener?: ConnectionListener) {}

  /**
   * Start listening
   */
  listen(port?: number, hostname?: string, backlog?: number, callback?: () => void): this;
  listen(port?: number, hostname?: string, callback?: () => void): this;
  listen(port?: number, callback?: () => void): this;
  listen(options: any, callback?: () => void): this;
  listen(...args: any[]): this {
    const callback = typeof args[args.length - 1] === 'function' 
      ? args.pop() 
      : undefined;

    const options = typeof args[0] === 'number'
      ? { port: args[0], host: args[1] || '0.0.0.0' }
      : args[0] || { port: 0 };

    const port = options.port || 0;
    const host = options.host || '0.0.0.0';

    // Assign port if not specified
    const actualPort = port === 0 
      ? Math.floor(Math.random() * 10000) + 49152
      : port;

    this.addressInfo = { port: actualPort, address: host, family: 'IPv4' };
    this.listening = true;

    setTimeout(() => {
      this.emit('listening');
      callback?.();
    }, 0);

    return this;
  }

  /**
   * Close server
   */
  close(callback?: () => void): this {
    this.listening = false;
    
    // Close all connections
    for (const conn of this.connections) {
      conn.end();
    }
    this.connections.clear();

    setTimeout(() => {
      this.emit('close');
      callback?.();
    }, 0);

    return this;
  }

  /**
   * Get server address
   */
  address(): any {
    return this.listening ? this.addressInfo : null;
  }

  /**
   * Ref server (keep process alive)
   */
  ref(): this {
    return this;
  }

  /**
   * Unref server
   */
  unref(): this {
    return this;
  }

  /**
   * Set max connections
   */
  maxConnections(count: number): this {
    this.maxConnections = count;
    return this;
  }

  /**
   * Simulate incoming connection
   */
  simulateConnection(): MockSocket {
    const socket = new MockSocket(Date.now());
    this.connections.add(socket);
    
    socket.on('close', () => {
      this.connections.delete(socket);
    });

    if (this.connectionListener) {
      this.connectionListener(socket);
    }

    this.emit('connection', socket);
    return socket;
  }

  /**
   * Event handling
   */
  on(event: string, listener: Function): this {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(listener);
    return this;
  }

  once(event: string, listener: Function): this {
    const onceListener = (...args: any[]) => {
      listener(...args);
      this.off(event, onceListener);
    };
    return this.on(event, onceListener);
  }

  off(event: string, listener: Function): this {
    this.listeners.get(event)?.delete(listener);
    return this;
  }

  protected emit(event: string, ...args: any[]): boolean {
    const listeners = this.listeners.get(event);
    if (!listeners || listeners.size === 0) return false;
    
    for (const listener of listeners) {
      listener(...args);
    }
    return true;
  }
}

// Module exports
export const net = {
  Socket: MockSocket,
  Server: MockServer,
  createSocket: (options?: any) => new MockSocket(Date.now(), options),
  createConnection: (options: any, listener?: ConnectionListener) => {
    const socket = new MockSocket(Date.now(), options);
    socket.connect(options, listener);
    return socket;
  },
  createServer: (listener?: ConnectionListener) => new MockServer(listener),
};
```

### 4.2 http Module Implementation

```typescript
// packages/core/src/process/executors/node/modules/http.ts

import { EventEmitter } from 'events';
import type { IncomingMessage, ServerResponse, Server, ClientRequest } from './http-types';

/**
 * Mock implementation of Node.js 'http' module
 */
export class HttpModule {
  private servers: Set<MockHttpServer> = new Set();
  private requests: Set<MockClientRequest> = new Set();

  /**
   * Create an HTTP server
   */
  createServer(requestListener?: (req: IncomingMessage, res: ServerResponse) => void): Server {
    const server = new MockHttpServer(requestListener);
    this.servers.add(server);
    
    server.on('close', () => {
      this.servers.delete(server);
    });

    return server;
  }

  /**
   * Make an HTTP request
   */
  request(url: string | URL, options?: any, callback?: (res: IncomingMessage) => void): ClientRequest {
    const request = new MockClientRequest(url, options);
    this.requests.add(request);

    request.on('close', () => {
      this.requests.delete(request);
    });

    if (callback) {
      request.on('response', callback);
    }

    return request;
  }

  /**
   * Make a GET request
   */
  get(url: string | URL, options?: any, callback?: (res: IncomingMessage) => void): ClientRequest {
    const request = this.request(url, { ...options, method: 'GET' }, callback);
    request.end();
    return request;
  }

  /**
   * Global agent
   */
  globalAgent: any = {
    maxSockets: Infinity,
    sockets: [],
    requests: [],
  };

  /**
   * Agent class
   */
  Agent = class Agent {
    maxSockets: number = 256;
    keepSocketAlive(socket: any): boolean { return true; }
    reuseSocket(socket: any, request: any): void {}
    destroy(): void {}
  };

  /**
   * Outgoing message class
   */
  OutgoingMessage = class OutgoingMessage extends EventEmitter {
    chunkedEncoding: boolean = true;
    shouldKeepAlive: boolean = true;
    useChunkedEncodingByDefault: boolean = true;
    sendDate: boolean = true;
    finished: boolean = false;
    headersSent: boolean = false;

    setHeader(name: string, value: string | string[]): void {}
    getHeader(name: string): any {}
    getHeaders(): Record<string, string> { return {}; }
    getHeaderNames(): string[] { return []; }
    hasHeader(name: string): boolean { return false; }
    removeHeader(name: string): void {}
    addTrailers(headers: any): void {}
    flushHeaders(): void {}
  };

  /**
   * Incoming message class
   */
  IncomingMessage = MockIncomingMessage;

  /**
   * Server response class
   */
  ServerResponse = MockServerResponse;
}

/**
 * Mock HTTP Server
 */
export class MockHttpServer extends EventEmitter implements Server {
  private listening: boolean = false;
  private addressInfo: any = null;
  private connections: Set<any> = new Set();

  constructor(private requestListener?: (req: IncomingMessage, res: ServerResponse) => void) {
    super();
  }

  listen(port?: number, hostname?: string, backlog?: number, callback?: () => void): this;
  listen(port?: number, hostname?: string, callback?: () => void): this;
  listen(port?: number, callback?: () => void): this;
  listen(options: any, callback?: () => void): this;
  listen(...args: any[]): this {
    const callback = typeof args[args.length - 1] === 'function' 
      ? args.pop() 
      : undefined;

    const options = typeof args[0] === 'number'
      ? { port: args[0], host: args[1] || '0.0.0.0' }
      : args[0] || { port: 0 };

    const actualPort = options.port === 0 
      ? Math.floor(Math.random() * 10000) + 49152
      : options.port;

    this.addressInfo = { port: actualPort, address: options.host || '0.0.0.0', family: 'IPv4' };
    this.listening = true;

    setTimeout(() => {
      this.emit('listening');
      callback?.();
    }, 0);

    return this;
  }

  close(callback?: () => void): this {
    this.listening = false;
    
    for (const conn of this.connections) {
      conn.destroy();
    }
    this.connections.clear();

    setTimeout(() => {
      this.emit('close');
      callback?.();
    }, 0);

    return this;
  }

  address(): any {
    return this.listening ? this.addressInfo : null;
  }

  ref(): this { return this; }
  unref(): this { return this; }
  maxConnections(count: number): this { return this; }

  /**
   * Simulate incoming request
   */
  simulateRequest(
    method: string,
    url: string,
    headers: Record<string, string> = {},
    body?: Uint8Array
  ): { request: MockIncomingMessage; response: MockServerResponse } {
    const request = new MockIncomingMessage(method, url, headers, body);
    const response = new MockServerResponse(request);

    if (this.requestListener) {
      this.requestListener(request, response);
    }

    this.emit('request', request, response);
    return { request, response };
  }
}

/**
 * Mock Incoming Message (request)
 */
export class MockIncomingMessage extends EventEmitter implements IncomingMessage {
  readonly method?: string;
  readonly url?: string;
  readonly headers: Record<string, string | string[]>;
  readonly rawHeaders: string[];
  readonly httpVersion: string = '1.1';
  readonly httpVersionMajor: number = 1;
  readonly httpVersionMinor: number = 1;
  readonly complete: boolean = true;
  
  private bodyData: Uint8Array | null = null;
  private readable: boolean = true;

  constructor(
    method: string,
    url: string,
    headers: Record<string, string>,
    body?: Uint8Array
  ) {
    super();
    this.method = method;
    this.url = url;
    this.headers = headers;
    this.rawHeaders = Object.entries(headers).flatMap(([k, v]) => [k, v as string]);
    this.bodyData = body || null;

    // Simulate data events
    if (this.bodyData) {
      setTimeout(() => {
        this.emit('data', this.bodyData);
        this.emit('end');
        this.readable = false;
      }, 0);
    } else {
      setTimeout(() => {
        this.emit('end');
      }, 0);
    }
  }

  setTimeout(msecs: number, callback?: () => void): this {
    if (callback) {
      setTimeout(callback, msecs);
    }
    return this;
  }

  // Stream methods
  read(size?: number): any { return null; }
  isPaused(): boolean { return false; }
  pause(): this { return this; }
  resume(): this { return this; }
  unpipe(destination?: any): this { return this; }
  unshift(chunk: any): void {}
  wrap(oldStream: any): this { return this; }

  pipe<T>(destination: T, options?: { end?: boolean }): T {
    return destination;
  }

  destroy(error?: Error): this {
    this.readable = false;
    if (error) this.emit('error', error);
    this.emit('close');
    return this;
  }

  // Properties for compatibility
  aborted: boolean = false;
  socket: any = null;
  connection: any = null;
  statusCode?: number;
  statusMessage?: string;
  trailers: Record<string, string> = {};
  rawTrailers: string[] = [];
  upgrade: boolean = false;

  // Methods
  setTimeoutCallback?: () => void;
  setTimeout(ms: number, cb?: () => void): this {
    this.setTimeoutCallback = cb;
    return this;
  }
}

/**
 * Mock Server Response
 */
export class MockServerResponse extends EventEmitter implements ServerResponse {
  statusCode: number = 200;
  statusMessage: string = 'OK';
  headers: Record<string, string | string[]> = {};
  headersSent: boolean = false;
  finished: boolean = false;
  sendDate: boolean = true;

  private bodyChunks: Uint8Array[] = [];
  private written: boolean = false;

  constructor(public req: IncomingMessage) {
    super();
  }

  writeHead(
    statusCode: number,
    statusMessageOrHeaders?: string | Record<string, string | string[]>,
    headers?: Record<string, string | string[]>
  ): this {
    this.statusCode = statusCode;
    
    if (typeof statusMessageOrHeaders === 'string') {
      this.statusMessage = statusMessageOrHeaders;
      if (headers) {
        Object.assign(this.headers, headers);
      }
    } else if (statusMessageOrHeaders) {
      Object.assign(this.headers, statusMessageOrHeaders);
    }

    this.headersSent = true;
    this.emit('header');
    return this;
  }

  write(chunk: Uint8Array | string, encoding?: string | (() => void), callback?: () => void): boolean {
    if (!this.written) {
      this.headersSent = true;
      this.written = true;
    }

    const buffer = typeof chunk === 'string' 
      ? new TextEncoder().encode(chunk)
      : chunk;

    this.bodyChunks.push(buffer);
    
    if (typeof encoding === 'function') {
      encoding();
    } else if (callback) {
      callback();
    }

    return true;
  }

  end(data?: Uint8Array | string | (() => void), encoding?: string | (() => void), callback?: () => void): this {
    if (!this.written) {
      this.headersSent = true;
      this.written = true;
    }

    if (typeof data === 'function') {
      callback = data;
      data = undefined;
    }

    if (data !== undefined) {
      const buffer = typeof data === 'string' 
        ? new TextEncoder().encode(data)
        : data;
      this.bodyChunks.push(buffer);
    }

    this.finished = true;
    
    setTimeout(() => {
      this.emit('finish');
      this.emit('close');
      callback?.();
    }, 0);

    return this;
  }

  getBody(): Uint8Array {
    const totalLength = this.bodyChunks.reduce((acc, chunk) => acc + chunk.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const chunk of this.bodyChunks) {
      result.set(chunk, offset);
      offset += chunk.length;
    }
    return result;
  }

  getBodyText(): string {
    return new TextDecoder().decode(this.getBody());
  }

  // Compatibility methods
  setHeader(name: string, value: string | string[]): this {
    this.headers[name.toLowerCase()] = value;
    return this;
  }

  getHeader(name: string): any {
    return this.headers[name.toLowerCase()];
  }

  getHeaderNames(): string[] {
    return Object.keys(this.headers);
  }

  hasHeader(name: string): boolean {
    return name.toLowerCase() in this.headers;
  }

  removeHeader(name: string): void {
    delete this.headers[name.toLowerCase()];
  }

  addTrailers(headers: any): void {}
  flushHeaders(): void {}
  assignSocket(socket: any): void {}
  detachSocket(socket: any): void {}
  
  writeContinue(callback?: () => void): void {
    callback?.();
  }

  writeProcessing(): void {}

  setTimeout(msecs: number, callback?: () => void): this {
    if (callback) setTimeout(callback, msecs);
    return this;
  }

  connection: any = null;
  socket: any = null;
  chunkedEncoding: boolean = true;
  shouldKeepAlive: boolean = true;
  useChunkedEncodingByDefault: boolean = true;
}

/**
 * Mock Client Request
 */
export class MockClientRequest extends EventEmitter implements ClientRequest {
  readonly method: string;
  readonly path: string;
  readonly host: string;
  readonly protocol: string;
  readonly headers: Record<string, string>;

  private bodyChunks: Uint8Array[] = [];
  private ended: boolean = false;
  private aborted: boolean = false;

  constructor(url: string | URL, options?: any) {
    super();
    
    const parsed = typeof url === 'string' ? new URL(url) : url;
    this.protocol = parsed.protocol;
    this.host = parsed.host;
    this.path = parsed.pathname + parsed.search;
    this.method = options?.method || 'GET';
    this.headers = options?.headers || {};
  }

  write(chunk: Uint8Array | string, encoding?: string | (() => void), callback?: () => void): boolean {
    if (this.ended || this.aborted) return false;

    const buffer = typeof chunk === 'string' 
      ? new TextEncoder().encode(chunk)
      : chunk;

    this.bodyChunks.push(buffer);
    
    if (typeof encoding === 'function') {
      encoding();
    } else if (callback) {
      callback();
    }

    return true;
  }

  end(data?: Uint8Array | string | (() => void), encoding?: string | (() => void), callback?: () => void): this {
    if (this.ended || this.aborted) return this;

    if (typeof data === 'function') {
      callback = data;
      data = undefined;
    }

    if (data !== undefined) {
      const buffer = typeof data === 'string' 
        ? new TextEncoder().encode(data)
        : data;
      this.bodyChunks.push(buffer);
    }

    this.ended = true;

    // Simulate response
    setTimeout(() => {
      const response = new MockIncomingMessage('GET', this.path, {}, new Uint8Array());
      response.statusCode = 200;
      response.statusMessage = 'OK';
      
      this.emit('response', response);
      callback?.();
    }, 0);

    return this;
  }

  abort(): void {
    if (this.aborted) return;
    
    this.aborted = true;
    this.emit('abort');
    this.emit('close');
  }

  setHeader(name: string, value: string): this {
    this.headers[name.toLowerCase()] = value;
    return this;
  }

  getHeader(name: string): any {
    return this.headers[name.toLowerCase()];
  }

  getHeaders(): Record<string, string> {
    return { ...this.headers };
  }

  removeHeader(name: string): void {
    delete this.headers[name.toLowerCase()];
  }

  setTimeout(msecs: number, callback?: () => void): this {
    if (callback) setTimeout(callback, msecs);
    return this;
  }

  setNoDelay(noDelay?: boolean): this { return this; }
  setSocketKeepAlive(enable?: boolean, initialDelay?: number): this { return this; }

  // Properties
  aborted: boolean = false;
  finished: boolean = false;
  connection: any = null;
  socket: any = null;
  chunkedEncoding: boolean = true;
  shouldKeepAlive: boolean = true;
  useChunkedEncodingByDefault: boolean = true;
  sendDate: boolean = false;
  trailers: Record<string, string> = {};
}

// Module exports
export const http = {
  createServer: (listener?: (req: IncomingMessage, res: ServerResponse) => void) => 
    new MockHttpServer(listener),
  request: (url: string | URL, options?: any, callback?: (res: IncomingMessage) => void) =>
    new MockClientRequest(url, options),
  get: (url: string | URL, options?: any, callback?: (res: IncomingMessage) => void) => {
    const req = new MockClientRequest(url, { ...options, method: 'GET' });
    req.end();
    return req;
  },
  Agent: class Agent {
    maxSockets: number = 256;
  },
  globalAgent: { maxSockets: Infinity },
  IncomingMessage: MockIncomingMessage,
  OutgoingMessage: class extends EventEmitter {},
  ServerResponse: MockServerResponse,
  Server: MockHttpServer,
};
```

### 4.3 https Module Implementation

```typescript
// packages/core/src/process/executors/node/modules/https.ts

import { http } from './http';
import type { ServerOptions, RequestOptions } from './https-types';

/**
 * Mock implementation of Node.js 'https' module
 * 
 * Extends http module with TLS/SSL simulation
 */
export class HttpsModule {
  /**
   * Create HTTPS server
   */
  createServer(options: ServerOptions, requestListener?: (req: any, res: any) => void) {
    // In a real implementation, this would handle TLS
    // For mocking, we just create a regular HTTP server
    return http.createServer(requestListener);
  }

  /**
   * Make HTTPS request
   */
  request(url: string | URL, options?: RequestOptions, callback?: (res: any) => void) {
    const req = http.request(url, { ...options, protocol: 'https:' }, callback);
    return req;
  }

  /**
   * Make HTTPS GET request
   */
  get(url: string | URL, options?: RequestOptions, callback?: (res: any) => void) {
    const req = this.request(url, { ...options, method: 'GET' }, callback);
    req.end();
    return req;
  }

  // Re-export from http
  Agent = http.Agent;
  globalAgent = http.globalAgent;
}

// Module exports
export const https = new HttpsModule();
```

### 4.4 Socket Emulation

```typescript
// packages/core/src/process/executors/node/modules/socket-emulation.ts

/**
 * Full-duplex socket emulation
 */
export class SocketEmulator {
  private readable: boolean = true;
  private writable: boolean = true;
  private destroyed: boolean = false;
  
  private readBuffer: Uint8Array[] = [];
  private writeBuffer: Uint8Array[] = [];
  
  private listeners: Map<string, Set<Function>> = new Map();

  // Socket state
  connecting: boolean = false;
  connected: boolean = false;
  bytesRead: number = 0;
  bytesWritten: number = 0;
  bufferSize: number = 0;

  // Address info
  localAddress?: string;
  localPort?: number;
  remoteAddress?: string;
  remotePort?: number;

  constructor() {}

  /**
   * Connect to remote
   */
  connect(port: number, host?: string, connectListener?: () => void): void;
  connect(options: any, connectListener?: () => void): void;
  connect(...args: any[]): void {
    this.connecting = true;

    const options = typeof args[0] === 'number'
      ? { port: args[0], host: args[1] }
      : args[0];

    const callback = typeof args[args.length - 1] === 'function'
      ? args.pop()
      : undefined;

    this.remotePort = options.port;
    this.remoteAddress = options.host || '127.0.0.1';
    this.localPort = Math.floor(Math.random() * 10000) + 49152;
    this.localAddress = '127.0.0.1';

    // Simulate connection delay
    setTimeout(() => {
      this.connecting = false;
      this.connected = true;
      this.emit('connect');
      this.emit('ready');
      callback?.();
    }, options.connectDelay || 0);
  }

  /**
   * Write data to socket
   */
  write(data: Uint8Array | string, encoding?: string | Function, callback?: Function): boolean {
    if (this.destroyed || !this.writable) {
      return false;
    }

    const buffer = typeof data === 'string'
      ? new TextEncoder().encode(data)
      : data;

    this.bytesWritten += buffer.length;
    this.writeBuffer.push(buffer);
    this.bufferSize += buffer.length;

    // Signal drain if buffer was full
    const wasFull = this.bufferSize > 16384;
    
    setTimeout(() => {
      this.bufferSize -= buffer.length;
      if (wasFull) {
        this.emit('drain');
      }
      callback?.();
    }, 0);

    return this.bufferSize < 16384;
  }

  /**
   * Read data from socket (internal)
   */
  private pushData(data: Uint8Array): void {
    if (!this.readable) return;

    this.bytesRead += data.length;
    this.readBuffer.push(data);
    this.emit('data', data);
  }

  /**
   * End writing
   */
  end(data?: Uint8Array | string, callback?: () => void): void {
    if (data) {
      this.write(data, callback);
    }
    this.writable = false;
    this.emit('finish');
    this.emit('end');
  }

  /**
   * Destroy socket
   */
  destroy(error?: Error): void {
    if (this.destroyed) return;

    this.destroyed = true;
    this.readable = false;
    this.writable = false;
    this.connected = false;
    this.connecting = false;
    
    this.readBuffer = [];
    this.writeBuffer = [];
    this.bufferSize = 0;

    this.emit('close');
    if (error) {
      this.emit('error', error);
    }
  }

  /**
   * Socket options
   */
  setNoDelay(noDelay?: boolean): this {
    return this;
  }

  setKeepAlive(enable?: boolean, initialDelay?: number): this {
    return this;
  }

  /**
   * Event handling
   */
  on(event: string, listener: Function): this {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(listener);
    return this;
  }

  once(event: string, listener: Function): this {
    const once = (...args: any[]) => {
      listener(...args);
      this.off(event, once);
    };
    return this.on(event, once);
  }

  off(event: string, listener: Function): this {
    this.listeners.get(event)?.delete(listener);
    return this;
  }

  addListener = this.on;
  removeListener = this.off;

  protected emit(event: string, ...args: any[]): boolean {
    const listeners = this.listeners.get(event);
    if (!listeners || listeners.size === 0) return false;
    
    for (const listener of listeners) {
      listener(...args);
    }
    return true;
  }
}
```

### 4.5 Server Emulation

```typescript
// packages/core/src/process/executors/node/modules/server-emulation.ts

import { EventEmitter } from 'events';

/**
 * Server emulation with connection handling
 */
export class ServerEmulator extends EventEmitter {
  private listening: boolean = false;
  private connections: Map<number, any> = new Map();
  private addressInfo: any = null;
  private maxConnections: number = Infinity;

  constructor(private handler?: (socket: any) => void) {
    super();
  }

  /**
   * Start listening
   */
  listen(port?: number, hostname?: string, backlog?: number, callback?: () => void): this;
  listen(port?: number, hostname?: string, callback?: () => void): this;
  listen(port?: number, callback?: () => void): this;
  listen(options: any, callback?: () => void): this;
  listen(...args: any[]): this {
    const callback = typeof args[args.length - 1] === 'function'
      ? args.pop()
      : undefined;

    const options = typeof args[0] === 'number'
      ? { port: args[0], host: args[1] || '0.0.0.0' }
      : args[0] || { port: 0 };

    const actualPort = options.port === 0
      ? Math.floor(Math.random() * 10000) + 49152
      : options.port;

    this.addressInfo = {
      port: actualPort,
      address: options.host || '0.0.0.0',
      family: 'IPv4',
    };
    this.listening = true;

    setTimeout(() => {
      this.emit('listening');
      callback?.();
    }, 0);

    return this;
  }

  /**
   * Close server
   */
  close(callback?: () => void): this {
    this.listening = false;

    for (const conn of this.connections.values()) {
      conn.destroy();
    }
    this.connections.clear();

    setTimeout(() => {
      this.emit('close');
      callback?.();
    }, 0);

    return this;
  }

  /**
   * Get address info
   */
  address(): any {
    return this.listening ? { ...this.addressInfo } : null;
  }

  /**
   * Ref/unref for process lifetime
   */
  ref(): this { return this; }
  unref(): this { return this; }

  /**
   * Set max connections
   */
  maxConnections(count: number): this {
    this.maxConnections = count;
    return this;
  }

  /**
   * Simulate incoming connection
   */
  simulateConnection(socket: any): void {
    if (this.connections.size >= this.maxConnections) {
      socket.destroy(new Error('Too many connections'));
      return;
    }

    this.connections.set(socket.id, socket);
    
    socket.on('close', () => {
      this.connections.delete(socket.id);
    });

    if (this.handler) {
      this.handler(socket);
    }

    this.emit('connection', socket);
  }
}
```

---

## 5. Fetch Mocking

### 5.1 Global Fetch Override

```typescript
// packages/core/src/network/fetch-mock.ts

import type { NetworkRequest, NetworkResponse, NetworkHandler } from './types';

/**
 * Fetch mocking utility that overrides global.fetch
 */
export class FetchMocker {
  private originalFetch: typeof fetch;
  private enabled: boolean = false;
  private handlers: Map<string | RegExp, NetworkHandler> = new Map();
  private defaultHandler?: NetworkHandler;
  private callHistory: FetchCall[] = [];

  constructor() {
    this.originalFetch = globalThis.fetch;
  }

  /**
   * Enable fetch mocking
   */
  enable(): void {
    if (this.enabled) return;
    
    this.enabled = true;
    const self = this;

    globalThis.fetch = async function(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
      const request = self.createRequest(input, init);
      const response = await self.handleRequest(request);
      return self.createResponse(response);
    };
  }

  /**
   * Disable fetch mocking and restore original
   */
  disable(): void {
    if (!this.enabled) return;
    
    this.enabled = false;
    globalThis.fetch = this.originalFetch;
  }

  /**
   * Register a fetch handler
   */
  on(url: string | RegExp, handler: NetworkHandler): this {
    this.handlers.set(url, handler);
    return this;
  }

  /**
   * Register handler by method
   */
  onMethod(method: string, url: string | RegExp, handler: NetworkHandler): this {
    const key = `${method}:${url instanceof RegExp ? url.source : url}`;
    const wrappedHandler: NetworkHandler = async (request) => {
      if (request.method !== method) {
        return this.passthrough(request);
      }
      return handler(request);
    };
    this.handlers.set(key, wrappedHandler);
    return this;
  }

  /**
   * Set default handler for unmatched requests
   */
  setDefault(handler: NetworkHandler): this {
    this.defaultHandler = handler;
    return this;
  }

  /**
   * Handle incoming request
   */
  private async handleRequest(request: NetworkRequest): Promise<NetworkResponse> {
    // Record call
    this.callHistory.push({
      url: request.url,
      method: request.method,
      headers: request.headers,
      body: request.body,
      timestamp: Date.now(),
    });

    // Find handler
    for (const [key, handler] of this.handlers.entries()) {
      if (this.matches(request, key)) {
        try {
          return await handler(request);
        } catch (error) {
          return this.createError(request, error);
        }
      }
    }

    // Use default handler
    if (this.defaultHandler) {
      return this.defaultHandler(request);
    }

    // No handler found
    return {
      status: 404,
      statusText: 'Not Found',
      headers: { 'Content-Type': 'application/json' },
      body: stringToStream(JSON.stringify({
        error: 'No mock handler registered',
        url: request.url,
        method: request.method,
      })),
    };
  }

  /**
   * Check if request matches key
   */
  private matches(request: NetworkRequest, key: string | RegExp): boolean {
    if (key instanceof RegExp) {
      return key.test(request.url);
    }

    if (key.includes(':')) {
      // Method-specific key
      const [method, urlPattern] = key.split(':');
      if (request.method !== method) return false;
      return this.urlMatches(request.url, urlPattern);
    }

    return this.urlMatches(request.url, key);
  }

  /**
   * Check URL match
   */
  private urlMatches(url: string, pattern: string): boolean {
    // Exact match
    if (url === pattern) return true;

    // Wildcard match
    if (pattern.includes('*')) {
      const regex = new RegExp('^' + pattern.replace(/\*/g, '.*') + '$');
      return regex.test(url);
    }

    // Path match (ignore query string)
    const urlPath = url.split('?')[0];
    const patternPath = pattern.split('?')[0];
    if (urlPath === patternPath) return true;

    return false;
  }

  /**
   * Create NetworkRequest from fetch arguments
   */
  private createRequest(input: RequestInfo | URL, init?: RequestInit): NetworkRequest {
    let url: string;
    let method = init?.method || 'GET';
    let headers: Record<string, string> = {};
    let body: ReadableStream<Uint8Array> | null = null;

    if (typeof input === 'string') {
      url = input;
    } else if (input instanceof URL) {
      url = input.toString();
    } else {
      url = input.url;
      method = input.method;
      headers = Object.fromEntries(input.headers.entries());
      body = input.body;
    }

    if (init) {
      method = init.method || method;
      headers = {
        ...headers,
        ...Object.fromEntries(new Headers(init.headers || {}).entries()),
      };
      if (init.body) {
        body = this.bodyToStream(init.body);
      }
    }

    return { url, method, headers, body };
  }

  /**
   * Create Response from NetworkResponse
   */
  private createResponse(networkResponse: NetworkResponse): Response {
    return new Response(networkResponse.body, {
      status: networkResponse.status,
      statusText: networkResponse.statusText,
      headers: networkResponse.headers,
    });
  }

  /**
   * Create error response
   */
  private createError(request: NetworkRequest, error: any): NetworkResponse {
    return {
      status: error.status || 500,
      statusText: error.statusText || 'Internal Server Error',
      headers: { 'Content-Type': 'application/json' },
      body: stringToStream(JSON.stringify({
        error: error.message || 'Unknown error',
      })),
    };
  }

  /**
   * Passthrough to real fetch
   */
  private async passthrough(request: NetworkRequest): Promise<NetworkResponse> {
    const response = await this.originalFetch(request.url, {
      method: request.method,
      headers: request.headers,
      body: request.body,
    });

    return {
      status: response.status,
      statusText: response.statusText,
      headers: Object.fromEntries(response.headers.entries()),
      body: response.body,
    };
  }

  /**
   * Get call history
   */
  getCalls(): FetchCall[] {
    return [...this.callHistory];
  }

  /**
   * Clear call history
   */
  clearHistory(): void {
    this.callHistory = [];
  }

  /**
   * Helper: Convert body to stream
   */
  private bodyToStream(body: any): ReadableStream<Uint8Array> | null {
    if (body instanceof ReadableStream) {
      return body;
    }
    if (typeof body === 'string') {
      return stringToStream(body);
    }
    if (body instanceof Uint8Array) {
      return new ReadableStream({
        start(controller) {
          controller.enqueue(body);
          controller.close();
        },
      });
    }
    return null;
  }
}

interface FetchCall {
  url: string;
  method: string;
  headers: Record<string, string>;
  body: any;
  timestamp: number;
}

// Helper function
function stringToStream(str: string): ReadableStream<Uint8Array> {
  return new ReadableStream({
    start(controller) {
      controller.enqueue(new TextEncoder().encode(str));
      controller.close();
    },
  });
}
```

### 5.2 Request Matching

```typescript
// packages/core/src/network/request-matcher.ts

export class RequestMatcher {
  /**
   * Match request against criteria
   */
  static match(
    request: NetworkRequest,
    criteria: RequestCriteria
  ): boolean {
    if (criteria.url && !this.urlMatches(request.url, criteria.url)) {
      return false;
    }
    if (criteria.method && request.method !== criteria.method) {
      return false;
    }
    if (criteria.headers && !this.headersMatch(request.headers, criteria.headers)) {
      return false;
    }
    if (criteria.bodyMatch && !this.bodyMatches(request.body, criteria.bodyMatch)) {
      return false;
    }
    return true;
  }

  /**
   * URL matching
   */
  static urlMatches(url: string, pattern: string | RegExp): boolean {
    if (pattern instanceof RegExp) {
      return pattern.test(url);
    }

    // Wildcard
    if (pattern.includes('*')) {
      const regex = new RegExp('^' + pattern.replace(/\*/g, '.*') + '$');
      return regex.test(url);
    }

    // Exact or prefix
    return url === pattern || url.startsWith(pattern + '/') || url.startsWith(pattern + '?');
  }

  /**
   * Headers matching
   */
  static headersMatch(
    requestHeaders: Record<string, string>,
    criteriaHeaders: Record<string, string | RegExp>
  ): boolean {
    for (const [key, value] of Object.entries(criteriaHeaders)) {
      const requestValue = requestHeaders[key.toLowerCase()] || requestHeaders[key];
      
      if (requestValue === undefined) {
        return false;
      }

      if (value instanceof RegExp) {
        if (!value.test(requestValue)) {
          return false;
        }
      } else if (requestValue !== value) {
        return false;
      }
    }
    return true;
  }

  /**
   * Body matching
   */
  static bodyMatches(
    body: any,
    match: any | ((body: any) => boolean)
  ): boolean {
    if (typeof match === 'function') {
      return match(body);
    }

    // JSON body match
    if (typeof body === 'string') {
      try {
        const parsed = JSON.parse(body);
        return this.objectMatches(parsed, match);
      } catch {
        return body.includes(String(match));
      }
    }

    return this.objectMatches(body, match);
  }

  /**
   * Object deep match
   */
  static objectMatches(obj: any, criteria: any): boolean {
    if (typeof criteria !== 'object' || criteria === null) {
      return obj === criteria;
    }

    for (const [key, value] of Object.entries(criteria)) {
      if (typeof value === 'object' && value !== null) {
        if (!this.objectMatches(obj[key], value)) {
          return false;
        }
      } else if (obj[key] !== value) {
        return false;
      }
    }
    return true;
  }
}

interface RequestCriteria {
  url?: string | RegExp;
  method?: string;
  headers?: Record<string, string | RegExp>;
  bodyMatch?: any;
}
```

### 5.3 Response Construction

```typescript
// packages/core/src/network/response-constructor.ts

export class ResponseConstructor {
  private status: number = 200;
  private statusText: string = 'OK';
  private headers: Record<string, string> = {};
  private body: any = null;
  private delay: number = 0;

  /**
   * Set status code
   */
  status(code: number, text?: string): this {
    this.status = code;
    if (text) this.statusText = text;
    return this;
  }

  /**
   * Set header
   */
  header(key: string, value: string): this {
    this.headers[key] = value;
    return this;
  }

  /**
   * Set multiple headers
   */
  headers(headers: Record<string, string>): this {
    Object.assign(this.headers, headers);
    return this;
  }

  /**
   * Set JSON body
   */
  json(data: any): this {
    this.body = JSON.stringify(data);
    this.headers['Content-Type'] = 'application/json';
    return this;
  }

  /**
   * Set text body
   */
  text(content: string): this {
    this.body = content;
    this.headers['Content-Type'] = 'text/plain';
    return this;
  }

  /**
   * Set HTML body
   */
  html(content: string): this {
    this.body = content;
    this.headers['Content-Type'] = 'text/html';
    return this;
  }

  /**
   * Set delay
   */
  delayResponse(ms: number): this {
    this.delay = ms;
    return this;
  }

  /**
   * Build response
   */
  build(): NetworkResponse {
    return {
      status: this.status,
      statusText: this.statusText,
      headers: this.headers,
      body: this.normalizeBody(),
    };
  }

  private normalizeBody(): ReadableStream<Uint8Array> | null {
    if (!this.body) return null;
    if (this.body instanceof ReadableStream) return this.body;
    return stringToStream(String(this.body));
  }
}
```

### 5.4 Headers Handling

```typescript
// packages/core/src/network/headers.ts

export class HeadersHandler {
  /**
   * Normalize headers to Record<string, string>
   */
  static normalize(headers: HeadersInit): Record<string, string> {
    if (headers instanceof Headers) {
      return Object.fromEntries(headers.entries());
    }
    if (Array.isArray(headers)) {
      return Object.fromEntries(headers);
    }
    return { ...headers } as Record<string, string>;
  }

  /**
   * Merge headers
   */
  static merge(
    base: Record<string, string>,
    override: Record<string, string>
  ): Record<string, string> {
    const result: Record<string, string> = {};
    
    // Copy base (normalize keys)
    for (const [key, value] of Object.entries(base)) {
      result[key.toLowerCase()] = value;
    }
    
    // Apply override
    for (const [key, value] of Object.entries(override)) {
      result[key.toLowerCase()] = value;
    }
    
    return result;
  }

  /**
   * Get header value (case-insensitive)
   */
  static get(headers: Record<string, string>, name: string): string | undefined {
    return headers[name.toLowerCase()];
  }

  /**
   * Set header value
   */
  static set(headers: Record<string, string>, name: string, value: string): void {
    headers[name.toLowerCase()] = value;
  }

  /**
   * Check if header exists
   */
  static has(headers: Record<string, string>, name: string): boolean {
    return name.toLowerCase() in headers;
  }

  /**
   * Remove header
   */
  static remove(headers: Record<string, string>, name: string): void {
    delete headers[name.toLowerCase()];
  }

  /**
   * Common header utilities
   */
  static utils = {
    isJson: (headers: Record<string, string>): boolean => {
      const contentType = headers['content-type'] || '';
      return contentType.includes('application/json');
    },
    
    getContentType: (headers: Record<string, string>): string => {
      return headers['content-type'] || 'application/octet-stream';
    },
    
    getContentLength: (headers: Record<string, string>): number => {
      return parseInt(headers['content-length'] || '0', 10);
    },
    
    isChunked: (headers: Record<string, string>): boolean => {
      return headers['transfer-encoding'] === 'chunked';
    },
  };
}
```

### 5.5 Body Handling

```typescript
// packages/core/src/network/body-handler.ts

export class BodyHandler {
  /**
   * Read body as text
   */
  static async asText(body: ReadableStream<Uint8Array> | null): Promise<string> {
    if (!body) return '';
    
    const chunks: Uint8Array[] = [];
    const reader = body.getReader();

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }
    } finally {
      reader.releaseLock();
    }

    const totalLength = chunks.reduce((acc, chunk) => acc + chunk.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const chunk of chunks) {
      result.set(chunk, offset);
      offset += chunk.length;
    }

    return new TextDecoder().decode(result);
  }

  /**
   * Read body as JSON
   */
  static async asJson(body: ReadableStream<Uint8Array> | null): Promise<any> {
    const text = await this.asText(body);
    return JSON.parse(text);
  }

  /**
   * Read body as bytes
   */
  static async asBytes(body: ReadableStream<Uint8Array> | null): Promise<Uint8Array> {
    if (!body) return new Uint8Array(0);
    
    const chunks: Uint8Array[] = [];
    const reader = body.getReader();

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }
    } finally {
      reader.releaseLock();
    }

    const totalLength = chunks.reduce((acc, chunk) => acc + chunk.length, 0);
    const result = new Uint8Array(totalLength);
    let offset = 0;
    for (const chunk of chunks) {
      result.set(chunk, offset);
      offset += chunk.length;
    }

    return result;
  }

  /**
   * Clone body stream
   */
  static clone(body: ReadableStream<Uint8Array> | null): [ReadableStream<Uint8Array>, ReadableStream<Uint8Array>] {
    if (!body) {
      const empty1 = new ReadableStream({ start: (c) => c.close() });
      const empty2 = new ReadableStream({ start: (c) => c.close() });
      return [empty1, empty2];
    }

    const tee = body.tee();
    return tee;
  }

  /**
   * Create body from string
   */
  static fromString(text: string): ReadableStream<Uint8Array> {
    return new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(text));
        controller.close();
      },
    });
  }

  /**
   * Create body from object (JSON)
   */
  static fromJson(data: any): ReadableStream<Uint8Array> {
    return this.fromString(JSON.stringify(data));
  }
}
```

---

## 6. HTTP Server Emulation

### 6.1 createServer() Implementation

```typescript
// packages/core/src/network/http-server.ts

import { EventEmitter } from 'events';
import type { Server, IncomingMessage, ServerResponse } from './http-types';

/**
 * HTTP Server emulation for OpenContainer
 */
export class HttpServerEmulator extends EventEmitter implements Server {
  private listening: boolean = false;
  private addressInfo: { port: number; address: string; family: string } | null = null;
  private requestHandler?: (req: IncomingMessage, res: ServerResponse) => void;
  private connections: Set<Connection> = new Set();
  private timeout: number = 0;

  constructor(requestListener?: (req: IncomingMessage, res: ServerResponse) => void) {
    super();
    this.requestHandler = requestListener;
  }

  /**
   * Start listening on port
   * Note: Port is simulated - no real socket binding occurs
   */
  listen(port?: number, hostname?: string, backlog?: number, callback?: () => void): this;
  listen(port?: number, hostname?: string, callback?: () => void): this;
  listen(port?: number, callback?: () => void): this;
  listen(options: any, callback?: () => void): this;
  listen(...args: any[]): this {
    const callback = typeof args[args.length - 1] === 'function'
      ? args.pop()
      : undefined;

    const options = this.parseListenArgs(args);
    
    const actualPort = options.port === 0
      ? Math.floor(Math.random() * 10000) + 49152
      : options.port || 8080;

    this.addressInfo = {
      port: actualPort,
      address: options.host || '0.0.0.0',
      family: 'IPv4',
    };
    this.listening = true;

    // Register with network interceptor
    this.registerWithInterceptor();

    setTimeout(() => {
      this.emit('listening');
      callback?.();
    }, 0);

    return this;
  }

  /**
   * Close server
   */
  close(callback?: () => void): this {
    this.listening = false;
    this.addressInfo = null;

    // Close all connections
    for (const conn of this.connections) {
      conn.destroy();
    }
    this.connections.clear();

    // Unregister from interceptor
    this.unregisterFromInterceptor();

    setTimeout(() => {
      this.emit('close');
      callback?.();
    }, 0);

    return this;
  }

  /**
   * Get server address
   */
  address(): any {
    return this.listening && this.addressInfo ? { ...this.addressInfo } : null;
  }

  /**
   * Set server timeout
   */
  setTimeout(msecs: number, callback?: () => void): this {
    this.timeout = msecs;
    if (callback) {
      this.on('timeout', callback);
    }
    return this;
  }

  /**
   * Ref server (keep process alive)
   */
  ref(): this {
    return this;
  }

  /**
   * Unref server
   */
  unref(): this {
    return this;
  }

  /**
   * Register server with network interceptor
   */
  private registerWithInterceptor(): void {
    // In a real implementation, this would register with the HttpInterceptor
    // to handle incoming requests to this server's port
    console.log(`Server registered on port ${this.addressInfo?.port}`);
  }

  /**
   * Unregister from interceptor
   */
  private unregisterFromInterceptor(): void {
    console.log(`Server unregistered from port ${this.addressInfo?.port}`);
  }

  /**
   * Handle incoming request
   */
  handleRequest(request: IncomingMessage, response: ServerResponse): void {
    if (this.requestHandler) {
      this.requestHandler(request, response);
    }
    this.emit('request', request, response);
  }

  /**
   * Parse listen arguments
   */
  private parseListenArgs(args: any[]): { port: number; host: string; backlog?: number } {
    if (args.length === 0) {
      return { port: 0, host: '0.0.0.0' };
    }

    if (typeof args[0] === 'object') {
      return {
        port: args[0].port || 0,
        host: args[0].host || '0.0.0.0',
        backlog: args[0].backlog,
      };
    }

    return {
      port: typeof args[0] === 'number' ? args[0] : 0,
      host: typeof args[1] === 'string' ? args[1] : '0.0.0.0',
      backlog: args[2],
    };
  }
}

interface Connection {
  id: string;
  destroy: () => void;
}

// Factory function
export function createServer(
  requestListener?: (req: IncomingMessage, res: ServerResponse) => void
): Server {
  return new HttpServerEmulator(requestListener);
}
```

### 6.2 ServerRequest and ServerResponse

```typescript
// packages/core/src/network/server-request-response.ts

import { EventEmitter } from 'events';

/**
 * ServerRequest - Mock HTTP request object
 */
export class ServerRequest extends EventEmitter {
  readonly method: string;
  readonly url: string;
  readonly headers: Record<string, string | string[]>;
  readonly rawHeaders: string[];
  readonly httpVersion: string = '1.1';
  readonly httpVersionMajor: number = 1;
  readonly httpVersionMinor: number = 1;
  readonly socket: any;
  readonly connection: any;

  private bodyChunks: Uint8Array[] = [];
  private complete: boolean = false;
  private readable: boolean = true;

  // URL components
  readonly pathname: string;
  readonly search: string;
  readonly query: Record<string, string>;

  constructor(
    method: string,
    url: string,
    headers: Record<string, string>,
    body?: Uint8Array
  ) {
    super();
    this.method = method.toUpperCase();
    this.url = url;
    this.headers = headers;
    this.rawHeaders = Object.entries(headers).flatMap(([k, v]) => [k, v as string]);
    this.socket = { remoteAddress: '127.0.0.1', remotePort: 12345 };
    this.connection = this.socket;

    // Parse URL
    const urlObj = new URL(url, 'http://localhost');
    this.pathname = urlObj.pathname;
    this.search = urlObj.search;
    this.query = Object.fromEntries(urlObj.searchParams.entries());

    // Queue body data events
    if (body) {
      this.bodyChunks.push(body);
    }
    
    setTimeout(() => {
      for (const chunk of this.bodyChunks) {
        this.emit('data', chunk);
      }
      this.complete = true;
      this.emit('end');
      this.readable = false;
    }, 0);
  }

  /**
   * Read body (streaming)
   */
  read(size?: number): any {
    if (!this.readable || this.bodyChunks.length === 0) {
      return null;
    }
    return this.bodyChunks.shift();
  }

  /**
   * Pause reading
   */
  pause(): this {
    return this;
  }

  /**
   * Resume reading
   */
  resume(): this {
    return this;
  }

  /**
   * Set timeout
   */
  setTimeout(msecs: number, callback?: () => void): this {
    if (callback) {
      setTimeout(callback, msecs);
    }
    return this;
  }

  /**
   * Get body as buffer
   */
  async getBody(): Promise<Uint8Array> {
    return new Promise((resolve) => {
      const chunks: Uint8Array[] = [];
      
      this.on('data', (chunk) => chunks.push(chunk));
      this.on('end', () => {
        const total = chunks.reduce((acc, c) => acc + c.length, 0);
        const result = new Uint8Array(total);
        let offset = 0;
        for (const chunk of chunks) {
          result.set(chunk, offset);
          offset += chunk.length;
        }
        resolve(result);
      });
    });
  }

  /**
   * Get body as text
   */
  async getText(): Promise<string> {
    const body = await this.getBody();
    return new TextDecoder().decode(body);
  }

  /**
   * Get body as JSON
   */
  async getJson(): Promise<any> {
    const text = await this.getText();
    return JSON.parse(text);
  }

  /**
   * Pipe to destination
   */
  pipe<T>(destination: T): T {
    return destination;
  }

  // Properties for compatibility
  aborted: boolean = false;
  statusCode?: number;
  statusMessage?: string;
  trailers: Record<string, string> = {};
  rawTrailers: string[] = [];
  upgrade: boolean = false;
}

/**
 * ServerResponse - Mock HTTP response object
 */
export class ServerResponse extends EventEmitter {
  statusCode: number = 200;
  statusMessage: string = 'OK';
  headers: Record<string, string | string[]> = {};
  headersSent: boolean = false;
  finished: boolean = false;
  sendDate: boolean = true;

  private bodyChunks: Uint8Array[] = [];
  private written: boolean = false;

  constructor(public req: ServerRequest) {
    super();
  }

  /**
   * Write status and headers
   */
  writeHead(
    statusCode: number,
    statusMessageOrHeaders?: string | Record<string, string | string[]>,
    headers?: Record<string, string | string[]>
  ): this {
    this.statusCode = statusCode;
    
    if (typeof statusMessageOrHeaders === 'string') {
      this.statusMessage = statusMessageOrHeaders;
      if (headers) {
        this.headers = { ...this.headers, ...headers };
      }
    } else if (statusMessageOrHeaders) {
      this.headers = { ...this.headers, ...statusMessageOrHeaders };
    }

    this.headersSent = true;
    this.emit('header');
    return this;
  }

  /**
   * Write body chunk
   */
  write(chunk: Uint8Array | string, encoding?: string | Function, callback?: Function): boolean {
    if (!this.written) {
      this.headersSent = true;
      this.written = true;
    }

    const buffer = typeof chunk === 'string'
      ? new TextEncoder().encode(chunk)
      : chunk;

    this.bodyChunks.push(buffer);
    
    if (typeof encoding === 'function') {
      encoding();
    } else if (callback) {
      callback();
    }

    return true;
  }

  /**
   * End response
   */
  end(data?: Uint8Array | string | Function, encoding?: string | Function, callback?: Function): this {
    if (!this.written) {
      this.headersSent = true;
      this.written = true;
    }

    if (typeof data === 'function') {
      callback = data;
      data = undefined;
    }

    if (data !== undefined) {
      const buffer = typeof data === 'string'
        ? new TextEncoder().encode(data)
        : data;
      this.bodyChunks.push(buffer);
    }

    this.finished = true;
    
    setTimeout(() => {
      this.emit('finish');
      this.emit('close');
      callback?.();
    }, 0);

    return this;
  }

  /**
   * Get response body
   */
  getBody(): Uint8Array {
    const total = this.bodyChunks.reduce((acc, c) => acc + c.length, 0);
    const result = new Uint8Array(total);
    let offset = 0;
    for (const chunk of this.bodyChunks) {
      result.set(chunk, offset);
      offset += chunk.length;
    }
    return result;
  }

  /**
   * Get body as text
   */
  getBodyText(): string {
    return new TextDecoder().decode(this.getBody());
  }

  /**
   * Get body as JSON
   */
  getBodyJson(): any {
    return JSON.parse(this.getBodyText());
  }

  // Compatibility methods
  setHeader(name: string, value: string | string[]): this {
    this.headers[name.toLowerCase()] = value;
    return this;
  }

  getHeader(name: string): any {
    return this.headers[name.toLowerCase()];
  }

  getHeaderNames(): string[] {
    return Object.keys(this.headers);
  }

  hasHeader(name: string): boolean {
    return name.toLowerCase() in this.headers;
  }

  removeHeader(name: string): void {
    delete this.headers[name.toLowerCase()];
  }

  addTrailers(headers: any): void {}
  flushHeaders(): void {}
  assignSocket(socket: any): void {}
  detachSocket(socket: any): void {}

  writeContinue(callback?: () => void): void {
    callback?.();
  }

  writeProcessing(): void {}

  setTimeout(msecs: number, callback?: () => void): this {
    if (callback) setTimeout(callback, msecs);
    return this;
  }
}
```

### 6.3 Request Handling

```typescript
// packages/core/src/network/request-handler.ts

import type { ServerRequest, ServerResponse } from './server-request-response';

export class RequestHandler {
  private handlers: Array<(req: ServerRequest, res: ServerResponse) => Promise<void>> = [];

  /**
   * Add middleware/handler
   */
  use(handler: (req: ServerRequest, res: ServerResponse) => Promise<void>): this {
    this.handlers.push(handler);
    return this;
  }

  /**
   * Handle request
   */
  async handle(req: ServerRequest, res: ServerResponse): Promise<void> {
    try {
      for (const handler of this.handlers) {
        if (res.finished) break;
        await handler(req, res);
      }
    } catch (error) {
      if (!res.headersSent) {
        res.writeHead(500);
      }
      res.end(JSON.stringify({ error: 'Internal Server Error' }));
    }
  }

  /**
   * Handle GET
   */
  get(path: string, handler: (req: ServerRequest, res: ServerResponse) => void): this {
    return this.use(async (req, res) => {
      if (req.method === 'GET' && this.pathMatches(req.pathname, path)) {
        await handler(req, res);
      }
    });
  }

  /**
   * Handle POST
   */
  post(path: string, handler: (req: ServerRequest, res: ServerResponse) => void): this {
    return this.use(async (req, res) => {
      if (req.method === 'POST' && this.pathMatches(req.pathname, path)) {
        await handler(req, res);
      }
    });
  }

  /**
   * Handle PUT
   */
  put(path: string, handler: (req: ServerRequest, res: ServerResponse) => void): this {
    return this.use(async (req, res) => {
      if (req.method === 'PUT' && this.pathMatches(req.pathname, path)) {
        await handler(req, res);
      }
    });
  }

  /**
   * Handle DELETE
   */
  delete(path: string, handler: (req: ServerRequest, res: ServerResponse) => void): this {
    return this.use(async (req, res) => {
      if (req.method === 'DELETE' && this.pathMatches(req.pathname, path)) {
        await handler(req, res);
      }
    });
  }

  /**
   * Check path match
   */
  private pathMatches(requestPath: string, pattern: string): boolean {
    if (pattern.includes('*')) {
      const regex = new RegExp('^' + pattern.replace(/\*/g, '.*') + '$');
      return regex.test(requestPath);
    }
    return requestPath === pattern;
  }
}
```

### 6.4 Response Writing

```typescript
// packages/core/src/network/response-writer.ts

export class ResponseWriter {
  private res: any;
  private headersSet: boolean = false;

  constructor(response: any) {
    this.res = response;
  }

  /**
   * Send JSON response
   */
  json(data: any, status: number = 200): void {
    this.res.writeHead(status, { 'Content-Type': 'application/json' });
    this.res.end(JSON.stringify(data));
  }

  /**
   * Send text response
   */
  text(content: string, status: number = 200, contentType: string = 'text/plain'): void {
    this.res.writeHead(status, { 'Content-Type': contentType });
    this.res.end(content);
  }

  /**
   * Send HTML response
   */
  html(content: string, status: number = 200): void {
    this.res.writeHead(status, { 'Content-Type': 'text/html' });
    this.res.end(content);
  }

  /**
   * Send binary response
   */
  send(data: Uint8Array, contentType: string, status: number = 200): void {
    this.res.writeHead(status, { 'Content-Type': contentType });
    this.res.end(data);
  }

  /**
   * Send file download
   */
  download(data: Uint8Array, filename: string, contentType: string): void {
    this.res.writeHead(200, {
      'Content-Type': contentType,
      'Content-Disposition': `attachment; filename="${filename}"`,
    });
    this.res.end(data);
  }

  /**
   * Send with streaming
   */
  async stream(
    iterable: AsyncIterable<Uint8Array>,
    contentType: string,
    status: number = 200
  ): Promise<void> {
    this.res.writeHead(status, {
      'Content-Type': contentType,
      'Transfer-Encoding': 'chunked',
    });

    for await (const chunk of iterable) {
      if (this.res.finished) break;
      this.res.write(chunk);
    }

    this.res.end();
  }

  /**
   * Send error response
   */
  error(message: string, status: number = 500): void {
    this.res.writeHead(status, { 'Content-Type': 'application/json' });
    this.res.end(JSON.stringify({ error: message }));
  }

  /**
   * Redirect
   */
  redirect(url: string, status: number = 302): void {
    this.res.writeHead(status, { 'Location': url });
    this.res.end();
  }

  /**
   * No content
   */
  noContent(): void {
    this.res.writeHead(204);
    this.res.end();
  }
}
```

### 6.5 Listening Ports (Simulated)

```typescript
// packages/core/src/network/port-manager.ts

export class PortManager {
  private usedPorts: Map<number, string> = new Map();
  private readonly minPort: number = 1024;
  private readonly maxPort: number = 65535;

  /**
   * Allocate a port
   */
  allocate(serverId: string, preferredPort?: number): number {
    if (preferredPort && !this.usedPorts.has(preferredPort)) {
      this.usedPorts.set(preferredPort, serverId);
      return preferredPort;
    }

    // Find available port
    for (let port = this.minPort; port <= this.maxPort; port++) {
      if (!this.usedPorts.has(port)) {
        this.usedPorts.set(port, serverId);
        return port;
      }
    }

    throw new Error('No available ports');
  }

  /**
   * Release a port
   */
  release(port: number): void {
    this.usedPorts.delete(port);
  }

  /**
   * Check if port is in use
   */
  isUsed(port: number): boolean {
    return this.usedPorts.has(port);
  }

  /**
   * Get server using port
   */
  getServer(port: number): string | undefined {
    return this.usedPorts.get(port);
  }

  /**
   * Get all used ports
   */
  getUsedPorts(): number[] {
    return Array.from(this.usedPorts.keys());
  }

  /**
   * Clear all ports
   */
  clear(): void {
    this.usedPorts.clear();
  }
}
```

---

## 7. WebSocket Simulation

### 7.1 WebSocket Class Emulation

```typescript
// packages/core/src/network/websocket.ts

import { EventEmitter } from 'events';

/**
 * WebSocket states
 */
export enum WebSocketState {
  CONNECTING = 0,
  OPEN = 1,
  CLOSING = 2,
  CLOSED = 3,
}

/**
 * Mock WebSocket implementation
 */
export class WebSocketEmulator extends EventEmitter implements WebSocket {
  readonly CONNECTING: number = WebSocketState.CONNECTING;
  readonly OPEN: number = WebSocketState.OPEN;
  readonly CLOSING: number = WebSocketState.CLOSING;
  readonly CLOSED: number = WebSocketState.CLOSED;

  readyState: number = WebSocketState.CONNECTING;
  bufferedAmount: number = 0;
  extensions: string = '';
  protocol: string = '';
  url: string;

  // Event handlers
  onopen: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;

  private connectionTimeout?: NodeJS.Timeout;
  private messageQueue: any[] = [];
  private connected: boolean = false;

  constructor(url: string, protocols?: string | string[]) {
    super();
    this.url = url;

    if (typeof protocols === 'string') {
      this.protocol = protocols;
    } else if (Array.isArray(protocols) && protocols.length > 0) {
      this.protocol = protocols[0];
    }

    // Simulate connection
    this.connect();
  }

  /**
   * Connect to server
   */
  private connect(): void {
    this.readyState = WebSocketState.CONNECTING;

    // Simulate connection delay
    this.connectionTimeout = setTimeout(() => {
      this.readyState = WebSocketState.OPEN;
      this.connected = true;
      this.emit('open');
      this.onopen?.(new Event('open'));

      // Flush message queue
      for (const message of this.messageQueue) {
        this.send(message);
      }
      this.messageQueue = [];
    }, 100);
  }

  /**
   * Send message
   */
  send(data: string | ArrayBuffer | Blob | ArrayBufferView): void {
    if (this.readyState === WebSocketState.CONNECTING) {
      this.messageQueue.push(data);
      this.bufferedAmount += this.getDataSize(data);
      return;
    }

    if (this.readyState !== WebSocketState.OPEN) {
      throw new DOMException('WebSocket is not open', 'InvalidStateError');
    }

    // Simulate sending
    setTimeout(() => {
      this.bufferedAmount -= this.getDataSize(data);
      this.emit('send', data);
    }, 0);
  }

  /**
   * Close connection
   */
  close(code?: number, reason?: string): void {
    if (this.readyState === WebSocketState.CLOSED || this.readyState === WebSocketState.CLOSING) {
      return;
    }

    this.readyState = WebSocketState.CLOSING;

    setTimeout(() => {
      this.readyState = WebSocketState.CLOSED;
      this.connected = false;
      this.emit('close', { code: code || 1000, reason: reason || '' });
      this.onclose?.({ code: code || 1000, reason: reason || '' } as CloseEvent);

      if (this.connectionTimeout) {
        clearTimeout(this.connectionTimeout);
      }
    }, 0);
  }

  /**
   * Get data size
   */
  private getDataSize(data: any): number {
    if (typeof data === 'string') return data.length;
    if (data instanceof ArrayBuffer) return data.byteLength;
    if (data instanceof Blob) return data.size;
    if (ArrayBuffer.isView(data)) return data.byteLength;
    return 0;
  }

  /**
   * Simulate incoming message
   */
  simulateMessage(data: any): void {
    if (this.readyState !== WebSocketState.OPEN) return;

    const event = {
      data,
      origin: this.url,
      source: null,
    };

    setTimeout(() => {
      this.emit('message', event);
      this.onmessage?.(event as MessageEvent);
    }, 0);
  }

  /**
   * Simulate connection error
   */
  simulateError(error: any): void {
    this.readyState = WebSocketState.CLOSED;
    this.connected = false;

    setTimeout(() => {
      this.emit('error', error);
      this.onerror?.(error as Event);
      this.emit('close', { code: 1006, reason: 'Connection error' });
      this.onclose?.({ code: 1006, reason: 'Connection error' } as CloseEvent);
    }, 0);
  }

  // Event listener methods
  addEventListener(type: string, listener: EventListenerOrEventListenerObject): void {
    this.on(type, listener);
  }

  removeEventListener(type: string, listener: EventListenerOrEventListenerObject): void {
    this.off(type, listener);
  }

  dispatchEvent(event: Event): boolean {
    this.emit(event.type, event);
    return true;
  }
}

// Factory function
export function createWebSocket(url: string, protocols?: string | string[]): WebSocket {
  return new WebSocketEmulator(url, protocols);
}
```

### 7.2 Connection Lifecycle

```typescript
// packages/core/src/network/websocket-lifecycle.ts

import { WebSocketEmulator, WebSocketState } from './websocket';

/**
 * WebSocket connection manager
 */
export class WebSocketLifecycle {
  private connections: Map<string, WebSocketEmulator> = new Map();

  /**
   * Create new connection
   */
  create(url: string, protocols?: string | string[]): WebSocketEmulator {
    const ws = new WebSocketEmulator(url, protocols);
    const id = `${url}:${Date.now()}`;
    
    this.connections.set(id, ws);

    ws.on('close', () => {
      setTimeout(() => {
        this.connections.delete(id);
      }, 1000);
    });

    return ws;
  }

  /**
   * Get connection by ID
   */
  get(id: string): WebSocketEmulator | undefined {
    return this.connections.get(id);
  }

  /**
   * Get all connections
   */
  getAll(): WebSocketEmulator[] {
    return Array.from(this.connections.values());
  }

  /**
   * Close all connections
   */
  closeAll(code?: number, reason?: string): void {
    for (const ws of this.connections.values()) {
      ws.close(code, reason);
    }
  }

  /**
   * Broadcast message to all connections
   */
  broadcast(data: any): void {
    for (const ws of this.getAll()) {
      if (ws.readyState === WebSocketState.OPEN) {
        ws.simulateMessage(data);
      }
    }
  }

  /**
   * Get connection statistics
   */
  getStats(): {
    total: number;
    connected: number;
    connecting: number;
    closing: number;
    closed: number;
  } {
    const connections = Array.from(this.connections.values());
    return {
      total: connections.length,
      connected: connections.filter(c => c.readyState === WebSocketState.OPEN).length,
      connecting: connections.filter(c => c.readyState === WebSocketState.CONNECTING).length,
      closing: connections.filter(c => c.readyState === WebSocketState.CLOSING).length,
      closed: connections.filter(c => c.readyState === WebSocketState.CLOSED).length,
    };
  }
}
```

### 7.3 Message Events

```typescript
// packages/core/src/network/websocket-messages.ts

/**
 * WebSocket message handler
 */
export class MessageHandler {
  private handlers: Map<string, Set<(data: any) => void>> = new Map();

  /**
   * Register message handler for type
   */
  on(type: string, handler: (data: any) => void): void {
    if (!this.handlers.has(type)) {
      this.handlers.set(type, new Set());
    }
    this.handlers.get(type)!.add(handler);
  }

  /**
   * Remove handler
   */
  off(type: string, handler: (data: any) => void): void {
    this.handlers.get(type)?.delete(handler);
  }

  /**
   * Handle incoming message
   */
  handle(message: MessageEvent): void {
    const data = message.data;
    
    // Try to parse as JSON
    if (typeof data === 'string') {
      try {
        const parsed = JSON.parse(data);
        const type = parsed.type || 'unknown';
        this.handlers.get(type)?.forEach(h => h(parsed));
        this.handlers.get('*')?.forEach(h => h(parsed));
        return;
      } catch {
        // Not JSON, treat as raw message
      }
    }

    // Raw message handlers
    this.handlers.get('raw')?.forEach(h => h(data));
  }

  /**
   * Send typed message
   */
  static send(ws: WebSocket, type: string, data: any): void {
    const message = JSON.stringify({ type, ...data });
    ws.send(message);
  }

  /**
   * Send raw message
   */
  static sendRaw(ws: WebSocket, data: any): void {
    if (typeof data === 'string') {
      ws.send(data);
    } else {
      ws.send(JSON.stringify(data));
    }
  }
}
```

### 7.4 Ready States

```typescript
// packages/core/src/network/websocket-states.ts

/**
 * WebSocket ready state utilities
 */
export class ReadyStateManager {
  /**
   * Check if WebSocket is connected
   */
  static isOpen(ws: WebSocket): boolean {
    return ws.readyState === WebSocket.OPEN;
  }

  /**
   * Check if WebSocket is connecting
   */
  static isConnecting(ws: WebSocket): boolean {
    return ws.readyState === WebSocket.CONNECTING;
  }

  /**
   * Check if WebSocket is closing
   */
  static isClosing(ws: WebSocket): boolean {
    return ws.readyState === WebSocket.CLOSING;
  }

  /**
   * Check if WebSocket is closed
   */
  static isClosed(ws: WebSocket): boolean {
    return ws.readyState === WebSocket.CLOSED;
  }

  /**
   * Get ready state name
   */
  static getStateName(ws: WebSocket): string {
    switch (ws.readyState) {
      case WebSocket.CONNECTING: return 'CONNECTING';
      case WebSocket.OPEN: return 'OPEN';
      case WebSocket.CLOSING: return 'CLOSING';
      case WebSocket.CLOSED: return 'CLOSED';
      default: return 'UNKNOWN';
    }
  }

  /**
   * Wait for WebSocket to open
   */
  static waitForOpen(ws: WebSocket, timeout: number = 5000): Promise<void> {
    if (ws.readyState === WebSocket.OPEN) {
      return Promise.resolve();
    }
    
    if (ws.readyState !== WebSocket.CONNECTING) {
      return Promise.reject(new Error('WebSocket is not connecting'));
    }

    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        ws.removeEventListener('open', onOpen);
        ws.removeEventListener('error', onError);
        reject(new Error('Connection timeout'));
      }, timeout);

      const onOpen = () => {
        clearTimeout(timeoutId);
        ws.removeEventListener('error', onError);
        resolve();
      };

      const onError = () => {
        clearTimeout(timeoutId);
        ws.removeEventListener('open', onOpen);
        reject(new Error('Connection failed'));
      };

      ws.addEventListener('open', onOpen);
      ws.addEventListener('error', onError);
    });
  }

  /**
   * Wait for WebSocket to close
   */
  static waitForClose(ws: WebSocket): Promise<CloseEvent> {
    if (ws.readyState === WebSocket.CLOSED) {
      return Promise.resolve({ code: 1000, reason: '', wasClean: true } as CloseEvent);
    }

    return new Promise((resolve) => {
      ws.addEventListener('close', function onClose(event: CloseEvent) {
        resolve(event);
      }, { once: true });
    });
  }
}
```

---

## 8. Network Types and Interfaces

### 8.1 NetworkRequest Interface

```typescript
// packages/core/src/network/types.ts

/**
 * HTTP methods enum
 */
export enum HttpMethod {
  GET = 'GET',
  POST = 'POST',
  PUT = 'PUT',
  DELETE = 'DELETE',
  PATCH = 'PATCH',
  HEAD = 'HEAD',
  OPTIONS = 'OPTIONS',
  CONNECT = 'CONNECT',
  TRACE = 'TRACE',
}

/**
 * HTTP status codes enum
 */
export enum HttpStatusCode {
  // Informational
  Continue = 100,
  SwitchingProtocols = 101,
  Processing = 102,

  // Success
  OK = 200,
  Created = 201,
  Accepted = 202,
  NonAuthoritativeInformation = 203,
  NoContent = 204,
  ResetContent = 205,
  PartialContent = 206,

  // Redirection
  MultipleChoices = 300,
  MovedPermanently = 301,
  Found = 302,
  SeeOther = 303,
  NotModified = 304,
  TemporaryRedirect = 307,
  PermanentRedirect = 308,

  // Client errors
  BadRequest = 400,
  Unauthorized = 401,
  PaymentRequired = 402,
  Forbidden = 403,
  NotFound = 404,
  MethodNotAllowed = 405,
  NotAcceptable = 406,
  RequestTimeout = 408,
  Conflict = 409,
  Gone = 410,
  LengthRequired = 411,
  PreconditionFailed = 412,
  PayloadTooLarge = 413,
  UriTooLong = 414,
  UnsupportedMediaType = 415,
  TooManyRequests = 429,

  // Server errors
  InternalServerError = 500,
  NotImplemented = 501,
  BadGateway = 502,
  ServiceUnavailable = 503,
  GatewayTimeout = 504,
  HttpVersionNotSupported = 505,
}

/**
 * Network request interface
 */
export interface NetworkRequest {
  url: string;
  method: HttpMethod | string;
  headers: Record<string, string>;
  body: ReadableStream<Uint8Array> | null;
  
  // Optional properties
  signal?: AbortSignal;
  credentials?: RequestCredentials;
  cache?: RequestCache;
  redirect?: RequestRedirect;
  referrer?: string;
  mode?: RequestMode;
}

/**
 * Network response interface
 */
export interface NetworkResponse {
  status: number;
  statusText: string;
  headers: Record<string, string>;
  body: ReadableStream<Uint8Array> | null;
  url?: string;
  
  // Optional properties
  redirected?: boolean;
  type?: ResponseType;
  ok?: boolean;
}

/**
 * Network handler function type
 */
export type NetworkHandler = (
  request: NetworkRequest,
  context: InterceptorContext
) => Promise<NetworkResponse> | NetworkResponse;
```

### 8.2 NetworkResponse Interface

```typescript
// Continued from types.ts

/**
 * Interceptor context interface
 */
export interface InterceptorContext {
  containerId: string;
  baseUrl: string;
  timeout: number;
  originalFetch?: typeof fetch;
  originalXHR?: {
    open: Function;
    send: Function;
  };
}

/**
 * Route entry interface
 */
export interface RouteEntry {
  id: string;
  methods: (HttpMethod | '*')[];
  pattern: RegExp | string;
  handler: NetworkHandler;
  priority: number;
  createdAt: number;
  hitCount: number;
}

/**
 * Route registration return type
 */
export interface RouteRegistration {
  id: string;
  entry: RouteEntry;
  unregister: () => boolean;
  update: (newHandler: NetworkHandler) => void;
}

/**
 * Request log entry
 */
export interface RequestLog {
  url: string;
  method: string;
  headers: Record<string, string>;
  body?: any;
  timestamp: number;
  response?: {
    status: number;
    headers: Record<string, string>;
  };
}
```

### 8.3 NetworkInterceptor Interface

```typescript
// Continued from types.ts

/**
 * Network interceptor interface
 */
export interface NetworkInterceptor {
  enable(): void;
  disable(): void;
  intercept(method: HttpMethod | '*', pattern: string | RegExp, handler: NetworkHandler): RouteRegistration;
  get(path: string | RegExp, handler: NetworkHandler): RouteRegistration;
  post(path: string | RegExp, handler: NetworkHandler): RouteRegistration;
  put(path: string | RegExp, handler: NetworkHandler): RouteRegistration;
  delete(path: string | RegExp, handler: NetworkHandler): RouteRegistration;
  clearRoutes(): void;
  getLog(): RequestLog[];
  clearLog(): void;
}

/**
 * Network sandbox interface
 */
export interface NetworkSandbox {
  interceptor: NetworkInterceptor;
  createScope(options: SandboxOptions): NetworkScope;
  getRequests(filter?: RequestFilter): RequestLog[];
  clearLog(): void;
}

/**
 * Network scope interface
 */
export interface NetworkScope {
  interceptor: NetworkInterceptor;
  enable(): void;
  disable(): void;
}

/**
 * Sandbox options
 */
export interface SandboxOptions {
  inheritRoutes?: boolean;
  baseUrl?: string;
  autoEnable?: boolean;
}

/**
 * Request filter
 */
export interface RequestFilter {
  method?: string;
  urlPattern?: RegExp;
  startTime?: number;
  endTime?: number;
}
```

### 8.4 Complete Types File

```typescript
// packages/core/src/network/types.ts - Complete file

/**
 * Network Simulation Types
 * 
 * Type definitions for OpenContainer network simulation layer
 */

// ==================== HTTP Methods ====================

export enum HttpMethod {
  GET = 'GET',
  POST = 'POST',
  PUT = 'PUT',
  DELETE = 'DELETE',
  PATCH = 'PATCH',
  HEAD = 'HEAD',
  OPTIONS = 'OPTIONS',
  CONNECT = 'CONNECT',
  TRACE = 'TRACE',
}

// ==================== HTTP Status Codes ====================

export enum HttpStatusCode {
  // 1xx Informational
  Continue = 100,
  SwitchingProtocols = 101,
  Processing = 102,
  EarlyHints = 103,

  // 2xx Success
  OK = 200,
  Created = 201,
  Accepted = 202,
  NonAuthoritativeInformation = 203,
  NoContent = 204,
  ResetContent = 205,
  PartialContent = 206,
  MultiStatus = 207,
  AlreadyReported = 208,
  ImUsed = 226,

  // 3xx Redirection
  MultipleChoices = 300,
  MovedPermanently = 301,
  Found = 302,
  SeeOther = 303,
  NotModified = 304,
  UseProxy = 305,
  TemporaryRedirect = 307,
  PermanentRedirect = 308,

  // 4xx Client Errors
  BadRequest = 400,
  Unauthorized = 401,
  PaymentRequired = 402,
  Forbidden = 403,
  NotFound = 404,
  MethodNotAllowed = 405,
  NotAcceptable = 406,
  ProxyAuthenticationRequired = 407,
  RequestTimeout = 408,
  Conflict = 409,
  Gone = 410,
  LengthRequired = 411,
  PreconditionFailed = 412,
  PayloadTooLarge = 413,
  UriTooLong = 414,
  UnsupportedMediaType = 415,
  RangeNotSatisfiable = 416,
  ExpectationFailed = 417,
  ImATeapot = 418,
  MisdirectedRequest = 421,
  UnprocessableEntity = 422,
  Locked = 423,
  FailedDependency = 424,
  TooEarly = 425,
  UpgradeRequired = 426,
  PreconditionRequired = 428,
  TooManyRequests = 429,
  RequestHeaderFieldsTooLarge = 431,
  UnavailableForLegalReasons = 451,

  // 5xx Server Errors
  InternalServerError = 500,
  NotImplemented = 501,
  BadGateway = 502,
  ServiceUnavailable = 503,
  GatewayTimeout = 504,
  HttpVersionNotSupported = 505,
  VariantAlsoNegotiates = 506,
  InsufficientStorage = 507,
  LoopDetected = 508,
  NotExtended = 510,
  NetworkAuthenticationRequired = 511,
}

// ==================== Request/Response ====================

export interface NetworkRequest {
  url: string;
  method: HttpMethod | string;
  headers: Record<string, string>;
  body: ReadableStream<Uint8Array> | null;
  signal?: AbortSignal;
  credentials?: RequestCredentials;
  cache?: RequestCache;
  redirect?: RequestRedirect;
  referrer?: string;
  mode?: RequestMode;
}

export interface NetworkResponse {
  status: number;
  statusText: string;
  headers: Record<string, string>;
  body: ReadableStream<Uint8Array> | null;
  url?: string;
  redirected?: boolean;
  type?: ResponseType;
}

// ==================== Handlers ====================

export type NetworkHandler = (
  request: NetworkRequest,
  context: InterceptorContext
) => Promise<NetworkResponse> | NetworkResponse;

export type ErrorHandler = (
  request: NetworkRequest,
  error: Error
) => Promise<NetworkResponse> | NetworkResponse;

// ==================== Context ====================

export interface InterceptorContext {
  containerId: string;
  baseUrl: string;
  timeout: number;
  originalFetch?: typeof fetch;
  originalXHR?: {
    open: Function;
    send: Function;
  };
}

export interface SandboxContext {
  containerId: string;
  baseUrl: string;
  interceptors: NetworkInterceptor[];
}

// ==================== Routes ====================

export interface RouteEntry {
  id: string;
  methods: (HttpMethod | '*')[];
  pattern: RegExp | string;
  handler: NetworkHandler;
  priority: number;
  createdAt: number;
  hitCount: number;
}

export interface RouteRegistration {
  id: string;
  entry: RouteEntry;
  unregister: () => boolean;
  update: (handler: NetworkHandler) => void;
}

export interface RouteConfig {
  method: HttpMethod | '*';
  pattern: string | RegExp;
  handler: string | NetworkHandler;
  priority?: number;
}

// ==================== Logging ====================

export interface RequestLog {
  url: string;
  method: string;
  headers: Record<string, string>;
  body?: any;
  timestamp: number;
  response?: {
    status: number;
    headers: Record<string, string>;
    size?: number;
  };
  duration?: number;
}

export interface RequestFilter {
  method?: string;
  urlPattern?: RegExp;
  startTime?: number;
  endTime?: number;
  status?: number;
}

// ==================== Configuration ====================

export interface NetworkConfig {
  enabled: boolean;
  baseUrl: string;
  timeout: number;
  maxLogSize: number;
  passthrough?: boolean;
  defaultHandler?: NetworkHandler;
}

export interface SandboxOptions {
  inheritRoutes?: boolean;
  baseUrl?: string;
  autoEnable?: boolean;
  config?: Partial<NetworkConfig>;
}

export interface NetworkScope {
  interceptor: NetworkInterceptor;
  enable(): void;
  disable(): void;
  id: string;
}

// ==================== Interceptor ====================

export interface NetworkInterceptor {
  enable(): void;
  disable(): void;
  isEnabled(): boolean;
  intercept(
    method: HttpMethod | HttpMethod[] | '*',
    pattern: string | RegExp,
    handler: NetworkHandler
  ): RouteRegistration;
  get(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration;
  post(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration;
  put(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration;
  delete(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration;
  patch(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration;
  any(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration;
  setDefaultHandler(handler: NetworkHandler): void;
  clearRoutes(): void;
  getLog(): RequestLog[];
  clearLog(): void;
  processRequest(request: NetworkRequest): Promise<NetworkResponse>;
}

// ==================== WebSocket ====================

export interface WebSocketMessage {
  type: string;
  data: any;
  timestamp?: number;
}

export interface WebSocketConnection {
  id: string;
  url: string;
  protocol: string;
  readyState: number;
  onmessage?: (event: WebSocketMessage) => void;
  onopen?: () => void;
  onclose?: (event: { code: number; reason: string }) => void;
  onerror?: (error: any) => void;
}

// ==================== Module Mocks ====================

export interface ModuleMock {
  name: string;
  exports: Record<string, any>;
}

export interface HttpModuleMock {
  createServer: (listener?: Function) => any;
  request: (url: string, options?: any) => any;
  get: (url: string, options?: any) => any;
  Agent: any;
  globalAgent: any;
}

export interface HttpsModuleMock extends HttpModuleMock {}

export interface NetModuleMock {
  createConnection: (options: any) => any;
  createServer: (listener?: Function) => any;
  Socket: any;
  Server: any;
}

// ==================== Utilities ====================

export type MaybePromise<T> = T | Promise<T>;

export interface StreamOptions {
  highWaterMark?: number;
  encoding?: string;
}
```

---

## 9. Use Cases

### 9.1 API Mocking for Testing

```typescript
// Example: API mocking for integration tests

import { NetworkSandbox } from '@opencontainer/core';

// Create sandbox for testing
const sandbox = new NetworkSandbox('test-container');
const { interceptor } = sandbox;

// Enable interception
interceptor.enable();

// Mock user API endpoints
interceptor.get('/api/users', () => ({
  status: 200,
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify([
    { id: 1, name: 'Alice', email: 'alice@example.com' },
    { id: 2, name: 'Bob', email: 'bob@example.com' },
  ]),
}));

interceptor.get('/api/users/:id', (req) => {
  const id = req.url.split('/').pop();
  return {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ id, name: `User ${id}`, email: `user${id}@example.com` }),
  };
});

interceptor.post('/api/users', async (req) => {
  const body = await parseBody(req.body);
  return {
    status: 201,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ id: Date.now(), ...body }),
  };
});

interceptor.delete('/api/users/:id', () => ({
  status: 204,
  body: null,
}));

// Run tests
async function runTests() {
  // Test GET all users
  const users = await fetch('/api/users');
  const usersData = await users.json();
  console.assert(usersData.length === 2);

  // Test GET single user
  const user = await fetch('/api/users/123');
  const userData = await user.json();
  console.assert(userData.id === '123');

  // Test POST user
  const newUser = await fetch('/api/users', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ name: 'Charlie', email: 'charlie@example.com' }),
  });
  const newUserData = await newUser.json();
  console.assert(newUserData.name === 'Charlie');

  // Test DELETE user
  const deleteRes = await fetch('/api/users/123', { method: 'DELETE' });
  console.assert(deleteRes.status === 204);
}

// Cleanup
function cleanup() {
  interceptor.disable();
  interceptor.clearRoutes();
  interceptor.clearLog();
}
```

### 9.2 Offline Development

```typescript
// Example: Offline development setup

import { NetworkSandbox } from '@opencontainer/core';

const sandbox = new NetworkSandbox('dev-container');
const { interceptor } = sandbox;

// Mock all external API dependencies
interceptor.intercept('*', /https:\/\/api\.external\.com\/.*/, (req) => {
  console.log('Mocking external API:', req.url);
  
  // Return cached/synthetic data
  return {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      data: 'synthetic-data',
      offline: true,
      timestamp: Date.now(),
    }),
  };
});

// Mock authentication
interceptor.post('/auth/login', (req) => ({
  status: 200,
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    token: 'mock-jwt-token-' + Date.now(),
    user: { id: 'dev-user', name: 'Developer' },
  }),
}));

interceptor.post('/auth/refresh', () => ({
  status: 200,
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ token: 'refreshed-mock-token' }),
}));

// Enable for offline development
interceptor.enable();

console.log('Offline mode enabled - all external APIs are mocked');
```

### 9.3 Network Condition Simulation

```typescript
// Example: Simulating various network conditions

import { NetworkSandbox } from '@opencontainer/core';

const sandbox = new NetworkSandbox('network-test');
const { interceptor } = sandbox;

// Simulate slow network (3G)
interceptor.intercept('*', /.*/, async (req) => {
  // 3G delay: 100-500ms
  const delay = 100 + Math.random() * 400;
  await new Promise(r => setTimeout(r, delay));
  
  return passthrough(req);
});

// Simulate intermittent failures
let requestCount = 0;
interceptor.intercept('*', /https:\/\/flaky-api\.com\/.*/, (req) => {
  requestCount++;
  
  // Fail every 3rd request
  if (requestCount % 3 === 0) {
    return {
      status: 503,
      statusText: 'Service Unavailable',
      body: JSON.stringify({ error: 'Service temporarily unavailable' }),
    };
  }
  
  return passthrough(req);
});

// Simulate rate limiting
const rateLimit = new Map<string, number>();
interceptor.intercept('*', /.*/, (req) => {
  const clientId = req.headers['x-client-id'] || 'anonymous';
  const count = rateLimit.get(clientId) || 0;
  
  if (count > 100) {
    return {
      status: 429,
      statusText: 'Too Many Requests',
      headers: {
        'Retry-After': '60',
        'X-RateLimit-Limit': '100',
        'X-RateLimit-Remaining': '0',
      },
      body: JSON.stringify({ error: 'Rate limit exceeded' }),
    };
  }
  
  rateLimit.set(clientId, count + 1);
  return passthrough(req);
});

// Simulate network timeout
interceptor.intercept('*', /https:\/\/slow-api\.com\/.*/, async (req) => {
  // Timeout after 30 seconds
  await new Promise((_, reject) => {
    setTimeout(() => reject(new Error('Request timeout')), 30000);
  });
});

// Simulate partial content
interceptor.get('/large-file', (req) => {
  const range = req.headers['range'];
  
  if (range) {
    const [start, end] = range.replace('bytes=', '').split('-').map(Number);
    return {
      status: 206,
      headers: {
        'Content-Range': `bytes ${start}-${end}/${1000000}`,
        'Accept-Ranges': 'bytes',
      },
      body: createPartialContent(start, end),
    };
  }
  
  return {
    status: 200,
    headers: { 'Content-Length': '1000000' },
    body: createFullContent(),
  };
});
```

### 9.4 Latency Injection

```typescript
// Example: Latency injection for performance testing

import { NetworkSandbox } from '@opencontainer/core';

const sandbox = new NetworkSandbox('latency-test');
const { interceptor } = sandbox;

// Latency profiles
const LATENCY_PROFILES = {
  // Fiber/DSL
  'fiber': { min: 1, max: 10 },
  '4g': { min: 50, max: 150 },
  '3g': { min: 100, max: 500 },
  '2g': { min: 500, max: 2000 },
  'satellite': { min: 500, max: 1000 },
};

interface LatencyProfile {
  min: number;
  max: number;
}

// Apply latency profile
function applyLatencyProfile(profile: LatencyProfile) {
  return async (req: NetworkRequest) => {
    const delay = profile.min + Math.random() * (profile.max - profile.min);
    await new Promise(r => setTimeout(r, delay));
    return passthrough(req);
  };
}

// Set latency profile
interceptor.intercept('*', /.*/, applyLatencyProfile(LATENCY_PROFILES['3g']));

// Variable latency based on endpoint
interceptor.get('/api/search', async (req) => {
  // Search is typically slower
  const delay = 200 + Math.random() * 800;
  await new Promise(r => setTimeout(r, delay));
  return passthrough(req);
});

interceptor.get('/api/users', async (req) => {
  // User data is typically cached/fast
  const delay = 10 + Math.random() * 50;
  await new Promise(r => setTimeout(r, delay));
  return passthrough(req);
});

// Request size-based latency
interceptor.intercept('POST', /.*/, async (req) => {
  // Larger requests take longer
  const contentLength = parseInt(req.headers['content-length'] || '0', 10);
  const baseDelay = 50;
  const perKbDelay = 10; // 10ms per KB
  const delay = baseDelay + (contentLength / 1024) * perKbDelay;
  
  await new Promise(r => setTimeout(r, delay));
  return passthrough(req);
});

// Response size-based latency
interceptor.get('/api/data', async (req) => {
  const response = await passthrough(req);
  
  // Simulate download time based on response size
  const contentLength = parseInt(response.headers['content-length'] || '0', 10);
  const downloadTime = contentLength / 1024; // ~1ms per KB
  
  await new Promise(r => setTimeout(r, downloadTime));
  return response;
});
```

---

## 10. Production Patterns

### 10.1 Complete Network Interceptor Implementation

```typescript
// packages/core/src/network/interceptor.ts - Production Implementation

import { HttpMethod, HttpStatusCode, type NetworkRequest, type NetworkResponse, type NetworkHandler, type RouteEntry, type RouteRegistration, type InterceptorContext, type RequestLog } from './types';
import { PatternMatcher } from './pattern-matcher';
import { ResponseBuilder } from './response-builder';

/**
 * Production-ready Network Interceptor
 * 
 * Features:
 * - Route priority and ordering
 * - Request/response logging
 * - Global fetch/XHR mocking
 * - Pattern matching with path parameters
 * - Async handler support
 * - Error handling and recovery
 */
export class NetworkInterceptorImpl implements NetworkInterceptor {
  private enabled: boolean = false;
  private routes: Map<string, RouteEntry> = new Map();
  private routeOrder: string[] = [];
  private defaultHandler?: NetworkHandler;
  private context: InterceptorContext;
  private requestLog: RequestLog[] = [];
  private maxLogSize: number = 1000;
  private passthroughMode: boolean = false;

  constructor(context?: Partial<InterceptorContext>) {
    this.context = {
      containerId: context?.containerId || 'default',
      baseUrl: context?.baseUrl || 'http://localhost',
      timeout: context?.timeout || 30000,
      originalFetch: globalThis.fetch,
    };
  }

  // ==================== Route Registration ====================

  enable(): void {
    if (this.enabled) return;
    this.enabled = true;
    this.installGlobalMocks();
  }

  disable(): void {
    if (!this.enabled) return;
    this.enabled = false;
    this.restoreGlobalMocks();
  }

  isEnabled(): boolean {
    return this.enabled;
  }

  intercept(
    method: HttpMethod | HttpMethod[] | '*',
    pattern: string | RegExp,
    handler: NetworkHandler
  ): RouteRegistration {
    const id = this.generateRouteId(method, pattern);
    const methods = this.normalizeMethods(method);
    const normalizedPattern = this.normalizePattern(pattern);

    const entry: RouteEntry = {
      id,
      methods,
      pattern: normalizedPattern,
      handler,
      priority: this.routeOrder.length,
      createdAt: Date.now(),
      hitCount: 0,
    };

    this.routes.set(id, entry);
    this.routeOrder.push(id);
    this.routeOrder.sort((a, b) => {
      const routeA = this.routes.get(a)!;
      const routeB = this.routes.get(b)!;
      return routeA.priority - routeB.priority;
    });

    return {
      id,
      entry,
      unregister: () => {
        const existed = this.routes.delete(id);
        if (existed) {
          const index = this.routeOrder.indexOf(id);
          if (index > -1) this.routeOrder.splice(index, 1);
        }
        return existed;
      },
      update: (newHandler: NetworkHandler) => {
        entry.handler = newHandler;
      },
    };
  }

  get(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration {
    return this.intercept(HttpMethod.GET, pattern, handler);
  }

  post(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration {
    return this.intercept(HttpMethod.POST, pattern, handler);
  }

  put(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration {
    return this.intercept(HttpMethod.PUT, pattern, handler);
  }

  delete(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration {
    return this.intercept(HttpMethod.DELETE, pattern, handler);
  }

  patch(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration {
    return this.intercept(HttpMethod.PATCH, pattern, handler);
  }

  any(pattern: string | RegExp, handler: NetworkHandler): RouteRegistration {
    return this.intercept('*', pattern, handler);
  }

  setDefaultHandler(handler: NetworkHandler): void {
    this.defaultHandler = handler;
  }

  clearRoutes(): void {
    this.routes.clear();
    this.routeOrder = [];
  }

  // ==================== Request Processing ====================

  async processRequest(request: NetworkRequest): Promise<NetworkResponse> {
    const startTime = Date.now();
    const logEntry: RequestLog = {
      url: request.url,
      method: request.method,
      headers: request.headers,
      timestamp: startTime,
    };

    try {
      if (!this.enabled || this.passthroughMode) {
        const response = await this.passthrough(request);
        logEntry.response = { status: response.status, headers: response.headers };
        logEntry.duration = Date.now() - startTime;
        this.log(logEntry);
        return response;
      }

      const match = this.findMatchingRoute(request);
      
      if (match) {
        const { entry, params } = match;
        entry.hitCount++;

        const context = { ...this.context, params };
        const response = await entry.handler(request, context);
        
        logEntry.response = { status: response.status, headers: response.headers };
        logEntry.duration = Date.now() - startTime;
        this.log(logEntry);
        
        return this.normalizeResponse(response, request);
      }

      if (this.defaultHandler) {
        const response = await this.defaultHandler(request, this.context);
        logEntry.response = { status: response.status, headers: response.headers };
        logEntry.duration = Date.now() - startTime;
        this.log(logEntry);
        return response;
      }

      const response = this.createNotFoundResponse(request);
      logEntry.response = { status: response.status, headers: response.headers };
      logEntry.duration = Date.now() - startTime;
      this.log(logEntry);
      return response;

    } catch (error) {
      const response = this.createErrorResponse(request, error as Error);
      logEntry.response = { status: response.status, headers: response.headers };
      logEntry.duration = Date.now() - startTime;
      this.log(logEntry);
      return response;
    }
  }

  // ==================== Global Mock Installation ====================

  private installGlobalMocks(): void {
    const originalFetch = this.context.originalFetch;
    const self = this;

    globalThis.fetch = async function(input: RequestInfo | URL, init?: RequestInit) {
      const request = self.createFetchRequest(input, init);
      const response = await self.processRequest(request);
      return self.toFetchResponse(response);
    };

    this.installXHRMock();
  }

  private restoreGlobalMocks(): void {
    if (this.context.originalFetch) {
      globalThis.fetch = this.context.originalFetch;
    }
    this.restoreXHRMock();
  }

  // ==================== Utilities ====================

  private findMatchingRoute(request: NetworkRequest): { entry: RouteEntry; params: Record<string, string> } | null {
    for (const id of this.routeOrder) {
      const entry = this.routes.get(id);
      if (!entry) continue;

      const result = PatternMatcher.match(entry.pattern, request.url);
      if (result.matched) {
        const methodMatch = entry.methods.includes('*') || entry.methods.includes(request.method);
        if (methodMatch) {
          return { entry, params: result.params };
        }
      }
    }
    return null;
  }

  private normalizeResponse(response: NetworkResponse, request: NetworkRequest): NetworkResponse {
    return {
      ...response,
      url: response.url || request.url,
      ok: response.status >= 200 && response.status < 300,
    };
  }

  private createFetchRequest(input: RequestInfo | URL, init?: RequestInit): NetworkRequest {
    let url: string;
    let method = 'GET';
    let headers: Record<string, string> = {};
    let body: ReadableStream<Uint8Array> | null = null;

    if (typeof input === 'string') {
      url = input;
    } else if (input instanceof URL) {
      url = input.toString();
    } else {
      url = input.url;
      method = input.method;
      headers = Object.fromEntries(input.headers.entries());
      body = input.body;
    }

    if (init) {
      method = init.method || method;
      if (init.headers) {
        headers = { ...headers, ...Object.fromEntries(new Headers(init.headers).entries()) };
      }
      if (init.body) {
        body = this.bodyToStream(init.body);
      }
    }

    return { url, method, headers, body };
  }

  private toFetchResponse(networkResponse: NetworkResponse): Response {
    return new Response(networkResponse.body, {
      status: networkResponse.status,
      statusText: networkResponse.statusText,
      headers: networkResponse.headers,
    });
  }

  private createNotFoundResponse(request: NetworkRequest): NetworkResponse {
    return {
      status: 404,
      statusText: 'Not Found',
      headers: { 'Content-Type': 'application/json' },
      body: this.stringToStream(JSON.stringify({
        error: 'No mock handler found',
        method: request.method,
        url: request.url,
      })),
      url: request.url,
    };
  }

  private createErrorResponse(request: NetworkRequest, error: Error): NetworkResponse {
    return {
      status: 500,
      statusText: 'Internal Server Error',
      headers: { 'Content-Type': 'application/json' },
      body: this.stringToStream(JSON.stringify({
        error: error.message,
        stack: error.stack,
      })),
      url: request.url,
    };
  }

  private async passthrough(request: NetworkRequest): Promise<NetworkResponse> {
    const response = await fetch(request.url, {
      method: request.method,
      headers: request.headers,
      body: request.body,
    });

    return {
      status: response.status,
      statusText: response.statusText,
      headers: Object.fromEntries(response.headers.entries()),
      body: response.body,
      url: request.url,
    };
  }

  private log(entry: RequestLog): void {
    this.requestLog.push(entry);
    if (this.requestLog.length > this.maxLogSize) {
      this.requestLog.shift();
    }
  }

  getLog(): RequestLog[] {
    return [...this.requestLog];
  }

  clearLog(): void {
    this.requestLog = [];
  }

  // ==================== Helpers ====================

  private generateRouteId(method: HttpMethod | HttpMethod[] | '*', pattern: string | RegExp): string {
    const methods = this.normalizeMethods(method).join('-');
    const patternStr = pattern instanceof RegExp ? pattern.source : pattern;
    return `${methods}:${patternStr}`;
  }

  private normalizeMethods(method: HttpMethod | HttpMethod[] | '*'): (HttpMethod | '*')[] {
    return Array.isArray(method) ? method : [method];
  }

  private normalizePattern(pattern: string | RegExp): RegExp | string {
    if (pattern instanceof RegExp) return pattern;
    if (pattern.includes('*')) {
      return new RegExp(`^${pattern.replace(/\*/g, '.*')}$`);
    }
    return pattern;
  }

  private bodyToStream(body: any): ReadableStream<Uint8Array> | null {
    if (body instanceof ReadableStream) return body;
    if (typeof body === 'string') return this.stringToStream(body);
    if (body instanceof Uint8Array) {
      return new ReadableStream({
        start(controller) {
          controller.enqueue(body);
          controller.close();
        },
      });
    }
    return null;
  }

  private stringToStream(str: string): ReadableStream<Uint8Array> {
    return new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(str));
        controller.close();
      },
    });
  }

  // XHR mock implementations
  private installXHRMock(): void {}
  private restoreXHRMock(): void {}
}
```

### 10.2 Architecture Diagrams

#### Complete System Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           OpenContainer Network Stack                            │
└─────────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────────┐
│                              Application Layer                                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │   fetch()   │  │ XMLHttpRequest │  │  http.get()  │  │ WebSocket() │            │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘            │
└─────────┼────────────────┼────────────────┼────────────────┼────────────────────┘
          │                │                │                │
          └────────────────┴────────────────┴────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Network Simulation Layer                               │
│                                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │                         Request Router                                   │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │    │
│  │  │   Pattern   │  │   Method    │  │  Priority   │  │    Route    │    │    │
│  │  │   Matcher   │  │   Checker   │  │   Sorter    │  │   Registry  │    │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                   │                                              │
│         ┌─────────────────────────┼─────────────────────────┐                   │
│         │                         │                         │                   │
│         ▼                         ▼                         │                   │
│  ┌─────────────┐          ┌─────────────┐                   │                   │
│  │    Fetch    │          │   Module    │                   │                   │
│  │  Interceptor│          │   Mocks     │                   │                   │
│  │             │          │  ┌───────┐  │                   │                   │
│  │  ┌───────┐  │          │  │ http │  │                   │                   │
│  │  │global │  │          │  └───────┘  │                   │                   │
│  │  │fetch  │  │          │  ┌───────┐  │                   │                   │
│  │  └───────┘  │          │  │https │  │                   │                   │
│  │             │          │  └───────┘  │                   │                   │
│  │  ┌───────┐  │          │  ┌───────┐  │                   │                   │
│  │  │  XHR  │  │          │  │  net  │  │                   │                   │
│  │  └───────┘  │          │  └───────┘  │                   │                   │
│  └─────────────┘          └─────────────┘                   │                   │
│                                                            │                   │
│                                         ┌──────────────────▼────────────────┐  │
│                                         │         Handler Pool               │  │
│                                         │  ┌─────────────────────────────┐  │  │
│                                         │  │  Static Response Handlers   │  │  │
│                                         │  │  Dynamic Response Handlers  │  │  │
│                                         │  │  Passthrough Handlers       │  │  │
│                                         │  │  Error Handlers             │  │  │
│                                         │  └─────────────────────────────┘  │  │
│                                         └───────────────────────────────────┘  │
│                                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │                        Response Builder                                  │    │
│  │   Status │ Headers │ Body (Stream) │ Delay │ Transform                 │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │                          Request Logger                                  │    │
│  │   URL │ Method │ Timestamp │ Duration │ Response Status │ Headers      │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────────┘
                                   │
                                   ▼ (No real network)
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           Simulated Responses                                    │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Request/Response Sequence Diagram

```
┌─────────────┐    ┌──────────────┐    ┌───────────────┐    ┌─────────────┐    ┌──────────────┐
│ Application │    │ Global Mock  │    │   Interceptor │    │   Handler   │    │   Response  │
│             │    │   (fetch)    │    │               │    │             │    │   Builder   │
└──────┬──────┘    └──────┬───────┘    └───────┬───────┘    └──────┬──────┘    └──────┬───────┘
       │                  │                    │                   │                  │
       │ fetch('/api/x')  │                    │                   │                  │
       │─────────────────▶│                    │                   │                  │
       │                  │                    │                   │                  │
       │                  │ normalizeRequest   │                   │                  │
       │                  │───────────────────▶│                   │                  │
       │                  │                    │                   │                  │
       │                  │                    │ findMatchingRoute │                  │
       │                  │                    │─────────┐         │                  │
       │                  │                    │         │         │                  │
       │                  │                    │◀────────┘         │                  │
       │                  │                    │                   │                  │
       │                  │                    │ executeHandler    │                  │
       │                  │                    │──────────────────▶│                  │
       │                  │                    │                   │                  │
       │                  │                    │                   │ processRequest   │
       │                  │                    │                   │─────────────────▶│
       │                  │                    │                   │                  │
       │                  │                    │                   │◀─────────────────│
       │                  │                    │                   │  NetworkResponse │
       │                  │                    │◀──────────────────│                  │
       │                  │                    │                   │                  │
       │                  │                    │ toFetchResponse   │                  │
       │                  │◀───────────────────│                   │                  │
       │                  │                    │                   │                  │
       │ Response         │                    │                   │                  │
       │◀─────────────────│                    │                   │                  │
       │                  │                    │                   │                  │
```

### 10.3 Example Mocking Setups

#### Complete API Mock Setup

```typescript
// examples/api-mock-setup.ts

import { NetworkSandbox, HttpMethod, HttpStatusCode } from '@opencontainer/core';

// Create sandbox
const sandbox = new NetworkSandbox('api-mock-example');
const { interceptor } = sandbox;

// Mock REST API
function setupRestApiMocks() {
  // In-memory "database"
  const users = [
    { id: '1', name: 'Alice', email: 'alice@example.com' },
    { id: '2', name: 'Bob', email: 'bob@example.com' },
  ];

  // GET /api/users - List all users
  interceptor.get('/api/users', () => ({
    status: HttpStatusCode.OK,
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(users),
  }));

  // GET /api/users/:id - Get single user
  interceptor.get(/\/api\/users\/(.+)/, (req) => {
    const id = req.url.match(/\/users\/(.+)/)?.[1];
    const user = users.find(u => u.id === id);
    
    if (!user) {
      return {
        status: HttpStatusCode.NotFound,
        body: JSON.stringify({ error: 'User not found' }),
      };
    }

    return {
      status: HttpStatusCode.OK,
      body: JSON.stringify(user),
    };
  });

  // POST /api/users - Create user
  interceptor.post('/api/users', async (req) => {
    const body = await readBody(req.body);
    const newUser = {
      id: String(Date.now()),
      ...body,
    };
    users.push(newUser);

    return {
      status: HttpStatusCode.Created,
      headers: { 'Location': `/api/users/${newUser.id}` },
      body: JSON.stringify(newUser),
    };
  });

  // PUT /api/users/:id - Update user
  interceptor.put(/\/api\/users\/(.+)/, async (req) => {
    const id = req.url.match(/\/users\/(.+)/)?.[1];
    const userIndex = users.findIndex(u => u.id === id);
    
    if (userIndex === -1) {
      return {
        status: HttpStatusCode.NotFound,
        body: JSON.stringify({ error: 'User not found' }),
      };
    }

    const body = await readBody(req.body);
    users[userIndex] = { ...users[userIndex], ...body };

    return {
      status: HttpStatusCode.OK,
      body: JSON.stringify(users[userIndex]),
    };
  });

  // DELETE /api/users/:id - Delete user
  interceptor.delete(/\/api\/users\/(.+)/, (req) => {
    const id = req.url.match(/\/users\/(.+)/)?.[1];
    const userIndex = users.findIndex(u => u.id === id);
    
    if (userIndex === -1) {
      return { status: HttpStatusCode.NotFound };
    }

    users.splice(userIndex, 1);
    return { status: HttpStatusCode.NoContent };
  });
}

// Mock GraphQL endpoint
function setupGraphQLMocks() {
  interceptor.post('/graphql', async (req) => {
    const body = await readBody(req.body);
    const { query, variables } = body;

    // Simple GraphQL mock response
    const response = {
      data: {
        users: users.map(u => ({
          id: u.id,
          name: u.name,
          email: u.email,
        })),
      },
    };

    return {
      status: HttpStatusCode.OK,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(response),
    };
  });
}

// Helper: Read request body
async function readBody(body: ReadableStream<Uint8Array> | null): Promise<any> {
  if (!body) return {};
  
  const reader = body.getReader();
  const chunks: Uint8Array[] = [];
  
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
  }
  
  const total = chunks.reduce((acc, c) => acc + c.length, 0);
  const result = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    result.set(chunk, offset);
    offset += chunk.length;
  }
  
  return JSON.parse(new TextDecoder().decode(result));
}

// Enable mocks
interceptor.enable();
setupRestApiMocks();
setupGraphQLMocks();

console.log('API mocks enabled!');
```

---

## Appendix: File Reference

### Core Network Files

```
packages/core/src/network/
├── interceptor.ts        # HttpInterceptor class
├── manager.ts            # NetworkManager for orchestration
├── types.ts              # Network types and interfaces
├── pattern-matcher.ts    # URL pattern matching
├── response-builder.ts   # Response construction
├── fetch-mock.ts         # Global fetch override
├── http-server.ts        # HTTP server emulation
├── websocket.ts          # WebSocket emulation
└── port-manager.ts       # Port allocation

packages/core/src/process/executors/node/modules/
├── http.ts               # http module mock
├── https.ts              # https module mock
├── net.ts                # net module mock
├── httpMock.ts           # HTTP utilities
└── network.ts            # Network utilities
```

---

*Generated: 2026-04-05*
