---
title: "Neodatabase Database Cloning Deep Dive"
subtitle: "How Neo4j enables instant database cloning with copy-on-write semantics"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/src.db/src.neodatabase
related: 01-storage-engine-deep-dive.md
---

# Database Cloning Deep Dive: Neodatabase

## Overview

This document covers Neo4j's database cloning capabilities - how copy-on-write semantics, snapshot isolation, and sparse file systems enable instant database cloning for development, testing, and staging environments.

## Part 1: The Database Cloning Problem

### Traditional Database Cloning Challenges

```
Traditional Database Copy (Slow, Expensive):

┌─────────────────────────────────────────────────────────┐
│ Production Database: 500GB                              │
│                                                          │
│ Step 1: Full Backup                                     │
│   - Read all 500GB from disk                            │
│   - Write to backup file: 500GB                         │
│   - Time: ~2-4 hours (depending on I/O)                 │
│                                                          │
│ Step 2: Transfer to Target                              │
│   - Copy 500GB backup to dev/staging server             │
│   - Network transfer: ~1-2 hours (1Gbps)                │
│                                                          │
│ Step 3: Restore                                         │
│   - Read 500GB backup                                   │
│   - Write 500GB to new database files                   │
│   - Time: ~2-4 hours                                    │
│                                                          │
│ Total Time: 5-10 hours                                  │
│ Total Storage: 500GB (prod) + 500GB (backup) + 500GB (clone) = 1.5TB
│                                                          │
│ Problems:                                                │
│ - Slow: Hours to create clone                           │
│ - Expensive: Full storage cost per clone                │
│ - Stale: Data outdated by time clone completes          │
└───────────────────────────────────────────────────────────┘
```

### Modern Cloning with Copy-on-Write

```
Copy-on-Write (CoW) Database Cloning:

┌─────────────────────────────────────────────────────────┐
│ Production Database: 500GB                              │
│                                                          │
│ Step 1: Create Snapshot                                 │
│   - Mark current state as immutable                     │
│   - Create metadata pointer                             │
│   - Time: < 1 second                                    │
│                                                          │
│ Step 2: Clone Database                                  │
│   - Create new database metadata                        │
│   - Point to same underlying data blocks                │
│   - Time: < 1 second                                    │
│                                                          │
│ Step 3: First Write to Clone                            │
│   - Allocate new block for modified data                │
│   - Write only changed data                             │
│   - Update clone's block pointers                       │
│   - Original data unchanged                             │
│                                                          │
│ Total Time: Instant (< 1 second)                        │
│ Storage Efficiency: Only store differences              │
│                                                          │
│ Benefits:                                                │
│ - Instant: Clones available immediately                 │
│ - Efficient: Share unchanged data blocks                │
│ - Fresh: Can clone from recent snapshot                 │
└───────────────────────────────────────────────────────────┘
```

## Part 2: Neo4j Cloning Architecture

### Multi-Version Concurrency Control (MVCC)

```
Neo4j MVCC for Snapshot Isolation:

┌─────────────────────────────────────────────────────────┐
│ Transaction Timeline                                    │
│                                                          │
│ T0: Initial State                                       │
│   Node 1: {name: "Alice", version: 0}                   │
│   Node 2: {name: "Bob", version: 0}                     │
│                                                          │
│ T1: Transaction A starts (TX_ID: 100)                   │
│   - Reads Node 1 (version 0)                            │
│                                                          │
│ T2: Transaction B starts (TX_ID: 101)                   │
│   - Updates Node 1 to {name: "Alice Updated"}           │
│   - Creates NEW version with TX_ID: 101                 │
│   - Old version (TX_ID: 100) still visible to TX A      │
│                                                          │
│ T3: Transaction C starts (TX_ID: 102)                   │
│   - Sees Node 1 with version TX_ID: 101                 │
│   - Old version still exists (for TX A)                 │
│                                                          │
│ Storage Layout:                                          │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Node Record 1 (Current)                             │ │
│ │   version: 101, data: "Alice Updated"               │ │
│ │   prev_version_ptr: ──> Node Record 1 (Old)         │ │
│ └─────────────────────────────────────────────────────┘ │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ Node Record 1 (Old)                                 │ │
│ │   version: 100, data: "Alice"                       │ │
│ │   prev_version_ptr: NULL                            │ │
│ └─────────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────────┘

Snapshot Creation:
- Record current TX ID as snapshot point
- All reads use data visible at that TX ID
- Unchanged data shared across snapshots
```

