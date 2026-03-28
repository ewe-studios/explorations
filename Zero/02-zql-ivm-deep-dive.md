---
title: "ZQL and IVM Engine Deep Dive"
subtitle: "Complete guide to Zero's query language and Incremental View Maintenance engine"
---

# ZQL and IVM Engine Deep Dive

## 1. Overview

This document provides a comprehensive deep dive into ZQL (Zero Query Language) and the IVM (Incremental View Maintenance) engine, covering:

- Query representation (AST)
- IVM operators
- Change types and propagation
- Stream processing
- View materialization

## 2. Query Representation

### 2.1 Abstract Syntax Tree (AST)

Zero represents queries as an AST:

```typescript
interface AST {
  type: 'select';
  table: string;
  columns?: string[];
  where?: Condition;
  join?: Join[];
  orderBy?: Ordering;
  limit?: number;
  offset?: number;
}

type Condition =
  | SimpleCondition
  | Conjunction  // AND
  | Disjunction; // OR

interface SimpleCondition {
  type: 'simple';
  left: ColumnReference | LiteralValue;
  operator: SimpleOperator;
  right: ColumnReference | LiteralValue;
}

type SimpleOperator = '=' | '!=' | '<' | '<=' | '>' | '>=' | 'like' | 'in';

interface ColumnReference {
  type: 'column';
  table?: string;
  column: string;
}

interface Ordering {
  columns: OrderPart[];
}

interface OrderPart {
  column: string;
  direction: 'asc' | 'desc';
}
```

### 2.2 Query Builder API

Zero provides a fluent query builder:

```typescript
// TypeScript API
const query = zero.query.issue
  .where('status', 'open')
  .where('priority', '>', 5)
  .join('user', 'author_id', 'id')
  .orderBy('created', 'desc')
  .limit(10);

// Compiles to AST
const ast: AST = {
  type: 'select',
  table: 'issue',
  where: {
    type: 'conjunction',
    conditions: [
      { type: 'simple', left: { column: 'status' }, operator: '=', right: 'open' },
      { type: 'simple', left: { column: 'priority' }, operator: '>', right: 5 },
    ],
  },
  join: [{ table: 'user', left: 'author_id', right: 'id' }],
  orderBy: { columns: [{ column: 'created', direction: 'desc' }] },
  limit: 10,
};
```

### 2.3 Named Queries

Define reusable queries with parameters:

```typescript
import {defineQuery} from 'zql';

const queries = defineQuery(({args: {projectId, status}}) =>
  zql.issue
    .where('projectId', projectId)
    .where('status', status)
    .orderBy('created', 'desc')
    .limit(50)
);

// Usage
const view = queries({projectId: 'abc', status: 'open'}).materialize(...);
```

## 3. IVM Operators

### 3.1 Operator Interface

All IVM operators implement a common interface:

```typescript
interface Operator {
  // Process a single change
  apply(change: Change): Change[];

  // Fetch initial data (for new subscriptions)
  fetch(): AsyncIterable<Change>;

  // Cleanup resources
  destroy(): void;
}
```

### 3.2 Scan Operator

The Scan operator reads from a table:

```typescript
class ScanOperator implements Operator {
  constructor(
    private table: string,
    private source: TableSource
  ) {}

  apply(change: Change): Change[] {
    // Forward all changes from the table
    if (change.relation === this.table) {
      return [change];
    }
    return [];
  }

  async *fetch(): AsyncIterable<Change> {
    const rows = await this.source.getAll(this.table);
    for (const row of rows) {
      yield {
        type: 'add',
        relation: this.table,
        node: { row, relationships: {} },
      };
    }
  }

  destroy(): void {
    // No cleanup needed
  }
}
```

### 3.3 Filter Operator

The Filter operator applies WHERE clauses:

