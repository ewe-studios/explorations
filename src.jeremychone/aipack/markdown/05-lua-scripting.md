# Aipack -- Lua Scripting

Lua is the scripting layer for all agent logic. The Rust host exposes 30+ modules under the `aip.*` namespace, allowing agents to perform file I/O, HTTP requests, code parsing, document processing, and more from within `.aip` files.

Source: `aipack/src/script/lua_engine.rs` — Lua VM wrapper
Source: `aipack/src/script/aipack_custom.rs` — custom return values
Source: `aipack/src/script/aip_modules/` — 30+ exposed modules
Source: `aipack/src/script/lua_helpers/` — value extraction helpers

## Lua Engine

```rust
// lua_engine.rs
struct LuaEngine {
    lua: mlua::Lua,
}

impl LuaEngine {
    fn init_aip(&self) {
        // Register all aip.* modules as globals
        // Under both "aip" and "utils" namespaces (for backwards compat)
        self.lua.globals().set("aip", self.create_aip_table())?;
        self.lua.globals().set("utils", self.create_aip_table())?;
    }

    fn init_null(&self) {
        // Set null-related helpers in global scope
        // null, Null, NULL, is_null(), nil_if_null(), value_or()
    }

    fn init_print(&self) {
        // Override Lua's print() to log to DB and publish to hub
        let print_fn = self.lua.create_function(|_, msg: String| {
            Hub::publish_sync(LuaPrint(msg.into()));
            // Also log to DB via RtLog
            Ok(())
        })?;
        self.lua.globals().set("print", print_fn)?;
    }

    fn new_with_ctx(&self, ctx: RuntimeCtx) -> LuaState {
        // Create engine with CTX global containing run/task/stage UIDs
        self.lua.globals().set("CTX", ctx.to_lua_table(&self.lua))?;
    }

    async fn eval_async(&self, script: &str, scope: LuaScope) -> Result<LuaValue, Error> {
        // Evaluate script with custom scope and additional Lua paths for require()
    }
}
```

### Context Global

```lua
-- Available as CTX in every script
CTX = {
    run_uid = "abc123",
    run_num = 42,
    parent_run_uid = nil,
    task_uid = "def456",
    task_num = 1,
    stage = "ai",  -- "before_all", "data", "ai", "output", "after_all"
    flow_redo_run_count = 0,
}
```

## AipackCustom Responses

Lua scripts can return special control values using the `_aipack_` key:

```lua
-- Skip the run
return { _aipack_ = { kind = "Skip", data = { reason = "No changes needed" } } }

-- Redo the run
return { _aipack_ = { kind = "Redo" } }

-- Modify inputs and options
return { _aipack_ = {
    kind = "BeforeAllResponse",
    data = {
        inputs = { { label = "file1", content = "..." } },
        before_all = "Custom before-all data",
        options = { model = "anthropic/claude-sonnet-4" }
    }
}}

-- Modify task data
return { _aipack_ = {
    kind = "DataResponse",
    data = {
        input = "Modified input",
        data = "Computed data",
        attachments = { "/path/to/file" },
        options = { temperature = 0.5 }
    }
}}
```

The Rust side parses the return value:

```rust
fn extract_aipack_custom(value: &LuaValue) -> Result<Option<AipackCustom>, Error> {
    if let LuaValue::Table(table) = value {
        if let Ok(custom) = table.get::<_, AipackCustom>("_aipack_") {
            return Ok(Some(custom));
        }
    }
    Ok(None)
}
```

## aip.* Modules

### aip.flow — Control Flow

```lua
-- Skip the current run
aip.flow.skip("Reason for skipping")

-- Redo the run
aip.flow.redo()

-- Return before_all response
aip.flow.before_all_response(inputs, before_all_data, options)

-- Return data response
aip.flow.data_response(input_data, computed_data, attachments)
```

### aip.file — File Operations

```lua
-- Read/write files
local content = aip.file.read("path/to/file.txt")
aip.file.write("path/to/file.txt", "new content")

-- Hash files
local hash = aip.file.hash("path/to/file.txt")  -- blake3

-- Parse structured files
local json_data = aip.file.json("config.json")
local toml_data = aip.file.toml("Cargo.toml")
local yaml_data = aip.file.yaml("docker-compose.yml")
local csv_data = aip.file.csv("data.csv")

-- Parse documents
local md_blocks = aip.file.md("README.md")
local html_content = aip.file.html("index.html")
local docx_content = aip.file.docx("document.docx")
local pdf_text = aip.file.pdf("report.pdf")

-- Get file spans (partial content)
local span = aip.file.spans("file.txt", { start_line = 10, end_line = 20 })
```

