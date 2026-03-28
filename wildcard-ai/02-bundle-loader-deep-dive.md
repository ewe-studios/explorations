---
title: "Bundle Loader Deep Dive"
subtitle: "OpenAPI loading, reference resolution, and operation indexing"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/wildcard-ai/02-bundle-loader-deep-dive.md
prerequisites: 00-zero-to-ai-engineer.md, 01-agents-json-specification-deep-dive.md
---

# Bundle Loader Deep Dive

## Overview

The Bundle Loader is responsible for:
1. Fetching agents.json from URL
2. Loading and parsing OpenAPI specifications
3. Applying overrides to OpenAPI operations
4. Indexing operations by operationId
5. Creating a Bundle for execution

---

## Bundle Structure

```rust
pub struct Bundle {
    /// Parsed agents.json configuration
    pub agents_json: AgentsJson,
    /// Full OpenAPI specification
    pub openapi: Value,
    /// OperationId -> Operation info mapping
    pub operations: HashMap<String, OperationInfo>,
}

pub struct OperationInfo {
    pub path: String,      // e.g., "/v1/products"
    pub method: String,    // e.g., "POST"
    pub operation: Value,  // Full operation object
}
```

---

## Loading Process

### Step 1: Fetch agents.json

```rust
use reqwest::blocking::{Client, Response};

pub fn load_agents_json(url: &str) -> Result<AgentsJson, LoaderError> {
    let client = Client::new();

    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(LoaderError::HttpStatus(
            response.status().as_u16(),
            response.text()?,
        ));
    }

    let agents_json: AgentsJson = response.json()?;

    // Validate version
    if agents_json.agents_json != "0.1.0" {
        return Err(LoaderError::UnsupportedVersion(agents_json.agents_json));
    }

    // Validate required fields
    if agents_json.sources.is_empty() {
        return Err(LoaderError::NoSources);
    }

    if agents_json.flows.is_empty() {
        return Err(LoaderError::NoFlows);
    }

    Ok(agents_json)
}
```

### Step 2: Load OpenAPI Source

```rust
use serde_yaml::Value as YamlValue;

pub fn load_openapi(client: &Client, url: &str) -> Result<Value, LoaderError> {
    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(LoaderError::HttpStatus(
            response.status().as_u16(),
            response.text()?,
        ));
    }

    // Parse based on content type/extension
    let content = response.text()?;

    if url.ends_with(".yaml") || url.ends_with(".yml") {
        let yaml: YamlValue = serde_yaml::from_str(&content)?;
        Ok(serde_json::to_value(yaml)?)
    } else {
        Ok(serde_json::from_str(&content)?)
    }
}
```

### Step 3: Apply Overrides

```rust
pub fn apply_overrides(mut openapi: Value, overrides: &[Override]) -> Value {
    for override_item in overrides {
        // Parse field path into components
        let path_components: Vec<&str> = override_item.field_path.split('.').collect();

        // Navigate to target and set value
        set_value_at_path(&mut openapi, &path_components, override_item.value.clone());
    }
    openapi
}

fn set_value_at_path(value: &mut Value, path: &[&str], new_value: Value) {
    if path.is_empty() {
        *value = new_value;
        return;
    }

    let current = &path[0];
    let remaining = &path[1..];

    if remaining.is_empty() {
        // This is the final component - set the value
        if let Value::Object(obj) = value {
            obj.insert(current.to_string(), new_value);
        }
        return;
    }

    // Navigate deeper
    if let Value::Object(obj) = value {
        if let Some(nested) = obj.get_mut(*current) {
            set_value_at_path(nested, remaining, new_value);
        }
    }
}
```

### Step 4: Index Operations

```rust
pub fn index_operations(openapi: &Value) -> HashMap<String, OperationInfo> {
    let mut operations = HashMap::new();

    let paths = match openapi.get("paths") {
        Some(Value::Object(p)) => p,
        _ => return operations,  // No paths defined
    };

    let http_methods = ["get", "post", "put", "delete", "patch", "options", "head"];

    for (path, path_item) in paths {
        let path_item = match path_item {
            Value::Object(item) => item,
            _ => continue,
        };

        for method in &http_methods {
            if let Some(operation) = path_item.get(*method) {
                if let Some(op_id) = operation.get("operationId").and_then(Value::as_str) {
                    operations.insert(
                        op_id.to_string(),
                        OperationInfo {
                            path: path.clone(),
                            method: method.to_string(),
                            operation: operation.clone(),
                        },
                    );
                }
            }
        }
    }

    operations
}
```

