# Aipack -- Agent System

Agents in Aipack are defined as `.aip` markdown files containing structured sections: options (TOML), scripts (Lua), and prompt parts (text). The `AgentDoc` parser uses a custom line-by-line state machine to extract these sections.

Source: `aipack/src/agent/agent_doc.rs` — .aip file parser
Source: `aipack/src/agent/agent_common.rs` — Agent struct
Source: `aipack/src/agent/agent_options.rs` — configuration options
Source: `aipack/src/agent/agent_ref.rs` — agent references
Source: `aipack/src/agent/agent_locator.rs` — agent resolution
Source: `aipack/src/agent/prompt_part.rs` — prompt part parsing

## Agent Structure

```rust
// agent_common.rs
pub struct Agent {
    inner: Arc<AgentInner>,
    model: ModelName,           // raw model from options
    model_resolved: ModelName,  // after alias resolution
    agent_options_ov: Option<Arc<AgentOptions>>,
    genai_chat_options: Arc<ChatOptions>,
}

struct AgentInner {
    name: String,
    agent_ref: AgentRef,
    file_name: String,
    file_path: SPath,
    agent_options: Arc<AgentOptions>,
    model_name: Option<ModelName>,
    before_all_script: Option<String>,
    prompt_parts: Vec<PromptPart>,
    data_script: Option<String>,
    output_script: Option<String>,
    after_all_script: Option<String>,
}
```

The `Agent` is clonable via `Arc`, allowing it to be shared across concurrent tasks without copying.

## .aip File Parser (AgentDoc)

The parser uses a state machine with `CaptureMode` enum:

```rust
// agent_doc.rs
enum CaptureMode {
    None,
    BeforeAllSection, BeforeAllCodeBlock,
    OptionsSection, OptionsTomlBlock,
    DataSection, DataCodeBlock,
    OutputSection, OutputCodeBlock,
    AfterAllSection, AfterAllCodeBlock,
    PromptPart,
}

enum InBlockState {
    OutsideBlock,
    InsideBlock,
}
```

### Parsing Algorithm

```
for each line in .aip file:
    if in code block:
        if line is closing fence (```):
            InBlockState → OutsideBlock
            Process captured content based on CaptureMode
        else:
            Append line to capture buffer
    else:
        if line matches section header (# Before All, # Options, etc.):
            CaptureMode → appropriate section
            if section has code block:
                InBlockState → InsideBlock
        elif line is text content and CaptureMode == PromptPart:
            Append to prompt_part.content
```

### Section Detection

| Header | CaptureMode | Content |
|--------|-------------|---------|
| `# Before All` | BeforeAllSection → BeforeAllCodeBlock | Lua script (```lua block) |
| `# Options` | OptionsSection → OptionsTomlBlock | TOML config (```toml block) |
| `# Data` | DataSection → DataCodeBlock | Lua script (```lua block) |
| `# Output` | OutputSection → OutputCodeBlock | Lua script (```lua block) |
| `# After All` | AfterAllSection → AfterAllCodeBlock | Lua script (```lua block) |
| `# User` / `# System` / `# Assistant` | PromptPart | Text content |

### Options Parsing

The `# Options` section uses TOML:

```toml
# Options
```toml
model = "anthropic/claude-sonnet-4-20250514"
temperature = 0.7
top_p = 0.9
input_concurrency = 4
allow_run_on_task_fail = false
```
```

Parsed into `AgentOptions`:

```rust
struct AgentOptions {
    model: Option<String>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    input_concurrency: Option<usize>,
    allow_run_on_task_fail: Option<bool>,
    model_aliases: Option<ModelAliases>,
}
```

### Prompt Part Parsing

```rust
// prompt_part.rs
pub struct PromptPart {
    pub kind: PartKind,
    pub content: String,
    pub options_str: Option<String>,
}

pub enum PartKind {
    Instruction,  // # User, # Inst, # Instruction
    System,       // # System
    Assistant,    // # Assistant, # Model, # Mind Trick, # Jedi Trick
}
```

Options can be specified inline with backticks:

```markdown
# User `cache = true`
Please analyze the following code...
```

The `PartOptions { cache: true }` tells the LLM client to use prompt caching for this part.

## Model Alias Resolution

```rust
// agent_options.rs
fn resolve_model(&self, model_name: &str) -> ModelName {
    // Direct alias lookup first
    if let Some(resolved) = self.model_aliases.get(model_name) {
        return resolved.clone();
    }

    // Strip reasoning suffixes and resolve base
    let suffixes = ["-zero", "-minimal", "-low", "-medium", "-high", "-xhigh", "-max"];
    for suffix in suffixes {
        if model_name.ends_with(suffix) {
            let base = &model_name[..model_name.len() - suffix.len()];
            if let Some(resolved) = self.model_aliases.get(base) {
                // Re-attach suffix to resolved model
                return format!("{resolved}{suffix}");
            }
        }
    }

    model_name.to_string()
}
```

This allows agents to use short model names like `sonnet` that resolve to full provider-prefixed names like `anthropic/claude-sonnet-4-20250514`. Reasoning suffixes (e.g., `sonnet-max`) are preserved after resolution.

## Agent Reference Resolution

```rust
// agent_ref.rs
enum PartialAgentRef {
    LocalPath(String),      // "fix-bug" or "path/to/agent"
    PackRef(PackRef),       // "ns@pack/agent"
}

enum AgentRef {
    LocalPath(String),
    PackRef(LocalPackRef),  // resolved with concrete pack_dir
}
```

Parsing uses `@` delimiter detection:

```rust
fn parse_agent_name(name: &str) -> PartialAgentRef {
    if name.contains('@') {
        let parts: Vec<&str> = name.splitn(2, '@').collect();
        let namespace = parts[0];
        let rest = parts[1];
        // Parse namespace@name into PackRef
        PartialAgentRef::PackRef(PackRef::new(namespace, rest))
    } else {
        PartialAgentRef::LocalPath(name.to_string())
    }
}
```

## Agent Locator

```rust
// agent_locator.rs
fn find_agent(name: &str, runtime: &Runtime, base_dir: &DirContext) -> Result<Agent, Error> {
    let partial_ref = parse_agent_name(name);

    // Load and merge config agent options from all config TOML files
    let merged_options = load_and_merge_configs(base_dir);

    match partial_ref {
        PartialAgentRef::LocalPath(path) => {
            // Try multiple .aip file patterns:
            // 1. Direct .aip file: path
            // 2. Append .aip: path.aip
            // 3. Append /main.aip: path/main.aip
            for pattern in [path.clone(), format!("{path}.aip"), format!("{path}/main.aip")] {
                if file_exists(&pattern) {
                    return parse_agent_file(&pattern, merged_options);
                }
            }
        }
        PartialAgentRef::PackRef(pack_ref) => {
            // Find pack directory via find_to_run_pack_dir()
            let pack_dir = find_to_run_pack_dir(&pack_ref)?;
            // Resolve sub-path and try .aip patterns
            // ...
        }
    }

    Err(Error::CommandAgentNotFound(name.to_string()))
}
```

The locator tries multiple file patterns in order, giving flexibility in how agents are named and organized.

See [Directory Context](07-directory-context.md) for pack resolution.
See [Execution Engine](03-execution-engine.md) for how agents are executed.
