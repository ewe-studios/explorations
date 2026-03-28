---
title: "Zero Client Deep Dive"
subtitle: "Complete guide to Zero's client architecture, connection management, and local caching"
---

# Zero Client Deep Dive

## 1. Overview

This document provides a comprehensive deep dive into Zero's client implementation, covering:

- Zero client class structure
- Connection manager
- Local SQLite cache
- Query subscription
- Optimistic mutations
- Connection status handling

## 2. Zero Client Class

### 2.1 Main Class Structure

```typescript
class Zero {
  // Core configuration
  private readonly schema: Schema;
  private readonly userID: string;
  private readonly serverURL: string;

  // Internal components
  private connectionManager: ConnectionManager;
  private queryCache: LocalSQLiteCache;
  private mutationQueue: MutationQueue;
  private activeClientsManager: ActiveClientsManager;

  // Public APIs
  public readonly query: QueryAPI;
  public readonly mutate: () => MutationAPI;
  public readonly connectionStatus: Observable<ConnectionStatus>;
  public readonly inspector: Inspector;

  constructor(options: ZeroOptions) {
    this.schema = options.schema;
    this.userID = options.userID;
    this.serverURL = options.server;

    this.connectionManager = new ConnectionManager(this.serverURL);
    this.queryCache = new LocalSQLiteCache(this.schema);
    this.mutationQueue = new MutationQueue();

    this.query = new QueryAPI(this.connectionManager, this.queryCache);
    this.mutate = () => new MutationAPI(this.connectionManager, this.mutationQueue);
  }

  async connect(): Promise<void> {
    await this.connectionManager.connect();
    await this.queryCache.init(this.schema);
  }

  disconnect(): void {
    this.connectionManager.disconnect();
  }
}
```

### 2.2 Public API

```typescript
// Query API
const query = zero.query.issue
  .where('status', 'open')
  .orderBy('created', 'desc')
  .limit(10);

const view = query.materialize((view) => {
  view.addListener((changes) => {
    console.log('Changes:', changes);
  });
});

// Mutation API
const result = await zero.mutate().issue.insert({
  id: 'issue-123',
  title: 'New bug',
  status: 'open',
  created: Date.now(),
});

// Connection status
zero.connectionStatus.subscribe((status) => {
  console.log('Connection:', status);
  // 'connecting' | 'connected' | 'disconnected' | 'reconnecting'
});
```

## 3. Connection Manager

### 3.1 WebSocket Connection

```typescript
class ConnectionManager {
  private ws: WebSocket | null = null;
  private status: ConnectionStatus = 'connecting';
  private retryCount = 0;
  private messageHandlers: Set<(msg: ServerMessage) => void> = new Set();

  async connect(): Promise<void> {
    const url = this.buildWebSocketURL();
    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      this.status = 'connected';
      this.retryCount = 0;
      this.sendSubscribeMessages();
    };

    this.ws.onclose = (event) => {
      this.status = 'disconnected';
      this.scheduleReconnect();
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };

    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      this.handleServerMessage(message);
    };

    return this.waitForConnected();
  }

  private buildWebSocketURL(): string {
    const params = new URLSearchParams({
      userID: this.userID,
      clientID: this.clientID,
      schemaVersion: this.schema.version.toString(),
    });
    return `${this.serverURL}/ws?${params}`;
  }

  private scheduleReconnect(): void {
    const delay = Math.min(
      1000 * Math.pow(2, this.retryCount),
      30000 // Max 30 seconds
    );
    this.retryCount++;
    setTimeout(() => this.connect(), delay);
  }

  send(message: ClientMessage): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }
}
```

### 3.2 Connection State Machine

```
                        ┌──────────────┐
                        │              │
                        │  Connecting  │
                        │              │
                        └──────┬───────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
         Success          Auth Failed     Network Error
              │                │                │
              ▼                ▼                ▼
    ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
    │              │  │              │  │              │
    │  Connected   │  │   Error      │  │ Disconnected │
    │              │  │              │  │              │
    └──────┬───────┘  └──────────────┘  └──────┬───────┘
           │                                   │
           │ Server close /                    │
           │ Network error                     │ Retry
           │                                   │
           └───────────────────────────────────┘
```

### 3.3 Message Handling

