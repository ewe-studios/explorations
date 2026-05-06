# Aipack -- Error System

Aipack uses a single, comprehensive `Error` enum with `derive_more` for automatic conversions from external error types. Errors are displayed to users with contextual messages and stored in the SQLite database for post-run analysis.

Source: `aipack/src/error.rs` — main error enum

## Error Enum

```rust
#[derive(Debug, From, Display)]
#[display("{self:?}")]
pub enum Error {
    // -- CLI commands
    #[display("Command Agent not found at: {_0}")]
    CommandAgentNotFound(String),

    #[display("User cancelled command.")]
    UserInterrupted,

    // -- Agent
    #[display("Model is missing for agent path: {agent_path}")]
    ModelMissing { agent_path: String },

    // -- Config
    #[display("Config invalid (config path: {path})\n  reason: {reason}")]
    Config { path: String, reason: String },

    // -- Pack
    #[display("Pack Identity '{origin_path}' is not valid.\nCause: {cause}")]
    InvalidPackIdentity { origin_path: String, cause: String },

    // -- Channels
    #[display("Send on channel '{name}' fail.\nCause: {cause}")]
    ChannelTx { name: &'static str, cause: String },
    #[display("Recieve on channel '{name}' fail.\nCause: {cause}")]
    ChannelRx { name: &'static str, cause: String },

    // -- Pack/Installer
    #[display("pack.toml file is missing at '{_0}'")]
    AipackTomlMissing(String),
    #[display("Fail to install pack: {aipack_ref}\nCause: {cause}")]
    FailToInstall { aipack_ref: String, cause: String },
    #[display("Cannot install version {new_version} because installed version {installed_version} is newer")]
    InstallFailInstalledVersionAbove { installed_version: String, new_version: String },
    #[display("Invalid prerelease format in version {version}")]
    InvalidPrereleaseFormat { version: String },

    // -- Run
    #[display("Before All Lua block did not return a valid structure.\nCause: {cause}")]
    BeforeAllFailWrongReturn { cause: String },
    #[display("Data Lua block did not return a valid structure.\nCause: {cause}")]
    DataFailWrongReturn { cause: String },

    // -- GenAI
    #[display("Environment API KEY missing: {env_name}")]
    GenAIEnvKeyMissing { model_iden: ModelIden, env_name: String },
    #[display("Fail to make AI Request.\nCause: {_0}")]
    GenAI(genai::Error),

    // -- External library errors (auto-from)
    #[from] Udiffx(udiffx::Error),
    #[from] FlumeRecv(flume::RecvError),
    #[from] Serde(serde_json::Error),
    #[from] Toml(toml::de::Error),
    #[from] Yaml(serde_yaml_ng::Error),
    #[from] Handlebars(handlebars::RenderError),
    #[from] SimpleFs(simple_fs::Error),
    #[from] Keyring(keyring::Error),
    #[from] Clap(clap::error::Error),
    #[from] Reqwest(reqwest::Error),
    #[from] Io(std::io::Error),

    // -- Custom errors
    #[display("{_0}")]
    #[from]
    Custom(String),
    #[display("Error: {_0}\n\tCause: {_1}")]
    CustomAndCause(String, String),
}
```

## Automatic Conversions

The `#[derive(From)]` macro implements `From<ExternalError> for Error` for all tagged variants. This means any function returning `Result<T>` can use `?` to propagate errors:

```rust
fn parse_agent_file(path: &str) -> Result<Agent> {
    let content = fs::read_to_string(path)?;  // io::Error → Error::Io
    let toml_val: toml::Value = toml::from_str(&content)?;  // toml::de::Error → Error::Toml
    // ...
}
```

## Custom Error Constructors

```rust
impl Error {
    pub fn custom(val: impl std::fmt::Display) -> Self {
        Self::Custom(val.to_string())
    }

    pub fn custom_and_cause(context: impl Into<String>, cause: impl std::fmt::Display) -> Self {
        Self::CustomAndCause(context.into(), cause.to_string())
    }

    /// Shortcut: "cc" = custom_and_cause
    pub fn cc(context: impl Into<String>, cause: impl std::fmt::Display) -> Self {
        Self::CustomAndCause(context.into(), cause.to_string())
    }
}

// Also: From<&str> for Error
impl From<&str> for Error {
    fn from(val: &str) -> Self {
        Self::Custom(val.to_string())
    }
}
```

The `cc` shorthand is used extensively for context-rich error messages:

```rust
return Err(Error::cc("Failed to resolve path", orig_path.to_string()));
```

## GenAI Error Conversion

The `genai::Error` conversion has special handling to extract API key errors:

```rust
impl From<genai::Error> for Error {
    fn from(genai_error: genai::Error) -> Self {
        match genai_error {
            genai::Error::Resolver { model_iden, resolver_error } => {
                if let genai::resolver::Error::ApiKeyEnvNotFound { env_name } = resolver_error {
                    // Convert to user-friendly message
                    Error::GenAIEnvKeyMissing { model_iden, env_name }
                } else {
                    Error::GenAI(genai::Error::Resolver { model_iden, resolver_error })
                }
            }
            other => Error::GenAI(other),
        }
    }
}
```

Without this special case, the raw genai error would show something like `resolver error: ApiKeyEnvNotFound("ANTHROPIC_API_KEY")`. Instead, the user sees `Environment API KEY missing: ANTHROPIC_API_KEY`.

## Error Display

All errors implement `std::fmt::Display` via `derive_more::Display`. The default format is the debug representation (`#[display("{self:?}")]`), which produces structured output:

```
Command Agent not found at: fix-bug
Config invalid (config path: /path/config.toml)
  reason: missing 'model' field
Fail to install pack: jc@coder
Cause: Failed to unzip pack: ...
```

## Error Storage

Errors during runs are stored in the SQLite `err` table:

```rust
// From 09-database-schema.md
RunBmc::set_end_error(mm, run_id, stage, error) -> {
    let err_c = ErrForCreate {
        stage: Some(stage),
        run_id: Some(run_id),
        content: Some(error.to_string()),
        ..
    };
    ErrBmc::create(mm, err_c)?;
    // Update run with end_state = Err
}
```

The TUI reads error records to display failure reasons in the run/task views.

See [Database Schema](09-database-schema.md) for the err entity.
See [Run System](04-run-system.md) for error handling in run orchestration.
