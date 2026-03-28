---
title: "Tool Format Conversion Deep Dive"
subtitle: "OpenAI and JSON tool formats for LLM integration"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/wildcard-ai/04-tool-format-conversion-deep-dive.md
prerequisites: 00-zero-to-ai-engineer.md, 01-agents-json-specification-deep-dive.md
---

# Tool Format Conversion Deep Dive

## Overview

Tool format conversion transforms `agents.json` flows into LLM-readable tool definitions. This document covers:
1. OpenAI function calling format
2. JSON tool format
3. Schema conversion
4. Prompt generation

---

## ToolFormat Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolFormat {
    /// OpenAI function calling format
    OpenAI,
    /// Generic JSON format
    Json,
}
```

---

## OpenAI Tool Format

### Structure

```json
{
  "type": "function",
  "function": {
    "name": "send_email",
    "description": "Send an email via Resend API",
    "parameters": {
      "type": "object",
      "properties": {
        "from": {
          "type": "string",
          "description": "Sender email address"
        },
        "to": {
          "type": "array",
          "items": {"type": "string"},
          "description": "Recipient email addresses"
        },
        "subject": {
          "type": "string",
          "description": "Email subject"
        }
      },
      "required": ["from", "to", "subject"],
      "additionalProperties": false
    }
  }
}
```

### Conversion Function

```rust
use serde_json::{json, Value};

pub fn flows_to_openai_tools(flows: &[Flow]) -> Vec<Value> {
    flows.iter().map(|flow| flow_to_openai_tool(flow)).collect()
}

fn flow_to_openai_tool(flow: &Flow) -> Value {
    let properties = convert_fields_to_properties(&flow.fields);

    let required: Vec<Value> = flow
        .fields
        .parameters
        .iter()
        .filter(|p| p.required)
        .map(|p| Value::String(p.name.clone()))
        .collect();

    json!({
        "type": "function",
        "function": {
            "name": flow.id,
            "description": flow.description,
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false
            }
        }
    })
}
```

### Field Conversion

```rust
fn convert_fields_to_properties(fields: &Fields) -> serde_json::Map<String, Value> {
    let mut properties = serde_json::Map::new();

    // Convert parameters
    for param in &fields.parameters {
        let param_schema = convert_parameter(param);
        properties.insert(param.name.clone(), param_schema);
    }

    // Convert request body
    if let Some(request_body) = &fields.request_body {
        if let Some(content) = &request_body.content {
            if let Some(json_content) = content.get("application/json") {
                if let Some(schema) = &json_content.schema {
                    properties.insert("requestBody".to_string(), schema.clone());
                }
            }
        }
    }

    properties
}

