---
title: "Zero to AI Engineer: Wildcard-AI Fundamentals"
subtitle: "First principles: AI/ML, APIs, OpenAPI, and LLM tool calling"
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/wildcard-ai/00-zero-to-ai-engineer.md
prerequisites: None - start here if new to AI/API integrations
---

# Zero to AI Engineer

## Introduction

This document takes you from **zero AI/API integration knowledge** to understanding the fundamentals needed to work with Wildcard-AI (agents.json). We'll build up concepts from first principles.

---

## Part 1: What is an API?

### APIs in Everyday Life

An **API (Application Programming Interface)** is like a menu at a restaurant:

- **Menu (API Documentation)**: Lists what you can order
- **Kitchen (Server)**: Prepares your order
- **Waiter (API)**: Takes your request and brings back results

You don't need to know how the kitchen works - you just need to know how to read the menu and place orders.

### Technical Definition

An API allows two computer programs to communicate. For web APIs:

```
Your Program --HTTP Request--> API Server
Your Program <--HTTP Response-- API Server
```

### Example: Sending an Email via Resend API

```bash
curl https://api.resend.com/emails \
  -H "Authorization: Bearer re_123456" \
  -H "Content-Type: application/json" \
  -d '{
    "from": "test@example.com",
    "to": ["recipient@example.com"],
    "subject": "Hello",
    "html": "<p>Hello from API!</p>"
  }'
```

Response:
```json
{
  "id": "email_abc123",
  "status": "queued"
}
```

**Key Concepts:**
- **Endpoint**: The URL (`/emails`)
- **Method**: HTTP verb (POST for creating)
- **Headers**: Metadata (authentication, content type)
- **Body**: The data you're sending
- **Response**: What the API returns

---

## Part 2: What is OpenAPI?

### The Problem

Every API has different:
- Endpoint URLs
- Parameter names
- Response formats
- Authentication methods

This makes it hard for developers (and AI) to learn each API.

### The Solution: OpenAPI Specification

**OpenAPI** is a standard format for describing APIs. Think of it as a "universal menu format" that all restaurants could use.

### Example OpenAPI Snippet

```yaml
paths:
  /emails:
    post:
      operationId: resend_post_emails
      summary: Send an email
      requestBody:
        content:
          application/json:
            schema:
              type: object
              properties:
                from:
                  type: string
                  description: Sender email
                to:
                  type: array
                  items:
                    type: string
                subject:
                  type: string
              required: [from, to, subject]
      responses:
        '200':
          description: Success
          content:
            application/json:
              schema:
                type: object
                properties:
                  id:
                    type: string
```

**What this tells us:**
- There's a POST endpoint at `/emails`
- It needs `from`, `to`, and `subject` parameters
- It returns an `id` in the response
- The operation is called `resend_post_emails`

### Why OpenAPI Matters for AI

LLMs can **read OpenAPI specs** and understand how to use APIs without being specifically trained on each one. This is like teaching someone to read any menu format instead of memorizing each restaurant's menu.

---

## Part 3: LLM Tool Calling

### What is Tool Calling?

**Tool calling** (also called function calling) lets an LLM request that your code execute a function and return the result.

### Traditional LLM Interaction

```
User: "What's the weather in London?"
LLM: "I don't have access to real-time weather data."
```

### Tool-Enabled LLM Interaction

```
User: "What's the weather in London?"
LLM: [requests to call get_weather(city="London")]
Your Code: Calls weather API, returns "20°C, partly cloudy"
LLM: "The weather in London is 20°C and partly cloudy."
```

### OpenAI Tool Calling Format

You provide tool definitions:

```python
tools = [{
    "type": "function",
    "function": {
        "name": "send_email",
        "description": "Send an email",
        "parameters": {
            "type": "object",
            "properties": {
                "from": {"type": "string", "description": "Sender email"},
                "to": {"type": "array", "items": {"type": "string"}},
                "subject": {"type": "string"},
                "html": {"type": "string"}
            },
            "required": ["from", "to", "subject"]
        }
    }
}]
```