```typescript
class FilterOperator implements Operator {
  constructor(
    private condition: Condition,
    private evaluator: ExpressionEvaluator
  ) {}

  apply(change: Change): Change[] {
    switch (change.type) {
      case 'add':
        // Check if row matches filter
        if (this.evaluator.evaluate(change.node.row, this.condition)) {
          return [change];
        }
        return [];

      case 'remove':
        // Check if removed row was matching
        if (this.evaluator.evaluate(change.node.row, this.condition)) {
          return [change];
        }
        return [];

      case 'edit':
        const oldMatch = this.evaluator.evaluate(change.oldNode.row, this.condition);
        const newMatch = this.evaluator.evaluate(change.node.row, this.condition);

        if (oldMatch && newMatch) {
          // Still matches, forward edit
          return [change];
        } else if (oldMatch && !newMatch) {
          // No longer matches, emit remove
          return [{ type: 'remove', node: change.oldNode }];
        } else if (!oldMatch && newMatch) {
          // Now matches, emit add
          return [{ type: 'add', node: change.node }];
        } else {
          // Never matched, ignore
          return [];
        }

      case 'child':
        // Child changes always pass through
        return [change];
    }
  }

  fetch(): AsyncIterable<Change> {
    // Filters don't fetch, they just pass through
    return emptyIterable();
  }

  destroy(): void {}
}
```

### 3.4 Join Operator

The Join operator combines rows from two tables:

```typescript
class JoinOperator implements Operator {
  private leftIndex: Map<Key, Node[]> = new Map();
  private rightIndex: Map<Key, Node[]> = new Map();

  constructor(
    private leftKey: string,
    private rightKey: string,
    private outputRelation: string
  ) {}

  apply(change: Change): Change[] {
    const isLeft = change.relation === this.leftTable;
    const isRight = change.relation === this.rightTable;

    if (!isLeft && !isRight) {
      return [];
    }

    const keyField = isLeft ? this.leftKey : this.rightKey;
    const keyValue = change.node.row[keyField];
    const index = isLeft ? this.leftIndex : this.rightIndex;
    const otherIndex = isLeft ? this.rightIndex : this.leftIndex;

    switch (change.type) {
      case 'add':
        // Store in index
        const existing = index.get(keyValue) || [];
        existing.push(change.node);
        index.set(keyValue, existing);

        // Join with matching rows from other side
        const matches = otherIndex.get(keyValue) || [];
        return matches.map(other => this.createJoinChange(change.node, other, 'add'));

      case 'remove':
        // Remove from index
        const nodes = index.get(keyValue) || [];
        const filtered = nodes.filter(n => n !== change.node);
        if (filtered.length === 0) {
          index.delete(keyValue);
        } else {
          index.set(keyValue, filtered);
        }

        // Emit remove for joined rows
        const otherMatches = otherIndex.get(keyValue) || [];
        return otherMatches.map(other => this.createJoinChange(change.node, other, 'remove'));

      case 'edit':
        // Handle key change (remove old, add new)
        const oldKey = change.oldNode.row[keyField];
        if (oldKey !== keyValue) {
          const oldChanges = this.handleKeyChange(change, oldKey, index, otherIndex);
          const newChanges = this.apply({ ...change, type: 'add' });
          return [...oldChanges, ...newChanges];
        }

        // Key unchanged, update index and emit edits
        const nodesToEdit = otherIndex.get(keyValue) || [];
        return nodesToEdit.map(other => this.createJoinChange(change.node, other, 'edit'));

      default:
        return [];
    }
  }

  private createJoinChange(left: Node, right: Node, type: ChangeType): Change {
    return {
      type,
      relation: this.outputRelation,
      node: {
        row: { ...left.row, ...right.row },
        relationships: {
          left: () => singleStream(left),
          right: () => singleStream(right),
        },
      },
    };
  }

  fetch(): AsyncIterable<Change> {
    // Joins don't fetch independently
    return emptyIterable();
  }

  destroy(): void {
    this.leftIndex.clear();
    this.rightIndex.clear();
  }
}
```

### 3.5 OrderBy Operator

The OrderBy operator maintains sorted order:

```typescript
class OrderByOperator implements Operator {
  private sortedNodes: Node[] = [];
  private comparator: Comparator;

  constructor(ordering: Ordering) {
    this.comparator = makeComparator(ordering);
  }

  apply(change: Change): Change[] {
    switch (change.type) {
      case 'add':
        // Insert at correct position (binary search)
        const insertIdx = this.findInsertPosition(change.node);
        this.sortedNodes.splice(insertIdx, 0, change.node);

        // Emit add with position info
        return [{
          ...change,
          position: { index: insertIdx, oldIndex: undefined },
        }];

      case 'remove':
        // Find and remove
        const removeIdx = this.sortedNodes.indexOf(change.node);
        if (removeIdx !== -1) {
          this.sortedNodes.splice(removeIdx, 1);
          return [{
            ...change,
            position: { index: undefined, oldIndex: removeIdx },
          }];
        }
        return [];

      case 'edit':
        const oldIdx = this.sortedNodes.indexOf(change.oldNode);
        const newIdx = this.findInsertPosition(change.node);

        // Update array
        this.sortedNodes.splice(oldIdx, 1);
        this.sortedNodes.splice(newIdx, 0, change.node);

        if (oldIdx === newIdx) {
          // Position unchanged, simple edit
          return [change];
        } else {
          // Position changed, emit move
          return [{
            ...change,
            position: { index: newIdx, oldIndex: oldIdx },
          }];
        }

      default:
        return [];
    }
  }

  private findInsertPosition(node: Node): number {
    let low = 0;
    let high = this.sortedNodes.length;

    while (low < high) {
      const mid = Math.floor((low + high) / 2);
      if (this.comparator(node.row, this.sortedNodes[mid].row) < 0) {
        high = mid;
      } else {
        low = mid + 1;
      }
    }

    return low;
  }

  fetch(): AsyncIterable<Change> {
    return emptyIterable();
  }

  destroy(): void {
    this.sortedNodes = [];
  }
}
```

### 3.6 Limit Operator

The Limit operator maintains a sliding window:

```typescript
class LimitOperator implements Operator {
  constructor(private limit: number) {}

  private nodes: Node[] = [];

  apply(change: Change): Change[] {
    const { type, node, position } = change;

    if (type === 'add') {
      const idx = position?.index ?? this.nodes.length;

      if (idx < this.limit) {
        // Insert within limit
        this.nodes.splice(idx, 0, node);

        // Check if we need to eject the last item
        if (this.nodes.length > this.limit) {
          const ejected = this.nodes.pop()!;
          return [
            change,
            { type: 'remove', node: ejected, ejected: true },
          ];
        }
        return [change];
      } else {
        // Insert outside limit, just track it
        this.nodes.splice(idx, 0, node);
        return [];
      }
    }

    if (type === 'remove') {
      const idx = position?.oldIndex ?? this.nodes.indexOf(change.node);
      if (idx !== -1) {
        this.nodes.splice(idx, 1);

        // Check if we need to promote an item
        if (this.nodes.length >= this.limit) {
          const promoted = this.nodes[this.limit - 1];
          return [
            change,
            { type: 'add', node: promoted, promoted: true },
          ];
        }
      }
      return [change];
    }

    return [change];
  }

  fetch(): AsyncIterable<Change> {
    return emptyIterable();
  }

  destroy(): void {
    this.nodes = [];
  }
}
```

## 4. Change Types

### 4.1 Change Type Definitions

```typescript
type Change = AddChange | RemoveChange | EditChange | ChildChange;

interface AddChange {
  type: 'add';
  relation: string;
  node: Node;
}

interface RemoveChange {
  type: 'remove';
  relation: string;
  node: Node;
}

interface EditChange {
  type: 'edit';
  relation: string;
  node: Node;
  oldNode: Node;
}

interface ChildChange {
  type: 'child';
  relation: string;
  node: Node;
  child: {
    relationshipName: string;
    change: Change;
  };
}
```

### 4.2 Node Structure

