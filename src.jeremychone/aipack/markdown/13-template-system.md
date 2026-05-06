# Aipack -- Template System

Aipack uses Handlebars for server-side template rendering within Lua scripts via the `aip.hbs` module. The template system is lightweight, with escaping disabled to allow raw output.

Source: `aipack/src/support/hbs.rs` — Handlebars wrapper
Source: `aipack/src/script/aip_modules/aip_hbs.rs` — Lua module binding

## Handlebars Integration

```rust
// hbs.rs
static HANDLEBARS: LazyLock<Arc<Handlebars>> = LazyLock::new(|| {
    let mut handlebars = Handlebars::new();
    // Disable escaping globally — aipack expects raw output
    handlebars.register_escape_fn(|s| s.to_string());
    Arc::new(handlebars)
});

pub fn hbs_render<T>(hbs_tmpl: &str, data_root: &T) -> Result<String>
where
    T: Serialize,
{
    let handlebars = &*HANDLEBARS;
    let res = handlebars.render_template(hbs_tmpl, data_root)?;
    Ok(res)
}
```

The `LazyLock` ensures a single Handlebars instance is shared across all renders. Escaping is disabled because aipack templates typically generate code or markdown where literal characters (`, `<`, `>`) should not be HTML-escaped.

## Lua Module: aip.hbs

```lua
-- Lua binding
local rendered = aip.hbs.render("Hello {{name}}!", { name = "World" })
-- → "Hello World!"

-- With arrays
local tmpl = [[
{{#each items}}
- {{this.name}}: {{this.value}}
{{/each}}
]]
local result = aip.hbs.render(tmpl, {
    items = {
        { name = "speed", value = "fast" },
        { name = "quality", value = "high" }
    }
})
```

## Integration with Lua Engine

The typical flow is:

1. Lua script processes data (file I/O, API calls, etc.)
2. Lua serializes the result to a table
3. Rust converts the Lua table to `serde_json::Value`
4. Handlebars renders the template with the JSON data
5. Rendered text is returned to Lua

```rust
// hbs.rs — test showing the full pipeline
async fn test_hbs_with_lua_ok() -> Result<()> {
    // 1. Run Lua script that loads files
    let script = r#"
        local file1 = aip.file.load("file-01.txt")
        local file2 = aip.file.load("agent-script/agent-before-all.aip")
        return {file1, file2}
    "#;
    let lua_engine = runtime.new_lua_engine_without_ctx_test_only()?;
    let data = lua_engine.eval(script, None).await?;

    // 2. Convert Lua return value to JSON
    let data = serde_json::to_value(data)?;
    let value = json!({ "data": data });

    // 3. Render Handlebars template with JSON data
    let tmpl = r#"
The files are:
{{#each data}}
- {{this.path}}
{{/each}}
    "#;
    let res = hbs_render(tmpl, &value)?;

    assert_contains(&res, "- file-01.txt");
    assert_contains(&res, "- agent-script/agent-before-all.aip");
}
```

## Supported Handlebars Features

The Handlebars crate supports:

| Feature | Syntax |
|---------|--------|
| Variable interpolation | `{{name}}` |
| Nested paths | `{{user.address.city}}` |
| Conditionals | `{{#if condition}}...{{/if}}` |
| Loops | `{{#each items}}...{{/each}}` |
| With blocks | `{{#with user}}...{{/with}}` |
| Unless | `{{#unless condition}}...{{/unless}}` |
| Partials | `{{> partial_name}}` |
| Helpers | Custom helpers (none registered by default) |

No custom helpers are registered in aipack's Handlebars instance — it uses the built-in helpers only. This keeps the template system predictable and avoids namespace collisions with Lua function names.

See [Lua Scripting](05-lua-scripting.md) for `aip.hbs` usage in scripts.
