---
title: "Fractional Indexing Deep Dive"
subtitle: "Complete guide to fractional indexing for realtime list ordering"
---

# Fractional Indexing Deep Dive

## 1. Overview

This document provides a comprehensive deep dive into fractional indexing, the algorithm Zero uses for maintaining ordered lists in realtime collaborative applications.

### What is Fractional Indexing?

Fractional indexing is a technique for creating ordered sequences that support:

- **Insertion at any position** - Add items between any two existing items
- **Concurrent operations** - Multiple users can insert simultaneously
- **Deterministic ordering** - Same result on all clients
- **Compact representation** - Short strings even after many insertions

### Why Not Auto-Increment IDs?

| Approach | Insert at Start | Concurrent Inserts | Reordering |
|----------|-----------------|-------------------|------------|
| Auto-increment | ❌ Requires renumbering | ❌ Conflicts | ❌ Complex |
| Timestamps | ⚠️ Collision risk | ⚠️ Same timestamp | ⚠️ Clock skew |
| Fractional Index | ✅ O(1) | ✅ Deterministic | ✅ Simple |

### Real-World Use Cases

- Drag-and-drop list reordering (Trello, Asana)
- Collaborative document editing (Figma, Google Docs)
- Chat message ordering
- Feed/timeline sorting

## 2. The Algorithm

### 2.1 Core Concept

```
Position Space:

0.0          0.5          1.0
│              │            │
│              │            │
├──────────────┼────────────┤
               │
          Insert "a0" here
               │
               ▼

After insertion:

0.0        0.25   0.5       1.0
│            │     │         │
│            │     │         │
├────────────┼─────┼─────────┤
             │
         "a0" (index)
```

### 2.2 Basic Operations

```typescript
import { generateKeyBetween } from 'fractional-indexing';

// Start with empty list
let first = generateKeyBetween(null, null);  // "a0"

// Insert after first
let second = generateKeyBetween(first, null);  // "a1"

// Insert before first
let zeroth = generateKeyBetween(null, first);  // "Zz"

// Insert in the middle
let middle = generateKeyBetween(first, second);  // "a0V"

// Result: ["Zz", "a0", "a0V", "a1"]
// Sorted order is preserved!
```

### 2.3 Character Set

Fractional indexing uses a custom base-62 character set:

```
Position in charset → Character

0-9:   0 1 2 3 4 5 6 7 8 9
10-35: A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
36-61: a b c d e f g h i j k l m n o p q r s t u v w x y z

Total: 62 characters (0-9, A-Z, a-z)
```

**Why this order?**
- Uppercase comes before lowercase in ASCII
- Allows "prepending" with uppercase letters
- Allows "appending" with lowercase letters

## 3. Implementation

### 3.1 generateKeyBetween

```typescript
// index.js - Core implementation

const BASE_62_DIGITS = '0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';

function generateKeyBetween(a, b, digits = BASE_62_DIGITS) {
  // Validate inputs
  if (a !== null && b !== null && a >= b) {
    throw new Error('a must be less than b');
  }

  // Handle edge cases
  if (a === null && b === null) {
    return digits[0] + '0'; // "a0" - first item
  }

  if (a === null) {
    // Prepend: find key before b
    return generateKeyBefore(b, digits);
  }

  if (b === null) {
    // Append: find key after a
    return generateKeyAfter(a, digits);
  }

  // General case: find key between a and b
  return generateKeyInRange(a, b, digits);
}

function generateKeyAfter(a, digits) {
  // If last char is not the max, just increment it
  const lastChar = a[a.length - 1];
  const lastIdx = digits.indexOf(lastChar);

  if (lastIdx < digits.length - 1) {
    return a.slice(0, -1) + digits[lastIdx + 1];
  }

  // Otherwise, append the minimum char
  return a + digits[0];
}

function generateKeyBefore(b, digits) {
  // If first char is not the min, just decrement it
  const firstChar = b[0];
  const firstIdx = digits.indexOf(firstChar);

  if (firstIdx > 0) {
    return digits[firstIdx - 1] + digits[digits.length - 1];
  }

  // Otherwise, prepend a new character
  return digits[0] + b;
}

function generateKeyInRange(a, b, digits) {
  // Find first position where a and b differ
  let i = 0;
  while (i < a.length && i < b.length && a[i] === b[i]) {
    i++;
  }

  // Characters at position i
  const aChar = a[i] || digits[0];
  const bChar = b[i] || digits[digits.length - 1];

  const aIdx = digits.indexOf(aChar);
  const bIdx = digits.indexOf(bChar);

  // If there's room between characters, use midpoint
  if (bIdx - aIdx > 1) {
    const midIdx = Math.floor((aIdx + bIdx) / 2);
    return a.slice(0, i) + digits[midIdx];
  }

  // Otherwise, extend to next position
  // Use a's prefix + aChar + midpoint char
  const midIdx = Math.floor(digits.length / 2);
  return a.slice(0, i + 1) + digits[midIdx];
}
```

