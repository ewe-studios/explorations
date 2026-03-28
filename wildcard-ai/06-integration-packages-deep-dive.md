---
title: "Integration Packages Deep Dive"
subtitle: "Building API integrations with SDK and REST patterns"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/wildcard-ai/06-integration-packages-deep-dive.md
prerequisites: 00-zero-to-ai-engineer.md, rust-revision.md
---

# Integration Packages Deep Dive

## Overview

Integration packages connect Wildcard-AI to external APIs. There are two patterns:

1. **SDK Pattern**: Use official SDKs (Stripe, etc.)
2. **REST API Handler**: Direct HTTP requests

---

## Integration Trait

```rust
use reqwest::blocking::Client;
use serde_json::Value;

pub trait Integration: Send + Sync {
    /// Source ID this integration handles
    fn source_id(&self) -> &str;

    /// Execute an operation
    fn execute(
        &self,
        client: &Client,
        operation_id: &str,
        auth: &AuthConfig,
        parameters: Value,
        request_body: Value,
    ) -> Value;

    /// Get operation metadata (optional)
    fn get_operation(&self, operation_id: &str) -> Option<&OperationInfo> {
        None
    }
}
```

---

## SDK Pattern: Stripe

### Structure

```
integrations/
└── stripe/
    ├── __init__.py (Python) or mod.rs (Rust)
    ├── map.py or map.rs    # operationId -> function mapping
    └── tools.py or tools.rs # SDK wrapper functions
```

### Python Example

```python
# integrations/stripe/tools.py
from pydantic import BaseModel

class Executor(BaseModel):
    @staticmethod
    def stripe_post_customers(api_key: str, **kwargs):
        import stripe
        stripe.api_key = api_key
        return stripe.Customer.create(**kwargs)

    @staticmethod
    def stripe_get_customers(api_key: str, **kwargs):
        import stripe
        stripe.api_key = api_key
        return stripe.Customer.list(**kwargs)

    @staticmethod
    def stripe_post_products(api_key: str, **kwargs):
        import stripe
        stripe.api_key = api_key
        return stripe.Product.create(**kwargs)

    @staticmethod
    def stripe_get_balance(api_key: str, **kwargs):
        import stripe
        stripe.api_key = api_key
        return stripe.Balance.retrieve(**kwargs)
```

```python
# integrations/stripe/map.py
from .tools import Executor
from ..types import ExecutorType

map_type = ExecutorType.SDK

map = {
    "stripe_post_customers": Executor.stripe_post_customers,
    "stripe_get_customers": Executor.stripe_get_customers,
    "stripe_post_products": Executor.stripe_post_products,
    "stripe_get_balance": Executor.stripe_get_balance,
}
```

### Rust Translation

```rust
// integrations/stripe/tools.rs
use reqwest::blocking::Client;
use serde_json::{json, Value};

pub struct StripeExecutor;

impl StripeExecutor {
    pub fn stripe_post_customers(
        client: &Client,
        api_key: &str,
        params: &Value,
    ) -> Value {
        let response = client
            .post("https://api.stripe.com/v1/customers")
            .basic_auth(api_key, Some(""))
            .form(&flatten_params(params))
            .send();

        match response {
            Ok(resp) => resp.json().unwrap_or(json!({"error": "Parse failed"})),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    pub fn stripe_post_products(
        client: &Client,
        api_key: &str,
        params: &Value,
    ) -> Value {
        let response = client
            .post("https://api.stripe.com/v1/products")
            .basic_auth(api_key, Some(""))
            .form(&flatten_params(params))
            .send();

        match response {
            Ok(resp) => resp.json().unwrap_or(json!({"error": "Parse failed"})),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    pub fn stripe_get_balance(
        client: &Client,
        api_key: &str,
        _params: &Value,
    ) -> Value {
        let response = client
            .get("https://api.stripe.com/v1/balance")
            .basic_auth(api_key, Some(""))
            .send();

        match response {
            Ok(resp) => resp.json().unwrap_or(json!({"error": "Parse failed"})),
            Err(e) => json!({"error": e.to_string()}),
        }
    }
}

fn flatten_params(params: &Value) -> Vec<(String, String)> {
    let mut result = Vec::new();

    if let Value::Object(obj) = params {
        for (key, value) in obj {
            result.push((key.clone(), value.to_string()));
        }
    }

    result
}
```

```rust
// integrations/stripe/map.rs
use super::tools::StripeExecutor;
use crate::integration::ExecutorType;

pub const MAP_TYPE: ExecutorType = ExecutorType::Sdk;

pub type OperationMap = phf::Map<&'static str, fn(&reqwest::blocking::Client, &str, &serde_json::Value) -> serde_json::Value>;

pub static MAP: OperationMap = phf::phf_map! {
    "stripe_post_customers" => StripeExecutor::stripe_post_customers,
    "stripe_get_customers" => StripeExecutor::stripe_get_customers,
    "stripe_post_products" => StripeExecutor::stripe_post_products,
    "stripe_get_balance" => StripeExecutor::stripe_get_balance,
};
```