fn convert_parameter(param: &Parameter) -> Value {
    let mut schema = serde_json::Map::new();

    // Type
    let param_type = param.r#type.as_deref().unwrap_or("string");
    schema.insert("type".to_string(), Value::String(param_type.to_string()));

    // Description
    if let Some(desc) = &param.description {
        schema.insert("description".to_string(), Value::String(desc.clone()));
    }

    // Enum
    if let Some(enum_values) = &param.enum_values {
        schema.insert(
            "enum".to_string(),
            Value::Array(enum_values.iter().map(|v| Value::String(v.clone())).collect()),
        );
    }

    // Array items
    if param_type == "array" {
        schema.insert(
            "items".to_string(),
            json!({"type": "string"}),
        );
    }

    Value::Object(schema)
}
```

### JSON Schema Conversion

```rust
/// Convert JSON Schema to OpenAI format
fn convert_schema_to_openai(schema: &Value) -> Value {
    match schema {
        Value::Object(obj) => {
            let mut result = serde_json::Map::new();

            // Handle oneOf -> anyOf conversion
            if let Some(one_of) = obj.get("oneOf") {
                let any_of: Vec<Value> = one_of
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(convert_schema_to_openai)
                    .collect();
                result.insert("anyOf".to_string(), Value::Array(any_of));

                // Copy other fields
                for (key, value) in obj {
                    if key != "oneOf" {
                        result.insert(key.clone(), value.clone());
                    }
                }
                return Value::Object(result);
            }

            // Handle object with properties
            if obj.get("type").and_then(Value::as_str) == Some("object") {
                result.insert("type".to_string(), Value::String("object".to_string()));

                if let Some(Value::Object(props)) = obj.get("properties") {
                    let converted_props: serde_json::Map<String, Value> = props
                        .iter()
                        .map(|(k, v)| (k.clone(), convert_schema_to_openai(v)))
                        .collect();
                    result.insert("properties".to_string(), Value::Object(converted_props));
                }

                if let Some(required) = obj.get("required") {
                    result.insert("required".to_string(), required.clone());
                }

                return Value::Object(result);
            }

            // Handle array
            if obj.get("type").and_then(Value::as_str) == Some("array") {
                result.insert("type".to_string(), Value::String("array".to_string()));

                if let Some(items) = obj.get("items") {
                    result.insert("items".to_string(), convert_schema_to_openai(items));
                }

                return Value::Object(result);
            }

            // Primitive types
            if let Some(t) = obj.get("type").and_then(Value::as_str) {
                result.insert("type".to_string(), Value::String(t.to_string()));
            }

            if let Some(desc) = obj.get("description") {
                result.insert("description".to_string(), desc.clone());
            }

            if let Some(format) = obj.get("format") {
                result.insert("format".to_string(), format.clone());
            }

            if let Some(enum_values) = obj.get("enum") {
                result.insert("enum".to_string(), enum_values.clone());
            }

            Value::Object(result)
        }
        other => other.clone(),
    }
}
```

---

## JSON Tool Format

### Structure

```json
{
  "type": "function",
  "function": {
    "name": "send_email",
    "description": "Send an email via Resend API",
    "parameters": {
      "parameters": [
        {
          "name": "from",
          "type": "string",
          "description": "Sender email address",
          "required": true
        }
      ],
      "requestBody": {
        "content": {
          "application/json": {
            "schema": {...}
          }
        }
      },
      "responses": {
        "success": {...}
      }
    }
  }
}
```

### Conversion Function

```rust
pub fn flows_to_json_tools(flows: &[Flow]) -> Vec<Value> {
    flows
        .iter()
        .map(|flow| {
            json!({
                "type": "function",
                "function": {
                    "name": flow.id,
                    "description": flow.description,
                    "parameters": flow.fields
                }
            })
        })
        .collect()
}
```

---

## Prompt Generation

### Flows Prompt

```rust
/// Generate a slim prompt for LLM system prompt
pub fn flows_prompt(flows: &[Flow]) -> String {
    flows
        .iter()
        .map(|flow| format!("{}: {}", flow.id, flow.description))
        .collect::<Vec<_>>()
        .join("\n")
}
```

### Example Output

```
create_product: Create a new product in Stripe
create_price: Create a price for a product
send_email: Send an email via Resend
get_balance: Get Stripe account balance
```

### Extended Prompt

```rust
/// Generate extended prompt with parameter details
pub fn flows_prompt_extended(flows: &[Flow]) -> String {
    let mut prompt = String::new();

    for flow in flows {
        prompt.push_str(&format!("\n## {}\n", flow.title));
        prompt.push_str(&format!("{}\n\n", flow.description));

        if !flow.fields.parameters.is_empty() {
            prompt.push_str("### Parameters\n");
            for param in &flow.fields.parameters {
                let required = if param.required { " (required)" } else { "" };
                prompt.push_str(&format!(
                    "- **{}** (`{}`{}): {}\n",
                    param.name,
                    param.r#type.as_deref().unwrap_or("string"),
                    required,
                    param.description.as_deref().unwrap_or("")
                ));
            }
            prompt.push('\n');
        }
    }

    prompt
}
```

---

## Complete Converter Module

```rust
use serde_json::{json, Value};

/// Tool format enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolFormat {
    OpenAI,
    Json,
}

/// Get tools in specified format
pub fn get_tools(agents_json: &AgentsJson, format: ToolFormat) -> Vec<Value> {
    match format {
        ToolFormat::OpenAI => flows_to_openai_tools(&agents_json.flows),
        ToolFormat::Json => flows_to_json_tools(&agents_json.flows),
    }
}

/// Get prompt for flows
pub fn get_tool_prompt(agents_json: &AgentsJson) -> String {
    flows_prompt(&agents_json.flows)
}

