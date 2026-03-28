---
title: "agents.json Specification Deep Dive"
subtitle: "Complete schema reference and usage guide"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/wildcard-ai/01-agents-json-specification-deep-dive.md
prerequisites: 00-zero-to-ai-engineer.md
---

# agents.json Specification Deep Dive

## Overview

The `agents.json` specification extends OpenAPI to enable LLM-driven API workflows. This document covers the complete schema structure with examples.

### Schema Version

Current version: `0.1.0`

### Core Structure

```json
{
  "agentsJson": "0.1.0",
  "info": { ... },
  "sources": [ ... ],
  "overrides": [ ... ],
  "flows": [ ... ]
}
```

---

## Info Section

### Structure

```json
{
  "info": {
    "title": "Stripe API Agent Flows",
    "version": "0.1.1",
    "description": "Agentic workflows for interacting with the Stripe API"
  }
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `title` | string | Yes | Human-readable name |
| `version` | string | Yes | SemVer version |
| `description` | string | Yes | LLM-readable description |

---

## Sources Section

### Structure

```json
{
  "sources": [
    {
      "id": "stripe",
      "path": "https://raw.githubusercontent.com/.../openapi.yaml"
    }
  ]
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Snake case identifier (globally unique) |
| `path` | string | Yes | URL or path to OpenAPI 3+ spec |

### Example: Multiple Sources

```json
{
  "sources": [
    {
      "id": "stripe",
      "path": "https://raw.githubusercontent.com/.../stripe/openapi.yaml"
    },
    {
      "id": "resend",
      "path": "https://raw.githubusercontent.com/.../resend/openapi.yaml"
    }
  ]
}
```

---

## Overrides Section

Overrides modify OpenAPI operation fields without changing the source spec.

### Structure

```json
{
  "overrides": [
    {
      "sourceId": "stripe",
      "operationId": "stripe_post_products",
      "fieldPath": "parameters.name.required",
      "value": true
    }
  ]
}
```

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `sourceId` | string | Yes | Must match a source ID |
| `operationId` | string | Yes | Must match OpenAPI operationId |
| `fieldPath` | string | Yes | JSON path to field |
| `value` | any | Yes | New value |

### Example: Making Field Required

```json
{
  "overrides": [
    {
      "sourceId": "resend",
      "operationId": "resend_post_emails",
      "fieldPath": "requestBody.content.application/json.schema.required",
      "value": ["from", "to", "subject"]
    }
  ]
}
```

---

## Flows Section

### Complete Flow Structure

```json
{
  "flows": [
    {
      "id": "manage_products_prices",
      "title": "Manage Products and Prices",
      "description": "Automates creation, updating, and retrieval of products and prices",
      "actions": [ ... ],
      "links": [ ... ],
      "fields": { ... }
    }
  ]
}
```

### Flow Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Snake case identifier |
| `title` | string | Yes | Human-readable name |
| `description` | string | Yes | LLM-readable description |
| `actions` | array | Yes | API operations to execute |
| `links` | array | No | Data mappings between actions |
| `fields` | object | Yes | Interface definition |

---

## Actions

### Structure

```json
{
  "actions": [
    {
      "id": "create_product",
      "sourceId": "stripe",
      "operationId": "stripe_post_products"
    },
    {
      "id": "create_price",
      "sourceId": "stripe",
      "operationId": "stripe_post_prices"
    }
  ]
}
```

### Action Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | Unique within flow |
| `sourceId` | string | Yes | References a source |
| `operationId` | string | Yes | OpenAPI operation ID |

---

## Links

Links define data flow between actions.

### Structure

```json
{
  "links": [
    {
      "origin": {
        "actionId": "create_product",
        "fieldPath": "responses.success.id"
      },
      "target": {
        "actionId": "create_price",
        "fieldPath": "parameters.product"
      }
    }
  ]
}
```

### Link Fields

| Field | Path | Required | Description |
|-------|------|----------|-------------|
| `origin.actionId` | string | No | Source action (null = flow parameters) |
| `origin.fieldPath` | string | Yes | Path to extract data from |
| `target.actionId` | string | No | Target action (null = flow response) |
| `target.fieldPath` | string | Yes | Path to set data to |

### Field Path Syntax

```
# Parameters
parameters.email
parameters.user.name

# Request body
requestBody.from
requestBody.data.items

# Response
responses.success.id
responses.success.data.items[0].name

# Array indexing
line_items[0].quantity
results[2].id
```

### Example: Complete Link Chain

```json
{
  "links": [
    {
      "origin": {
        "actionId": null,
        "fieldPath": "parameters.product_name"
      },
      "target": {
        "actionId": "create_product",
        "fieldPath": "parameters.name"
      }
    },
    {
      "origin": {
        "actionId": "create_product",
        "fieldPath": "responses.success.id"
      },
      "target": {
        "actionId": "create_price",
        "fieldPath": "parameters.product"
      }
    },
    {
      "origin": {
        "actionId": "create_price",
        "fieldPath": "responses.success.id"
      },
      "target": {
        "actionId": null,
        "fieldPath": "responses.success.price_id"
      }
    }
  ]
}
```

---

## Fields

Fields define the flow's interface.

### Structure

```json
{
  "fields": {
    "parameters": [
      {
        "name": "product_name",
        "type": "string",
        "description": "Name of the product",
        "required": true
      }
    ],
    "requestBody": {
      "content": {
        "application/json": {
          "schema": { ... },
          "example": { ... }
        }
      },
      "required": true
    },
    "responses": {
      "success": {
        "type": "object",
        "description": "Response schema"
      }
    }
  }
}
```

### Parameters

```json
{
  "parameters": [
    {
      "name": "email",
      "type": "string",
      "description": "Recipient email address",
      "required": true
    },
    {
      "name": "amount",
      "type": "integer",
      "description": "Amount in cents",
      "required": false
    }
  ]
}
```

#### Parameter Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Parameter name |
| `type` | string | No | Type (string, integer, array, object) |
| `description` | string | No | LLM-readable description |
| `required` | boolean | No | Default: false |
| `enum` | array | No | Allowed values |

### Request Body

```json
{
  "requestBody": {
    "content": {
      "application/json": {
        "schema": {
          "type": "object",
          "properties": {
            "from": { "type": "string" },
            "to": {
              "type": "array",
              "items": { "type": "string" }
            },
            "subject": { "type": "string" }
          },
          "required": ["from", "to", "subject"]
        },
        "example": {
          "from": "test@example.com",
          "to": ["user@example.com"],
          "subject": "Hello"
        }
      }
    },
    "required": true
  }
}
```

### Responses

```json
{
  "responses": {
    "success": {
      "type": "object",
      "properties": {
        "id": { "type": "string" },
        "status": { "type": "string" }
      }
    },
    "example": {
      "id": "email_123",
      "status": "queued"
    }
  }
}
```

---

## Complete Example: Stripe Flow

```json
{
  "agentsJson": "0.1.0",
  "info": {
    "title": "Stripe Product Flow",
    "version": "1.0.0",
    "description": "Create product with price"
  },
  "sources": [
    {
      "id": "stripe",
      "path": "https://raw.githubusercontent.com/.../stripe/openapi.yaml"
    }
  ],
  "overrides": [],
  "flows": [
    {
      "id": "create_product_with_price",
      "title": "Create Product with Price",
      "description": "Creates a product and associated price in Stripe",
      "actions": [
        {
          "id": "create_product",
          "sourceId": "stripe",
          "operationId": "stripe_post_products"
        },
        {
          "id": "create_price",
          "sourceId": "stripe",
          "operationId": "stripe_post_prices"
        }
      ],
      "links": [
        {
          "origin": {
            "actionId": null,
            "fieldPath": "parameters.name"
          },
          "target": {
            "actionId": "create_product",
            "fieldPath": "parameters.name"
          }
        },
        {
          "origin": {
            "actionId": "create_product",
            "fieldPath": "responses.success.id"
          },
          "target": {
            "actionId": "create_price",
            "fieldPath": "parameters.product"
          }
        },
        {
          "origin": {
            "actionId": null,
            "fieldPath": "parameters.unit_amount"
          },
          "target": {
            "actionId": "create_price",
            "fieldPath": "parameters.unit_amount"
          }
        }
      ],
      "fields": {
        "parameters": [
          {
            "name": "name",
            "type": "string",
            "description": "Product name",
            "required": true
          },
          {
            "name": "unit_amount",
            "type": "integer",
            "description": "Price in cents",
            "required": true
          },
          {
            "name": "currency",
            "type": "string",
            "description": "ISO currency code",
            "required": true
          }
        ],
        "responses": {
          "success": {
            "type": "object",
            "properties": {
              "product_id": { "type": "string" },
              "price_id": { "type": "string" }
            }
          }
        }
      }
    }
  ]
}
```

---

## Validation Rules

1. **IDs MUST be unique within their scope**
   - Flow IDs unique in flows array
   - Action IDs unique within flow
   - Source IDs unique in sources array

2. **References MUST resolve**
   - sourceId must match a defined source
   - operationId must exist in OpenAPI spec
   - Link actionIds must match defined actions

3. **Field paths MUST be valid**
   - Dot notation with optional array indices
   - Must start with parameters, requestBody, or responses

---

## Best Practices

### Descriptions for LLMs

```json
{
  "description": "Send an email via Resend. Use this flow when the user wants to send an email to one or more recipients. Requires sender email, recipient list, and subject."
}
```

### Parameter Naming

```json
// Good
{ "name": "recipient_email", "type": "string" }
{ "name": "product_id", "type": "string" }

// Bad
{ "name": "email", "type": "string" }  // Too generic
{ "name": "id", "type": "string" }     // Unclear what ID
```

### Link Structure

```json
// Good: Clear data flow
{
  "origin": { "actionId": "create_order", "fieldPath": "responses.success.order_id" },
  "target": { "actionId": "charge_payment", "fieldPath": "parameters.order_id" }
}
```

---

## Schema Validation

The JSON Schema is available at:
- Local: `agents_json/agentsJson.schema.json`
- Online: Documentation website

### Validation with Python

```python
import json
import jsonschema

with open('agentsJson.schema.json') as f:
    schema = json.load(f)

with open('stripe/agents.json') as f:
    data = json.load(f)

jsonschema.validate(data, schema)  # Raises if invalid
```

---

*This document is a reference guide. For examples see the `agents_json/` directory.*