LLM responds with tool calls:

```json
{
  "choices": [{
    "message": {
      "tool_calls": [{
        "function": {
          "name": "send_email",
          "arguments": "{\"from\": \"test@example.com\", \"to\": [\"user@example.com\"], \"subject\": \"Hello\", \"html\": \"<p>Hi!</p>\"}"
        }
      }]
    }
  }]
}
```

You execute the function and return results.

---

## Part 4: Multi-Step Workflows (Flows)

### The Problem with Single API Calls

Many tasks require **multiple API calls** in sequence:

**Example: Creating a product with a price in Stripe**

1. Create Product → Get Product ID
2. Create Price with Product ID → Get Price ID
3. Create Checkout Session with Price ID → Get Payment URL

Without flows, the LLM must:
1. Call "create product" tool
2. See the result
3. Call "create price" tool with product ID
4. See the result
5. Call "create checkout session" tool

This is slow, error-prone, and requires multiple LLM calls.

### The Solution: Flows

A **flow** is a predefined sequence of API calls that executes as one tool:

```json
{
  "id": "create_product_with_price",
  "description": "Create a product with an associated price",
  "actions": [
    {"id": "create_product", "operationId": "stripe_post_products"},
    {"id": "create_price", "operationId": "stripe_post_prices"}
  ],
  "links": [
    {
      "origin": {"actionId": "create_product", "fieldPath": "responses.success.id"},
      "target": {"actionId": "create_price", "fieldPath": "parameters.product"}
    }
  ]
}
```

Now the LLM makes **one tool call** and the entire workflow executes.

---

## Part 5: Data Linking

### What are Links?

**Links** define how data flows from one action to another in a flow.

### Link Structure

```json
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
```

**Translation:** "Take the `id` from the create_product response and use it as the `product` parameter for create_price."

### Field Path Syntax

Field paths use **dot notation** to navigate nested structures:

- `parameters.email` - The email parameter
- `responses.success.id` - The id in the success response
- `responses.success.data.items[0].name` - The name of the first item

**Array indexing:**
- `items[0]` - First item
- `items[0].price` - Price of the first item

### Link Resolution Process

```
1. Execute action 1 (create_product)
   Response: {"id": "prod_abc123", ...}

2. Resolve link origin
   FieldPath: "responses.success.id"
   Value: "prod_abc123"

3. Apply to link target
   FieldPath: "parameters.product"
   Action 2 parameters: {"product": "prod_abc123", ...}

4. Execute action 2 (create_price)
```

---

## Part 6: Authentication Patterns

### Why Authentication Matters

APIs need to know:
- **Who** is making the request
- **What** they're allowed to do

### Authentication Types

#### 1. API Key

A secret key passed with each request:

```bash
curl https://api.resend.com/emails \
  -H "X-API-Key: re_123456"
```

Or in URL parameters:
```bash
curl https://api.example.com/data?api_key=abc123
```

#### 2. Bearer Token

Typically used with OAuth or JWT:

```bash
curl https://api.example.com/user \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIs..."
```

#### 3. Basic Auth

Username and password (base64 encoded):

```bash
curl https://api.example.com/data \
  -H "Authorization: Basic dXNlcm5hbWU6cGFzc3dvcmQ="
```

#### 4. OAuth 1.0

Complex signature-based auth (used by Twitter):

```
Authorization: OAuth
  oauth_consumer_key="...",
  oauth_token="...",
  oauth_signature_method="HMAC-SHA1",
  oauth_signature="...",
  ...
```

#### 5. OAuth 2.0

Token-based with refresh capability:

```
1. User authorizes app
2. App receives access_token + refresh_token
3. App uses access_token until it expires
4. App uses refresh_token to get new access_token
```

### agents.json Auth Configuration