```typescript
class ConnectionManager {
  private handleServerMessage(message: ServerMessage): void {
    switch (message.type) {
      case 'pull-response':
        this.handlePullResponse(message);
        break;

      case 'changes':
        this.handleChanges(message);
        break;

      case 'mutation-result':
        this.handleMutationResult(message);
        break;

      case 'error':
        this.handleError(message);
        break;
    }
  }

  private handlePullResponse(message: PullResponseMessage): void {
    const { queryID, results } = message;
    this.queryCache.applyResults(queryID, results);
    this.notifyQueryReady(queryID);
  }

  private handleChanges(message: ChangesMessage): void {
    const { queryID, changes } = message;

    // Apply changes to local cache
    this.queryCache.applyChanges(changes);

    // Notify subscribers
    this.notifyQuerySubscribers(queryID, changes);

    // Send acknowledgment
    this.send({
      type: 'ack',
      queryID,
      changeIDs: changes.map(c => c.id),
    });
  }

  private handleMutationResult(message: MutationResultMessage): void {
    const { mutationID, success, error } = message;
    this.mutationQueue.confirm(mutationID, success, error);
  }
}
```

## 4. Local SQLite Cache

### 4.1 Schema Initialization

```typescript
class LocalSQLiteCache {
  private db: SQLiteDB;
  private schema: Schema;
  private queryResults: Map<string, Set<string>> = new Map(); // queryID -> row IDs

  constructor(schema: Schema) {
    this.schema = schema;
  }

  async init(schema: Schema): Promise<void> {
    const tx = await this.db.begin();

    // Create tables for each schema table
    for (const [tableName, tableSchema] of Object.entries(schema.tables)) {
      const columns = this.buildColumnDefs(tableSchema.columns);
      const primaryKey = tableSchema.primaryKey;

      await tx.exec(`
        CREATE TABLE IF NOT EXISTS ${tableName} (
          ${columns},
          PRIMARY KEY (${primaryKey})
        )
      `);

      // Create indexes
      for (const index of tableSchema.indexes || []) {
        await tx.exec(`
          CREATE INDEX IF NOT EXISTS ${index.name}
          ON ${tableName} (${index.columns.join(', ')})
        `);
      }
    }

    // Create metadata table for tracking
    await tx.exec(`
      CREATE TABLE IF NOT EXISTS _zero_metadata (
        key TEXT PRIMARY KEY,
        value TEXT
      )
    `);

    await tx.commit();
  }

  private buildColumnDefs(columns: Record<string, ColumnSchema>): string {
    return Object.entries(columns)
      .map(([name, schema]) => `${name} ${this.mapType(schema.type)}`)
      .join(',\n');
  }

  private mapType(type: string): string {
    switch (type) {
      case 'string': return 'TEXT';
      case 'number': return 'REAL';
      case 'boolean': return 'INTEGER';
      case 'json': return 'TEXT'; // JSON stored as string
      default: return 'TEXT';
    }
  }
}
```

### 4.2 Query Subscription

```typescript
class LocalSQLiteCache {
  async subscribeToQuery(
    queryID: string,
    ast: AST
  ): Promise<Row[]> {
    // Store query mapping
    this.queryResults.set(queryID, new Set());

    // For simple queries, we can execute directly on SQLite
    if (this.canExecuteLocally(ast)) {
      const sql = this.compileToSQL(ast);
      const rows = await this.db.all(sql);

      // Track result set
      const rowIDs = new Set(rows.map(r => r.id));
      this.queryResults.set(queryID, rowIDs);

      return rows;
    }

    // Complex queries need server-side IVM
    return [];
  }

  async applyChanges(changes: Change[]): Promise<void> {
    const tx = await this.db.begin();

    for (const change of changes) {
      switch (change.type) {
        case 'add':
          await tx.run(
            `INSERT OR REPLACE INTO ${change.relation} (...) VALUES (...)`,
            this.rowToValues(change.node.row)
          );
          break;

        case 'remove':
          await tx.run(
            `DELETE FROM ${change.relation} WHERE id = ?`,
            change.node.row.id
          );
          break;

        case 'edit':
          await tx.run(
            `UPDATE ${change.relation} SET ... WHERE id = ?`,
            [...this.rowToUpdateValues(change.node.row), change.node.row.id]
          );
          break;
      }
    }

    await tx.commit();

    // Notify query subscribers
    this.notifySubscribers(changes);
  }

  private canExecuteLocally(ast: AST): boolean {
    // Can execute locally if:
    // - Single table (no joins)
    // - Simple filters (equality, comparison)
    // - OrderBy on indexed columns
    // - Limit/Offset

    return !ast.join &&
           ast.where !== undefined &&
           this.isSimpleWhere(ast.where);
  }
}
```

