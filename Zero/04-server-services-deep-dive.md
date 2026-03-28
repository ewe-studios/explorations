---
title: "Zero Server Services Deep Dive"
subtitle: "Complete guide to Zero's server architecture and service implementation"
---

# Zero Server Services Deep Dive

## 1. Overview

This document provides a comprehensive deep dive into Zero's server-side services:

- Change Source (PostgreSQL logical replication)
- Change Streamer
- Replicator
- Mutagen (mutation processing)
- View Syncer
- Worker architecture

## 2. Service Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Zero Cache Server                     │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │                  Main Process                    │    │
│  │  - HTTP server (WebSocket upgrade)              │    │
│  │  - Worker dispatcher                            │    │
│  │  - OpenTelemetry integration                    │    │
│  └─────────────────────────────────────────────────┘    │
│                          │                               │
│         ┌────────────────┼────────────────┐             │
│         ▼                ▼                ▼             │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐       │
│  │  Worker 1   │ │  Worker 2   │ │  Worker N   │       │
│  │             │ │             │ │             │       │
│  │ ┌─────────┐ │ │ ┌─────────┐ │ │ ┌─────────┐ │       │
│  │ │ View    │ │ │ │ View    │ │ │ │ View    │ │       │
│  │ │ Syncer  │ │ │ │ Syncer  │ │ │ │ Syncer  │ │       │
│  │ └─────────┘ │ │ └─────────┘ │ │ └─────────┘ │       │
│  │ ┌─────────┐ │ │ ┌─────────┐ │ │ ┌─────────┐ │       │
│  │ │ Replica-│ │ │ │ Replica-│ │ │ │ Replica-│ │       │
│  │ │   tor   │ │ │ │   tor   │ │ │ │   tor   │ │       │
│  │ └─────────┘ │ │ └─────────┘ │ │ └─────────┘ │       │
│  └─────────────┘ └─────────────┘ └─────────────┘       │
│                                                          │
│  ┌─────────────────────────────────────────────────┐    │
│  │               Shared Services                    │    │
│  │  ┌──────────────┐  ┌──────────────┐            │    │
│  │  │ Change Source│  │   Mutagen    │            │    │
│  │  │ (PostgreSQL) │  │ (Mutations)  │            │    │
│  │  └──────────────┘  └──────────────┘            │    │
│  │  ┌──────────────┐  ┌──────────────┐            │    │
│  │  │ChangeStreamer│  │  Litestream  │            │    │
│  │  │              │  │  (Backups)   │            │    │
│  │  └──────────────┘  └──────────────┘            │    │
│  └─────────────────────────────────────────────────┘    │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

## 3. Change Source Service

### 3.1 PostgreSQL Logical Replication

```typescript
class ChangeSource {
  private client: PoolClient;
  private replicationSlot: string;
  private subscribers: Set<(change: Change) => void> = new Set();
  private currentLSN: string = '0/0';

  async connect(config: PostgreSQLConfig): Promise<void> {
    this.client = await pool.connect();

    // Create replication slot if not exists
    await this.client.query(`
      SELECT pg_create_logical_replication_slot(
        '${this.replicationSlot}',
        'pgoutput',
        false, false, false
      )
    `);

    // Start replication
    await this.client.query(`
      START_REPLICATION SLOT "${this.replicationSlot}" LOGICAL ${this.currentLSN}
      (proto_version '1', publication 'zero_publication')
    `);

    // Listen for copyData messages (WAL data)
    this.client.connection.on('copyData', (msg) => {
      this.handleWALMessage(msg);
    });

    // Handle replication feedback
    this.client.connection.on('replicationStart', () => {
      console.log('Replication started');
    });
  }

  private handleWALMessage(msg: Buffer): void {
    // Parse WAL message
    const message = this.parseWALMessage(msg);

    if (message.type === 'begin') {
      this.handleTransactionBegin(message);
    } else if (message.type === 'relation') {
      this.handleRelation(message);
    } else if (message.type === 'insert') {
      this.handleInsert(message);
    } else if (message.type === 'update') {
      this.handleUpdate(message);
    } else if (message.type === 'delete') {
      this.handleDelete(message);
    } else if (message.type === 'commit') {
      this.handleTransactionCommit(message);
    }

    // Update LSN
    this.currentLSN = message.lsn;
  }

  subscribe(handler: (change: Change) => void): () => void {
    this.subscribers.add(handler);
    return () => this.subscribers.delete(handler);
  }

  private notifySubscribers(change: Change): void {
    for (const handler of this.subscribers) {
      handler(change);
    }
  }
}
```