```typescript
interface Node {
  row: Row;
  relationships: Record<string, () => Stream<Node | 'yield'>>;
}

type Row = Record<string, Value>;
type Value = string | number | boolean | null | undefined;
```

### 4.3 Change Flow Example

Consider this query:

```sql
SELECT issues.*, users.name as author_name
FROM issues
JOIN users ON issues.author_id = users.id
WHERE issues.status = 'open'
ORDER BY issues.created DESC
LIMIT 10
```

When a new issue is created:

```
1. Scan (issues table):
   Input: INSERT issue #42
   Output: AddChange(issue #42)

2. Filter (status = 'open'):
   Input: AddChange(issue #42, status='open')
   Output: AddChange(issue #42)  [passes filter]

3. Join (with users):
   Input: AddChange(issue #42, author_id=123)
   Lookup: user #123 exists in right index
   Output: AddChange(joined row: issue #42 + user name)

4. OrderBy (created DESC):
   Input: AddChange(joined row, created=2024-01-15)
   Position: index 3 (sorted by date)
   Output: AddChange(position: 3)

5. Limit (10):
   Input: AddChange(position: 3)
   Check: 3 < 10, within limit
   Output: AddChange(final)

Final: Client receives AddChange for issue #42 at position 3
```

## 5. Stream Processing

### 5.1 Stream Abstraction

```typescript
type Stream<T> = () => Iterable<T | 'yield'>;

// Empty stream
function emptyStream(): Stream<never> {
  return () => [];
}

// Single-value stream
function singleStream<T>(value: T): Stream<T> {
  return () => [value];
}

// Transform stream
function mapStream<T, U>(stream: Stream<T>, fn: (t: T) => U): Stream<U> {
  return () => {
    const result: U[] = [];
    for (const item of stream()) {
      if (item !== 'yield') {
        result.push(fn(item));
      }
    }
    return result;
  };
}

// Filter stream
function filterStream<T>(stream: Stream<T>, predicate: (t: T) => boolean): Stream<T> {
  return () => {
    const result: T[] = [];
    for (const item of stream()) {
      if (item !== 'yield' && predicate(item)) {
        result.push(item);
      }
    }
    return result;
  };
}
```

### 5.2 Yield Mechanism

The 'yield' marker allows operators to pause execution:

```typescript
class ExpensiveOperator implements Operator {
  async *fetch(): AsyncIterable<Change | 'yield'> {
    const rows = await this.slowQuery();

    for (const row of rows) {
      yield { type: 'add', node: { row, relationships: {} } };

      // Yield control periodically to avoid blocking
      if (++this.count % 100 === 0) {
        yield 'yield';
      }
    }
  }
}
```

## 6. View Materialization

### 6.1 Materialized View

```typescript
class MaterializedView {
  private rows: Node[] = [];
  private listeners: Set<(changes: Change[]) => void> = new Set();

  // Apply changes and notify listeners
  applyChanges(changes: Change[]): Change[] {
    const relevantChanges: Change[] = [];

    for (const change of changes) {
      const processed = this.applyChange(change);
      if (processed) {
        relevantChanges.push(processed);
      }
    }

    if (relevantChanges.length > 0) {
      this.notifyListeners(relevantChanges);
    }

    return relevantChanges;
  }

  private applyChange(change: Change): Change | null {
    switch (change.type) {
      case 'add':
        this.rows.push(change.node);
        return change;

      case 'remove':
        const idx = this.rows.indexOf(change.node);
        if (idx !== -1) {
          this.rows.splice(idx, 1);
          return change;
        }
        return null;

      case 'edit':
        const editIdx = this.rows.indexOf(change.oldNode);
        if (editIdx !== -1) {
          this.rows[editIdx] = change.node;
          return change;
        }
        return null;

      default:
        return change;
    }
  }

  // Subscribe to changes
  addListener(listener: (changes: Change[]) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private notifyListeners(changes: Change[]): void {
    for (const listener of this.listeners) {
      listener(changes);
    }
  }

  // Get current rows
  getRows(): Node[] {
    return [...this.rows];
  }

  destroy(): void {
    this.rows = [];
    this.listeners.clear();
  }
}
```