### 4.3 Optimistic Updates

```typescript
class LocalSQLiteCache {
  private optimisticChanges: Map<string, Change> = new Map();

  async applyOptimisticChange(change: Change): Promise<void> {
    // Store for potential rollback
    const changeID = generateChangeID();
    this.optimisticChanges.set(changeID, change);

    // Apply to cache immediately
    await this.applyChanges([change]);
  }

  async confirmOptimisticChange(changeID: string): Promise<void> {
    this.optimisticChanges.delete(changeID);
  }

  async rollbackOptimisticChange(changeID: string): Promise<void> {
    const change = this.optimisticChanges.get(changeID);
    if (!change) return;

    // Reverse the change
    const reverseChange = this.createReverseChange(change);
    await this.applyChanges([reverseChange]);

    this.optimisticChanges.delete(changeID);
  }

  private createReverseChange(change: Change): Change {
    switch (change.type) {
      case 'add':
        return { type: 'remove', node: change.node };
      case 'remove':
        return { type: 'add', node: change.node };
      case 'edit':
        return {
          type: 'edit',
          node: change.oldNode,
          oldNode: change.node,
        };
      default:
        return change;
    }
  }
}
```

## 5. Query API

### 5.1 Query Builder

```typescript
class QueryBuilder<T extends keyof SchemaTables> {
  private ast: AST = {
    type: 'select',
    table: T as string,
  };

  where(column: string, operator: string, value: unknown): QueryBuilder<T> {
    const condition: SimpleCondition = {
      type: 'simple',
      left: { type: 'column', column },
      operator: operator as SimpleOperator,
      right: { type: 'literal', value },
    };

    if (this.ast.where === undefined) {
      this.ast.where = condition;
    } else {
      this.ast.where = {
        type: 'conjunction',
        conditions: [this.ast.where, condition],
      };
    }

    return this;
  }

  orderBy(column: string, direction: 'asc' | 'desc' = 'asc'): QueryBuilder<T> {
    this.ast.orderBy = {
      columns: [{ column, direction }],
    };
    return this;
  }

  limit(n: number): QueryBuilder<T> {
    this.ast.limit = n;
    return this;
  }

  materialize(callback: (view: MaterializedView<T>) => void): MaterializedView<T> {
    const view = new MaterializedView(this.ast, callback);
    return view;
  }
}
```

### 5.2 Materialized View

```typescript
class MaterializedView<T extends keyof SchemaTables> {
  private listeners: Set<(changes: Change[]) => void> = new Set();
  private rows: Map<string, Row> = new Map();
  private unsubscribe: (() => void) | null = null;

  constructor(
    private ast: AST,
    private callback: (view: MaterializedView<T>) => void
  ) {
    this.subscribe();
  }

  private async subscribe(): Promise<void> {
    // Get initial results
    const initialRows = await zeroClient.queryCache.subscribeToQuery(
      this.queryID,
      this.ast
    );

    for (const row of initialRows) {
      this.rows.set(row.id, row);
    }

    // Call callback with initial data
    this.callback(this);

    // Subscribe to changes
    this.unsubscribe = zeroClient.connectionManager.subscribe(
      'changes',
      (message: ChangesMessage) => {
        if (message.queryID === this.queryID) {
          this.applyChanges(message.changes);
        }
      }
    );
  }

  private applyChanges(changes: Change[]): void {
    for (const change of changes) {
      switch (change.type) {
        case 'add':
          this.rows.set(change.node.row.id, change.node.row);
          break;
        case 'remove':
          this.rows.delete(change.node.row.id);
          break;
        case 'edit':
          this.rows.set(change.node.row.id, change.node.row);
          break;
      }
    }

    // Notify listeners
    for (const listener of this.listeners) {
      listener(changes);
    }
  }

  addListener(listener: (changes: Change[]) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  getRows(): Row[] {
    return Array.from(this.rows.values());
  }

  destroy(): void {
    this.unsubscribe?.();
    this.listeners.clear();
    this.rows.clear();
  }
}
```

