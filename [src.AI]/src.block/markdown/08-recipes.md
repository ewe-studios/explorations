---
title: Goose Recipes — YAML Automation Workflows with Sub-Recipes, Parameters, and Extensions
---

# Goose Recipes — YAML Automation Workflows with Sub-Recipes, Parameters, and Extensions

**Recipes are YAML (or JSON) files that define structured automation workflows for goose.** A recipe specifies a prompt or instructions, optional extensions to enable, settings (provider, model, temperature), parameters that users can fill in, sub-recipes for multi-step workflows, and a response JSON schema for structured output. Recipes are discovered from local directories, built-in paths, and the `--recipe` CLI flag. The recipe engine handles parameter interpolation, sub-recipe DAG execution, extension activation, and retry configuration.

Source: `crates/goose/src/recipe/mod.rs` — 775 lines, `recipe/build_recipe/`, `recipe/local_recipes/`, `recipe/validate_recipe/`

## Recipe Format

### Minimal Recipe
```yaml
version: "1.0.0"
title: "Code Review"
description: "Review the current diff for bugs and improvements"
prompt: "Review the git diff and provide actionable feedback"
```

### Complete Recipe
```yaml
version: "1.0.0"
title: "Deploy Preview Environment"
description: "Build, test, and deploy a preview environment"
author:
  contact: "team@example.com"
instructions: |
  You are a deployment specialist. Follow these rules:
  - Always verify the build passes before deploying
  - Run integration tests after deployment
  - Report the preview URL when done
prompt: "Deploy the current branch to a preview environment"
parameters:
  - name: "environment"
    description: "Which environment to deploy to"
    requirement: user_prompt
    type: string
  - name: "branch"
    description: "Branch to deploy"
    requirement: optional
    default: "main"
extensions:
  - name: "developer"
    enabled: true
  - name: "computercontroller"
    enabled: false
settings:
  goose_provider: "anthropic"
  goose_model: "claude-sonnet-4-20250514"
  temperature: 0.1
activities:
  - "Building application..."
  - "Running tests..."
  - "Deploying to preview..."
sub_recipes:
  - name: "build"
    path: "./sub-recipes/build.yaml"
  - name: "test"
    path: "./sub-recipes/test.yaml"
    values:
      suite: "integration"
  - name: "deploy"
    path: "./sub-recipes/deploy.yaml"
retry:
  max_attempts: 3
  on_failure:
    type: "command"
    command: "notify-team.sh"
    timeout_seconds: 30
response:
  json_schema:
    type: object
    properties:
      preview_url: { type: string }
      build_status: { type: string }
      test_results: { type: object }
    required: ["preview_url", "build_status"]
```

## The Recipe Struct — Every Field

```rust
// crates/goose/src/recipe/mod.rs:31-76
pub struct Recipe {
    pub version: String,                               // Semver, default "1.0.0"
    pub title: String,                                  // Short title for UIs
    pub description: String,                            // Longer description
    pub instructions: Option<String>,                   // System-level instructions for the model
    pub prompt: Option<String>,                         // Initial prompt (at least one of instructions/prompt required)
    pub extensions: Option<Vec<ExtensionConfig>>,       // Extensions to enable for this recipe
    pub settings: Option<Settings>,                     // Provider, model, temperature overrides
    pub activities: Option<Vec<String>>,                // UI activity pills shown during loading
    pub author: Option<Author>,                         // contact + metadata
    pub parameters: Option<Vec<RecipeParameter>>,       // User-fillable parameters
    pub response: Option<Response>,                     // JSON schema for structured output
    pub sub_recipes: Option<Vec<SubRecipe>>,            // Sub-recipe references
    pub retry: Option<RetryConfig>,                     // Retry + on_failure actions
}
```

### SubRecipe — DAG Execution

```rust
pub struct SubRecipe {
    pub name: String,                                     // Identifier
    pub path: String,                                     // Path to sub-recipe YAML file
    pub values: Option<HashMap<String, String>>,          // Parameter values (all values as strings)
    pub sequential_when_repeated: bool,                   // If true: run sequentially even if repeated
    pub description: Option<String>,                      // Human-readable description
}
```