```python
# API Key
auth = ApiKeyAuthConfig(
    type=AuthType.API_KEY,
    key_value="re_123456"
)

# Bearer Token
auth = BearerAuthConfig(
    type=AuthType.BEARER,
    token="eyJhbGciOiJIUzI1NiIs..."
)

# Basic Auth
auth = BasicAuthConfig(
    type=AuthType.BASIC,
    credentials=("username", "password")
)

# OAuth 2.0
auth = OAuth2AuthConfig(
    type=AuthType.OAUTH2,
    token="access_token",
    refresh_token="refresh_token",
    scopes=["read", "write"]
)
```

---

## Part 7: Putting It All Together

### Complete Example: Sending Email with agents.json

#### Step 1: Load the agents.json Bundle

```python
from agentsjson import load_agents_json

bundle = load_agents_json(
    "https://raw.githubusercontent.com/wild-card-ai/agents-json/master/agents_json/resend/agents.json"
)
```

This downloads:
- The agents.json file (flows, links, fields)
- The OpenAPI spec (API endpoint definitions)
- Creates an index of all operations

#### Step 2: Get Tool Definitions for the LLM

```python
from agentsjson import get_tools, ToolFormat

tools = get_tools(bundle.agentsJson, ToolFormat.OPENAI)
```

This converts all flows into OpenAI tool format that the LLM understands.

#### Step 3: Call the LLM with Tools

```python
from openai import OpenAI

client = OpenAI(api_key="sk-...")

response = client.chat.completions.create(
    model="gpt-4o",
    messages=[
        {"role": "user", "content": "Send an email to test@example.com"}
    ],
    tools=tools
)
```

The LLM will respond with a tool call like:
```json
{
  "tool_calls": [{
    "function": {
      "name": "resend_post_emails_flow",
      "arguments": "{\"from\": \"noreply@example.com\", \"to\": [\"test@example.com\"], \"subject\": \"Hello\", \"html\": \"<p>Hi!</p>\"}"
    }
  }]
}
```

#### Step 4: Execute the Flow

```python
from agentsjson import execute
from agentsjson.core.models.auth import ApiKeyAuthConfig, AuthType

auth = ApiKeyAuthConfig(type=AuthType.API_KEY, key_value="re_...")
result = execute(bundle.agentsJson, response, ToolFormat.OPENAI, auth)
print(result)
# Output: {"resend_post_emails_flow": {"id": "email_abc123"}}
```

### What Happens During Execution?

```
1. Parse LLM's tool call
   - Flow ID: resend_post_emails_flow
   - Arguments: {from, to, subject, html}

2. Find the flow definition
   - Actions: [send_email_action]
   - Links: [requestBody -> requestBody]

3. Execute each action:
   a. Resolve link parameters
      - Copy arguments to action parameters

   b. Call the API via integration
      - Look up operation in map.py
      - Execute with auth: stripe.post_emails(auth, **params)

   c. Store response in execution trace

4. Return aggregated results
```

---

## Part 8: Core Concepts Summary

### agents.json Specification

| Concept | Description | Example |
|---------|-------------|---------|
| **Source** | An OpenAPI API definition | Stripe, Resend, Twitter |
| **Flow** | A sequence of actions to execute | "Send email", "Create product with price" |
| **Action** | A single API operation | `stripe_post_products` |
| **Link** | Data mapping between actions | product ID → price creation |
| **Override** | Customization of OpenAPI defaults | Making a field required |
| **Field** | Interface definition | Parameters, requestBody, responses |

### Execution Flow

```mermaid
flowchart LR
    A[User Query] --> B[LLM with Tools]
    B --> C[Tool Call]
    C --> D[Parse Flow ID]
    D --> E[Load Flow]
    E --> F[For Each Action]
    F --> G[Resolve Links]
    G --> H[Execute API]
    H --> I[Store Response]
    I --> F
    F --> J[Return Results]
```

### File Organization