### 3.2 WAL Message Parsing

```typescript
class WALParser {
  parse(msg: Buffer): WALMessage {
    const type = msg.toString('utf8', 0, 1);

    switch (type) {
      case 'R': // Relation
        return this.parseRelation(msg);
      case 'I': // Insert
        return this.parseInsert(msg);
      case 'U': // Update
        return this.parseUpdate(msg);
      case 'D': // Delete
        return this.parseDelete(msg);
      case 'B': // Begin
        return this.parseBegin(msg);
      case 'C': // Commit
        return this.parseCommit(msg);
      default:
        throw new Error(`Unknown WAL message type: ${type}`);
    }
  }

  private parseRelation(msg: Buffer): RelationMessage {
    const reader = new BufferReader(msg);
    reader.skip(1); // Message type

    return {
      type: 'relation',
      relationId: reader.readInt32(),
      schemaName: reader.readString(),
      tableName: reader.readString(),
      replicaIdentity: reader.readInt8(),
      columns: this.parseColumns(reader),
    };
  }

  private parseInsert(msg: Buffer): InsertMessage {
    const reader = new BufferReader(msg);
    reader.skip(1); // Message type

    return {
      type: 'insert',
      relationId: reader.readInt32(),
      tupleKind: reader.readInt8(),
      columns: this.parseTuple(reader),
    };
  }

  private parseUpdate(msg: Buffer): UpdateMessage {
    const reader = new BufferReader(msg);
    reader.skip(1); // Message type

    return {
      type: 'update',
      relationId: reader.readInt32(),
      tupleKind: reader.readInt8(), // 'K'ey, 'O'ld, or 'N'ew
      keyColumns: this.parseTuple(reader),
      newColumns: this.parseTuple(reader),
    };
  }

  private parseDelete(msg: Buffer): DeleteMessage {
    const reader = new BufferReader(msg);
    reader.skip(1); // Message type

    return {
      type: 'delete',
      relationId: reader.readInt32(),
      tupleKind: reader.readInt8(),
      keyColumns: this.parseTuple(reader),
    };
  }
}
```

### 3.3 Change Transformation

```typescript
class ChangeTransformer {
  private relationCache: Map<number, RelationInfo> = new Map();

  transform(walMessage: WALMessage): Change | null {
    switch (walMessage.type) {
      case 'relation':
        this.relationCache.set(walMessage.relationId, {
          schemaName: walMessage.schemaName,
          tableName: walMessage.tableName,
          columns: walMessage.columns,
        });
        return null;

      case 'insert':
        return this.transformInsert(walMessage);

      case 'update':
        return this.transformUpdate(walMessage);

      case 'delete':
        return this.transformDelete(walMessage);

      default:
        return null;
    }
  }

  private transformInsert(msg: InsertMessage): Change {
    const relation = this.relationCache.get(msg.relationId)!;
    const row = this.tupleToRow(msg.columns, relation.columns);
    const primaryKey = this.extractPrimaryKey(row, relation);

    return {
      type: 'add',
      relation: relation.tableName,
      node: {
        row,
        relationships: {},
      },
    };
  }

  private transformUpdate(msg: UpdateMessage): Change {
    const relation = this.relationCache.get(msg.relationId)!;
    const oldRow = this.tupleToRow(msg.keyColumns, relation.columns);
    const newRow = this.tupleToRow(msg.newColumns, relation.columns);

    return {
      type: 'edit',
      relation: relation.tableName,
      node: {
        row: newRow,
        relationships: {},
      },
      oldNode: {
        row: oldRow,
        relationships: {},
      },
    };
  }

  private transformDelete(msg: DeleteMessage): Change {
    const relation = this.relationCache.get(msg.relationId)!;
    const row = this.tupleToRow(msg.keyColumns, relation.columns);

    return {
      type: 'remove',
      relation: relation.tableName,
      node: {
        row,
        relationships: {},
      },
    };
  }

  private tupleToRow(
    columns: TupleColumn[],
    columnDefs: ColumnDef[]
  ): Row {
    const row: Row = {};

    for (let i = 0; i < columns.length; i++) {
      const col = columns[i];
      const def = columnDefs.find(d => d.name === col.name)!;

      row[def.name] = this.parseValue(col.value, def.type);
    }

    return row;
  }
}
```

## 4. Change Streamer Service

### 4.1 Change Distribution

