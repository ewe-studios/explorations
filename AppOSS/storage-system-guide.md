---
source: /home/darkvoid/Boxxed/@formulas/src.AppOSS
reference_projects: n8n, baserow, Penpot, Budibase
created_at: 2026-04-02
audience: Inexperienced software engineers
tags: storage, database, persistence, beginner
---

# Building a Resilient Storage System: A Beginner's Guide

## Introduction

This guide teaches you how to build a production-grade storage system from first principles. We'll cover everything from basic concepts to expert-level patterns, using real examples from AppOSS projects.

### What You'll Learn

1. Basic storage concepts (files, databases)
2. How to choose the right storage solution
3. Building a database layer
4. Caching strategies
5. File storage
6. Backup and recovery
7. Scaling your storage

---

## Part 1: Storage Fundamentals

### 1.1 What is Storage?

Storage is where your application keeps data permanently (or semi-permanently).

```
┌─────────────────────────────────────────────────────────┐
│                   Your Application                       │
│                                                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                │
│  │  Users  │  │Documents│  │ Settings│   ← In Memory  │
│  └─────────┘  └─────────┘  └─────────┘      (RAM)     │
│       │            │            │                      │
│       └────────────┼────────────┘                      │
│                    │                                   │
│                    ▼                                   │
│              ┌──────────┐                              │
│              │ Storage  │   ← Persistent (Disk/Cloud)  │
│              └──────────┘                              │
└─────────────────────────────────────────────────────────┘
```

### 1.2 Types of Storage

| Type | Example | Best For |
|------|---------|----------|
| **Files** | JSON, binary files | Small apps, config |
| **Relational DB** | PostgreSQL, MySQL | Structured data, transactions |
| **NoSQL DB** | MongoDB, Redis | Flexible schema, caching |
| **Object Storage** | AWS S3, GCP Cloud Storage | Files, images, backups |
| **In-Memory** | Redis, Memcached | Fast access, sessions |

### 1.3 Basic Example: File Storage

The simplest storage is a file:

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Default)]
struct Document {
    id: String,
    name: String,
    content: String,
}

struct FileStorage {
    path: String,
}

impl FileStorage {
    fn new(path: &str) -> Self {
        FileStorage { path: path.to_string() }
    }
    
    fn save(&self, doc: &Document) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(doc)?;
        fs::write(&self.path, json)?;
        Ok(())
    }
    
    fn load(&self) -> Result<Option<Document>, Box<dyn std::error::Error>> {
        if !Path::new(&self.path).exists() {
            return Ok(None);
        }
        
        let json = fs::read_to_string(&self.path)?;
        let doc: Document = serde_json::from_str(&json)?;
        Ok(Some(doc))
    }
}

// Usage
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let storage = FileStorage::new("data.json");
    
    let doc = Document {
        id: "1".to_string(),
        name: "My Document".to_string(),
        content: "Hello, World!".to_string(),
    };
    
    storage.save(&doc)?;
    
    if let Some(loaded) = storage.load()? {
        println!("Loaded: {}", loaded.name);
    }
    
    Ok(())
}
```

**Limitations of file storage:**
- No concurrent access (can corrupt data)
- No querying (must load everything)
- No transactions (partial writes possible)
- Doesn't scale

---

## Part 2: Database Basics

### 2.1 What is a Database?

A database is a specialized program for storing and retrieving data efficiently.

```
Your App → Database Driver → Database Server → Disk
            (e.g., sqlx)       (e.g., PostgreSQL)
```

### 2.2 SQL vs NoSQL

**SQL (Relational):**
```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Users     │     │  Documents  │     │   Teams     │
├─────────────┤     ├─────────────┤     ├─────────────┤
│ id (PK)     │────▶│ user_id (FK)│     │ id (PK)     │
│ email       │     │ team_id (FK)│────▶│ name        │
│ name        │     │ name        │     │ created_at  │
│ created_at  │     │ content     │     └─────────────┘
└─────────────┘     │ created_at  │
                    └─────────────┘
```

**NoSQL (Document):**
```
{
  "id": "doc-123",
  "name": "My Doc",
  "user": { "id": "user-1", "email": "..." },  // Embedded
  "content": "...",
  "tags": ["work", "important"]  // Array
}
```

### 2.3 Setting Up PostgreSQL

```bash
# Install PostgreSQL (Ubuntu/Debian)
sudo apt install postgresql postgresql-contrib

# Start service
sudo systemctl start postgresql