### Physical Storage with Copy-on-Write

```
Neo4j Store File COW:

┌─────────────────────────────────────────────────────────┐
│ Original Database (500GB)                               │
│                                                          │
│ neostore.nodestore.db (100GB)                           │
│ ┌─────────┬─────────┬─────────┬─────────┐              │
│ │Block 0  │Block 1  │Block 2  │ ...     │              │
│ │Node 1   │Node 2   │Node 3   │         │              │
│ └─────────┴─────────┴─────────┴─────────┘              │
│                                                          │
│ After Clone (No Changes Yet):                           │
│                                                          │
│ Original DB          Cloned DB                          │
│ ┌─────────┐         ┌─────────┐                         │
│ │metadata │         │metadata │                         │
│ │ptr ─────┼─┐       │ptr ─────┼─┐                       │
│ └─────────┘ │       └─────────┘ │                       │
│             │                   │                       │
│             └───────┬───────────┘                       │
│                     ▼                                   │
│           ┌─────────────────────┐                      │
│           │Shared Data Blocks   │                      │
│           │Block 0, 1, 2, ...   │                      │
│           │(Read-only)          │                      │
│           └─────────────────────┘                      │
│                                                          │
│ After Write to Clone (Update Node 2):                   │
│                                                          │
│ Original DB          Cloned DB                          │
│ ┌─────────┐         ┌─────────┐                         │
│ │metadata │         │metadata │                         │
│ │ptr ─────┼─┐       │ptr ─────┼─┐                       │
│ └─────────┘ │       └─────────┘ │                       │
│             │                   │                       │
│             ▼                   ▼                       │
│           ┌───────────┐   ┌─────────────┐              │
│           │Original   │   │Modified     │              │
│           │Blocks     │   │Blocks       │              │
│           │Block 0    │   │Block 1'     │ ← New copy   │
│           │Block 1    │   │(Node 2 upd) │              │
│           │Block 2    │   └─────────────┘              │
│           └───────────┘                                │
│                                                          │
│ Storage Used: 500GB (original) + ~8KB (one modified block)
└───────────────────────────────────────────────────────────┘
```

### Neo4j Fabric for Multi-Database

```
Neo4j Fabric Architecture:

┌─────────────────────────────────────────────────────────┐
│ Neo4j Fabric (Multi-Database Support)                   │
│                                                          │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ System Database (system)                            │ │
│ │ - User management                                   │ │
│ │ - Database metadata                                 │ │
│ │ - Access control                                    │ │
│ └─────────────────────────────────────────────────────┘ │
│                                                          │
│ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐     │
│ │ neo4j        │ │ neo4j-clone1 │ │ neo4j-clone2 │     │
│ │ (Production) │ │ (Dev)        │ │ (Staging)    │     │
│ │ 500GB        │ │ 500GB*       │ │ 500GB*       │     │
│ │              │ │ (*shared)    │ │ (*shared)    │     │
│ └──────────────┘ └──────────────┘ └──────────────┘     │
│                                                          │
│ Query Across Databases:                                 │
│ USE neo4j;                                              │
│ CALL dbms.cloneDatabase('neo4j', 'neo4j-clone1', {     │
│   targetUri: 'bolt://localhost:7687',                  │
│   overwriteExisting: false                             │
│ });                                                     │
│                                                          │
│ Switch Between Clones:                                  │
│ USE neo4j-clone1;                                       │
│ MATCH (n:Person) RETURN count(n);                       │
└───────────────────────────────────────────────────────────┘
```