```typescript
class ChangeStreamer {
  private changeBuffer: Change[] = [];
  private batchTimer: NodeJS.Timeout | null = null;
  private readonly BATCH_INTERVAL_MS = 50;
  private readonly MAX_BATCH_SIZE = 1000;

  constructor(private changeSource: ChangeSource) {
    // Subscribe to change source
    this.changeSource.subscribe((change) => {
      this.onChange(change);
    });
  }

  private onChange(change: Change): void {
    this.changeBuffer.push(change);

    // Flush if batch is full
    if (this.changeBuffer.length >= this.MAX_BATCH_SIZE) {
      this.flush();
      return;
    }

    // Schedule flush if not already scheduled
    if (!this.batchTimer) {
      this.batchTimer = setTimeout(() => {
        this.flush();
      }, this.BATCH_INTERVAL_MS);
    }
  }

  private flush(): void {
    if (this.batchTimer) {
      clearTimeout(this.batchTimer);
      this.batchTimer = null;
    }

    if (this.changeBuffer.length === 0) {
      return;
    }

    const changes = this.changeBuffer;
    this.changeBuffer = [];

    // Distribute to all workers
    this.distributeToWorkers(changes);
  }

  private distributeToWorkers(changes: Change[]): void {
    // Send changes to all worker processes
    for (const worker of this.workers) {
      worker.send({
        type: 'changes',
        changes,
      });
    }
  }
}
```

### 4.2 Worker Communication

```typescript
class WorkerDispatcher {
  private workers: Map<number, Worker> = new Map();
  private clientAssignments: Map<string, number> = new Map();

  constructor(private workerCount: number) {
    for (let i = 0; i < workerCount; i++) {
      this.spawnWorker(i);
    }
  }

  private spawnWorker(id: number): void {
    const worker = fork(require.resolve('./worker'));

    worker.on('message', (msg) => {
      this.handleWorkerMessage(id, msg);
    });

    worker.on('exit', (code) => {
      if (code !== 0) {
        // Restart crashed worker
        console.log(`Worker ${id} exited, restarting...`);
        this.spawnWorker(id);
      }
    });

    this.workers.set(id, worker);
  }

  private handleWorkerMessage(workerId: number, msg: WorkerMessage): void {
    switch (msg.type) {
      case 'client-connected':
        this.clientAssignments.set(msg.clientID, workerId);
        break;

      case 'client-disconnected':
        this.clientAssignments.delete(msg.clientID);
        break;

      case 'mutation':
        this.forwardToMutagen(msg.mutation);
        break;
    }
  }

  routeToWorker(clientID: string, msg: any): void {
    const workerId = this.clientAssignments.get(clientID);
    if (workerId !== undefined) {
      const worker = this.workers.get(workerId);
      worker?.send(msg);
    }
  }

  broadcastToWorkers(msg: any): void {
    for (const worker of this.workers.values()) {
      worker.send(msg);
    }
  }
}
```

## 5. Replicator Service

### 5.1 Client Replication

```typescript
class Replicator {
  private clientStates: Map<string, ClientState> = new Map();
  private queryViews: Map<string, MaterializedView> = new Map();

  constructor(
    private changeStreamer: ChangeStreamer,
    private ivmFactory: IVMFactory
  ) {
    // Subscribe to changes from streamer
    this.changeStreamer.subscribe((changes) => {
      this.processChanges(changes);
    });
  }

  async registerClient(clientID: string, schema: Schema): Promise<void> {
    this.clientStates.set(clientID, {
      clientID,
      schema,
      subscriptions: new Map(),
      lastMutationID: 0,
    });
  }

  async subscribeToQuery(
    clientID: string,
    queryID: string,
    ast: AST
  ): Promise<Row[]> {
    const clientState = this.clientStates.get(clientID)!;

    // Get or create view for this query
    let view = this.queryViews.get(queryID);
    if (!view) {
      view = this.ivmFactory.createView(ast);
      this.queryViews.set(queryID, view);
    }

    // Register client subscription
    clientState.subscriptions.set(queryID, {
      queryID,
      view,
      lastSentChangeID: 0,
    });

    // Return initial results
    return view.getRows();
  }

  private processChanges(changes: Change[]): void {
    // Apply changes to all affected views
    for (const view of this.queryViews.values()) {
      const viewChanges = view.applyChanges(changes);

      if (viewChanges.length > 0) {
        // Find subscribed clients and send changes
        for (const [clientID, clientState] of this.clientStates) {
          for (const [queryID, subscription] of clientState.subscriptions) {
            if (subscription.view === view) {
              this.sendChangesToClient(clientID, queryID, viewChanges);
            }
          }
        }
      }
    }
  }

  private sendChangesToClient(
    clientID: string,
    queryID: string,
    changes: Change[]
  ): void {
    const message: ServerMessage = {
      type: 'changes',
      queryID,
      changes,
    };

    this.sendToClient(clientID, message);
  }

  private sendToClient(clientID: string, message: ServerMessage): void {
    // Send via WebSocket connection
    // Implementation depends on WebSocket server
  }
}
```

