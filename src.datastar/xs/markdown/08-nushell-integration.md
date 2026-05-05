# xs -- Nushell Integration

## Overview

xs embeds a full Nushell runtime. Every processor (actor, service, action) executes Nushell closures. The server also exposes an `/eval` endpoint for ad-hoc script evaluation. The integration provides custom commands (prefixed with `.`) that give Nushell scripts direct access to the store.

## Nu Engine

**File**: `src/nu/engine.rs`

```rust
pub struct Engine {
    pub state: EngineState,
}
```

### Initialization

```rust
impl Engine {
    pub fn new() -> Self {
        // 1. Create EngineState with default context
        // 2. Add nu-command's full command set
        // 3. Merge parent process environment variables
        // 4. Set PWD to current directory
    }
}
```

### Key Methods

| Method | Purpose |
|--------|---------|
| `new()` | Creates engine with default + shell + CLI contexts |
| `add_commands(commands)` | Registers custom `.` commands |
| `add_alias(name, target)` | Creates command aliases |
| `eval(input, expression)` | Evaluates an expression with pipeline input |
| `parse_closure(script)` | Parses a script string into a Closure object |
| `add_module(name, content)` | Registers a module via temp file + VFS |
| `run_closure_in_job(...)` | Runs closure on a named background job |

### run_closure_in_job

This is the primary execution method for processors:

```rust
pub fn run_closure_in_job(
    &mut self,
    closure: &Closure,
    args: Vec<Value>,
    pipeline_input: PipelineData,
    job_name: &str,
) -> Result<Value>
```

It:
1. Creates a named background job in Nushell's job system
2. Injects arguments into the closure's captures
3. Feeds `pipeline_input` as stdin
4. Evaluates the closure body
5. Collects the result as a single `Value`

## Custom Nushell Commands

All custom commands are prefixed with `.` to avoid collisions with built-in Nushell commands.

### .append (Buffered — Actors)

**File**: `src/nu/commands/append_buffered.rs`

In actors, `.append` buffers frames rather than writing immediately:

```nushell
.append "topic.name" --meta {key: "value"} --ttl "last:5"
```

Buffered frames are flushed after the actor closure returns. This ensures atomicity — either all outputs from a single invocation are written, or none.

### .append (Direct — Services/Actions/Eval)

**File**: `src/nu/commands/append.rs`

In services and eval context, `.append` writes immediately:

```nushell
"hello world" | .append "messages"
# Streams stdin to CAS, appends frame with hash

.append "events" --meta {type: "click", x: 100, y: 200}
# No CAS content, just inline meta
```

Flags:
- `--meta <json>`: Inline metadata
- `--ttl <policy>`: TTL policy string
- `--with-timestamp`: Include timestamp in response

### .cas

**File**: `src/nu/commands/cas.rs`

Retrieves CAS content by hash:

```nushell
.cas "sha256-abc123..."
# Returns the raw content as a string/binary
```

### .cat (Sync — Actors)

Returns a list of frames (not a stream). Used in actors where blocking is acceptable:

```nushell
.cat --topic "user.*" --last 10
# Returns list of 10 most recent user.* frames
```

### .cat (Streaming — Services/Actions/Eval)

Returns a `ListStream` that can be piped and processed lazily:

```nushell
.cat --follow --topic "events.*" | each {|frame|
    # Process each frame as it arrives
}
```

Flags (both variants):
- `--follow`: Stream indefinitely
- `--pulse <ms>`: Heartbeat interval
- `--new`: Skip historical
- `--after <id>`: Start after ID
- `--from <id>`: Start from ID
- `--limit <n>`: Max frames
- `--last <n>`: Last N frames
- `--topic <pattern>`: Topic filter
- `--with-timestamp`: Add timestamp field

### .get

Retrieves a single frame by SCRU128 ID:

```nushell
.get "0v4fkdz7k2mfj9f2nn8yx3h71"
# Returns the frame record
```

### .last (Sync — Actors)

```nushell
.last "user.messages" 5
# Returns list of 5 most recent frames for topic
```

### .last (Streaming — Services/Actions/Eval)

```nushell
.last "user.messages" --follow
# Returns last frame, then streams new ones
```

### .remove / .rm

Removes a frame by ID:

```nushell
.rm "0v4fkdz7k2mfj9f2nn8yx3h71"
```

### .id

SCRU128 ID operations:

