---
title: "Flow Execution Deep Dive"
subtitle: "Action execution, link resolution, and response aggregation"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/wildcard-ai/03-flow-execution-deep-dive.md
prerequisites: 00-zero-to-ai-engineer.md, 02-bundle-loader-deep-dive.md
---

# Flow Execution Deep Dive

## Overview

Flow execution is the core of Wildcard-AI. This document covers:
1. Flow execution lifecycle
2. Link resolution and parameter mapping
3. Action execution via integrations
4. Response aggregation
5. Error handling

---

## Execution Lifecycle

### States

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Initial   │ ──► │  Executing  │ ──► │  Completed  │
└─────────────┘     └─────────────┘     └─────────────┘
                          │
                          ▼
                    ┌─────────────┐
                    │   Error     │
                    └─────────────┘
```

### Flow

```rust
1. Parse flow parameters from LLM tool call
2. For each action in order:
   a. Resolve incoming links → parameters
   b. Execute action via integration
   c. Store response in execution trace
3. Resolve outgoing links → flow response
4. Return aggregated response
```

---

## Execution Trace

The execution trace tracks state during flow execution:

```rust
use serde_json::Value;
use std::collections::HashMap;

pub struct ExecutionTrace {
    /// Flow-level parameters
    pub flow_parameters: Value,
    /// Flow-level request body
    pub flow_request_body: Value,
    /// Per-action traces
    pub actions: HashMap<String, ActionTrace>,
}

pub struct ActionTrace {
    /// Resolved parameters for this action
    pub parameters: Value,
    /// Resolved request body for this action
    pub request_body: Value,
    /// Response from executing this action
    pub response: Option<Value>,
}
```

---

## Link Resolution

### Link Types

1. **Parameter Links**: Flow parameters → Action parameters
2. **Chain Links**: Action N response → Action N+1 parameters
3. **Response Links**: Action N response → Flow response

### Resolution Algorithm

```rust
fn resolve_links(
    flow: &Flow,
    target_action_id: &str,
    trace: &ExecutionTrace,
) -> (Value, Value) {
    let mut parameters = json!({});
    let mut request_body = json!({});

    // Find all links targeting this action
    for link in &flow.links {
        if link.target.action_id.as_ref() == Some(&target_action_id.to_string()) {
            // Get source value
            let source_value = get_link_source(&link.origin, trace);

            // Apply to target path
            let target_path = &link.target.field_path;

            if target_path.starts_with("parameters") {
                set_json_path(&mut parameters, target_path, source_value);
            } else if target_path.starts_with("requestBody") {
                set_json_path(&mut request_body, target_path, source_value);
            }
        }
    }

    (parameters, request_body)
}
```

### Getting Link Source

```rust
fn get_link_source(origin: &LinkOrigin, trace: &ExecutionTrace) -> Value {
    match &origin.action_id {
        // Flow parameters
        None => {
            get_json_path(&trace.flow_parameters, &origin.field_path)
                .unwrap_or(json!(null))
        }
        // Action response
        Some(action_id) => {
            let action_trace = match trace.actions.get(action_id) {
                Some(t) => t,
                None => return json!(null),
            };

            match &action_trace.response {
                Some(response) => {
                    get_json_path(response, &origin.field_path)
                        .unwrap_or(json!(null))
                }
                None => json!(null),
            }
        }
    }
}
```

### Path Navigation

```rust
/// Get value at JSON path (e.g., "data.items[0].id")
fn get_json_path(value: &Value, path: &str) -> Option<Value> {
    let parts = parse_path(path);
    let mut current = value;

    for part in parts {
        current = match current {
            Value::Object(obj) => obj.get(&part.name)?,
            Value::Array(arr) => {
                if let Some(index) = part.index {
                    arr.get(index)?
                } else {
                    return None;
                }
            }
            _ => return None,
        };
    }

    Some(current.clone())
}