### 6.2 View Factory

```typescript
class ViewFactory {
  createView(query: AST): MaterializedView {
    const pipeline = this.buildPipeline(query);
    const view = new MaterializedView();

    // Initialize view with current data
    this.initializeView(pipeline, view);

    // Connect pipeline to view
    this.connectPipeline(pipeline, view);

    return view;
  }

  private buildPipeline(query: AST): Operator[] {
    const operators: Operator[] = [];

    // Add scan operator
    operators.push(new ScanOperator(query.table, this.tableSource));

    // Add filter operators
    if (query.where) {
      operators.push(new FilterOperator(query.where, this.evaluator));
    }

    // Add join operators
    if (query.join) {
      for (const join of query.join) {
        operators.push(new JoinOperator(join.left, join.right, join.table));
      }
    }

    // Add order operator
    if (query.orderBy) {
      operators.push(new OrderByOperator(query.orderBy));
    }

    // Add limit operator
    if (query.limit) {
      operators.push(new LimitOperator(query.limit));
    }

    return operators;
  }

  private async initializeView(operators: Operator[], view: MaterializedView): Promise<void> {
    const [scanOperator] = operators;

    for await (const change of scanOperator.fetch()) {
      const finalChange = this.processThroughPipeline(change, operators);
      if (finalChange) {
        view.applyChanges([finalChange]);
      }
    }
  }

  private connectPipeline(operators: Operator[], view: MaterializedView): void {
    // Subscribe to change source
    const changeSource = this.changeSource.subscribe(change => {
      const finalChange = this.processThroughPipeline(change, operators);
      if (finalChange) {
        view.applyChanges([finalChange]);
      }
    });
  }

  private processThroughPipeline(change: Change, operators: Operator[]): Change | null {
    let changes: Change[] = [change];

    for (const operator of operators) {
      const output: Change[] = [];
      for (const c of changes) {
        output.push(...operator.apply(c));
      }
      changes = output;
    }

    return changes[0] ?? null;
  }
}
```

## 7. Performance Optimizations

### 7.1 Index Usage

```typescript
class IndexedFilterOperator extends FilterOperator {
  private index: Map<Value, Node[]> = new Map();

  constructor(condition: SimpleCondition, evaluator: ExpressionEvaluator) {
    super(condition, evaluator);

    // Build index on the filtered column
    if (condition.type === 'simple' && condition.operator === '=') {
      this.indexColumn = condition.left.column;
    }
  }

  apply(change: Change): Change[] {
    // Update index
    if (this.indexColumn) {
      const keyValue = change.node.row[this.indexColumn];
      this.updateIndex(keyValue, change);
    }

    return super.apply(change);
  }

  private updateIndex(key: Value, change: Change): void {
    const nodes = this.index.get(key) || [];

    switch (change.type) {
      case 'add':
        nodes.push(change.node);
        break;
      case 'remove':
        const idx = nodes.indexOf(change.node);
        if (idx !== -1) nodes.splice(idx, 1);
        break;
    }

    if (nodes.length === 0) {
      this.index.delete(key);
    } else {
      this.index.set(key, nodes);
    }
  }
}
```

### 7.2 Change Batching

```typescript
class BatchedView extends MaterializedView {
  private batch: Change[] = [];
  private timer: NodeJS.Timeout | null = null;
  private batchTimeMs: number;

  constructor(batchTimeMs: number = 50) {
    super();
    this.batchTimeMs = batchTimeMs;
  }

  applyChanges(changes: Change[]): void {
    this.batch.push(...changes);

    if (!this.timer) {
      this.timer = setTimeout(() => {
        this.flush();
      }, this.batchTimeMs);
    }
  }

  private flush(): void {
    if (this.batch.length > 0) {
      this.notifyListeners(this.batch);
      this.batch = [];
    }
    this.timer = null;
  }
}
```

---

*Next: [03-client-deep-dive.md](03-client-deep-dive.md)*