## Part 3: Cloning Implementation

### Neo4j Backup and Clone

```cypher
-- Create a backup (for cloning)
CALL dbms.backup('neo4j', 'full-backup-2024-01-15')
YIELD backupId, successful, message
RETURN *;

-- Clone database from backup
CALL dbms.cloneDatabase(
  'neo4j',                              -- Source database
  'neo4j-dev',                          -- Target database name
  {
    targetUri: 'bolt://localhost:7687', -- Target URI
    overwriteExisting: false,           -- Don't overwrite
    backupId: 'full-backup-2024-01-15'  -- Optional: specific backup
  }
)
YIELD databaseName, successful, message
RETURN *;

-- Clone with filtering (subset of data)
CALL dbms.cloneDatabase(
  'neo4j',
  'neo4j-dev-filtered',
  {
    targetUri: 'bolt://localhost:7687',
    filtering: {
      labels: ['Person', 'Company'],    -- Only these labels
      relationships: ['WORKS_AT', 'FRIENDS_WITH']
    }
  }
);

-- List all clones
CALL dbms.databases()
YIELD name, address, currentStatus
WHERE name STARTS WITH 'neo4j-'
RETURN name, currentStatus;
```

### Shell-Based Cloning (Neo4j Admin)

```bash
#!/bin/bash
# Neo4j Database Clone Script

NEO4J_HOME="/var/lib/neo4j"
BACKUP_DIR="/backup/neo4j"
SOURCE_DB="neo4j"
CLONE_DB="neo4j-dev-$(date +%Y%m%d)"

# Step 1: Create online backup
echo "Creating backup of $SOURCE_DB..."
$NEO4J_HOME/bin/neo4j-admin database dump $SOURCE_DB \
    --to-path=$BACKUP_DIR \
    --dump-name=clone-source

# Step 2: Load as new database
echo "Creating clone $CLONE_DB..."
$NEO4J_HOME/bin/neo4j-admin database load $CLONE_DB \
    --from-path=$BACKUP_DIR \
    --dump-name=clone-source \
    --overwrite-destination=false

# Step 3: Start the cloned database
echo "Starting cloned database..."
$NEO4J_HOME/bin/neo4j-admin database start $CLONE_DB

# Step 4: Verify
echo "Verifying clone..."
$NEO4J_HOME/bin/cypher-shell \
    -u neo4j -p password \
    "USE $CLONE_DB RETURN count(*) AS node_count;"

echo "Clone complete: $CLONE_DB"
```

### Kubernetes Database Cloning

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: neo4j-clone-job
spec:
  template:
    spec:
      containers:
        - name: neo4j-admin
          image: neo4j:5.15-enterprise
          command:
            - /bin/sh
            - -c
            - |
              # Clone production to staging
              cypher-shell -u neo4j -p $PASSWORD \
                "CALL dbms.cloneDatabase('neo4j', 'neo4j-staging', {
                  targetUri: 'bolt://neo4j-staging:7687',
                  overwriteExisting: true
                });"
          env:
            - name: PASSWORD
              valueFrom:
                secretKeyRef:
                  name: neo4j-secret
                  key: password
      restartPolicy: OnFailure
---
apiVersion: v1
kind: CronJob
metadata:
  name: neo4j-daily-clone
spec:
  schedule: "0 3 * * *"  # Daily at 3 AM
  jobTemplate:
    spec:
      template:
        spec:
          containers:
            - name: neo4j-admin
              image: neo4j:5.15-enterprise
              command:
                - /bin/sh
                - -c
                - |
                  CLONE_NAME="neo4j-dev-$(date +%Y%m%d)"
                  cypher-shell -u neo4j -p $PASSWORD \
                    "CALL dbms.cloneDatabase('neo4j', '$CLONE_NAME', {
                      targetUri: 'bolt://neo4j-dev:7687',
                      overwriteExisting: false
                    });"
          restartPolicy: OnFailure