/// Parse path into components
fn parse_path(path: &str) -> Vec<PathPart> {
    path.split('.').map(|part| {
        if part.ends_with(']') && part.contains('[') {
            // Array access: items[0]
            let (name, index) = part.split_once('[').unwrap();
            let index: usize = index.trim_end_matches(']').parse().unwrap();
            PathPart { name: name.to_string(), index: Some(index) }
        } else {
            PathPart { name: part.to_string(), index: None }
        }
    }).collect()
}

struct PathPart {
    name: String,
    index: Option<usize>,
}
```

---

## Action Execution

### Execution Flow

```rust
pub fn execute_action(
    action: &Action,
    parameters: Value,
    request_body: Value,
    auth: &AuthConfig,
    integration: &dyn Integration,
) -> Value {
    // Execute via integration
    integration.execute(
        action.operation_id,
        auth,
        parameters,
        request_body,
    )
}
```

### Integration Call

```rust
pub trait Integration {
    fn execute(
        &self,
        operation_id: &str,
        auth: &AuthConfig,
        parameters: Value,
        request_body: Value,
    ) -> Value;
}
```

---

## Complete Flow Executor

```rust
use foundation_core::valtron::{TaskIterator, TaskStatus, NoSpawner};
use serde_json::Value;
use std::collections::HashMap;

pub struct FlowExecutor {
    flow: Flow,
    trace: ExecutionTrace,
    current_action_index: usize,
    auth: AuthConfig,
    integrations: HashMap<String, Box<dyn Integration>>,
}

impl FlowExecutor {
    pub fn new(flow: Flow, auth: AuthConfig) -> Self {
        Self {
            flow,
            trace: ExecutionTrace {
                flow_parameters: json!({}),
                flow_request_body: json!({}),
                actions: HashMap::new(),
            },
            current_action_index: 0,
            auth,
            integrations: HashMap::new(),
        }
    }

    pub fn with_integrations(mut self, integrations: HashMap<String, Box<dyn Integration>>) -> Self {
        self.integrations = integrations;
        self
    }

    pub fn with_parameters(mut self, parameters: Value, request_body: Value) -> Self {
        self.trace.flow_parameters = parameters;
        self.trace.flow_request_body = request_body;
        self
    }
}

impl TaskIterator for FlowExecutor {
    type Ready = Value;
    type Pending = ();
    type Spawner = NoSpawner;

    fn next(&mut self) -> Option<TaskStatus<Self::Ready, Self::Pending, Self::Spawner>> {
        // Check if all actions complete
        if self.current_action_index >= self.flow.actions.len() {
            let response = self.aggregate_response();
            return Some(TaskStatus::Ready(response));
        }

        // Get current action
        let action = &self.flow.actions[self.current_action_index];

        // Resolve links for this action
        let (params, body) = resolve_links(&self.flow, &action.id, &self.trace);

        // Get integration
        let integration = match self.integrations.get(&action.source_id) {
            Some(i) => i.as_ref(),
            None => {
                let error = json!({
                    "error": format!("Unknown integration: {}", action.source_id)
                });
                self.trace.actions.insert(
                    action.id.clone(),
                    ActionTrace {
                        parameters: params.clone(),
                        request_body: body.clone(),
                        response: Some(error.clone()),
                    },
                );
                self.current_action_index += 1;
                return Some(TaskStatus::Pending(()));
            }
        };

        // Execute action
        let response = integration.execute(
            &action.operation_id,
            &self.auth,
            params.clone(),
            body.clone(),
        );

        // Store in trace
        self.trace.actions.insert(
            action.id.clone(),
            ActionTrace {
                parameters: params,
                request_body: body,
                response: Some(response),
            },
        );

        self.current_action_index += 1;

        Some(TaskStatus::Pending(()))
    }
}

