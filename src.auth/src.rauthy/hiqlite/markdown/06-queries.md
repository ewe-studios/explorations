---
title: Queries
prev: 05-network.md
next: 07-backup.md
---

# Queries

Query execution and routing.

## Query Types

| Type | Routing | Consistency |
|------|---------|-------------|
| `EXECUTE` | Leader | Strong |
| `QUERY` | Local | Eventual |
| `QUERY_AS` | Local | Eventual |

## Execute (Writes)

### Leader Execution

```rust
// hiqlite/src/query/execute.rs
pub async fn execute(&self, sql: &str) -> Result<ExecuteResult, Error> {
    if self.is_leader() {
        // Execute locally on leader
        let entry = LogEntry::new(sql);
        self.raft.propose(entry).await?;
        
        // Apply to state machine
        let result = self.state_machine.apply(&entry).await?;
        Ok(result)
    } else {
        // Forward to leader
        let leader = self.get_leader()?;
        forward_execute(leader, sql).await
    }
}
```

### Forwarding to Leader

```rust
async fn forward_execute(
    leader_addr: &str,
    sql: &str,
) -> Result<ExecuteResult, Error> {
    let client = HttpClient::new(leader_addr);
    let response = client
        .post("/api/execute")
        .json(&ExecuteRequest { sql })
        .send()
        .await?;
    
    Ok(response.json().await?)
}
```

**Aha:** Non-leaders automatically forward writes to leader.

## Query (Reads)

### Local Reads

```rust
// hiqlite/src/query/read.rs
pub async fn query<T: FromRow>(&self, sql: &str) -> Result<Vec<T>, Error> {
    // Read from local SQLite
    let reader = self.sqlite_reader()?;
    let results = reader.query::<T>(sql)?;
    Ok(results)
}
```

### With Parameters

```rust
pub async fn query_with_params<T: FromRow>(
    &self,
    sql: &str,
    params: &[&dyn ToSql],
) -> Result<Vec<T>, Error> {
    let reader = self.sqlite_reader()?;
    let mut stmt = reader.prepare(sql)?;
    
    let results = stmt.query_map(params, |row| {
        T::from_row(row)
    })?;
    
    results.collect()
}
```

## Query As

### Automatic Mapping

```rust
// hiqlite/src/query/mapping.rs
#[derive(FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

// Automatic mapping
let users: Vec<User> = node
    .query_as("SELECT * FROM users WHERE active = ?", &[&true])
    .await?;
```

### Custom Mapping

```rust
pub async fn query_map<T, F>(&self, sql: &str, f: F) -> Result<Vec<T>, Error>
where
    F: Fn(&mut Row<'_>) -> Result<T, Error>,
{
    let reader = self.sqlite_reader()?;
    let mut stmt = reader.prepare(sql)?;
    
    let results = stmt.query_map([], |row| f(row))?;
    results.collect()
}
```

## Consistent Reads

### Read from Leader

```rust
pub async fn query_consistent<T: FromRow>(
    &self,
    sql: &str,
) -> Result<Vec<T>, Error> {
    if !self.is_leader() {
        // Forward to leader for consistent read
        let leader = self.get_leader()?;
        return forward_query(leader, sql).await;
    }
    
    // Local read (leader)
    self.query(sql).await
}
```

## Transactions

### Transaction Execute

```rust
// hiqlite/src/query/transaction.rs
pub async fn transaction<F, T>(&self, f: F) -> Result<T, Error>
where
    F: FnOnce(&Transaction) -> Result<T, Error>,
{
    let sqls = vec![
        "BEGIN".to_string(),
        // ... statements
        "COMMIT".to_string(),
    ];
    
    let entry = LogEntry::Transaction(sqls);
    self.raft.propose(entry).await?;
    
    // Apply transaction
    self.state_machine.apply(&entry).await?
}
```

## Batch Execute

```rust
pub async fn batch_execute(&self, sqls: &[&str]) -> Result<(), Error> {
    let entry = LogEntry::Batch(sqls.to_vec());
    self.raft.propose(entry).await?;
    Ok(())
}
```

## Returning

### Execute with Return

```rust
pub async fn execute_returning<T: FromRow>(
    &self,
    sql: &str,
) -> Result<Vec<T>, Error> {
    // Execute via Raft
    let entry = LogEntry::Execute(sql);
    self.raft.propose(entry).await?;
    
    // Get RETURNING values from state machine
    let results = self.state_machine.last_returning()?;
    
    Ok(results)
}
```

## Next Steps

Continue to [Backup →](07-backup.html).
