# Deep Research Server - Deep Dive Exploration

## Overview

**Deep Research Server** is a Model Context Protocol (MCP) server for ChatGPT Deep Research, providing semantic search and document retrieval capabilities.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.VarcelLabs/deep-research-server`

---

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  ChatGPT Deep   │ →── │  MCP Server      │ →── │  OpenAI         │
│  Research       │     │  (Next.js API)   │     │  Vector Store   │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                │
                                ↓
                       ┌──────────────────┐
                       │  Document Store  │
                       │  (by ID lookup)  │
                       └──────────────────┘
```

---

## Features

| Feature | Description |
|---------|-------------|
| **Search Tool** | Semantic search using OpenAI Vector Store API |
| **Fetch Tool** | Document retrieval by ID with full content |
| **Sample Data** | 5 pre-loaded documents for testing |
| **MCP Compliant** | Follows OpenAI's MCP specification |

---

## MCP Protocol

The Model Context Protocol (MCP) allows AI assistants to access external tools and data sources.

### Server Definition

```typescript
// app/mcp/route.ts (conceptual)
import { McpServer } from 'mcp-handler';

const server = new McpServer({
  name: 'deep-research-server',
  version: '1.0.0',
});

// Register tools
server.tool('search', {
  description: 'Semantic search through the knowledge base',
  inputSchema: {
    type: 'object',
    properties: {
      query: { type: 'string' },
    },
    required: ['query'],
  },
  execute: async ({ query }) => {
    const results = await semanticSearch(query);
    return { results };
  },
});

server.tool('fetch', {
  description: 'Fetch document by ID',
  inputSchema: {
    type: 'object',
    properties: {
      id: { type: 'string' },
    },
    required: ['id'],
  },
  execute: async ({ id }) => {
    const doc = await fetchDocument(id);
    return doc;
  },
});
```

---

## OpenAI Vector Store Integration

### Search Implementation

```typescript
import OpenAI from 'openai';

const openai = new OpenAI({
  apiKey: process.env.OPENAI_API_KEY,
});

const vectorStoreId = process.env.OPENAI_VECTOR_STORE_ID;

export async function semanticSearch(query: string, limit: number = 5) {
  // Search the vector store
  const results = await openai.beta.vectorStores.files.search(vectorStoreId, {
    query,
    limit,
  });

  // Format results
  return results.data.map(file => ({
    id: file.id,
    filename: file.filename,
    // Content would be retrieved separately
  }));
}

export async function fetchDocument(fileId: string) {
  const file = await openai.beta.vectorStores.files.retrieve(
    vectorStoreId,
    fileId
  );

  // Download file content
  const content = await openai.files.content(fileId);
  return {
    id: file.id,
    filename: file.filename,
    content: await content.text(),
  };
}
```

---

## Sample Documents

The server includes 5 sample documents covering technical topics:

1. **Technical Documentation** - API reference
2. **Research Paper** - Academic content
3. **User Guide** - How-to documentation
4. **Release Notes** - Version history
5. **Architecture Overview** - System design

---

## Connecting to ChatGPT

### Setup Steps

1. **Access ChatGPT Settings**
   - Go to https://chatgpt.com/#settings

2. **Navigate to Connectors**
   - Click on the "Connectors" tab

3. **Add MCP Server**
   - Add your server URL: `http://your-domain/mcp`

4. **Test Connection**
   - The server should appear as available for deep research

---

## Project Structure

```
deep-research-server/
├── app/
│   └── mcp/
│       └── route.ts       # MCP server endpoint
├── scripts/
│   └── test-client.mjs    # Sample test client
├── package.json
└── README.md
```

---

## Test Client

```javascript
// scripts/test-client.mjs
import { MCPClient } from 'mcp-handler';

const serverUrl = process.argv[2] || 'http://localhost:3000';
const client = new MCPClient(serverUrl);

// Test search
const searchResults = await client.callTool('search', {
  query: 'API documentation',
});
console.log('Search results:', searchResults);

// Test fetch
const document = await client.callTool('fetch', {
  id: 'file-abc123',
});
console.log('Document:', document);
```

**Usage:**
```bash
node scripts/test-client.mjs https://your-server.com
```

---

## Vercel Deployment

### Requirements