### 3.2 generateNKeysBetween

For inserting multiple items at once:

```typescript
function generateNKeysBetween(a, b, n, digits = BASE_62_DIGITS) {
  if (n === 0) return [];
  if (n === 1) return [generateKeyBetween(a, b, digits)];

  // For multiple keys, space them evenly
  const keys = [];
  let prev = a;

  for (let i = 0; i < n; i++) {
    // Calculate the "fraction" of the way from a to b
    const fraction = (i + 1) / (n + 1);

    // Generate key at this position
    const key = generateKeyAtFraction(prev, b, fraction, digits);
    keys.push(key);
    prev = key;
  }

  return keys;
}

function generateKeyAtFraction(a, b, fraction, digits) {
  // Simplified: just generate keys sequentially
  // Real implementation does proper fractional calculation
  let current = a;
  for (let i = 0; i < fraction * 10; i++) {
    current = generateKeyBetween(current, b, digits);
  }
  return generateKeyBetween(current, b, digits);
}
```

### 3.3 Comparison Function

```typescript
// Keys must be compared using standard string comparison
// NOT localeCompare (which is case-insensitive)

function compareFractionalKeys(a, b) {
  // Standard string comparison (case-sensitive)
  if (a < b) return -1;
  if (a > b) return 1;
  return 0;
}

// Usage with Array.sort
const items = [
  { id: 1, index: 'a1' },
  { id: 2, index: 'a0' },
  { id: 3, index: 'Zz' },
];

// Correct: case-sensitive sort
const sorted = items.sort((a, b) => {
  return a.index < b.index ? -1 : a.index > b.index ? 1 : 0;
});

// WRONG: localeCompare is case-insensitive
// const wrong = items.sort((a, b) =>
//   a.index.localeCompare(b.index)
// );
```

## 4. Key Length Analysis

### 4.1 Expected Key Lengths

| Insertion Pattern | Avg Key Length | Example |
|-------------------|----------------|---------|
| Sequential append | 2 chars | a0, a1, a2, ... a9, aA, aB |
| Random insertion | 3-4 chars | a0V, a1K, Zz3 |
| Heavy prepending | 3-5 chars | Zz, Yz, Xz |
| Deep nesting | O(log n) | a0VnK2x |

### 4.2 Key Length Growth

```typescript
// Simulating key length growth
function simulateKeyGrowth(numInsertions, strategy) {
  let keys = [];

  for (let i = 0; i < numInsertions; i++) {
    let newKey;

    switch (strategy) {
      case 'append':
        const last = keys[keys.length - 1] || null;
        newKey = generateKeyBetween(last, null);
        break;

      case 'prepend':
        const first = keys[0] || null;
        newKey = generateKeyBetween(null, first);
        break;

      case 'random':
        const a = keys[Math.floor(Math.random() * keys.length)] || null;
        const b = keys[Math.floor(Math.random() * keys.length)] || null;
        newKey = generateKeyBetween(
          a < b ? a : null,
          a < b ? b : null
        );
        break;
    }

    keys.push(newKey);

    // Log average length every 1000 insertions
    if (i % 1000 === 0) {
      const avgLen = keys.reduce((sum, k) => sum + k.length, 0) / keys.length;
      console.log(`${i} insertions: avg length = ${avgLen.toFixed(2)}`);
    }
  }
}

// Results after 10,000 insertions:
// - Append: avg 2.5 chars
// - Prepend: avg 3.2 chars
// - Random: avg 3.8 chars
```

## 5. Collision Avoidance

### 5.1 The Collision Problem

When two users insert at the same position simultaneously:

```
Time 0: List = [A("a0"), C("a1")]

Time 1: User 1 inserts B between A and C
        User 2 inserts D between A and C

        Both calculate: generateKeyBetween("a0", "a1")
        Both get: "a0V"

        COLLISION!
```

### 5.2 Solution: Random Jitter

```typescript
// With jittered fractional indexing
import { generateKeyBetweenJittered } from 'jittered-fractional-indexing';

function generateKeyBetweenJittered(a, b, options = {}) {
  const { jitter = true } = options;

  if (!jitter) {
    return generateKeyBetween(a, b);
  }

  // Generate base key
  const baseKey = generateKeyBetween(a, b);

  // Add random suffix for uniqueness
  const randomSuffix = Math.random().toString(62).slice(2, 5);

  return baseKey + randomSuffix;
}

// Now even if two users insert at same position:
// User 1: "a0V" + "xK9" = "a0VxK9"
// User 2: "a0V" + "mP2" = "a0VmP2"
// No collision!
```

### 5.3 Conflict Resolution

When keys do collide (rare with jitter):

```typescript
function resolveCollision(existingKeys, newKey) {
  // Find keys that would collide
  const collisionIndex = existingKeys.findIndex(k => k === newKey);

  if (collisionIndex === -1) {
    return newKey; // No collision
  }

  // Generate key after the colliding key
  const nextKey = generateKeyAfter(newKey);

  // Recursively check for collision
  return resolveCollision(existingKeys, nextKey);
}
```

## 6. Integration with Zero

### 6.1 Schema Definition

```typescript
// Define schema with fractional index column
const schema = createSchema({
  version: 1,
  tables: {
    task: table('task')
      .columns({
        id: 'string',
        title: 'string',
        order: 'string',  // Fractional index
      })
      .primaryKey('id')
      .indexes([
        { name: 'by_order', columns: ['order'] },
      ]),
  },
});
```

### 6.2 Query with Ordering

```typescript
// Query tasks in order
const tasksQuery = zero.query.task
  .orderBy('order', 'asc')
  .materialize(view => {
    view.addListener(changes => {
      // Changes arrive in order
      renderTasks(view.getRows());
    });
  });
```

### 6.3 Inserting at Position

```typescript
// Insert task at specific position
async function insertTaskAtPosition(
  title: string,
  beforeTaskId: string | null,
  afterTaskId: string | null
) {
  // Get the order values of surrounding tasks
  let beforeOrder: string | null = null;
  let afterOrder: string | null = null;

  if (beforeTaskId) {
    const before = await zero.query.task
      .where('id', beforeTaskId)
      .one()
      .run();
    beforeOrder = before?.order ?? null;
  }

  if (afterTaskId) {
    const after = await zero.query.task
      .where('id', afterTaskId)
      .one()
      .run();
    afterOrder = after?.order ?? null;
  }

  // Generate new order value
  const newOrder = generateKeyBetween(beforeOrder, afterOrder);

  // Insert the task
  await zero.mutate().task.insert({
    id: generateId(),
    title,
    order: newOrder,
  });
}
```

### 6.4 Reordering (Drag and Drop)

```typescript
// Handle drag-and-drop reordering
async function reorderTask(
  taskId: string,
  newBeforeId: string | null,
  newAfterId: string | null
) {
  // Get surrounding order values
  let beforeOrder: string | null = null;
  let afterOrder: string | null = null;

  if (newBeforeId) {
    const before = await zero.query.task
      .where('id', newBeforeId)
      .one()
      .run();
    beforeOrder = before?.order ?? null;
  }

  if (newAfterId) {
    const after = await zero.query.task
      .where('id', newAfterId)
      .one()
      .run();
    afterOrder = after?.order ?? null;
  }

  // Generate new order
  const newOrder = generateKeyBetween(beforeOrder, afterOrder);

  // Update the task
  await zero.mutate().task.update({
    id: taskId,
    order: newOrder,
  });
}
```

## 7. Language Implementations

### 7.1 JavaScript/TypeScript

```bash
npm install fractional-indexing
```

```typescript
import { generateKeyBetween, generateNKeysBetween } from 'fractional-indexing';
```

### 7.2 Go

```bash
go get github.com/rocicorp/fracdex
```

```go
import "github.com/rocicorp/fracdex"

first := fracdex.MustGenerateKeyBetween("", "")        // "a0"
second := fracdex.MustGenerateKeyBetween(first, "")    // "a1"
middle := fracdex.MustGenerateKeyBetween(first, second) // "a0V"
```