# Create user and database
sudo -u postgres psql
CREATE USER myapp WITH PASSWORD 'secret';
CREATE DATABASE myapp_db OWNER myapp;
\q
```

### 2.4 Basic Database Operations (CRUD)

```rust
use sqlx::{PgPool, Row};

// Create (Insert)
async fn create_document(pool: &PgPool, name: &str, content: &str) -> Result<i32, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO documents (name, content) VALUES ($1, $2) RETURNING id"
    )
    .bind(name)
    .bind(content)
    .fetch_one(pool)
    .await?;
    
    let id: i32 = row.get("id");
    Ok(id)
}

// Read (Select)
async fn get_document(pool: &PgPool, id: i32) -> Result<Option<(String, String)>, sqlx::Error> {
    let row = sqlx::query_as::<_, (i32, String, String)>(
        "SELECT id, name, content FROM documents WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    
    Ok(row.map(|(_, name, content)| (name, content)))
}

// Update
async fn update_document(pool: &PgPool, id: i32, name: &str, content: &str) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE documents SET name = $1, content = $2 WHERE id = $3"
    )
    .bind(name)
    .bind(content)
    .bind(id)
    .execute(pool)
    .await?;
    
    Ok(result.rows_affected())
}

// Delete
async fn delete_document(pool: &PgPool, id: i32) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    
    Ok(result.rows_affected())
}
```

---

## Part 3: Connection Pooling

### 3.1 Why Connection Pooling?

Creating a database connection is expensive. A pool keeps connections ready:

```
Without Pool:          With Pool:
Request → Create Conn  Request → Get from Pool
       → Query              → Query
       → Close Conn         → Return to Pool
(Slow: ~50ms)          (Fast: ~1ms)
```

### 3.2 Setting Up a Pool

```rust
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::time::Duration;

async fn create_pool(database_url: &str) -> Result<Pool<Postgres>, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)           // Max 10 connections
        .min_connections(2)            // Keep at least 2 idle
        .connect_timeout(Duration::from_secs(30))  // Wait up to 30s
        .idle_timeout(Duration::from_secs(600))    // Close idle after 10min
        .max_lifetime(Duration::from_secs(1800))   // Max connection age 30min
        .connect(database_url)
        .await
}

// Usage
#[tokio::main]
async fn main() {
    let pool = create_pool("postgresql://user:pass@localhost/mydb").await.unwrap();
    
    // Pool is cheap to clone - share across threads
    let pool_clone = pool.clone();
    
    // Use in handlers
    let docs = get_all_documents(&pool).await;
}
```

---

## Part 4: Transactions

### 4.1 What are Transactions?

Transactions ensure multiple operations succeed or fail together (ACID properties):

```
BEGIN;
  UPDATE accounts SET balance = balance - 100 WHERE id = 1;
  UPDATE accounts SET balance = balance + 100 WHERE id = 2;
COMMIT;

-- If anything fails:
ROLLBACK;
```

### 4.2 Using Transactions in Rust

```rust
use sqlx::{PgPool, Transaction, Postgres};

async fn transfer_money(
    pool: &PgPool,
    from_id: i32,
    to_id: i32,
    amount: i64,
) -> Result<(), sqlx::Error> {
    // Start transaction
    let mut tx = pool.begin().await?;
    
    // Debit
    sqlx::query(
        "UPDATE accounts SET balance = balance - $1 WHERE id = $2"
    )
    .bind(amount)
    .bind(from_id)
    .execute(&mut *tx)
    .await?;
    
    // Credit
    sqlx::query(
        "UPDATE accounts SET balance = balance + $1 WHERE id = $2"
    )
    .bind(amount)
    .bind(to_id)
    .execute(&mut *tx)
    .await?;
    
    // Check sufficient funds
    let row = sqlx::query("SELECT balance FROM accounts WHERE id = $1")
        .bind(from_id)
        .fetch_one(&mut *tx)
        .await?;
    
    let balance: i64 = row.get("balance");
    if balance < 0 {
        // Rollback on error
        tx.rollback().await?;
        return Err(sqlx::Error::RowNotFound);
    }
    
    // Commit transaction
    tx.commit().await?;
    
    Ok(())
}
```

---

## Part 5: Caching

### 5.1 Why Cache?

Database queries are slow. Caching stores frequently-accessed data in fast memory:

```
Request Flow with Cache:

1. Check Cache (Redis) ──Hit──▶ Return (1ms)
       │
       │ Miss
       ▼