```

## Part 4: File System Support

### ZFS Snapshots for Neo4j

```bash
# ZFS-based instant cloning

# Create ZFS dataset for Neo4j
zfs create tank/neo4j
zfs set compression=lz4 tank/neo4j

# Start Neo4j with data on ZFS
NEO4J_DATA_PATH="/tank/neo4j/data"

# Create instant snapshot
zfs snapshot tank/neo4j@pre-clone

# Create writable clone from snapshot
zfs clone tank/neo4j@pre-clone tank/neo4j-clone1

# Mount clone at different location
zfs set mountpoint=/var/lib/neo4j-clone1 tank/neo4j-clone1

# Configure Neo4j to use clone
# In neo4j-clone1.conf:
# dbms.directories.data=/var/lib/neo4j-clone1/data

# Storage efficiency:
# zfs list -t snapshot
# NAME                      USED  AVAIL  REFER  MOUNTPOINT
# tank/neo4j@pre-clone     1.2G      -   500G  -
# tank/neo4j-clone1        890M   499G   500G  /var/lib/neo4j-clone1
# Only 890MB additional for 500GB clone!
```

### Btrfs Subvolume Snapshots

```bash
# Btrfs-based cloning

# Create subvolume for Neo4j
btrfs subvolume create /srv/neo4j-data

# Start Neo4j
NEO4J_DATA_PATH="/srv/neo4j-data"

# Create snapshot
btrfs subvolume snapshot /srv/neo4j-data /srv/neo4j-data-snapshot

# Create writable clone
btrfs subvolume snapshot /srv/neo4j-data-snapshot /srv/neo4j-clone1

# Configure Neo4j for clone
# dbms.directories.data=/srv/neo4j-clone1

# Check space usage
btrfs filesystem usage /srv/
# Only differential storage used
```

### LVM Thin Provisioning

```bash
# LVM thin provisioning for Neo4j

# Create thin pool
lvcreate --type thin-pool --size 1T --name thin-pool vg-data

# Create thin volume for production
lvcreate --virtual-size 500G --thin-pool thin-pool --name neo4j-prod

# Create thin volume for clone (instant, same data)
lvcreate --snapshot --name neo4j-clone1 /dev/vg-data/neo4j-prod

# Both volumes share same data blocks initially
# lvs shows actual usage:
#   LV              VG        Attr       LSize   Pool     Origin
#   neo4j-prod      vg-data   Vwi-a-tz-- 500.00g thin-pool
#   neo4j-clone1    vg-data   Vwi---tz-- 500.00g thin-pool neo4j-prod

# Mount clone
mount /dev/vg-data/neo4j-clone1 /var/lib/neo4j-clone1
```

## Part 5: Development Workflows

### Git-like Database Branching

```bash
#!/bin/bash
# Database branching workflow (like Git)

# Create main branch (production)
neo4j-admin database create-branch neo4j-main

# Feature branch for development
neo4j-admin database create-branch \
    --from neo4j-main \
    --name feature-new-schema

# Work on feature branch
# ... schema changes, testing ...

# Create test data branch
neo4j-admin database create-branch \
    --from neo4j-main \
    --name test-data-load

# Load test data
# ... bulk import ...

# Compare branches
neo4j-admin database diff \
    --source neo4j-main \
    --target feature-new-schema

# Merge changes (application-level)
# Export changes from feature branch
cypher-shell -u neo4j -p password \
    "USE feature-new-schema MATCH (n) RETURN n" > changes.cypher

# Apply to main
cypher-shell -u neo4j -p password \
    -f changes.cypher

# Delete old branches
neo4j-admin database delete feature-old
```

### CI/CD with Database Clones

```yaml
# GitHub Actions with Neo4j clones