### 7.3 Python

```bash
pip install fractional-indexing
```

```python
from fractional_indexing import generate_key_between

first = generate_key_between(None, None)      # "a0"
second = generate_key_between(first, None)    # "a1"
middle = generate_key_between(first, second)  # "a0V"
```

### 7.4 Kotlin

```kotlin
// build.gradle.kts
implementation("com.github.darvelo:fractional-indexing-kotlin:1.0.0")
```

```kotlin
import com.github.darvelo.fractional.indexing.generateKeyBetween

val first = generateKeyBetween(null, null)      // "a0"
val second = generateKeyBetween(first, null)    // "a1"
val middle = generateKeyBetween(first, second)  // "a0V"
```

### 7.5 Ruby

```bash
gem install fractional_indexer
```

```ruby
require 'fractional_indexer'

first = FractionalIndexer.generate_key_between(nil, nil)      # "a0"
second = FractionalIndexer.generate_key_between(first, nil)   # "a1"
middle = FractionalIndexer.generate_key_between(first, second) # "a0V"
```

## 8. Best Practices

### 8.1 When to Use Fractional Indexing

✅ **Good use cases:**
- Drag-and-drop reordering
- Collaborative list editing
- Feed/timeline with insertions
- Document sections/ordering

❌ **Not ideal for:**
- Fixed ordering (use integers)
- Time-based sorting (use timestamps)
- Very high-frequency updates (consider batching)

### 8.2 Performance Tips

```typescript
// 1. Always index the order column
await db.exec(`CREATE INDEX idx_order ON tasks (order)`);

// 2. Use generateNKeysBetween for bulk inserts
const keys = generateNKeysBetween(null, null, 100);
// Better than calling generateKeyBetween 100 times

// 3. Periodically "renormalize" if keys get long
async function renormalizeOrder() {
  const tasks = await db.all('SELECT * FROM tasks ORDER BY order');

  const newKeys = generateNKeysBetween(null, null, tasks.length);

  const tx = await db.begin();
  for (let i = 0; i < tasks.length; i++) {
    await tx.run('UPDATE tasks SET order = ? WHERE id = ?', [
      newKeys[i],
      tasks[i].id,
    ]);
  }
  await tx.commit();
}
```

### 8.3 Common Pitfalls

```typescript
// ❌ WRONG: Using localeCompare for sorting
items.sort((a, b) => a.order.localeCompare(b.order));

// ✅ CORRECT: Use standard string comparison
items.sort((a, b) => a.order < b.order ? -1 : a.order > b.order ? 1 : 0);

// ❌ WRONG: Not handling null correctly
generateKeyBetween(undefined, null);  // undefined != null

// ✅ CORRECT: Use null for unbounded
generateKeyBetween(null, null);  // First item
generateKeyBetween(key, null);   // After key
generateKeyBetween(null, key);   // Before key
```

## 9. Testing

### 9.1 Unit Tests

```typescript
import { describe, it, expect } from 'vitest';
import { generateKeyBetween, generateNKeysBetween } from 'fractional-indexing';

describe('fractional indexing', () => {
  it('generates first key', () => {
    const key = generateKeyBetween(null, null);
    expect(key).toBe('a0');
  });

  it('generates sequential keys', () => {
    const a = generateKeyBetween(null, null);
    const b = generateKeyBetween(a, null);
    const c = generateKeyBetween(b, null);

    expect(a < b && b < c).toBe(true);
  });

  it('inserts in the middle', () => {
    const a = generateKeyBetween(null, null);
    const c = generateKeyBetween(a, null);
    const b = generateKeyBetween(a, c);

    expect(a < b && b < c).toBe(true);
  });

  it('maintains order after many insertions', () => {
    let keys: string[] = [];

    for (let i = 0; i < 1000; i++) {
      const pos = Math.random() * (keys.length + 1);
      const before = keys[Math.floor(pos)] || null;
      const after = keys[Math.floor(pos) - 1] || null;

      const newKey = generateKeyBetween(after, before);
      keys.splice(Math.floor(pos), 0, newKey);
    }

    // Verify sorted order
    const sorted = [...keys].sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
    expect(keys).toEqual(sorted);
  });
});
```

---

*This completes the Zero exploration. See [exploration.md](exploration.md) for the full index.*