---

## Complete Loader Implementation

```rust
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoaderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("HTTP {0}: {1}")]
    HttpStatus(u16, String),

    #[error("Unsupported agents.json version: {0}")]
    UnsupportedVersion(String),

    #[error("No sources defined in agents.json")]
    NoSources,

    #[error("No flows defined in agents.json")]
    NoFlows,

    #[error("Source not found: {0}")]
    SourceNotFound(String),
}

pub struct BundleLoader {
    client: Client,
}

impl BundleLoader {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// Load agents.json and create Bundle
    pub fn load(&self, url: &str) -> Result<Bundle, LoaderError> {
        // Step 1: Load agents.json
        let agents_json = self.load_agents_json(url)?;

        // Step 2: Load OpenAPI source
        let source = agents_json
            .sources
            .first()
            .ok_or(LoaderError::NoSources)?;

        let openapi = self.load_openapi(&source.path)?;

        // Step 3: Apply overrides
        let openapi = self.apply_overrides(openapi, &agents_json.overrides);

        // Step 4: Index operations
        let operations = self.index_operations(&openapi);

        Ok(Bundle {
            agents_json,
            openapi,
            operations,
        })
    }

    fn load_agents_json(&self, url: &str) -> Result<AgentsJson, LoaderError> {
        let response = self.client.get(url).send()?;

        if !response.status().is_success() {
            return Err(LoaderError::HttpStatus(
                response.status().as_u16(),
                response.text()?,
            ));
        }

        let agents_json: AgentsJson = response.json()?;

        // Validate
        if agents_json.agents_json != "0.1.0" {
            return Err(LoaderError::UnsupportedVersion(agents_json.agents_json));
        }

        Ok(agents_json)
    }

    fn load_openapi(&self, url: &str) -> Result<Value, LoaderError> {
        let response = self.client.get(url).send()?;

        if !response.status().is_success() {
            return Err(LoaderError::HttpStatus(
                response.status().as_u16(),
                response.text()?,
            ));
        }

        let content = response.text()?;

        if url.ends_with(".yaml") || url.ends_with(".yml") {
            let yaml: serde_yaml::Value = serde_yaml::from_str(&content)?;
            Ok(serde_json::to_value(yaml)?)
        } else {
            Ok(serde_json::from_str(&content)?)
        }
    }

    fn apply_overrides(&self, mut openapi: Value, overrides: &[Override]) -> Value {
        for override_item in overrides {
            let path: Vec<&str> = override_item.field_path.split('.').collect();
            self.set_value_at_path(&mut openapi, &path, override_item.value.clone());
        }
        openapi
    }

    fn set_value_at_path(&self, value: &mut Value, path: &[&str], new_value: Value) {
        if path.is_empty() {
            *value = new_value;
            return;
        }

        let current = path[0];
        let remaining = &path[1..];

        if remaining.is_empty() {
            if let Value::Object(obj) = value {
                obj.insert(current.to_string(), new_value);
            }
            return;
        }

        if let Value::Object(obj) = value {
            if let Some(nested) = obj.get_mut(current) {
                self.set_value_at_path(nested, remaining, new_value);
            }
        }
    }

    fn index_operations(&self, openapi: &Value) -> HashMap<String, OperationInfo> {
        let mut operations = HashMap::new();

        let paths = match openapi.get("paths") {
            Some(Value::Object(p)) => p,
            _ => return operations,
        };

        let methods = ["get", "post", "put", "delete", "patch", "options", "head"];

        for (path, path_item) in paths {
            let path_item = match path_item {
                Value::Object(item) => item,
                _ => continue,
            };

            for method in &methods {
                if let Some(operation) = path_item.get(*method) {
                    if let Some(op_id) = operation.get("operationId").and_then(Value::as_str) {
                        operations.insert(
                            op_id.to_string(),
                            OperationInfo {
                                path: path.clone(),
                                method: method.to_string(),
                                operation: operation.clone(),
                            },
                        );
                    }
                }
            }
        }

        operations
    }
}

impl Default for BundleLoader {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## Usage Examples

### Basic Loading

```rust
use wildcard_ai::loader::BundleLoader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let loader = BundleLoader::new();

    let bundle = loader.load(
        "https://raw.githubusercontent.com/wild-card-ai/agents-json/master/agents_json/stripe/agents.json"
    )?;

    println!("Loaded {} flows", bundle.agents_json.flows.len());
    println!("Indexed {} operations", bundle.operations.len());

    Ok(())
}
```

### Custom HTTP Client

```rust
use reqwest::blocking::{Client, ClientBuilder};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .build()?;

    let loader = BundleLoader::with_client(client);

    let bundle = loader.load("https://.../agents.json")?;

    Ok(())
}
```

### Accessing Operation Info

```rust
fn get_operation_details(bundle: &Bundle, operation_id: &str) {
    if let Some(op) = bundle.operations.get(operation_id) {
        println!("Path: {}", op.path);
        println!("Method: {}", op.method);
        println!("Operation: {:?}", op.operation);
    }
}
```

---

## Error Handling

### Error Types

```rust
#[derive(Error, Debug)]
pub enum LoaderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON/YAML parse error: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("HTTP {status}: {message}")]
    HttpStatus { status: u16, message: String },

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(String),

    #[error("No sources defined")]
    NoSources,

    #[error("No flows defined")]
    NoFlows,

    #[error("Source fetch failed: {0}")]
    SourceFetchFailed(String),
}
```

### Recovery Strategies

```rust
use std::time::Duration;