```nushell
.id              # Generate new ID
.id unpack <id>  # Decompose into components
.id pack         # Reconstruct from components (piped as input)
```

## Virtual File System (VFS) for Modules

**File**: `src/nu/vfs.rs`

Nushell modules can be stored as frames in the xs store. They're loaded into a virtual filesystem at engine initialization.

### How It Works

1. Frames with topics ending in `.nu` are treated as module definitions
2. Topic path maps to filesystem path: `discord.api.nu` → `discord/api/mod.nu`
3. Modules are loaded from CAS content
4. Scripts can then `use discord/api` naturally

### Loading Process

```rust
pub fn load_modules(store: &Store, engine: &mut Engine, as_of: Option<Scru128Id>) {
    // 1. Scan all frames up to as_of
    // 2. Collect latest frame for each *.nu topic
    // 3. Read CAS content for each
    // 4. Register as module in engine via add_module()
}
```

The `as_of` parameter enables point-in-time module loading — actors see modules as they existed at their registration time.

## NuScriptConfig

**File**: `src/nu/config.rs`

All processor scripts must evaluate to a record containing at minimum a `run:` field:

```rust
pub struct NuScriptConfig {
    pub run_closure: Closure,
    pub full_config_value: Value,  // The entire record for other fields
}
```

The parsing process:
1. Evaluate the script as a Nushell expression
2. Extract the `run` field as a Closure
3. Keep the full record for processor-specific config extraction

### Config Record Fields (by processor type)

**Actor:**
```nushell
{
    run: {|frame, state| ...}
    start: "first" | "new" | {after: "<id>"}
    pulse: 5000           # Optional heartbeat ms
    return_options: {suffix: "out", target: "cas", ttl: "last:10"}
}
```

**Service:**
```nushell
{
    run: {|| ...}
    duplex: true          # Enable send/recv
    return_options: {suffix: "recv", ttl: "forever"}
}
```

**Action:**
```nushell
{
    run: {|frame| ...}
    return_options: {suffix: "response", ttl: "last:100"}
}
```

## xs.nu Module (User-Facing)

**File**: `xs.nu`

The `xs.nu` module wraps the `xs` CLI binary for use in regular Nushell sessions (outside the server):

```nushell
# Install the module
xs nu --install

# Then in any Nushell session:
use xs.nu *

# Now you have:
.cat --follow --topic "events.*"
.append "messages" --meta {text: "hello"}
.last "messages" 5
.cas "sha256-..."
```

### Key Exports

| Command | Wraps |
|---------|-------|
| `.cat` | `xs cat` |
| `.cas` | `xs cas` |
| `.cas-post` | `xs cas-post` |
| `.get` | `xs get` |
| `.last` | `xs last` |
| `.append` | `xs append` |
| `.remove` / `.rm` | `xs remove` |
| `.eval` | `xs eval` |
| `.id` / `.id unpack` / `.id pack` | `xs scru128` |
| `.export` | Custom export logic |
| `.import` | Custom import logic |
| `.tmp-spawn` | Create temp store, run closure, cleanup |
| `h. get` / `h. post` | HTTP request helpers |

### xs-addr

The module uses `$env.XS_ADDR` to determine the store address. Default: `~/.local/share/cross.stream/store`

### .export / .import

```nushell
# Export entire store to a directory
.export ./backup/

# Creates:
# ./backup/frames.jsonl   (all frames as NDJSON)
# ./backup/cas/           (all CAS content files)

# Import from exported directory
.import ./backup/
```

### .tmp-spawn

Creates an ephemeral store for testing:

```nushell
.tmp-spawn {
    .append "test" --meta {x: 1}
    .last "test"
    # store is destroyed when closure exits
}
```

## Utility Functions

**File**: `src/nu/util.rs`

### Type Conversions

```rust
pub fn json_to_value(json: serde_json::Value, span: Span) -> Value
pub fn value_to_json(value: Value) -> serde_json::Value
pub fn frame_to_value(frame: &Frame, span: Span, with_timestamp: bool) -> Value
pub fn frame_to_pipeline(frame: &Frame, with_timestamp: bool) -> PipelineData
pub fn write_pipeline_to_cas(input: PipelineData, store: &Store, span: Span) -> Result<ssri::Integrity>
```

These bridge the gap between Nushell's `Value` type system and Rust's serde ecosystem. The `frame_to_value` function converts a Frame into a Nushell Record with fields: `id`, `topic`, `hash`, `meta`, `ttl`, and optionally `timestamp`.