2. Query Database (PostgreSQL) ──▶ Store in Cache
       │
       ▼
3. Return Result
```

### 5.2 Setting Up Redis

```bash
# Install Redis
sudo apt install redis-server

# Start service
sudo systemctl start redis

# Test
redis-cli ping  # Should return "PONG"
```

### 5.3 Basic Caching Pattern

```rust
use redis::{Client, AsyncCommands};

struct CachedRepo {
    db: PgPool,
    redis: Client,
}

impl CachedRepo {
    async fn get_document(&self, id: i32) -> Result<Option<Document>, Error> {
        let cache_key = format!("document:{}", id);
        
        // 1. Try cache first
        let mut conn = self.redis.get_async_connection().await?;
        let cached: Option<String> = conn.get(&cache_key).await?;
        
        if let Some(json) = cached {
            // Cache hit!
            return Ok(Some(serde_json::from_str(&json)?));
        }
        
        // 2. Cache miss - query database
        let doc = get_document_from_db(&self.db, id).await?;
        
        // 3. Store in cache (5 minute TTL)
        if let Some(ref d) = doc {
            let json = serde_json::to_string(d)?;
            let _: () = conn.set_ex(&cache_key, json, 300).await?;
        }
        
        Ok(doc)
    }
    
    async fn invalidate_document(&self, id: i32) -> Result<(), Error> {
        let cache_key = format!("document:{}", id);
        let mut conn = self.redis.get_async_connection().await?;
        let _: () = conn.del(&cache_key).await?;
        Ok(())
    }
}
```

### 5.4 Cache Strategies

| Strategy | Description | When to Use |
|----------|-------------|-------------|
| **Cache-Aside** | App checks cache, then DB | Most common |
| **Write-Through** | Write to cache and DB together | Read-heavy, data must be fresh |
| **Write-Behind** | Write to cache, async to DB | High write throughput |
| **TTL** | Auto-expire after time | Session data, temp data |

---

## Part 6: File Storage

### 6.1 Local File Storage

```rust
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

struct LocalFileStorage {
    base_dir: PathBuf,
}

impl LocalFileStorage {
    fn new(base_dir: &str) -> Self {
        LocalFileStorage {
            base_dir: PathBuf::from(base_dir),
        }
    }
    
    async fn save_file(&self, data: &[u8], extension: &str) -> Result<String, Error> {
        let filename = format!("{}.{}", Uuid::new_v4(), extension);
        let path = self.base_dir.join(&filename);
        
        fs::create_dir_all(&self.base_dir).await?;
        fs::write(&path, data).await?;
        
        Ok(filename)
    }
    
    async fn get_file(&self, filename: &str) -> Result<Vec<u8>, Error> {
        let path = self.base_dir.join(filename);
        let data = fs::read(&path).await?;
        Ok(data)
    }
    
    async fn delete_file(&self, filename: &str) -> Result<(), Error> {
        let path = self.base_dir.join(filename);
        fs::remove_file(&path).await?;
        Ok(())
    }
}
```

### 6.2 S3-Compatible Storage

```rust
use aws_sdk_s3::{Client, Config, Region};
use aws_credential_types::Credentials;

struct S3Storage {
    client: Client,
    bucket: String,
}

impl S3Storage {
    fn new(
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        bucket: &str,
        region: &str,
    ) -> Self {
        let creds = Credentials::from_keys(access_key, secret_key, None);
        let config = Config::builder()
            .credentials_provider(creds)
            .region(Region::new(region))
            .endpoint_url(endpoint)
            .build();
        
        let client = Client::from_conf(config);
        
        S3Storage {
            client,
            bucket: bucket.to_string(),
        }
    }
    
    async fn upload(&self, key: &str, data: &[u8]) -> Result<(), Error> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(data.to_vec().into())
            .send()
            .await?;
        
        Ok(())
    }
    
    async fn download(&self, key: &str) -> Result<Vec<u8>, Error> {
        let resp = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        
        let data = resp.body.collect().await?.into_bytes();
        Ok(data.to_vec())
    }
    
    fn get_url(&self, key: &str) -> String {
        format!("https://{}.s3.amazonaws.com/{}", self.bucket, key)
    }
}
```

---

## Part 7: Backup and Recovery

### 7.1 Database Backup

```bash
#!/bin/bash
# backup.sh

# PostgreSQL backup
pg_dump -U myapp myapp_db > backup_$(date +%Y%m%d_%H%M%S).sql