// OpenAI conversion
pub fn flows_to_openai_tools(flows: &[Flow]) -> Vec<Value> {
    flows.iter().map(flow_to_openai_tool).collect()
}

fn flow_to_openai_tool(flow: &Flow) -> Value {
    let mut properties = serde_json::Map::new();

    // Parameters
    for param in &flow.fields.parameters {
        let mut param_schema = serde_json::Map::new();

        param_schema.insert(
            "type".to_string(),
            Value::String(param.r#type.clone().unwrap_or_else(|| "string".to_string())),
        );

        if let Some(desc) = &param.description {
            param_schema.insert("description".to_string(), Value::String(desc.clone()));
        }

        if let Some(enum_values) = &param.enum_values {
            param_schema.insert(
                "enum".to_string(),
                Value::Array(enum_values.iter().map(|v| Value::String(v.clone())).collect()),
            );
        }

        properties.insert(param.name.clone(), Value::Object(param_schema));
    }

    // Request body
    if let Some(request_body) = &flow.fields.request_body {
        if let Some(content) = &request_body.content {
            if let Some(json_content) = content.get("application/json") {
                if let Some(schema) = &json_content.schema {
                    properties.insert("requestBody".to_string(), schema.clone());
                }
            }
        }
    }

    // Required
    let required: Vec<Value> = flow
        .fields
        .parameters
        .iter()
        .filter(|p| p.required)
        .map(|p| Value::String(p.name.clone()))
        .collect();

    json!({
        "type": "function",
        "function": {
            "name": flow.id,
            "description": flow.description,
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false
            }
        }
    })
}

// JSON conversion
pub fn flows_to_json_tools(flows: &[Flow]) -> Vec<Value> {
    flows
        .iter()
        .map(|flow| {
            json!({
                "type": "function",
                "function": {
                    "name": flow.id,
                    "description": flow.description,
                    "parameters": flow.fields
                }
            })
        })
        .collect()
}

// Prompt generation
pub fn flows_prompt(flows: &[Flow]) -> String {
    flows
        .iter()
        .map(|flow| format!("{}: {}", flow.id, flow.description))
        .collect::<Vec<_>>()
        .join("\n")
}
```

---

## Usage with OpenAI

### Example

```rust
use openai_api_rust::{Chat, ChatCompletion, ChatCompletionMessage, Role, Tool};

// Get tools
let tools = get_tools(&bundle.agents_json, ToolFormat::OpenAI);

// Create OpenAI client
let client = Chat::new("sk-...");

// Call with tools
let messages = vec![ChatCompletionMessage {
    role: Role::User,
    content: "Send an email to test@example.com",
    name: None,
}];

let chat = ChatCompletion::builder(messages)
    .tools(tools)
    .build();

let response = client.chat_complete(&chat)?;

// Extract tool call
if let Some(tool_calls) = &response.choices[0].message.tool_calls {
    for tool_call in tool_calls {
        println!("Tool: {}", tool_call.function.name);
        println!("Arguments: {}", tool_call.function.arguments);
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
    fn test_flow_to_openai_tool() {
        let flow = create_test_flow();
        let tool = flow_to_openai_tool(&flow);

        assert_eq!(tool["type"], "function");
        assert_eq!(tool["function"]["name"], "test_flow");
        assert!(tool["function"]["parameters"]["properties"].is_object());
    }

    #[test]
    fn test_flows_prompt() {
        let flows = vec![create_test_flow()];
        let prompt = flows_prompt(&flows);

        assert!(prompt.contains("test_flow"));
        assert!(prompt.contains("Test flow description"));
    }
}
```

---

## Best Practices

### Descriptions

```rust
// Good: Clear, action-oriented description
"description": "Send an email to one or more recipients. Requires sender email, recipient list, and subject. Optional HTML or text content."

// Bad: Too vague
"description": "Send email"
```

### Parameter Naming

```rust
// Good: Specific names
{ "name": "recipient_emails", "type": "array" }
{ "name": "product_id", "type": "string" }

// Bad: Generic names
{ "name": "emails", "type": "array" }
{ "name": "id", "type": "string" }
```

---

*This document covers tool format conversion. See 05-authentication-system-deep-dive.md for authentication details.*