fn load_with_retry(loader: &BundleLoader, url: &str, max_retries: u32) -> Result<Bundle, LoaderError> {
    let mut last_error = None;

    for attempt in 0..max_retries {
        match loader.load(url) {
            Ok(bundle) => return Ok(bundle),
            Err(e) => {
                last_error = Some(e);

                // Don't retry on parse errors
                if matches!(last_error, Some(LoaderError::Parse(_))) {
                    break;
                }

                // Backoff before retry
                std::thread::sleep(Duration::from_millis(100 * (attempt + 1) as u64));
            }
        }
    }

    Err(last_error.unwrap())
}
```

---

## Caching

### In-Memory Cache

```rust
use moka::sync::Cache;
use std::time::Duration;

pub struct CachedBundleLoader {
    inner: BundleLoader,
    cache: Cache<String, Bundle>,
}

impl CachedBundleLoader {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: BundleLoader::new(),
            cache: Cache::builder()
                .max_capacity(100)
                .time_to_live(ttl)
                .build(),
        }
    }

    pub fn load(&self, url: &str) -> Result<Bundle, LoaderError> {
        // Check cache first
        if let Some(bundle) = self.cache.get(url) {
            return Ok(bundle);
        }

        // Load and cache
        let bundle = self.inner.load(url)?;
        self.cache.insert(url.to_string(), bundle.clone());

        Ok(bundle)
    }
}
```

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_override() {
        let loader = BundleLoader::new();
        let mut openapi = json!({
            "paths": {
                "/users": {
                    "post": {
                        "operationId": "create_user",
                        "parameters": {
                            "required": false
                        }
                    }
                }
            }
        });

        let override_item = Override {
            source_id: "test".to_string(),
            operation_id: "create_user".to_string(),
            field_path: "paths./users.post.parameters.required".to_string(),
            value: json!(true),
            additional: HashMap::new(),
        };

        loader.apply_overrides(&mut openapi, &[override_item]);

        let result = &openapi["paths"]["/users"]["post"]["parameters"]["required"];
        assert_eq!(result, &json!(true));
    }

    #[test]
    fn test_index_operations() {
        let loader = BundleLoader::new();
        let openapi = json!({
            "paths": {
                "/users": {
                    "get": { "operationId": "list_users" },
                    "post": { "operationId": "create_user" }
                },
                "/products": {
                    "get": { "operationId": "list_products" }
                }
            }
        });

        let operations = loader.index_operations(&openapi);

        assert_eq!(operations.len(), 3);
        assert!(operations.contains_key("list_users"));
        assert!(operations.contains_key("create_user"));
        assert!(operations.contains_key("list_products"));
    }
}
```

---

## Performance Considerations

1. **Connection Reuse**: Use a single Client instance
2. **Caching**: Cache loaded bundles to avoid refetching
3. **Lazy Loading**: Load OpenAPI specs on demand
4. **Parallel Loading**: Fetch multiple sources concurrently

---

*This document covers the Bundle Loader. See 03-flow-execution-deep-dive.md for execution details.*