```
agents-json/
├── agents_json/           # agents.json files + OpenAPI specs
│   ├── stripe/
│   │   ├── agents.json    # Flows for Stripe
│   │   └── openapi.yaml   # Stripe OpenAPI spec
│   └── resend/
│       └── agents.json    # Flows for Resend
│
├── python/agentsjson/     # Python core library
│   ├── core/
│   │   ├── executor.py    # Flow execution
│   │   ├── loader.py      # Bundle loading
│   │   └── parsetools.py  # Tool conversion
│   └── integrations/      # API-specific code
│       ├── stripe/
│       │   ├── map.py     # operationId → function
│       │   └── tools.py   # SDK wrappers
│       └── resend/
│           └── ...
```

---

## Part 9: Exercises

### Exercise 1: Read an agents.json File

1. Open `/path/to/agents_json/resend/agents.json`
2. Identify:
   - How many flows are defined?
   - What are the flow IDs?
   - What parameters does each flow require?

### Exercise 2: Trace a Link

For the `manage_products_prices` flow in Stripe:

1. Find the link from `create_product` to `create_price`
2. What field is extracted from the response?
3. What field is set in the next action's parameters?

### Exercise 3: Create a Simple Flow

Create an agents.json file for a hypothetical API:

```json
{
  "agentsJson": "0.1.0",
  "info": {
    "title": "Weather API",
    "version": "1.0.0",
    "description": "Get weather data"
  },
  "sources": [
    {
      "id": "weather",
      "path": "https://api.weather.com/openapi.yaml"
    }
  ],
  "flows": [
    {
      "id": "get_weather",
      "title": "Get Weather",
      "description": "Get current weather for a city",
      "actions": [
        {
          "id": "fetch_weather",
          "sourceId": "weather",
          "operationId": "weather_get_current"
        }
      ],
      "links": [],
      "fields": {
        "parameters": [
          {
            "name": "city",
            "type": "string",
            "description": "City name",
            "required": true
          }
        ],
        "responses": {
          "success": {
            "type": "object",
            "description": "Weather data"
          }
        }
      }
    }
  ]
}
```

---

## Part 10: Next Steps

You now understand the fundamentals:

1. **APIs** - How programs communicate
2. **OpenAPI** - Standard API descriptions
3. **LLM Tool Calling** - How LLMs request function execution
4. **Flows** - Multi-step API workflows
5. **Links** - Data mapping between steps
6. **Authentication** - API security patterns

### Continue Your Journey

Next, read these documents in order:

1. **[01-agents-json-specification-deep-dive.md](01-agents-json-specification-deep-dive.md)** - Complete schema reference
2. **[02-bundle-loader-deep-dive.md](02-bundle-loader-deep-dive.md)** - How bundles are loaded
3. **[03-flow-execution-deep-dive.md](03-flow-execution-deep-dive.md)** - Flow execution internals
4. **[04-tool-format-conversion-deep-dive.md](04-tool-format-conversion-deep-dive.md)** - Tool format conversion

### Hands-On Practice

1. Clone the repository
2. Run the example notebooks in `examples/`
3. Create a custom agents.json for an API you use
4. Add a new integration package

---

## Quick Reference: Key Terms

| Term | Definition |
|------|------------|
| **API** | Application Programming Interface - how programs communicate |
| **OpenAPI** | Standard format for describing REST APIs |
| **Operation ID** | Unique identifier for an API operation |
| **Flow** | Predefined sequence of API actions |
| **Action** | Single API operation within a flow |
| **Link** | Data mapping from one action to another |
| **Field Path** | Dot notation path to a value (e.g., `responses.success.id`) |
| **Bundle** | Loaded agents.json + OpenAPI spec + operation index |
| **Tool** | Function definition for LLM consumption |
| **Executor** | Code that actually calls the API (SDK or HTTP) |

---

*This document is a living guide. Revisit sections as concepts become clearer through implementation.*
