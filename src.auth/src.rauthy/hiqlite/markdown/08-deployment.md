---
title: Deployment
prev: 07-backup.md
---

# Deployment

Configuration and cluster setup.

## Configuration

### TOML Config

```toml
# hiqlite.toml
[node]
node_id = 1
node_addr = "192.168.1.10:2380"
data_dir = "/data/hiqlite"

[peers]
nodes = [
    "1:192.168.1.10:2380",
    "2:192.168.1.11:2380",
    "3:192.168.1.12:2380",
]

[server]
bind = "0.0.0.0"
port = 2380
raft_port = 2381
dashboard_port = 8080

[database]
page_size = 4096
journal_size_limit = 16384
wal_autocheckpoint = 4000

[backup]
enabled = true
schedule = "0 2 * * *"
s3_url = "s3://bucket/backups/"
retention_days = 30

[tls]
enabled = true
cert_path = "/etc/hiqlite/cert.pem"
key_path = "/etc/hiqlite/key.pem"
```

### Environment Variables

```bash
export HIQLITE_NODE_ID=1
export HIQLITE_NODE_ADDR=192.168.1.10:2380
export HIQLITE_PEERS="1:192.168.1.10:2380,2:192.168.1.11:2380,3:192.168.1.12:2380"
export HIQLITE_DATA_DIR=/data/hiqlite
```

## Single Node

```rust
use hiqlite::{start_node, NodeConfig};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = NodeConfig {
        node_id: 1,
        node_addr: "127.0.0.1:2380".to_string(),
        peers: vec!["1:127.0.0.1:2380".to_string()],
        data_dir: "/data/hiqlite".to_string(),
    };
    
    let node = start_node(config).await?;
    
    // Use node
    node.execute("CREATE TABLE users (id INTEGER PRIMARY KEY)").await?;
    
    Ok(())
}
```

## 3-Node Cluster

### Node 1

```toml
[node]
node_id = 1
node_addr = "192.168.1.10:2380"
data_dir = "/data/hiqlite"

[peers]
nodes = [
    "1:192.168.1.10:2380",
    "2:192.168.1.11:2380",
    "3:192.168.1.12:2380",
]
```

### Node 2

```toml
[node]
node_id = 2
node_addr = "192.168.1.11:2380"
data_dir = "/data/hiqlite"

[peers]
nodes = [
    "1:192.168.1.10:2380",
    "2:192.168.1.11:2380",
    "3:192.168.1.12:2380",
]
```

### Node 3

```toml
[node]
node_id = 3
node_addr = "192.168.1.12:2380"
data_dir = "/data/hiqlite"

[peers]
nodes = [
    "1:192.168.1.10:2380",
    "2:192.168.1.11:2380",
    "3:192.168.1.12:2380",
]
```

## Docker

### Single Node

```yaml
# docker-compose.yml
version: '3'

services:
  hiqlite:
    image: ghcr.io/sebadob/hiqlite:latest
    ports:
      - "2380:2380"
      - "8080:8080"
    volumes:
      - ./config.toml:/app/hiqlite.toml
      - hiqlite-data:/data/hiqlite

volumes:
  hiqlite-data:
```

### HA Cluster

```yaml
# docker-compose.ha.yml
version: '3'

services:
  hiqlite-1:
    image: ghcr.io/sebadob/hiqlite:latest
    hostname: hiqlite-1
    volumes:
      - ./config-1.toml:/app/hiqlite.toml
      - hiqlite-data-1:/data/hiqlite
    networks:
      - hiqlite

  hiqlite-2:
    image: ghcr.io/sebadob/hiqlite:latest
    hostname: hiqlite-2
    volumes:
      - ./config-2.toml:/app/hiqlite.toml
      - hiqlite-data-2:/data/hiqlite
    networks:
      - hiqlite

  hiqlite-3:
    image: ghcr.io/sebadob/hiqlite:latest
    hostname: hiqlite-3
    volumes:
      - ./config-3.toml:/app/hiqlite.toml
      - hiqlite-data-3:/data/hiqlite
    networks:
      - hiqlite

  haproxy:
    image: haproxy:latest
    ports:
      - "2380:2380"
    volumes:
      - ./haproxy.cfg:/usr/local/etc/haproxy/haproxy.cfg
    networks:
      - hiqlite

networks:
  hiqlite:
    driver: bridge

volumes:
  hiqlite-data-1:
  hiqlite-data-2:
  hiqlite-data-3:
```

## Dashboard

Built-in web UI at `http://node:8080/dashboard`

Features:
- Cluster status
- Node health
- Query execution
- Metrics

## Monitoring

### Metrics Endpoint

```toml
[metrics]
enabled = true
bind = "0.0.0.0:9090"
```

### Prometheus

```yaml
scrape_configs:
  - job_name: 'hiqlite'
    static_configs:
      - targets: ['node1:9090', 'node2:9090', 'node3:9090']
```

## Performance Tuning

### SSD Required

- **M2 SSD** — 24.5k inserts/s
- **SATA SSD** — 16.5k inserts/s
- **HDD** — Not recommended

### Memory

- **Single node** — ~100MB
- **HA cluster** — ~150MB per node

### CPU

- Prefers fewer cores with higher single-core speed
- Benefits from fast memory

## Summary

| Setup | Nodes | Memory | Use Case |
|-------|-------|--------|----------|
| Single | 1 | ~100MB | Development |
| HA | 3 | ~150MB | Production |

**Aha:** Start simple, scale to HA when needed.