---

## REST API Handler Pattern

### Python Example

```python
# integrations/resend/tools.py
import requests

class Executor:
    @staticmethod
    def resend_post_emails(api_key: str, **kwargs):
        response = requests.post(
            "https://api.resend.com/emails",
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json"
            },
            json=kwargs
        )
        return response.json()

    @staticmethod
    def resend_get_domains(api_key: str, **kwargs):
        response = requests.get(
            "https://api.resend.com/domains",
            headers={"Authorization": f"Bearer {api_key}"}
        )
        return response.json()
```

### Rust Translation

```rust
// integrations/resend/tools.rs
use reqwest::blocking::{Client, RequestBuilder};
use serde_json::{json, Value};

pub struct ResendExecutor;

impl ResendExecutor {
    pub fn resend_post_emails(
        client: &Client,
        api_key: &str,
        params: &Value,
    ) -> Value {
        let response = client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(params)
            .send();

        match response {
            Ok(resp) => resp.json().unwrap_or(json!({"error": "Parse failed"})),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    pub fn resend_get_domains(
        client: &Client,
        api_key: &str,
        _params: &Value,
    ) -> Value {
        let response = client
            .get("https://api.resend.com/domains")
            .header("Authorization", format!("Bearer {}", api_key))
            .send();

        match response {
            Ok(resp) => resp.json().unwrap_or(json!({"error": "Parse failed"})),
            Err(e) => json!({"error": e.to_string()}),
        }
    }
}
```

---

## Integration Registry

### Python

```python
# integrations/__init__.py
from typing import Dict, Type
from .types import IntegrationType

INTEGRATIONS: Dict[str, IntegrationType] = {
    "stripe": "integrations.stripe",
    "resend": "integrations.resend",
    "twitter": "integrations.twitter",
}

def get_integration(source_id: str):
    if source_id not in INTEGRATIONS:
        raise ValueError(f"Unknown integration: {source_id}")

    module = __import__(INTEGRATIONS[source_id], fromlist=["map"])
    return module.map, module.map_type
```

### Rust

```rust
// integrations/mod.rs
use std::collections::HashMap;
use std::sync::OnceLock;

pub mod stripe;
pub mod resend;
pub mod types;

use crate::integration::Integration;

static REGISTRY: OnceLock<HashMap<&'static str, Box<dyn Integration>>> = OnceLock::new();

pub fn init_registry() {
    let mut map = HashMap::new();

    map.insert("stripe", Box::new(stripe::StripeIntegration::new()) as Box<dyn Integration>);
    map.insert("resend", Box::new(resend::ResendIntegration::new()) as Box<dyn Integration>);

    REGISTRY.set(map).expect("Failed to initialize registry");
}

pub fn get_integration(source_id: &str) -> &'static dyn Integration {
    REGISTRY
        .get()
        .expect("Registry not initialized")
        .get(source_id)
        .unwrap_or_else(|| panic!("Unknown integration: {}", source_id))
}
```

---

## Creating New Integrations

### Step 1: Create Module Structure

```
integrations/
└── myapi/
    ├── mod.rs
    ├── map.rs
    └── tools.rs
```

### Step 2: Define Operations

```rust
// integrations/myapi/tools.rs
use reqwest::blocking::Client;
use serde_json::{json, Value};

pub struct MyApiExecutor;

impl MyApiExecutor {
    pub fn myapi_get_users(
        client: &Client,
        api_key: &str,
        params: &Value,
    ) -> Value {
        let mut request = client
            .get("https://api.myapi.com/users")
            .header("Authorization", format!("Bearer {}", api_key));

        // Add query parameters
        if let Value::Object(obj) = params {
            for (key, value) in obj {
                if !value.is_null() {
                    request = request.query(&[(key, value.to_string())]);
                }
            }
        }

        match request.send() {
            Ok(resp) => resp.json().unwrap_or(json!({"error": "Parse failed"})),
            Err(e) => json!({"error": e.to_string()}),
        }
    }

    pub fn myapi_post_users(
        client: &Client,
        api_key: &str,
        params: &Value,
    ) -> Value {
        match client
            .post("https://api.myapi.com/users")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(params)
            .send()
        {
            Ok(resp) => resp.json().unwrap_or(json!({"error": "Parse failed"})),
            Err(e) => json!({"error": e.to_string()}),
        }
    }
}
```

### Step 3: Create Operation Map