impl FlowExecutor {
    fn aggregate_response(&self) -> Value {
        // Check for flow response links
        for link in &self.flow.links {
            if link.target.action_id.is_none()
                && link.target.field_path.starts_with("responses")
            {
                let source_value = get_link_source(&link.origin, &self.trace);
                let mut result = json!({});
                set_json_path(&mut result, &link.target.field_path, source_value);
                return result;
            }
        }

        // Default: return last action's response
        if let Some(last_action) = self.flow.actions.last() {
            if let Some(trace) = self.trace.actions.get(&last_action.id) {
                return trace.response.clone().unwrap_or(json!({}));
            }
        }

        json!({})
    }
}
```

---

## Example: Multi-Action Flow

### Flow Definition

```json
{
  "id": "create_product_with_price",
  "actions": [
    {"id": "create_product", "operationId": "stripe_post_products"},
    {"id": "create_price", "operationId": "stripe_post_prices"}
  ],
  "links": [
    {
      "origin": {"actionId": null, "fieldPath": "parameters.name"},
      "target": {"actionId": "create_product", "fieldPath": "parameters.name"}
    },
    {
      "origin": {"actionId": "create_product", "fieldPath": "responses.success.id"},
      "target": {"actionId": "create_price", "fieldPath": "parameters.product"}
    }
  ]
}
```

### Execution Trace

```
Initial State:
  flow_parameters: {"name": "Test Product", "unit_amount": 1000}
  actions: {}

After create_product:
  flow_parameters: {"name": "Test Product", "unit_amount": 1000}
  actions: {
    "create_product": {
      "parameters": {"name": "Test Product"},
      "response": {"id": "prod_123", ...}
    }
  }

After create_price:
  flow_parameters: {"name": "Test Product", "unit_amount": 1000}
  actions: {
    "create_product": {...},
    "create_price": {
      "parameters": {"product": "prod_123", "unit_amount": 1000},
      "response": {"id": "price_456", ...}
    }
  }

Final Response:
  {"success": {"price_id": "price_456"}}
```

---

## Error Handling

### Error Types

```rust
#[derive(Debug)]
pub enum ExecutionError {
    LinkResolutionError {
        link_index: usize,
        reason: String,
    },
    ActionExecutionError {
        action_id: String,
        error: String,
    },
    IntegrationNotFound(String),
    InvalidFieldPath(String),
}
```

### Error Propagation

```rust
fn execute_with_error_handling(
    flow: &Flow,
    auth: &AuthConfig,
) -> Result<Value, ExecutionError> {
    let mut executor = FlowExecutor::new(flow.clone(), auth.clone());

    for status in executor.iter() {
        match status {
            TaskStatus::Ready(result) => {
                if result.get("error").is_some() {
                    return Err(ExecutionError::ActionExecutionError {
                        action_id: "unknown".to_string(),
                        error: result["error"].as_str().unwrap().to_string(),
                    });
                }
                return Ok(result);
            }
            TaskStatus::Pending(_) => continue,
            _ => {}
        }
    }

    Err(ExecutionError::ActionExecutionError {
        action_id: "unknown".to_string(),
        error: "Execution completed without result".to_string(),
    })
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
    fn test_single_action_flow() {
        let flow = Flow {
            id: "test_flow".to_string(),
            title: "Test".to_string(),
            description: "Test flow".to_string(),
            actions: vec![Action {
                id: "action1".to_string(),
                source_id: "test".to_string(),
                operation_id: "test_op".to_string(),
                additional: HashMap::new(),
            }],
            links: vec![],
            fields: create_test_fields(),
            additional: None,
        };

        let auth = AuthConfig::ApiKey(ApiKeyAuth {
            key_value: "test_key".to_string(),
            key_name: None,
            key_prefix: None,
        });

        let mut executor = FlowExecutor::new(flow, auth);
        executor = executor.with_integrations(create_mock_integrations());

        let mut result = None;
        for status in executor.iter() {
            if let TaskStatus::Ready(r) = status {
                result = Some(r);
            }
        }

        assert!(result.is_some());
    }

    #[test]
    fn test_link_resolution() {
        // Test that links correctly map data between actions
    }

    #[test]
    fn test_response_aggregation() {
        // Test that flow response is correctly aggregated
    }
}
```

---

## Performance Considerations

1. **Sequential Execution**: Actions execute in order - can't parallelize
2. **Link Resolution**: O(n) where n = number of links
3. **Memory**: Execution trace grows with each action

---

*This document covers flow execution. See 04-tool-format-conversion-deep-dive.md for tool conversion details.*