- **Fluid Compute** - For efficient serverless execution
- **Environment Variables:**
  - `OPENAI_API_KEY`
  - `OPENAI_VECTOR_STORE_ID`

### Deploy

```bash
# Install Vercel CLI
npm i -g vercel

# Deploy
vercel deploy
```

Or use the one-click deploy button in the README.

---

## MCP Handler

The project uses [mcp-handler](https://www.npmjs.com/package/mcp-handler) for easy MCP server setup:

```typescript
// app/mcp/route.ts
import { createMcpHandler } from 'mcp-handler';

const tools = {
  search: {
    description: 'Search the knowledge base',
    handler: async (params) => {
      const results = await semanticSearch(params.query);
      return { content: results };
    },
  },
  fetch: {
    description: 'Fetch document by ID',
    handler: async (params) => {
      const doc = await fetchDocument(params.id);
      return { content: doc };
    },
  },
};

export const POST = createMcpHandler({ tools });
```

---

## Rust Implementation Considerations

### MCP Server

```rust
use axum::{extract::Json, routing::post, Router};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct MCPRequest {
    jsonrpc: String,
    id: Option<u32>,
    method: String,
    params: serde_json::Value,
}

#[derive(Serialize)]
struct MCPResponse {
    jsonrpc: String,
    id: Option<u32>,
    result: serde_json::Value,
}

pub struct MCPServer {
    vector_store: Arc<VectorStoreClient>,
}

impl MCPServer {
    pub fn new(vector_store_id: &str, api_key: &str) -> Self {
        Self {
            vector_store: Arc::new(VectorStoreClient::new(vector_store_id, api_key)),
        }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/mcp", post(handle_mcp_request))
            .with_state(self)
    }

    async fn handle_request(&self, request: MCPRequest) -> MCPResponse {
        let result = match request.method.as_str() {
            "search" => {
                #[derive(Deserialize)]
                struct SearchParams {
                    query: String,
                    limit: Option<usize>,
                }

                let params: SearchParams = serde_json::from_value(request.params).unwrap();
                let results = self.vector_store.search(&params.query, params.limit.unwrap_or(5)).await;
                serde_json::to_value(results).unwrap()
            }
            "fetch" => {
                #[derive(Deserialize)]
                struct FetchParams {
                    id: String,
                }

                let params: FetchParams = serde_json::from_value(request.params).unwrap();
                let doc = self.vector_store.fetch(&params.id).await;
                serde_json::to_value(doc).unwrap()
            }
            _ => {
                return MCPResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: serde_json::json!({ "error": "Unknown method" }),
                };
            }
        };

        MCPResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result,
        }
    }
}
```

### Vector Store Client

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct VectorStoreClient {
    client: Client,
    api_key: String,
    store_id: String,
}

#[derive(Deserialize)]
struct VectorSearchResponse {
    data: Vec<VectorFile>,
}

#[derive(Deserialize)]
struct VectorFile {
    id: String,
    filename: String,
}

impl VectorStoreClient {
    pub fn new(store_id: &str, api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            store_id: store_id.to_string(),
        }
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<VectorFile>> {
        let response = self.client
            .post(format!(
                "https://api.openai.com/v1/vector_stores/{}/files/search",
                self.store_id
            ))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "query": query,
                "limit": limit,
            }))
            .send()
            .await?
            .json::<VectorSearchResponse>()
            .await?;

        Ok(response.data)
    }

    pub async fn fetch(&self, file_id: &str) -> Result<String> {
        // Fetch file content from OpenAI
        let response = self.client
            .get(format!("https://api.openai.com/v1/files/{}/content", file_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?
            .text()
            .await?;

        Ok(response)
    }
}
```

---

## Key Takeaways

1. **MCP Protocol** - Standardized interface for AI tool integration
2. **Vector Store** - Semantic search via OpenAI Vector Store API
3. **Document Retrieval** - Fetch by ID with full content
4. **ChatGPT Integration** - Connect as a Deep Research connector
5. **Test Client** - Simple Node.js client for testing

---

## See Also

- [MCP Specification](https://platform.openai.com/docs/mcp)
- [OpenAI Vector Store](https://platform.openai.com/docs/assistants/tools/vector-stores)
- [Main Vercel Labs Exploration](./exploration.md)