## 6. Mutagen Service

### 6.1 Mutation Processing

```typescript
class Mutagen {
  private pendingMutations: Map<string, PendingMutation> = new Map();
  private mutationQueue: MutationRequest[] = [];
  private processing: boolean = false;

  constructor(
    private database: DatabaseConnection,
    private changeSource: ChangeSource
  ) {}

  async processMutation(mutation: MutationRequest): Promise<MutationResult> {
    const mutationID = generateMutationID();
    const pending: PendingMutation = {
      id: mutationID,
      request: mutation,
      status: 'pending',
    };

    this.pendingMutations.set(mutationID, pending);
    this.mutationQueue.push(mutation);

    // Start processing if not already
    if (!this.processing) {
      this.processQueue();
    }

    // Wait for completion
    return new Promise((resolve, reject) => {
      pending.resolve = resolve;
      pending.reject = reject;
    });
  }

  private async processQueue(): Promise<void> {
    this.processing = true;

    while (this.mutationQueue.length > 0) {
      const mutation = this.mutationQueue.shift()!;
      const pending = this.pendingMutations.get(mutation.id)!;

      try {
        pending.status = 'processing';

        // Apply mutation to database
        const result = await this.applyMutation(mutation);

        pending.status = 'confirmed';
        this.pendingMutations.delete(mutation.id);
        pending.resolve(result);

      } catch (error) {
        pending.status = 'failed';
        this.pendingMutations.delete(mutation.id);
        pending.reject(error);
      }
    }

    this.processing = false;
  }

  private async applyMutation(
    mutation: MutationRequest
  ): Promise<MutationResult> {
    const tx = await this.database.begin();

    try {
      switch (mutation.type) {
        case 'insert':
          await this.executeInsert(tx, mutation);
          break;

        case 'update':
          await this.executeUpdate(tx, mutation);
          break;

        case 'delete':
          await this.executeDelete(tx, mutation);
          break;

        case 'custom':
          await this.executeCustom(tx, mutation);
          break;
      }

      await tx.commit();

      // Mutation applied successfully
      // ChangeSource will capture the WAL events automatically

      return { success: true };

    } catch (error) {
      await tx.rollback();
      return { success: false, error: String(error) };
    }
  }

  private async executeInsert(
    tx: Transaction,
    mutation: InsertMutation
  ): Promise<void> {
    const columns = Object.keys(mutation.value);
    const values = Object.values(mutation.value);
    const placeholders = columns.map((_, i) => `$${i + 1}`).join(', ');

    await tx.query(
      `INSERT INTO ${mutation.table} (${columns.join(', ')}) VALUES (${placeholders})`,
      values
    );
  }

  private async executeUpdate(
    tx: Transaction,
    mutation: UpdateMutation
  ): Promise<void> {
    const setClause = Object.entries(mutation.value)
      .map(([col, val], i) => `${col} = $${i + 1}`)
      .join(', ');

    await tx.query(
      `UPDATE ${mutation.table} SET ${setClause} WHERE id = $${Object.keys(mutation.value).length + 1}`,
      [...Object.values(mutation.value), mutation.where.id]
    );
  }
}
```

### 6.2 Custom Mutators

```typescript
class CustomMutatorRegistry {
  private mutators: Map<string, CustomMutator> = new Map();

  register(name: string, mutator: CustomMutator): void {
    this.mutators.set(name, mutator);
  }

  async execute(
    name: string,
    tx: Transaction,
    args: unknown
  ): Promise<unknown> {
    const mutator = this.mutators.get(name);

    if (!mutator) {
      throw new Error(`Unknown mutator: ${name}`);
    }

    return mutator(tx, args);
  }
}

// Example custom mutator
const createIssueWithComment: CustomMutator = async (tx, args: {
  issue: IssueInsert;
  comment: CommentInsert;
}) => {
  // Insert issue
  const issueResult = await tx.query(
    'INSERT INTO issue (...) VALUES (...) RETURNING id',
    flattenIssue(args.issue)
  );

  // Insert comment with issue ID
  const commentWithIssueId = {
    ...args.comment,
    issueId: issueResult.rows[0].id,
  };

  await tx.query(
    'INSERT INTO comment (...) VALUES (...)',
    flattenComment(commentWithIssueId)
  );

  return { issueId: issueResult.rows[0].id };
};
```