**Aha:** `sequential_when_repeated` solves a subtle concurrency problem. If a sub-recipe is referenced multiple times in a workflow with different parameters, should the invocations run in parallel (faster) or sequentially (safer)? This flag lets the recipe author decide per sub-recipe. Deployment recipes typically use `sequential_when_repeated: true` to avoid race conditions; data processing recipes use `false` for parallelism.

### RecipeParameter

```rust
pub enum RecipeParameterRequirement { Required, Optional, UserPrompt }

pub struct RecipeParameter {
    pub name: String,
    pub description: String,
    pub requirement: RecipeParameterRequirement,
    pub default: Option<String>,       // Only for Optional
    pub type: Option<String>,          // "string", "number", "boolean"
}
```

`UserPrompt` means the parameter value is solicited from the user interactively at recipe start — goose CLI renders a prompt and waits for input before executing.

## Recipe Discovery

Recipes are discovered from multiple sources:

```rust
// crates/goose/src/recipe/local_recipes/
// 1. Built-in recipes: embedded in the binary via include_dir!
// 2. ~/.config/goose/recipes/ — user-level recipes
// 3. <working_dir>/.goose/recipes/ — project-level recipes  
// 4. --recipe <path> CLI flag — explicit path
// 5. Remote recipes: GitHub URLs, gists
```

## Recipe Build Pipeline

```rust
// crates/goose/src/recipe/build_recipe/
// 1. Parse YAML/JSON → Recipe struct
// 2. Validate (required fields, parameter types, sub-recipe DAG cycles)
// 3. Resolve sub-recipes (recursive, detect circular references)
// 4. Interpolate parameters — replace {{parameter_name}} in prompts/instructions
// 5. Activate extensions listed in extensions: section
// 6. Override settings (provider, model, temperature)
// 7. If response.json_schema is set → configure StructuredOutput on the provider
```

### Template Interpolation

Recipe prompts and instructions support Jinja2-style template variables referencing parameters:

```yaml
prompt: "Deploy branch {{branch}} to {{environment}} and verify the deployment"
```

Parameters are interpolated using `minijinja` before the prompt is sent to the LLM.

## Response Schema — Structured Output

```yaml
response:
  json_schema:
    type: object
    properties:
      preview_url: { type: string, description: "The deployed preview URL" }
      build_status: { type: string, enum: ["success", "failure"] }
      test_summary: { type: string }
    required: ["preview_url", "build_status"]
```

When `response.json_schema` is set, goose configures the LLM provider for structured output (OpenAI's `response_format: json_schema`, Anthropic's tool use with structured output). The agent's `FinalOutputTool` validates the response against the schema before marking the task complete.

## Retry Configuration

```rust
pub struct RetryConfig {
    pub max_attempts: u32,          // Maximum retry attempts (default: 3)
    pub success_checks: Vec<SuccessCheck>,
    pub on_failure: Option<FailureAction>,
}

pub struct SuccessCheck {
    pub type: String,               // "command", "file_exists", "http_status"
    pub command: Option<String>,
    pub expected_exit_code: Option<i32>,
}

pub struct FailureAction {
    pub type: String,               // "command", "prompt"
    pub command: Option<String>,    // Shell command to run on failure
    pub timeout_seconds: Option<u32>,
}
```

### Success Checks

After the recipe's main prompt completes, success checks run to validate the result:
- `command`: Run a shell command, check exit code
- `file_exists`: Verify a file was created
- `http_status`: Check an HTTP endpoint returns 2xx

If any check fails, the recipe is retried (up to `max_attempts`).

### On-Failure Actions

When a recipe exhausts retries, the `on_failure` action runs:
- `command`: Execute a notification script (e.g., send Slack message)
- `prompt`: Feed the failure context to the LLM for a recovery attempt

## Cross-References

- [Architecture](01-architecture.md) — Where recipe retry logic integrates with the reply loop
- [Extensions](02-extensions.md) — Recipe extension configuration format
- [CLI Deep Dive](12-cli.md) — `goose run recipe.yaml` and `goose session --recipe`

## Next

→ [Autopilot & Retry](09-autopilot-retry.md) — Model auto-selection with trigger rules and retry management.
