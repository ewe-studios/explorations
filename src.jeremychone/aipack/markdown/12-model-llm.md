# Aipack -- Model & LLM

Aipack routes LLM requests across multiple providers (Anthropic, OpenAI, Gemini, Groq, DeepSeek, Fireworks, Together, xAI, ZAI) with automatic API key resolution, model alias resolution, and per-call pricing calculation.

Source: `aipack/src/run/genai_client.rs` — client initialization
Source: `aipack/src/run/pricing/pricer.rs` — pricing calculator
Source: `aipack/src/run/pricing/pricing_types.rs` — pricing type definitions
Source: `aipack/src/run/pricing/mod.rs` — pricing module
Source: `aipack/src/run/pricing/data/` — per-provider pricing data
Source: `aipack/src/agent/agent_options.rs` — model alias resolution

## GenAI Client Initialization

```rust
// genai_client.rs
pub fn new_genai_client() -> Result<Client> {
    let options = ChatOptions::default().with_normalize_reasoning_content(true);

    let client = Client::builder()
        .with_chat_options(options)
        .with_auth_resolver_fn(|model: ModelIden| {
            // 1. Get the expected env var name from the provider adapter
            let Some(key_name) = model.adapter_kind.default_key_env_name() else {
                return Ok(None);  // e.g., ollama has no API key
            };

            // 2. Try environment variable first
            if let Some(key) = std::env::var(key_name).ok() {
                return Ok(Some(AuthData::from_single(key)));
            }

            // 3. Fall back to error (keyring disabled for now)
            Err(genai::resolver::Error::ApiKeyEnvNotFound { env_name: key_name })
        })
        .build();

    Ok(client)
}
```

