# R8Y - Deep Dive

## Overview

**r8y** is a full-stack TypeScript monorepo application built with Bun, Svelte 5, and MySQL. It serves as a demonstration platform for modern web development patterns including real-time updates, database integration, and AI features.

---

## Project Structure

```
r8y/
├── apps/
│   ├── bg/           # Background worker service
│   └── web/          # SvelteKit web application
├── packages/
│   ├── channel-sync/ # Real-time channel synchronization
│   └── db/           # Drizzle ORM database layer
├── bun.lock          # Bun lockfile
└── package.json      # Root package configuration
```

---

## Tech Stack

| Component | Technology |
|-----------|------------|
| Runtime | Bun |
| Frontend | Svelte 5 (Runes) |
| Styling | Tailwind CSS v4 |
| Database | MySQL |
| ORM | Drizzle |
| Real-time | Channel-sync |
| Icons | Lucide (via @lucide/svelte) |

---

## Setup

### Prerequisites

- Bun (https://bun.sh)
- Docker (for MySQL)

### Environment Variables

```bash
# Default MySQL URL for local development
MYSQL_URL=mysql://root:root@localhost:3306/r8y
```

### Development Commands

```bash
# Install dependencies
bun install

# Start MySQL + all apps in dev mode
bun dev

# Run dev mode without MySQL
bun dev:no-db

# Database commands
bun db:up        # Start MySQL container
bun db:down      # Stop MySQL (keeps data)
bun db:destroy   # Stop MySQL and delete data
bun db:setup     # Start MySQL, wait, push schema
bun db:push      # Push Drizzle schema

# Build and check
bun build    # Build all apps
bun check    # Type check all packages
bun lint     # Lint all packages
```

---

## Database Layer (packages/db)

### Drizzle Schema

```typescript
// Example schema pattern
import { mysqlTable, varchar, text, timestamp } from 'drizzle-orm/mysql-core';

export const videos = mysqlTable('videos', {
  id: varchar('id', { length: 21 }).primaryKey(),
  title: varchar('title', { length: 255 }).notNull(),
  description: text('description'),
  createdAt: timestamp('created_at').defaultNow().notNull(),
  updatedAt: timestamp('updated_at').defaultNow().onUpdateNow()
});
```

### Database Access

```typescript
import { drizzle } from 'drizzle-orm/mysql2';
import { createPool } from 'mysql2/promise';

const pool = createPool(process.env.MYSQL_URL!);
const db = drizzle(pool);

// Query with Drizzle
const videos = await db.select().from(videos).limit(10);
```

---

## Web Application (apps/web)

### Svelte 5 Conventions

```svelte
<script lang="ts">
  // Runes for reactivity
  let count = $state(0);
  let doubled = $derived(count * 2);

  // Props with $props()
  let { title, items = [] } = $props();

  // Effects
  $effect(() => {
    console.log('count changed:', count);
  });

  // Event handlers
  function increment() {
    count++;
  }
</script>

<!-- Snippets instead of slots -->
{#snippet header()}
  <h1>{title}</h1>
{/snippet}

<!-- Async boundaries -->
<svelte:boundary>
  {#await loadData()}
    <p>Loading...</p>
  {:then data}
    <p>{data}</p>
  {/await}
</svelte:boundary>
```

### Remote Functions

```typescript
// apps/web/src/lib/server/data.remote.ts
import { action } from '@tanstack/react-server';
import { db } from '$lib/server/db';
import { videos } from '$lib/db/schema';

export const getVideos = queryOptions({
  queryKey: ['videos'],
  queryFn: async () => {
    return db.select().from(videos).limit(10);
  }
});
```

---

## Real-Time Features (packages/channel-sync)

### Channel Synchronization

```typescript
import { createChannel } from '@r8y/channel-sync';

const videoChannel = createChannel<{
  events: {
    'video:updated': { id: string; title: string };
    'video:deleted': { id: string };
  };
}>();

// Publish event
await videoChannel.publish('video:updated', {
  id: 'video_123',
  title: 'Updated Title'
});

// Subscribe
videoChannel.subscribe('video:updated', (event) => {
  // Update UI
});
```

---

## Background Jobs (apps/bg)

### Worker Pattern

```typescript
// apps/bg/src/worker.ts
import { Consumer } from 'bullmq';
import { processVideo } from './processors/video';

const videoConsumer = new Consumer('video-queue', async (job) => {
  switch (job.name) {
    case 'process-video':
      return await processVideo(job.data);
    default:
      throw new Error(`Unknown job: ${job.name}`);
  }
});
```

---

## Future Features (from todos)

1. **Durability pass** - Improve error handling and recovery
2. **Advanced search** - Full-text search across videos
3. **AI agent integration** - Video recommendation/search
4. **Comments system** - Sponsor page comments
5. **Query params with Runed** - Better URL state management
6. **More video links** - Cross-page navigation
7. **Graceful shutdown** - `runtime.dispose()` with node runtime

---

## Production Rust Implementation

### Architecture

```
r8y-rs/
├── apps/
│   ├── web/           # Axum web server
│   └── bg/            # Background worker
├── crates/
│   ├── r8y-db/        # SQLx database layer
│   ├── r8y-api/       # API types
│   └── r8y-ws/        # WebSocket real-time
└── migrations/        # SQL migrations
```

### Key Crates

- `axum` - Web framework
- `sqlx` - Async SQL (MySQL)
- `tokio-tungstenite` - WebSockets
- `tokio` - Async runtime
- `serde` - Serialization
- `tower` - Middleware
- `askama` - Template rendering

### Database Layer

```rust
use sqlx::MySqlPool;

pub struct Database {
    pool: MySqlPool,
}

impl Database {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = MySqlPool::connect(url).await?;
        Ok(Self { pool })
    }

    pub async fn get_videos(&self, limit: i64) -> Result<Vec<Video>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM videos LIMIT ?")
            .bind(limit)
            .fetch_all(&self.pool)
            .await
    }
}
```

### Real-Time with WebSockets

```rust
use axum::{
    extract::ws::{WebSocket, Message},
    extract::State,
};

async fn ws_handler(
    ws: WebSocket,
    State(channels): State<ChannelRegistry>,
) {
    let mut rx = channels.subscribe("videos");
    while let Ok(event) = rx.recv().await {
        ws.send(Message::Text(event.to_json())).await?;
    }
}
```