```rust
// integrations/myapi/map.rs
use super::tools::MyApiExecutor;
use crate::integration::ExecutorType;

pub const MAP_TYPE: ExecutorType = ExecutorType::RestApiHandler;

pub type OperationFn = fn(&reqwest::blocking::Client, &str, &serde_json::Value) -> serde_json::Value;

pub static MAP: phf::Map<&'static str, OperationFn> = phf::phf_map! {
    "myapi_get_users" => MyApiExecutor::myapi_get_users,
    "myapi_post_users" => MyApiExecutor::myapi_post_users,
};
```

### Step 4: Create Integration Struct

```rust
// integrations/myapi/mod.rs
use reqwest::blocking::Client;
use serde_json::Value;
use crate::integration::{Integration, AuthConfig, OperationInfo};
use crate::integrations::types::ExecutorType;

pub mod map;
pub mod tools;

pub struct MyApiIntegration;

impl MyApiIntegration {
    pub fn new() -> Self {
        Self
    }
}

impl Integration for MyApiIntegration {
    fn source_id(&self) -> &str {
        "myapi"
    }

    fn execute(
        &self,
        client: &Client,
        operation_id: &str,
        auth: &AuthConfig,
        parameters: Value,
        request_body: Value,
    ) -> Value {
        // Get API key from auth
        let api_key = match auth {
            AuthConfig::ApiKey(api) => &api.key_value,
            AuthConfig::Bearer(bearer) => &bearer.token,
            _ => return json!({"error": "Unsupported auth type"}),
        };

        // Merge parameters and request_body
        let mut params = parameters.clone();
        if let Value::Object(mut obj) = params {
            if let Value::Object(body) = request_body {
                for (k, v) in body {
                    obj.insert(k, v);
                }
            }
            params = Value::Object(obj);
        }

        // Look up and execute operation
        if let Some(op_fn) = map::MAP.get(operation_id) {
            op_fn(client, api_key, &params)
        } else {
            json!({"error": format!("Unknown operation: {}", operation_id)})
        }
    }
}
```

### Step 5: Register Integration

```rust
// In main integrations/mod.rs
pub mod myapi;

pub fn init_registry() {
    let mut map = HashMap::new();

    map.insert("stripe", Box::new(stripe::StripeIntegration::new()) as Box<dyn Integration>);
    map.insert("resend", Box::new(resend::ResendIntegration::new()) as Box<dyn Integration>);
    map.insert("myapi", Box::new(myapi::MyApiIntegration::new()) as Box<dyn Integration>);

    REGISTRY.set(map).expect("Failed to initialize registry");
}
```

---

## Available Integrations

| API | Type | Auth | Operations |
|-----|------|------|------------|
| Stripe | SDK | API Key | Products, Prices, Customers, Payments |
| Resend | REST | Bearer | Emails, Domains, API Keys |
| Twitter | SDK | OAuth 1.0 | Tweets, Users, Search |
| Giphy | REST | API Key | Search, Trending, Translate |
| Slack | REST | Bearer | Chat, Channels, Users |
| HubSpot | REST | Bearer | Contacts, Companies, Deals |
| Google Sheets | SDK | OAuth 2.0 | Spreadsheets, Values |
| Alpaca | REST | API Key | Trading, Market Data |
| Rootly | REST | API Key | Incidents, Responders |
| Linkup | REST | API Key | Search, People, Companies |

---

## Testing Integrations

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stripe_get_balance() {
        let client = Client::new();
        let result = StripeExecutor::stripe_get_balance(
            &client,
            "sk_test_123",
            &json!({}),
        );

        // Should return error for invalid key (not crash)
        assert!(result.get("error").is_some() || result.get("object").is_some());
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    #[ignore]  // Requires real API key
    fn test_real_stripe_request() {
        let client = Client::new();
        let api_key = std::env::var("STRIPE_TEST_KEY").unwrap();

        let result = StripeExecutor::stripe_get_balance(
            &client,
            &api_key,
            &json!({}),
        );

        assert!(result.get("object").is_some());
    }
}
```

---

## Best Practices

### 1. Error Handling

```rust
// Always return JSON, even on error
match response.send() {
    Ok(resp) => resp.json().unwrap_or(json!({"error": "Parse failed"})),
    Err(e) => json!({"error": e.to_string()}),
}
```

### 2. Auth Flexibility

```rust
// Support multiple auth types
let api_key = match auth {
    AuthConfig::ApiKey(api) => &api.key_value,
    AuthConfig::Bearer(bearer) => &bearer.token,
    _ => return json!({"error": "Unsupported auth"}),
};
```

### 3. Operation Discovery

```rust
// Provide operation metadata when possible
fn get_operation(&self, operation_id: &str) -> Option<&OperationInfo> {
    self.operations.get(operation_id)
}
```

---

*This document covers integration patterns. See 07-valtron-executor-guide.md for execution details.*