# Compress
gzip backup_*.sql

# Upload to S3
aws s3 cp backup_*.sql.gz s3://my-backups/postgresql/

# Delete local backups older than 7 days
find . -name "backup_*.sql.gz" -mtime +7 -delete
```

### 7.2 Automated Backups with Cron

```bash
# Add to crontab (run daily at 2 AM)
crontab -e

0 2 * * * /path/to/backup.sh >> /var/log/backup.log 2>&1
```

### 7.3 Point-in-Time Recovery

PostgreSQL WAL (Write-Ahead Log) allows recovery to any point:

```bash
# Configure postgresql.conf
wal_level = replica
archive_mode = on
archive_command = 'cp %p /var/lib/postgresql/wal_archive/%f'

# Recovery
recovery_target_time = '2024-01-15 14:30:00'
```

---

## Part 8: Scaling Storage

### 8.1 Read Replicas

```
         ┌─────────────┐
         │   Master    │  ← Writes
         │  (Read/Write)│
         └──────┬──────┘
                │
        ┌───────┼───────┐
        │       │       │
        ▼       ▼       ▼
   ┌────────┐ ┌────────┐ ┌────────┐
   │Replica │ │Replica │ │Replica │  ← Reads
   │ (Read) │ │ (Read) │ │ (Read) │
   └────────┘ └────────┘ └────────┘
```

```rust
// Use different pools for read/write
struct DbPools {
    write_pool: PgPool,   // Master
    read_pool: PgPool,    // Replica
}

impl DbPools {
    async fn get_user(&self, id: i32) -> Result<User, Error> {
        // Read from replica
        get_user_from_pool(&self.read_pool, id).await
    }
    
    async fn update_user(&self, user: &User) -> Result<(), Error> {
        // Write to master
        update_user_in_pool(&self.write_pool, user).await
    }
}
```

### 8.2 Sharding

Split data across multiple databases:

```
Users 1-1000    → Shard A
Users 1001-2000 → Shard B
Users 2001-3000 → Shard C
```

```rust
fn get_shard(user_id: i32) -> PgPool {
    let shard_num = (user_id - 1) / 1000;
    
    match shard_num {
        0 => pools.shard_a.clone(),
        1 => pools.shard_b.clone(),
        2 => pools.shard_c.clone(),
        _ => pools.default.clone(),
    }
}

async fn get_user(user_id: i32) -> Result<User, Error> {
    let pool = get_shard(user_id);
    get_user_from_pool(&pool, user_id).await
}
```

### 8.3 Connection Pool Tuning

```rust
// Production settings
PgPoolOptions::new()
    .max_connections(50)           // Increase for high traffic
    .min_connections(10)           // Keep more idle connections
    .acquire_timeout(Duration::from_secs(60))  // Wait longer under load
    .after_connect(|conn, _| {
        // Set session variables
        Box::pin(async move {
            sqlx::query("SET application_name = 'myapp'")
                .execute(&mut *conn)
                .await?;
            Ok(())
        })
    })
```

---

## Summary: Storage Checklist

### For Beginners

- [ ] Start with SQLite for development
- [ ] Use PostgreSQL for production
- [ ] Always use connection pooling
- [ ] Use transactions for multi-step operations
- [ ] Back up your database regularly
- [ ] Add caching when queries are slow

### For Intermediate Engineers

- [ ] Implement read replicas for scaling reads
- [ ] Use Redis for session/caching
- [ ] Store files in S3, not on server
- [ ] Monitor database performance
- [ ] Set up automated backups
- [ ] Use migrations for schema changes

### For Advanced Engineers

- [ ] Implement sharding for very large datasets
- [ ] Use connection poolers (PgBouncer)
- [ ] Set up point-in-time recovery
- [ ] Implement CDC (Change Data Capture)
- [ ] Use read/write splitting
- [ ] Plan for disaster recovery

---

## Resources

### Learning

- [SQL Tutorial](https://sqlbolt.com/)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [Redis Documentation](https://redis.io/docs/)

### Tools

- **Database GUI**: DBeaver, pgAdmin
- **Migration Tool**: sqlx migrate, flyway
- **Backup Tool**: pg_dump, WAL-G
- **Monitoring**: pg_stat_statements, Prometheus

### Managed Services

- **PostgreSQL**: AWS RDS, Supabase, Neon
- **Redis**: AWS ElastiCache, Redis Cloud
- **S3**: AWS S3, Cloudflare R2, DigitalOcean Spaces