name: Neo4j Integration Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    services:
      neo4j:
        image: neo4j:5.15-enterprise
        env:
          NEO4J_AUTH: neo4j/password
          NEO4J_ACCEPT_LICENSE_AGREEMENT: eval
        ports:
          - 7687:7687
        options: >-
          --health-cmd="cypher-shell -u neo4j -p password 'RETURN 1'"
          --health-interval=10s
          --health-timeout=5s
          --health-retries=5

    steps:
      - uses: actions/checkout@v3

      - name: Clone production database
        run: |
          # Download latest production snapshot
          aws s3 cp s3://backups/neo4j-latest.dump /tmp/neo4j.dump

          # Load as test database
          neo4j-admin database load neo4j-test \
            --from-path=/tmp \
            --dump-name=neo4j

      - name: Run migrations
        run: |
          cypher-shell -u neo4j -p password \
            -f migrations/new-schema.cypher

      - name: Run integration tests
        run: |
          cargo test --features integration-tests

      - name: Compare schemas
        run: |
          # Export before/after schemas
          cypher-shell -u neo4j -p password \
            "CALL db.schema.visualization()" > schema-after.txt

      - name: Cleanup
        if: always()
        run: |
          neo4j-admin database delete neo4j-test
```

### Local Development with Clones

```rust
use std::process::Command;

/// Manage local Neo4j database clones for development
pub struct Neo4jCloneManager {
    neo4j_home: String,
    base_database: String,
}

impl Neo4jCloneManager {
    pub fn new(neo4j_home: &str, base_database: &str) -> Self {
        Self {
            neo4j_home: neo4j_home.to_string(),
            base_database: base_database.to_string(),
        }
    }

    /// Create a new development clone
    pub fn create_clone(&self, clone_name: &str) -> Result<(), String> {
        // Use neo4j-admin to clone
        let output = Command::new(&format!("{}/bin/neo4j-admin", self.neo4j_home))
            .args([
                "database",
                "dump",
                &self.base_database,
                "--to-path=/tmp/neo4j-clones",
                &format!("--dump-name={}", clone_name),
            ])
            .output();

        match output {
            Ok(out) if out.status.success() => Ok(()),
            Ok(out) => Err(format!(
                "Clone failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )),
            Err(e) => Err(format!("Command failed: {}", e)),
        }
    }

    /// Switch to a specific clone
    pub fn switch_clone(&self, clone_name: &str) -> Result<(), String> {
        // Stop current
        Command::new(&format!("{}/bin/neo4j", self.neo4j_home))
            .arg("stop")
            .output()
            .map_err(|e| e.to_string())?;

        // Update symlink
        std::os::unix::fs::symlink(
            format!("/var/lib/neo4j/data/{}", clone_name),
            "/var/lib/neo4j/data/current",
        )
        .map_err(|e| e.to_string())?;

        // Start with new clone
        Command::new(&format!("{}/bin/neo4j", self.neo4j_home))
            .arg("start")
            .output()
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// List available clones
    pub fn list_clones(&self) -> Result<Vec<String>, String> {
        let clones_path = format!("{}/data/databases", self.neo4j_home);
        let mut clones = Vec::new();

        for entry in std::fs::read_dir(clones_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().into_string().map_err(|_| "Invalid name")?;
            if name.starts_with("neo4j") {
                clones.push(name);
            }
        }

        Ok(clones)
    }

    /// Delete old clones (cleanup)
    pub fn delete_clone(&self, clone_name: &str) -> Result<(), String> {
        Command::new(&format!("{}/bin/neo4j-admin", self.neo4j_home))
            .args(["database", "delete", clone_name])
            .output()
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

// Usage in development:
// let manager = Neo4jCloneManager::new("/var/lib/neo4j", "neo4j-prod");
// manager.create_clone("feature-auth")?;
// manager.switch_clone("feature-auth")?;
```

---

*This document is part of the Neodatabase exploration series. See [exploration.md](./exploration.md) for the complete index.*
