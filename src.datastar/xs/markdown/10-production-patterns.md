# xs -- Production Patterns

## Deployment

### Local Single-User

The default mode. One `xs serve` process per store directory:

```bash
xs serve ~/.local/share/cross.stream/store
```

Access via Unix socket at `~/.local/share/cross.stream/store/sock`.

### Networked (TCP/TLS)

Expose to the network:

```bash
xs serve ./store --expose :8080          # Plain TCP
xs serve ./store --expose :443           # TLS (needs cert config)
```

### P2P (Iroh)

No port forwarding needed:

```bash
IROH_SECRET=<persistent-key> xs serve ./store --expose iroh://
# Prints connection ticket to stdout
```

Clients connect via ticket:
```bash
xs cat iroh://<ticket> --follow
```

## Store Management

### Backup: Export/Import

Using xs.nu:
```nushell
# Export
.export ./backup-2024-01-15/

# Creates:
# ./backup-2024-01-15/frames.jsonl
# ./backup-2024-01-15/cas/

# Import into new store
xs serve ./new-store &
.import ./backup-2024-01-15/
```

### CAS Deduplication

Content is automatically deduplicated. If you append the same file 1000 times, CAS stores it once. Only frame metadata (IDs, topics) grows.

### TTL for Storage Control

Use TTL policies to bound storage:
```nushell
# Only keep last 100 log entries per service
.append "service.api.logs" --ttl "last:100" --meta {level: "info", msg: "request"}

# Session data expires after 1 hour
.append "sessions.abc123" --ttl "time:3600000" --meta {user: "alice"}

# Transient notifications — never stored
.append "notifications.toast" --ttl "ephemeral" --meta {msg: "saved!"}
```

## Processor Patterns

### Event Aggregator (Actor)

Count events by type:

```nushell
# Register
.append "stats.register" (
    {
        run: {|frame, state|
            let counts = if ($state == null) { {} } else { $state }
            let topic = $frame.topic
            let new_count = ($counts | get -i $topic | default 0) + 1
            let new_counts = ($counts | upsert $topic $new_count)
            {next: $new_counts, out: $new_counts}
        }
        start: "first"
        return_options: {suffix: "summary", ttl: "last:1"}
    } | to nuon
)
```

### Background Worker (Service)

Process jobs from a queue:

```nushell
.append "worker.spawn" (
    {
        run: {||
            .cat --follow --new --topic "jobs.*" | each {|frame|
                let job = $frame.meta
                # Process the job
                let result = (do_work $job)
                .append "results" --meta {job_id: $frame.id, result: $result}
            }
        }
    } | to nuon
)
```

### HTTP Webhook Handler (Action)

```nushell
.append "webhook.define" (
    {
        run: {|frame|
            let payload = $frame.meta
            # Validate and process webhook
            if ($payload.event == "push") {
                .append "ci.trigger" --meta {repo: $payload.repo, sha: $payload.sha}
            }
            {status: "ok"}
        }
        return_options: {suffix: "ack", ttl: "last:50"}
    } | to nuon
)
```

### Module Hot-Loading

Store reusable code as modules:

```nushell
# Store a module
.append "utils.nu" ("
    export def double [x: int] { $x * 2 }
    export def triple [x: int] { $x * 3 }
" | to text)

# Processors registered AFTER this frame can `use utils`
```

## Monitoring

### Log Stream

The server prints a compact log line for every frame:
```
14:32:01.123 x3h71 user.messages
14:32:01.456 k9f2n xs.pulse
14:32:02.789 yn8yx worker.spawn
```

Format: `HH:MM:SS.mmm <last-5-of-id> <topic>`

### Heartbeat Monitoring

Use `--pulse` to detect connection health:
```bash
xs cat ./store --follow --pulse 5000
# If no xs.pulse frame arrives within 5s, connection may be broken
```

### Processor Health

Check if services are running:
```nushell
.last "worker.running" 1    # Latest .running frame
.last "worker.stopped" 1    # Latest .stopped frame
.last "worker.shutdown" 1   # Latest .shutdown frame
```

## Performance Considerations

### Write Throughput

- Single writer (append_lock mutex serializes all writes)
- fsync on every write (SyncAll mode)
- Throughput: ~10k-50k frames/sec depending on content size and disk speed
- For higher throughput: batch multiple frames in a single request (import)

### Read Throughput

- Multiple concurrent readers (no read locks)
- Point reads: O(1) with bloom filter
- Sequential scans: limited by disk bandwidth
- Live streaming: near-zero latency (broadcast channel)

### Memory Usage

- 32 MiB block cache (configurable at compile time)
- Each follower: 100-frame buffer (mpsc channel capacity)
- Broadcast channel: 100 frames (lagging receivers catch up from store)

### Disk Usage

- Frames: ~100-200 bytes each (JSON + key overhead)
- CAS: actual content size (deduplicated)
- Topic index: ~50 bytes per frame per topic depth level
- LSM compaction: temporary 2x space during merge

## Error Recovery

### Crash Recovery

fjall + cacache both handle crash recovery:
- fjall replays the WAL (write-ahead log) on startup
- cacache uses atomic file writes (write-to-temp + rename)
- No manual recovery needed

### Store Locked

If `xs serve` crashes without cleanup, the lock file persists. The next `xs serve` will print "store is locked" and exit. Solution: ensure the old process is dead, then restart.

### Processor Errors

- **Actor error**: Emits `<topic>.inactive` frame, stops processing. Re-register to restart.
- **Service error**: Emits `<topic>.shutdown` with error info. Does NOT auto-restart on error.
- **Action error**: Emits `<topic>.error` frame with error details. Action remains registered for future calls.

## Integration Patterns

### With Datastar (Frontend)

xs serves as the backend event store for Datastar's SSE streaming:
```bash
# Client subscribes via SSE
curl -H "Accept: text/event-stream" "http://localhost:8080/?follow&topic=ui.*"
```

Datastar's `data-on-sse` attribute connects directly to xs streams.

### With http-nu

The `http-nu` project uses xs as its request/response log:
1. HTTP request arrives at http-nu
2. Request is appended to xs as a frame
3. Nushell route handler processes the frame
4. Response is appended as a frame
5. http-nu sends the response to the client

### With yoke/yoagent

LLM agent harness uses xs for:
- Tool execution logs
- Conversation history
- Agent state persistence
- Inter-agent communication via topics