## 6. Mutation API

### 6.1 Mutation Queue

```typescript
class MutationQueue {
  private pending: Map<string, PendingMutation> = new Map();
  private processing: boolean = false;

  async enqueue(mutation: MutationRequest): Promise<MutationResult> {
    const mutationID = generateMutationID();
    const pending: PendingMutation = {
      id: mutationID,
      request: mutation,
      status: 'pending',
      createdAt: Date.now(),
    };

    this.pending.set(mutationID, pending);

    // Apply optimistic update
    await this.applyOptimisticUpdate(mutation);

    // Start processing if not already
    if (!this.processing) {
      this.processQueue();
    }

    // Return promise that resolves when mutation completes
    return new Promise((resolve, reject) => {
      pending.resolve = resolve;
      pending.reject = reject;
    });
  }

  private async processQueue(): Promise<void> {
    this.processing = true;

    while (this.pending.size > 0) {
      const [mutationID, mutation] = this.pending.entries().next().value;

      try {
        mutation.status = 'processing';

        const result = await zeroClient.connectionManager.sendMutation(
          mutation.request
        );

        if (result.success) {
          mutation.status = 'confirmed';
          this.pending.delete(mutationID);
          mutation.resolve(result);
        } else {
          throw new Error(result.error);
        }
      } catch (error) {
        mutation.status = 'failed';

        // Rollback optimistic update
        await this.rollbackOptimisticUpdate(mutation.request);

        this.pending.delete(mutationID);
        mutation.reject(error);
      }
    }

    this.processing = false;
  }
}
```

### 6.2 CRUD Mutations

```typescript
class MutationAPI {
  constructor(
    private connectionManager: ConnectionManager,
    private mutationQueue: MutationQueue
  ) {}

  issue = {
    insert: (value: IssueInsert): Promise<MutationResult> => {
      return this.mutationQueue.enqueue({
        type: 'insert',
        table: 'issue',
        value,
      });
    },

    update: (value: IssueUpdate): Promise<MutationResult> => {
      return this.mutationQueue.enqueue({
        type: 'update',
        table: 'issue',
        value,
        where: { id: value.id },
      });
    },

    delete: (id: string): Promise<MutationResult> => {
      return this.mutationQueue.enqueue({
        type: 'delete',
        table: 'issue',
        where: { id },
      });
    },
  };

  // Generic method for custom mutations
  async mutate<T>(mutator: string, args: unknown): Promise<T> {
    return this.mutationQueue.enqueue({
      type: 'custom',
      mutator,
      args,
    });
  }
}
```

## 7. Inspector (Debug UI)

### 7.1 Inspector Class

```typescript
class Inspector {
  private clientGroup: ClientGroup;
  private dialog: HTMLDialogElement;

  constructor(private zero: Zero) {
    this.clientGroup = new ClientGroup(zero);
    this.setupDialog();
  }

  private setupDialog(): void {
    this.dialog = document.createElement('dialog');
    this.dialog.className = 'zero-inspector';
    this.dialog.innerHTML = this.renderTemplate();
    document.body.appendChild(this.dialog);

    // Keyboard shortcut to open (Ctrl+Shift+Z)
    document.addEventListener('keydown', (e) => {
      if (e.ctrlKey && e.shiftKey && e.key === 'Z') {
        this.dialog.showModal();
      }
    });
  }

  private renderTemplate(): string {
    return `
      <div class="inspector-content">
        <h2>Zero Inspector</h2>

        <section>
          <h3>Connection Status</h3>
          <div id="connection-status"></div>
        </section>

        <section>
          <h3>Active Queries</h3>
          <div id="active-queries"></div>
        </section>

        <section>
          <h3>Pending Mutations</h3>
          <div id="pending-mutations"></div>
        </section>

        <section>
          <h3>Client Group</h3>
          <div id="client-group"></div>
        </section>
      </div>
    `;
  }

  update(): void {
    this.updateConnectionStatus();
    this.updateActiveQueries();
    this.updatePendingMutations();
    this.updateClientGroup();
  }

  private updateConnectionStatus(): void {
    const statusEl = this.dialog.querySelector('#connection-status');
    const status = this.zero.connectionStatus.getValue();
    statusEl!.textContent = status;
  }
}
```

---

*Next: [04-server-services-deep-dive.md](04-server-services-deep-dive.md)*