### aip.code — Code Parsing

```lua
-- Parse code files
local ast = aip.code.parse("main.rs")

-- Get code blocks from markdown
local blocks = aip.code.md_blocks("file.md")

-- Extract language-specific code
local rust_functions = aip.code.rust_functions("src/lib.rs")
```

### aip.udiffx — Unified Diff

```lua
-- Apply unified diffs
aip.udiffx.apply(old_content, diff_text)

-- Parse diffs
local changes = aip.udiffx.parse(diff_text)
```

### aip.web — HTTP Requests

```lua
local response = aip.web.get("https://api.example.com/data", {
    headers = { Authorization = "Bearer token" }
})
local response = aip.web.post("https://api.example.com/data", {
    body = { key = "value" }
})
```

### aip.cmd — Shell Commands

```lua
local output = aip.cmd.run("git status")
local exit_code = aip.cmd.run_sync("cargo build")
```

### aip.git — Git Operations

```lua
local status = aip.git.status()
local diff = aip.git.diff()
local log = aip.git.log({ max_count = 10 })
```

### aip.hbs — Handlebars Templates

```lua
local rendered = aip.hbs.render("Hello {{name}}!", { name = "World" })
```

### aip.text — Text Processing

```lua
local lines = aip.text.split(text, "\n")
local trimmed = aip.text.trim(text)
local formatted = aip.text.format(text, { max_width = 80 })
```

### aip.md — Markdown Processing

```lua
-- Iterate over markdown blocks
for block in aip.md.blocks(content) do
    print(block.kind, block.content)
end

-- Extract sections
local sections = aip.md.sections(content)

-- Extract references
local refs = aip.md.refs(content)

-- Extract headings
local headings = aip.md.headings(content)
```

### aip.hash / aip.uuid

```lua
local blake3_hash = aip.hash.blake3(data)
local sha256_hash = aip.hash.sha256(data)

local uuid = aip.uuid.v4()  -- Random UUID
local uuid7 = aip.uuid.v7() -- Time-ordered UUID
```

### aip.time — Time Utilities

```lua
local now = aip.time.now()  -- Current timestamp
local formatted = aip.time.format(now, "%Y-%m-%d %H:%M:%S")
local epoch_us = aip.time.now_micro()  -- Microsecond precision
```

### aip.path — Path Manipulation

```lua
local resolved = aip.path.resolve("~/project/file.txt")
local joined = aip.path.join("dir", "subdir", "file.txt")
local ext = aip.path.extension("file.rs")  -- "rs"
```

### aip.semver — Semantic Versioning

```lua
local cmp = aip.semver.compare("1.2.3", "1.3.0")  -- -1
local satisfies = aip.semver.satisfies("1.2.3", ">=1.0.0")  -- true
```

### Other Modules

| Module | Purpose |
|--------|---------|
| `aip.run` | Run sub-agents |
| `aip.task` | Task operations |
| `aip.agent` | Agent operations |
| `aip.shape` | Data shaping (keys, records) |
| `aip.json/toml/yaml/csv` | Format-specific operations |
| `aip.html/pdf/docx` | Document processing |
| `aip.rust` | Rust-related operations |
| `aip.lua` | Lua introspection |
| `aip.tag` | Tag operations |
| `aip.zip` | ZIP file operations |
| `aip.editor` | Editor integration |

## Null Handling

Lua doesn't have a `null` value — it uses `nil`, which can't be stored in tables. Aipack provides a special `Null` value:

```lua
-- In Lua scripts
local value = aip.file.json("config.json")
if is_null(value.api_key) then
    value.api_key = "default"
end

-- nil_if_null: converts Lua nil to Null marker
local safe = nil_if_null(maybe_nil)

-- value_or: default value
local result = value_or(maybe_nil, "default")
```

## serde_to_lua_value

Special JSON-to-Lua conversion handles the null problem:

```rust
fn serde_to_lua_value(value: &serde_json::Value, lua: &Lua) -> LuaValue {
    match value {
        serde_json::Value::Null => LuaValue::Nil,  // But wrapped for table storage
        serde_json::Value::Bool(b) => LuaValue::Boolean(*b),
        serde_json::Value::Number(n) => LuaValue::Number(n.as_f64().unwrap()),
        serde_json::Value::String(s) => LuaValue::String(s.into()),
        serde_json::Value::Array(arr) => { ... }
        serde_json::Value::Object(obj) => { ... }
    }
}
```

See [Run System](04-run-system.md) for how scripts are executed in the run flow.
See [Template System](13-template-system.md) for Handlebars integration.