## 7. View Syncer Service

### 7.1 View Synchronization

```typescript
class ViewSyncer {
  private views: Map<string, MaterializedView> = new Map();
  private subscriptions: Map<string, Set<string>> = new Map(); // queryID -> clientIDs

  constructor(private replicator: Replicator) {}

  async createView(queryID: string, ast: AST): Promise<void> {
    const view = this.replicator.createView(ast);
    this.views.set(queryID, view);
    this.subscriptions.set(queryID, new Set());
  }

  async subscribe(clientID: string, queryID: string): Promise<Row[]> {
    // Add client to subscription list
    const subscribers = this.subscriptions.get(queryID)!;
    subscribers.add(clientID);

    // Get view
    const view = this.views.get(queryID);
    if (!view) {
      throw new Error(`Unknown query: ${queryID}`);
    }

    // Return current rows
    return view.getRows();
  }

  async unsubscribe(clientID: string, queryID: string): Promise<void> {
    const subscribers = this.subscriptions.get(queryID);
    if (subscribers) {
      subscribers.delete(clientID);

      // Clean up view if no subscribers
      if (subscribers.size === 0) {
        const view = this.views.get(queryID);
        view?.destroy();
        this.views.delete(queryID);
        this.subscriptions.delete(queryID);
      }
    }
  }

  async broadcastChanges(queryID: string, changes: Change[]): Promise<void> {
    const subscribers = this.subscriptions.get(queryID);
    if (!subscribers) return;

    for (const clientID of subscribers) {
      this.sendChanges(clientID, queryID, changes);
    }
  }

  private sendChanges(
    clientID: string,
    queryID: string,
    changes: Change[]
  ): void {
    // Send via WebSocket
  }
}
```

## 8. Worker Implementation

### 8.1 Worker Process

```typescript
// worker.ts
import { parentPort } from 'worker_threads';

class Worker {
  private viewSyncer: ViewSyncer;
  private replicator: Replicator;
  private clientConnections: Map<string, WebSocket> = new Map();

  constructor() {
    this.viewSyncer = new ViewSyncer();
    this.replicator = new Replicator();

    this.setupMessageHandler();
  }

  private setupMessageHandler(): void {
    parentPort?.on('message', (msg: ParentMessage) => {
      switch (msg.type) {
        case 'client-connected':
          this.handleClientConnect(msg);
          break;

        case 'client-message':
          this.handleClientMessage(msg);
          break;

        case 'changes':
          this.handleChanges(msg);
          break;

        case 'client-disconnected':
          this.handleClientDisconnect(msg);
          break;
      }
    });
  }

  private handleClientConnect(msg: ClientConnectMessage): void {
    const { clientID, schema } = msg;

    // Register client
    this.replicator.registerClient(clientID, schema);

    // Notify parent
    parentPort?.postMessage({
      type: 'client-registered',
      clientID,
    });
  }

  private handleClientMessage(msg: ClientMessageWrapper): void {
    const { clientID, message } = msg;

    switch (message.type) {
      case 'subscribe':
        this.handleSubscribe(clientID, message);
        break;

      case 'unsubscribe':
        this.handleUnsubscribe(clientID, message);
        break;

      case 'mutate':
        this.handleMutate(clientID, message);
        break;
    }
  }

  private async handleSubscribe(
    clientID: string,
    message: SubscribeMessage
  ): Promise<void> {
    const { queryID, ast } = message;

    // Get initial results
    const rows = await this.replicator.subscribeToQuery(
      clientID,
      queryID,
      ast
    );

    // Send initial results
    this.sendToClient(clientID, {
      type: 'pull-response',
      queryID,
      results: rows,
    });
  }

  private handleChanges(msg: ChangesMessage): void {
    const { changes } = msg;

    // Apply changes to all views
    for (const [queryID, view] of this.viewSyncer.views) {
      const viewChanges = view.applyChanges(changes);

      if (viewChanges.length > 0) {
        this.viewSyncer.broadcastChanges(queryID, viewChanges);
      }
    }
  }

  private sendToClient(clientID: string, message: ServerMessage): void {
    parentPort?.postMessage({
      type: 'send-to-client',
      clientID,
      message,
    });
  }
}

// Start worker
const worker = new Worker();
```

---

*Next: [05-zero-virtual-deep-dive.md](05-zero-virtual-deep-dive.md)*
