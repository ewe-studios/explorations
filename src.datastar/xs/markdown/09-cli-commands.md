# xs -- CLI Commands

## Binary: xs

**File**: `src/main.rs`

The `xs` binary provides subcommands for interacting with xs stores. Built with clap 4.

## xs serve

Starts the xs server.

```bash
xs serve <path> [--expose <address>]
```

### Arguments

- `<path>` — Directory for the store (created if needed). Contains `fjall/`, `cacache/`, and `sock`.
- `--expose <address>` — Optional additional listener:
  - `:8080` — TCP on port 8080
  - `/tmp/xs.sock` — Additional Unix socket
  - `iroh://` — Iroh P2P (prints connection ticket to stdout)

### Startup

1. Open/create store at `<path>`
2. Initialize Nushell engine
3. Start log stream printer
4. Start actor/service/action processors
5. Bind Unix socket at `<path>/sock`
6. Optionally bind `--expose` listener
7. Append `xs.start` frame
8. Enter accept loop

### Shutdown

On SIGINT (Ctrl-C):
1. Append `xs.stopping` frame
2. Wait up to 3 seconds for services to drain
3. Exit

## xs cat

Stream frames from the store.

```bash
xs cat <addr> [flags]
```

### Flags

| Flag | Short | Default | Purpose |
|------|-------|---------|---------|
| `--follow` | `-f` | off | Stream indefinitely |
| `--pulse <ms>` | | none | Heartbeat interval (enables follow) |
| `--new` | `-n` | false | Skip historical, live only |
| `--after <id>` | | none | Start after this ID (exclusive) |
| `--from <id>` | | none | Start from this ID (inclusive) |
| `--limit <n>` | `-l` | none | Maximum frames to return |
| `--last <n>` | | none | Return last N frames |
| `--topic <pattern>` | `-t` | none | Topic filter (wildcards: `user.*`) |
| `--sse` | | false | Output as SSE instead of NDJSON |
| `--with-timestamp` | | false | Add timestamp field to output |

### Output Format

Default (NDJSON):
```
{"id":"...","topic":"user.messages","hash":"sha256-...","meta":null,"ttl":null}
{"id":"...","topic":"user.events","hash":null,"meta":{"type":"click"},"ttl":null}
```

With `--sse`:
```
id: 0v4fkdz7k2mfj9f2nn8yx3h71
data: {"id":"0v4fkdz7k2mfj9f2nn8yx3h71","topic":"user.messages","hash":"sha256-..."}

```

### Broken Pipe Handling

On Unix, `xs cat` detects broken pipes (piped to `head`, `grep`, etc.) and exits cleanly without error using `AsyncFd` polling on stdout.

## xs append

Append a frame to the store.

```bash
echo "content" | xs append <addr> <topic> [flags]
```

### Arguments

- `<addr>` — Store address (path, host:port, or iroh ticket)
- `<topic>` — Topic name for the frame

### Flags

| Flag | Default | Purpose |
|------|---------|---------|
| `--meta <json>` | none | Inline JSON metadata |
| `--ttl <policy>` | forever | TTL policy string |
| `--with-timestamp` | false | Include timestamp in response |

### Behavior

- If stdin is piped: reads all stdin, stores in CAS, frame gets `hash`
- If stdin is a terminal (no pipe): no CAS content, frame only has `meta` (if specified)
- Response: JSON frame with assigned ID

## xs cas

Retrieve content from CAS by hash.

```bash
xs cas <addr> <hash>
```

### Optimization

For local Unix socket connections, if `<addr>` is a path, the CLI reads directly from the filesystem (`cacache/content-v2/...`) without going through the HTTP API. This avoids serialization overhead for large blobs.

## xs cas-post

Store content in CAS and return the hash.

```bash
echo "content" | xs cas-post <addr>
```

Response: SRI hash string (`sha256-abc123...`)

## xs remove

Remove a frame by ID.

```bash
xs remove <addr> <id>
```

Removes from both the stream keyspace and all topic index entries.

## xs last

Get the most recent frame(s).

```bash
xs last <addr> [topic] [count] [flags]
```

### Arguments

- `<addr>` — Store address
- `[topic]` — Optional topic filter
- `[count]` — Number of frames (default 1)

### Flags

| Flag | Purpose |
|------|---------|
| `--follow` | After returning last N, continue streaming new frames |
| `--with-timestamp` | Add timestamp field |

### Disambiguation (ADR 0002)

Because topics cannot start with digits:
- `xs last store 5` — last 5 frames for topic "store"
- `xs last 5` — last 5 frames for all topics
- `xs last store` — last 1 frame for topic "store"

## xs get

Get a single frame by ID.

```bash
xs get <addr> <id> [--with-timestamp]
```

Returns JSON frame. Exits with code 1 if not found.

## xs import

Import a pre-formed frame (preserving original ID).

```bash
echo '{"id":"...","topic":"...","hash":"..."}' | xs import <addr>
```

Used for replication and backup restore. The frame's original SCRU128 ID is preserved.

## xs version

Get server version.

```bash
xs version <addr>
```

Returns the version string (e.g., `0.12.1-dev`).

## xs eval

Evaluate a Nushell script with store access.

```bash
xs eval <addr> [file] [-c <script>]
```

### Modes

- `xs eval <addr> script.nu` — Evaluate a file
- `xs eval <addr> -c '.cat --last 5'` — Evaluate inline script
- `echo '.cat --last 5' | xs eval <addr>` — Evaluate from stdin

The script runs with all `.` commands available (`.cat`, `.append`, `.get`, etc.).

## xs nu

Manage the xs.nu Nushell module.

```bash
xs nu [--install] [--clean] [--lib-path <path>] [--autoload-path <path>]
```

### Flags

| Flag | Purpose |
|------|---------|
| `--install` | Install xs.nu to Nushell's lib directory |
| `--clean` | Remove xs.nu from Nushell |
| `--lib-path` | Custom library installation path |
| `--autoload-path` | Custom autoload stub path |

Installation creates two files:
1. `xs.nu` — The module source (in NU_LIB_DIRS)
2. `xs-use.nu` — Autoload stub that imports the module on shell startup

## xs scru128

Generate and manipulate SCRU128 IDs.

```bash
xs scru128                    # Generate new ID
xs scru128 unpack <id>        # Decompose into components
echo '{"timestamp":...}' | xs scru128 pack  # Reconstruct from components
```

### unpack output

```json
{
  "timestamp": 1714924800.123,
  "counter_hi": 42,
  "counter_lo": 7,
  "node": "a1b2c3d4"
}
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (message printed to stderr) or NotFound (no message) |

## Address Resolution

All commands that take `<addr>`:
- `/path/to/store` → Unix socket at `<path>/sock`
- `./relative/store` → Unix socket at `<path>/sock`
- `:8080` → TCP `127.0.0.1:8080`
- `host:8080` → TCP
- `https://host:8080` → TLS
- `https://user:pass@host:8080` → TLS with auth
- `iroh://<ticket>` → Iroh P2P

Environment variable `XS_ADDR` provides the default if `<addr>` is omitted (in xs.nu module only — the binary always requires it).