The auth resolver is called per-request with the target model. It extracts the expected environment variable name from the provider adapter (e.g., `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) and looks it up. This means a single aipack process can use different providers by specifying different models.

### Supported Providers

| Provider | Adapter | Env Variable |
|----------|---------|-------------|
| Anthropic | `anthropic` | `ANTHROPIC_API_KEY` |
| OpenAI | `openai` | `OPENAI_API_KEY` |
| Gemini | `gemini` | `GEMINI_API_KEY` |
| Groq | `groq` | `GROQ_API_KEY` |
| DeepSeek | `deepseek` | `DEEPSEEK_API_KEY` |
| Fireworks | `fireworks` | `FIREWORKS_API_KEY` |
| Together | `together` | `TOGETHER_API_KEY` |
| xAI | `xai` | `XAI_API_KEY` |
| ZAI | `zai` | `ZAI_API_KEY` |

## Model Alias Resolution

```rust
// agent_options.rs
fn resolve_model(&self, model_name: &str) -> ModelName {
    // 1. Direct alias lookup
    if let Some(resolved) = self.model_aliases.get(model_name) {
        return resolved.clone();
    }

    // 2. Strip reasoning suffixes, resolve base, re-attach suffix
    let suffixes = ["-zero", "-minimal", "-low", "-medium", "-high", "-xhigh", "-max"];
    for suffix in suffixes {
        if model_name.ends_with(suffix) {
            let base = &model_name[..model_name.len() - suffix.len()];
            if let Some(resolved) = self.model_aliases.get(base) {
                return format!("{resolved}{suffix}");
            }
        }
    }

    // 3. Return as-is (already a full model name)
    model_name.to_string()
}
```

This allows agents to use short names:

```toml
# Options
```toml
model = "sonnet"           # → "anthropic/claude-sonnet-4-20250514"
model = "sonnet-max"       # → "anthropic/claude-sonnet-4-20250514-max"
model = "o3"               # → "openai/o3"
model = "anthropic/claude-sonnet-4"  # → as-is (already full)
```

The alias resolution table is defined in the agent's `# Options` section or merged from config files.

## Pricing Calculator

```rust
// pricing/pricer.rs
pub fn price_it(provider_type: &str, model_name: &str, usage: &Usage) -> Option<AiPrice> {
    // 1. Normalize model name (strip "::" namespace prefix)
    let model_name = normalize_model_name(model_name);

    // 2. Normalize provider type (e.g., "openai_resp" → "openai")
    let provider_type = normalize_provider_type(provider_type);

    // 3. Find provider and model (longest-prefix match)
    let model = find_model_entry(provider_type, model_name)?;

    // 4. Split prompt tokens: normal, cached, cache-creation
    let prompt_tokens = usage.prompt_tokens.unwrap_or(0) as f64;
    let (normal, cached, cache_creation) = match &usage.prompt_tokens_details {
        Some(details) => {
            let c = details.cached_tokens.unwrap_or(0) as f64;
            let cc = details.cache_creation_tokens.unwrap_or(0) as f64;
            (prompt_tokens - c - cc, c, cc)
        }
        None => (prompt_tokens, 0.0, 0.0),
    };

    // 5. Split completion tokens: normal, reasoning
    let completion = usage.completion_tokens.unwrap_or(0) as f64;
    let (completion_normal, completion_reasoning) = match usage.completion_tokens_details {
        Some(details) if details.reasoning_tokens.is_some() => {
            let r = details.reasoning_tokens.unwrap() as f64;
            (completion - r, r)
        }
        _ => (completion, 0.0),
    };

    // 6. Calculate cost components
    let cost_normal = normal * model.input_normal / 1_000_000;
    let cost_cached = cached * model.input_cached.unwrap_or(model.input_normal) / 1_000_000;
    let cost_cache_creation = cache_creation * (1.25 * model.input_normal) / 1_000_000;
    let cost_completion = completion_normal * model.output_normal / 1_000_000;
    let cost_reasoning = completion_reasoning * model.output_reasoning.unwrap_or(model.output_normal) / 1_000_000;

    // 7. Sum and round to 4 decimal places
    let cost = (cost_normal + cost_cached + cost_cache_creation + cost_completion + cost_reasoning) * 10_000;
    let cost = cost.round() / 10_000;

    Some(AiPrice {
        cost,
        cost_cache_write: (cache_creation > 0.0).then(|| ...),
        cost_cache_saving: (cached > 0.0).then(|| ...),
    })
}
```

### Longest-Prefix Model Matching

```rust
fn find_model_entry(provider_type: &str, model_name: &str) -> Option<&ModelPricing> {
    let provider = PROVIDERS.iter().find(|p| p.name == provider_type)?;

    let mut model: Option<&ModelPricing> = None;
    for m in provider.models.iter() {
        if model_name.starts_with(m.name) {
            // Keep the longest matching prefix
            if model.map(|m| m.name.len()).unwrap_or(0) < m.name.len() {
                model = Some(m);
            }
        }
    }
    model
}
```

This allows pricing entries like `"claude-sonnet-4"` to match `"claude-sonnet-4-20250514"` and `"claude-sonnet-4-20250514-thinking"`. The longest match wins, ensuring specific model variants get their own pricing.

### Pricing Data Files

```
pricing/data/
├── mod.rs           — PROVIDERS array assembly
├── data_anthropic.rs — Anthropic model pricing
├── data_openai.rs    — OpenAI model pricing
├── data_gemini.rs    — Gemini model pricing
├── data_groq.rs      — Groq model pricing
├── data_deepseek.rs  — DeepSeek model pricing
├── data_fireworks.rs — Fireworks model pricing
├── data_together.rs  — Together model pricing
├── data_xai.rs       — xAI model pricing
└── data_zai.rs       — ZAI model pricing
```

Each data file defines `ModelPricing` entries:

```rust
struct ModelPricing {
    name: &'static str,         // Model name prefix for matching
    input_normal: f64,          // $ per million input tokens
    input_cached: Option<f64>,  // $ per million cached input tokens
    output_normal: f64,         // $ per million output tokens
    output_reasoning: Option<f64>, // $ per million reasoning tokens
}
```

### AiPrice Output

```rust
// model/types
pub struct AiPrice {
    pub cost: f64,                      // Total cost in USD
    pub cost_cache_write: Option<f64>,  // Cost of creating cache entries
    pub cost_cache_saving: Option<f64>, // Savings from using cached prompts
}
```

The `AiPrice` is stored on each task record and aggregated into the run's `total_cost`.

See [Run System](04-run-system.md) for pricing integration in the AI processing pipeline.
See [Agent System](02-agent-system.md) for model alias configuration.
