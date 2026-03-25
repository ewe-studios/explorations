# Superglue JavaScript Implementation Analysis

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.superglue/superglue-js/`

---

## Table of Contents

1. [Package Overview](#package-overview)
2. [Client Architecture](#client-architecture)
3. [Type System](#type-system)
4. [API Methods Deep Dive](#api-methods-deep-dive)
5. [GraphQL Integration](#graphql-integration)
6. [File Upload Handling](#file-upload-handling)
7. [Use Cases and Examples](#use-cases-and-examples)
8. [Browser vs Node.js Considerations](#browser-vs-nodejs-considerations)

---

## Package Overview

### Package Structure

```
superglue-js/
├── src/
│   └── superglue.ts      # Main client implementation (771 lines)
├── package.json
├── tsconfig.json
└── README.md
```

### Package Configuration

**File:** `package.json`

```json
{
  "name": "@superglue/client",
  "version": "2.3.11",
  "description": "JavaScript/TypeScript client for Superglue",
  "main": "dist/index.js",
  "types": "dist/index.d.ts",
  "scripts": {
    "build": "tsc",
    "test": "jest"
  },
  "dependencies": {
    "axios": "^1.6.0"
  },
  "devDependencies": {
    "typescript": "^5.0.0",
    "@types/node": "^20.0.0"
  }
}
```

### Key Characteristics

- **Zero external dependencies** (except axios for HTTP)
- **Isomorphic** - Works in both browser and Node.js
- **Type-safe** - Full TypeScript support
- **Promise-based** - Async/await support
- **GraphQL native** - Direct GraphQL API communication

---

## Client Architecture

### Class Structure

```typescript
export class SuperglueClient {
  private endpoint: string;
  private apiKey: string;

  // GraphQL query fragment for config data
  private static configQL = `...`;

  constructor({endpoint, apiKey}: {endpoint?: string, apiKey: string})

  // Internal request handler
  private async request<T>(query: string, variables?: Record<string, any>): Promise<T>

  // Main API methods
  async call<T>(args: ApiCallArgs): Promise<RunResult & { data: T }>
  async extract<T>(args: ExtractArgs): Promise<RunResult & { data: T }>
  async transform<T>(args: TransformArgs): Promise<RunResult & { data: T }>

  // List operations
  async listRuns(...): Promise<{ items: RunResult[], total: number }>
  async listApis(...): Promise<{ items: ApiConfig[], total: number }>
  async listTransforms(...): Promise<{ items: TransformConfig[], total: number }>
  async listExtracts(...): Promise<{ items: ExtractConfig[], total: number }>

  // Get single operations
  async getRun(id: string): Promise<RunResult>
  async getApi(id: string): Promise<ApiConfig>
  async getTransform(id: string): Promise<TransformConfig>
  async getExtract(id: string): Promise<ExtractConfig>

  // CRUD operations
  async upsertApi(id: string, input: Partial<ApiConfig>): Promise<ApiConfig>
  async deleteApi(id: string): Promise<boolean>
  async upsertExtraction(...): Promise<ExtractConfig>
  async deleteExtraction(...): Promise<boolean>
  async upsertTransformation(...): Promise<TransformConfig>
  async deleteTransformation(...): Promise<boolean>

  // Utility operations
  async updateApiConfigId(oldId: string, newId: string): Promise<ApiConfig>
  async generateSchema(instruction: string, responseData: string): Promise<any>
}
```

### Initialization Flow

```typescript
// Default endpoint is the cloud service
const superglue = new SuperglueClient({
  apiKey: "your-api-key"
});

// Or self-hosted instance
const selfHosted = new SuperglueClient({
  endpoint: "http://localhost:3000",
  apiKey: "your-auth-token"
});
```

### Request Pipeline

```typescript
private async request<T>(
  query: string,
  variables?: Record<string, any>
): Promise<T> {
  try {
    const response = await axios.post(this.endpoint, {
      query,
      variables,
    }, {
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`,
      }
    });

    // Check for GraphQL errors
    if(response.data.errors) {
      throw new Error(response.data.errors[0].message);
    }

    const json = response.data;
    return json.data as T;
  } catch (error) {
    console.error(error);
    throw error;
  }
}
```

---

## Type System

### Enum Definitions

```typescript
// HTTP Methods
export enum HttpMethod {
  GET = "GET",
  POST = "POST",
  PUT = "PUT",
  DELETE = "DELETE",
  PATCH = "PATCH",
  HEAD = "HEAD",
  OPTIONS = "OPTIONS"
}

// Cache Control
export enum CacheMode {
  ENABLED = "ENABLED",      // Read and write to cache
  READONLY = "READONLY",    // Read only
  WRITEONLY = "WRITEONLY",  // Write only
  DISABLED = "DISABLED"     // No caching
}

// File Format Types
export enum FileType {
  CSV = "CSV",
  JSON = "JSON",
  XML = "XML",
  AUTO = "AUTO"  // Auto-detect
}

// Authentication Types
export enum AuthType {
  NONE = "NONE",
  OAUTH2 = "OAUTH2",
  HEADER = "HEADER",
  QUERY_PARAM = "QUERY_PARAM"
}

// Decompression Methods
export enum DecompressionMethod {
  GZIP = "GZIP",
  DEFLATE = "DEFLATE",
  NONE = "NONE",
  AUTO = "AUTO",
  ZIP = "ZIP"
}

// Pagination Types
export enum PaginationType {
  OFFSET_BASED = "OFFSET_BASED",
  PAGE_BASED = "PAGE_BASED",
  DISABLED = "DISABLED"
}
```

### Base Types

```typescript
// Base configuration interface
export interface BaseConfig {
  id: string;
  version?: string;
  createdAt?: Date;
  updatedAt?: Date;
}

// Base result interface
export interface BaseResult {
  id: string;
  success: boolean;
  data?: any;
  error?: string;
  startedAt: Date;
  completedAt: Date;
}

// Pagination configuration
export type Pagination = {
  type: PaginationType;
  pageSize?: number;
};
```

### Configuration Types

#### ApiConfig

```typescript
export interface ApiConfig extends BaseConfig {
  urlHost: string;
  urlPath?: string;
  instruction: string;
  method?: HttpMethod;
  queryParams?: Record<string, any>;
  headers?: Record<string, any>;
  body?: string;
  documentationUrl?: string;
  responseSchema?: any;
  responseMapping?: string;
  authentication?: AuthType;
  pagination?: Pagination;
  dataPath?: string;
}
```

#### ExtractConfig

```typescript
export interface ExtractConfig extends BaseConfig {
  urlHost: string;
  urlPath?: string;
  instruction: string;
  queryParams?: Record<string, any>;
  method?: HttpMethod;
  headers?: Record<string, any>;
  body?: string;
  documentationUrl?: string;
  decompressionMethod?: DecompressionMethod;
  authentication?: AuthType;
  fileType?: FileType;
  dataPath?: string;
}
```

#### TransformConfig

```typescript
export interface TransformConfig extends BaseConfig {
  instruction: string;
  responseSchema: any;
  responseMapping?: string;
}
```

### Input Types

#### ApiInput

```typescript
export type ApiInput = {
  urlHost: string;
  urlPath?: string;
  queryParams?: Record<string, any>;
  instruction: string;
  method?: HttpMethod;
  headers?: Record<string, any>;
  body?: string;
  documentationUrl?: string;
  responseSchema?: any;
  responseMapping?: any;
  authentication?: AuthType;
  pagination?: Pagination;
  dataPath?: string;
  version?: string;
};

export type ApiInputRequest = {
  id?: string;
  endpoint: ApiInput;
};
```

#### ExtractInput

```typescript
export type ExtractInput = {
  urlHost: string;
  urlPath?: string;
  queryParams?: Record<string, any>;
  instruction: string;
  method?: HttpMethod;
  headers?: Record<string, any>;
  body?: string;
  documentationUrl?: string;
  decompressionMethod?: DecompressionMethod;
  authentication?: AuthType;
  version?: string;
};

export type ExtractInputRequest = {
  id?: string;
  endpoint: ExtractInput;
};
```

#### TransformInput

```typescript
export type TransformInput = {
  instruction: string;
  responseSchema: any;
  responseMapping?: string;
  version?: string;
};

export type TransformInputRequest = {
  id?: string;
  endpoint?: TransformInput;
};
```

### Request Options

```typescript
export type RequestOptions = {
  cacheMode?: CacheMode;
  timeout?: number;       // milliseconds
  retries?: number;
  retryDelay?: number;    // milliseconds
  webhookUrl?: string;    // Callback URL for async completion
};
```

### Method Argument Types

```typescript
// Arguments for call()
export interface ApiCallArgs {
  id?: string;                          // Use saved config by ID
  endpoint?: ApiInput;                  // Or provide config inline
  payload?: Record<string, any>;        // Variables for template substitution
  credentials?: Record<string, any>;    // Auth credentials
  options?: RequestOptions;
}

// Arguments for extract()
export interface ExtractArgs {
  id?: string;
  endpoint?: ExtractInput;
  file?: File | Blob;                   // For browser file uploads
  options?: RequestOptions;
  payload?: Record<string, any>;
  credentials?: Record<string, any>;
}

// Arguments for transform()
export interface TransformArgs {
  id?: string;
  endpoint?: TransformInput;
  data: Record<string, any>;           // Data to transform
  options?: RequestOptions;
}
```

### Result Types

```typescript
export type RunResult = BaseResult & {
  config: ApiConfig | ExtractConfig | TransformConfig;
};

export type ResultList = {
  items: RunResult[];
  total: number;
};

export type ConfigList = {
  items: ApiConfig[];
  total: number;
};
```

---

## API Methods Deep Dive

### 1. call() - Execute API Transformation

**Purpose:** Execute an API call with automatic data transformation

**Signature:**
```typescript
async call<T = unknown>({
  id,
  endpoint,
  payload,
  credentials,
  options
}: ApiCallArgs): Promise<RunResult & { data: T }>
```

**GraphQL Mutation:**
```graphql
mutation Call(
  $input: ApiInputRequest!
  $payload: JSON
  $credentials: JSON
  $options: RequestOptions
) {
  call(
    input: $input
    payload: $payload
    credentials: $credentials
    options: $options
  ) {
    id
    success
    data
    error
    startedAt
    completedAt
    config { ...ConfigFragment }
  }
}
```

**Implementation:**
```typescript
async call<T = unknown>({ id, endpoint, payload, credentials, options }: ApiCallArgs): Promise<RunResult & { data: T }> {
  const mutation = `
    mutation Call($input: ApiInputRequest!, $payload: JSON, $credentials: JSON, $options: RequestOptions) {
      call(input: $input, payload: $payload, credentials: $credentials, options: $options) {
        id
        success
        data
        error
        startedAt
        completedAt
        ${SuperglueClient.configQL}
      }
    }
  `;

  const result = await this.request<{ call: RunResult & { data: T } }>(mutation, {
    input: { id, endpoint },
    payload,
    credentials,
    options
  }).then(data => data?.call);

  if (result.error) {
    throw new Error(result.error);
  }

  return result;
}
```

**Usage Example:**
```typescript
// Using saved configuration
const result = await superglue.call({
  id: "my-saved-api",
  credentials: { api_key: process.env.API_KEY }
});

// Or inline configuration
const config = {
  urlHost: "https://api.example.com",
  urlPath: "/v1/products",
  instruction: "get all products with name and price",
  responseSchema: {
    type: "object",
    properties: {
      products: {
        type: "array",
        items: {
          type: "object",
          properties: {
            name: { type: "string" },
            price: { type: "number" }
          }
        }
      }
    }
  }
};

const result = await superglue.call({
  endpoint: config,
  payload: { category: "electronics" },
  credentials: { api_key: "secret" },
  options: {
    cacheMode: CacheMode.ENABLED,
    timeout: 30000,
    retries: 3
  }
});

console.log(result.data);
```

### 2. extract() - Extract Data from Files or APIs

**Purpose:** Extract and parse data from files or API endpoints

**Signature:**
```typescript
async extract<T = any>({
  id,
  endpoint,
  file,
  payload,
  credentials,
  options
}: ExtractArgs): Promise<RunResult & { data: T }>
```

**Implementation with File Upload Support:**
```typescript
async extract<T = any>({
  id,
  endpoint,
  file,
  payload,
  credentials,
  options
}: ExtractArgs): Promise<RunResult & { data: T }> {
  const mutation = `
    mutation Extract($input: ExtractInputRequest!, $payload: JSON, $credentials: JSON, $options: RequestOptions) {
      extract(input: $input, payload: $payload, credentials: $credentials, options: $options) {
        id
        success
        data
        error
        startedAt
        completedAt
        ${SuperglueClient.configQL}
      }
    }
  `;

  // Handle file upload via multipart/form-data
  if (file) {
    const operations = {
      query: mutation,
      variables: {
        input: { file: null },
        payload,
        credentials,
        options
      }
    };

    const formData = new FormData();
    formData.append('operations', JSON.stringify(operations));
    formData.append('map', JSON.stringify({ "0": ["variables.input.file"] }));
    formData.append('0', file);

    const response = await axios.post(this.endpoint, formData, {
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
      }
    });

    if (response.data.errors) {
      throw new Error(response.data.errors[0].message);
    }

    return response.data.data.extract;
  }

  // Standard JSON request for URL-based extraction
  return this.request<{ extract: RunResult & { data: T } }>(mutation, {
    input: { id, endpoint },
    payload,
    credentials,
    options
  }).then(data => data.extract);
}
```

**Usage Examples:**

```typescript
// Extract from URL
const result = await superglue.extract({
  endpoint: {
    urlHost: "https://example.com",
    urlPath: "/data.csv",
    instruction: "extract all rows with product data",
    fileType: FileType.CSV
  }
});

// Extract from file (browser)
const fileInput = document.querySelector('input[type="file"]');
const file = fileInput.files[0];

const result = await superglue.extract({
  endpoint: {
    instruction: "parse this CSV and extract products"
  },
  file: file
});

// Extract from compressed file
const result = await superglue.extract({
  endpoint: {
    urlHost: "https://example.com",
    urlPath: "/data.csv.gz",
    instruction: "extract product data",
    decompressionMethod: DecompressionMethod.GZIP,
    fileType: FileType.CSV
  }
});
```

### 3. transform() - Transform Existing Data

**Purpose:** Transform existing data to a new schema without fetching from external sources

**Signature:**
```typescript
async transform<T = any>({
  id,
  endpoint,
  data,
  options
}: TransformArgs): Promise<RunResult & { data: T }>
```

**Implementation:**
```typescript
async transform<T = any>({
  id,
  endpoint,
  data,
  options
}: TransformArgs): Promise<RunResult & { data: T }> {
  const mutation = `
    mutation Transform($input: TransformInputRequest!, $data: JSON!, $options: RequestOptions) {
      transform(input: $input, data: $data, options: $options) {
        id
        success
        data
        error
        startedAt
        completedAt
        ${SuperglueClient.configQL}
      }
    }
  `;

  return this.request<{ transform: RunResult & { data: T } }>(mutation, {
    input: { id, endpoint },
    data,
    options
  }).then(data => data.transform);
}
```

**Usage Example:**
```typescript
const sourceData = {
  product_name: "Widget",
  cost: 19.99,
  qty_available: 100
};

const result = await superglue.transform({
  endpoint: {
    instruction: "convert to our internal product format",
    responseSchema: {
      type: "object",
      properties: {
        name: { type: "string" },
        price: { type: "number" },
        inventory: { type: "integer" }
      }
    }
  },
  data: sourceData
});

console.log(result.data);
// Output: { name: "Widget", price: 19.99, inventory: 100 }
```

### 4. List Operations

#### listRuns()

```typescript
async listRuns(
  limit: number = 100,
  offset: number = 0,
  configId?: string
): Promise<{ items: RunResult[], total: number }> {
  const query = `
    query ListRuns($limit: Int!, $offset: Int!, $configId: ID) {
      listRuns(limit: $limit, offset: $offset, configId: $configId) {
        items {
          id
          success
          data
          error
          startedAt
          completedAt
          ${SuperglueClient.configQL}
        }
        total
      }
    }
  `;

  const response = await this.request<{
    listRuns: { items: RunResult[], total: number }
  }>(query, { limit, offset, configId });

  return response.listRuns;
}
```

#### listApis()

```typescript
async listApis(
  limit: number = 10,
  offset: number = 0
): Promise<{ items: ApiConfig[], total: number }> {
  const query = `
    query ListApis($limit: Int!, $offset: Int!) {
      listApis(limit: $limit, offset: $offset) {
        items {
          id
          version
          createdAt
          updatedAt
          urlHost
          urlPath
          instruction
          method
          queryParams
          headers
          body
          documentationUrl
          responseSchema
          responseMapping
          authentication
          pagination {
            type
            pageSize
          }
          dataPath
        }
        total
      }
    }
  `;

  const response = await this.request<{
    listApis: { items: ApiConfig[], total: number }
  }>(query, { limit, offset });

  return response.listApis;
}
```

#### listTransforms() and listExtracts()

```typescript
async listTransforms(
  limit: number = 10,
  offset: number = 0
): Promise<{ items: TransformConfig[], total: number }> {
  const query = `
    query ListTransforms($limit: Int!, $offset: Int!) {
      listTransforms(limit: $limit, offset: $offset) {
        items {
          id
          version
          createdAt
          updatedAt
          responseSchema
          responseMapping
          instruction
        }
        total
      }
    }
  `;

  const response = await this.request<{
    listTransforms: { items: TransformConfig[], total: number }
  }>(query, { limit, offset });

  return response.listTransforms;
}

async listExtracts(
  limit: number = 10,
  offset: number = 0
): Promise<{ items: ExtractConfig[], total: number }> {
  const query = `
    query ListExtracts($limit: Int!, $offset: Int!) {
      listExtracts(limit: $limit, offset: $offset) {
        items {
          id
          version
          createdAt
          updatedAt
          urlHost
          urlPath
          instruction
          queryParams
          method
          headers
          body
          documentationUrl
          decompressionMethod
          authentication
          fileType
          dataPath
        }
        total
      }
    }
  `;

  const response = await this.request<{
    listExtracts: { items: ExtractConfig[], total: number }
  }>(query, { limit, offset });

  return response.listExtracts;
}
```

### 5. Get Single Operations

```typescript
async getRun(id: string): Promise<RunResult> {
  const query = `
    query GetRun($id: ID!) {
      getRun(id: $id) {
        id
        success
        data
        error
        startedAt
        completedAt
        ${SuperglueClient.configQL}
      }
    }
  `;

  const response = await this.request<{ getRun: RunResult }>(query, { id });
  return response.getRun;
}

async getApi(id: string): Promise<ApiConfig> {
  const query = `
    query GetApi($id: ID!) {
      getApi(id: $id) {
        id
        version
        createdAt
        updatedAt
        urlHost
        urlPath
        instruction
        method
        queryParams
        headers
        body
        documentationUrl
        responseSchema
        responseMapping
        authentication
        pagination {
          type
          pageSize
        }
        dataPath
      }
    }
  `;

  const response = await this.request<{ getApi: ApiConfig }>(query, { id });
  return response.getApi;
}

// Similar pattern for getTransform() and getExtract()
```

### 6. CRUD Operations

#### Upsert Operations

```typescript
async upsertApi(id: string, input: Partial<ApiConfig>): Promise<ApiConfig> {
  const mutation = `
    mutation UpsertApi($id: ID!, $input: JSON!) {
      upsertApi(id: $id, input: $input) {
        id
        version
        createdAt
        updatedAt
        urlHost
        urlPath
        instruction
        method
        queryParams
        headers
        body
        documentationUrl
        responseSchema
        responseMapping
        authentication
        pagination {
          type
          pageSize
        }
        dataPath
      }
    }
  `;

  const response = await this.request<{ upsertApi: ApiConfig }>(mutation, { id, input });
  return response.upsertApi;
}

async upsertExtraction(id: string, input: Partial<ExtractConfig>): Promise<ExtractConfig> {
  const mutation = `
    mutation UpsertExtraction($id: ID!, $input: JSON!) {
      upsertExtraction(id: $id, input: $input) {
        id
        version
        createdAt
        updatedAt
        urlHost
        urlPath
        instruction
        queryParams
        method
        headers
        body
        documentationUrl
        decompressionMethod
        authentication
        fileType
        dataPath
      }
    }
  `;

  const response = await this.request<{ upsertExtraction: ExtractConfig }>(mutation, { id, input });
  return response.upsertExtraction;
}

async upsertTransformation(id: string, input: Partial<TransformConfig>): Promise<TransformConfig> {
  const mutation = `
    mutation UpsertTransformation($id: ID!, $input: JSON!) {
      upsertTransformation(id: $id, input: $input) {
        id
        version
        createdAt
        updatedAt
        responseSchema
        responseMapping
        instruction
      }
    }
  `;

  const response = await this.request<{ upsertTransformation: TransformConfig }>(mutation, { id, input });
  return response.upsertTransformation;
}
```

#### Delete Operations

```typescript
async deleteApi(id: string): Promise<boolean> {
  const mutation = `
    mutation DeleteApi($id: ID!) {
      deleteApi(id: $id)
    }
  `;

  const response = await this.request<{ deleteApi: boolean }>(mutation, { id });
  return response.deleteApi;
}

async deleteExtraction(id: string): Promise<boolean> {
  const mutation = `
    mutation DeleteExtraction($id: ID!) {
      deleteExtraction(id: $id)
    }
  `;

  const response = await this.request<{ deleteExtraction: boolean }>(mutation, { id });
  return response.deleteExtraction;
}

async deleteTransformation(id: string): Promise<boolean> {
  const mutation = `
    mutation DeleteTransformation($id: ID!) {
      deleteTransformation(id: $id)
    }
  `;

  const response = await this.request<{ deleteTransformation: boolean }>(mutation, { id });
  return response.deleteTransformation;
}
```

### 7. Utility Operations

#### updateApiConfigId()

```typescript
async updateApiConfigId(oldId: string, newId: string): Promise<ApiConfig> {
  const mutation = `
    mutation UpdateApiConfigId($oldId: ID!, $newId: ID!) {
      updateApiConfigId(oldId: $oldId, newId: $newId) {
        id
        version
        createdAt
        updatedAt
        urlHost
        urlPath
        instruction
        method
        queryParams
        headers
        body
        documentationUrl
        responseSchema
        responseMapping
        authentication
        pagination {
          type
          pageSize
        }
        dataPath
      }
    }
  `;

  const response = await this.request<{ updateApiConfigId: ApiConfig }>(mutation, { oldId, newId });
  return response.updateApiConfigId;
}
```

#### generateSchema()

```typescript
async generateSchema(instruction: string, responseData: string): Promise<any> {
  const query = `
    query GenerateSchema($instruction: String!, $responseData: String) {
      generateSchema(instruction: $instruction, responseData: $responseData)
    }
  `;

  const response = await this.request<{ generateSchema: string }>(
    query,
    { instruction, responseData }
  );

  return response.generateSchema;
}
```

---

## GraphQL Integration

### Config Query Fragment

The client uses a shared GraphQL fragment for all config-related queries:

```typescript
private static configQL = `
  config {
    ... on ApiConfig {
      id
      version
      createdAt
      updatedAt
      urlHost
      urlPath
      instruction
      method
      queryParams
      headers
      body
      documentationUrl
      responseSchema
      responseMapping
      authentication
      pagination {
        type
        pageSize
      }
      dataPath
    }
    ... on ExtractConfig {
      id
      version
      createdAt
      updatedAt
      urlHost
      urlPath
      instruction
      queryParams
      method
      headers
      body
      documentationUrl
      decompressionMethod
      authentication
      fileType
      dataPath
    }
    ... on TransformConfig {
      id
      version
      createdAt
      updatedAt
      responseSchema
      responseMapping
      instruction
    }
  }
`;
```

### Why GraphQL?

1. **Flexible Queries** - Request only the fields you need
2. **Type Safety** - Schema-enforced type checking
3. **Single Round-trip** - Get all related data in one request
4. **Union Types** - Handle different config types polymorphically
5. **Introspection** - Client can discover API capabilities

---

## File Upload Handling

### Browser File Upload

The `extract()` method supports file uploads using multipart/form-data:

```typescript
if (file) {
  const operations = {
    query: mutation,
    variables: {
      input: { file: null },
      payload,
      credentials,
      options
    }
  };

  const formData = new FormData();
  formData.append('operations', JSON.stringify(operations));

  // GraphQL file upload spec: https://github.com/jaydenseric/graphql-multipart-request-spec
  formData.append('map', JSON.stringify({ "0": ["variables.input.file"] }));
  formData.append('0', file);

  const response = await axios.post(this.endpoint, formData, {
    headers: {
      'Authorization': `Bearer ${this.apiKey}`,
      // Note: Content-Type is set automatically by browser with boundary
    }
  });

  return response.data.data.extract;
}
```

### Node.js File Upload

For Node.js, use the `form-data` package:

```typescript
import FormData from 'form-data';
import fs from 'fs';

const form = new FormData();
form.append('operations', JSON.stringify(operations));
form.append('map', JSON.stringify({ "0": ["variables.input.file"] }));
form.append('0', fs.createReadStream('/path/to/file.csv'));

const response = await axios.post(endpoint, form, {
  headers: {
    'Authorization': `Bearer ${apiKey}`,
    ...form.getHeaders()
  }
});
```

---

## Use Cases and Examples

### 1. E-commerce Product Sync

```typescript
import { SuperglueClient, CacheMode } from "@superglue/client";

const superglue = new SuperglueClient({ apiKey: process.env.SUPERGLUE_KEY });

// Configure Shopify product extraction
const shopifyConfig = {
  urlHost: "https://mystore.myshopify.com",
  urlPath: "/admin/api/2024-01/products.json",
  instruction: "get all products with variants",
  authentication: AuthType.HEADER,
  headers: {
    "X-Shopify-Access-Token": "{shopify_token}"
  },
  responseSchema: {
    type: "object",
    properties: {
      products: {
        type: "array",
        items: {
          type: "object",
          properties: {
            id: { type: "string" },
            title: { type: "string" },
            variants: {
              type: "array",
              items: {
                type: "object",
                properties: {
                  sku: { type: "string" },
                  price: { type: "number" },
                  inventory: { type: "integer" }
                }
              }
            }
          }
        }
      }
    }
  }
};

// First call - generates and caches configuration
const result = await superglue.call({
  endpoint: shopifyConfig,
  credentials: { shopify_token: process.env.SHOPIFY_TOKEN },
  options: { cacheMode: CacheMode.WRITEONLY }
});

// Subsequent calls - uses cached configuration
const cachedResult = await superglue.call({
  id: result.config.id,
  options: { cacheMode: CacheMode.READONLY }
});

console.log(`Synced ${cachedResult.data.products.length} products`);
```

### 2. CSV Data Import

```typescript
// Browser context
async function importCSVFile(file: File) {
  const superglue = new SuperglueClient({ apiKey: getApiKey() });

  const result = await superglue.extract({
    endpoint: {
      instruction: "Extract customer data. Map columns: Name -> name, Email -> email, Phone -> phone"
    },
    file: file,
    options: {
      timeout: 60000  // Large files may take longer
    }
  });

  return result.data;
}

// Usage in React component
function CSVImporter() {
  const [data, setData] = useState(null);

  const handleFileSelect = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      const importedData = await importCSVFile(file);
      setData(importedData);
    }
  };

  return (
    <input type="file" accept=".csv" onChange={handleFileSelect} />
  );
}
```

### 3. Multi-Step Data Pipeline

```typescript
async function runDataPipeline() {
  const superglue = new SuperglueClient({ apiKey: process.env.SUPERGLUE_KEY });

  // Step 1: Extract raw data from API
  const extracted = await superglue.extract({
    endpoint: {
      urlHost: "https://api.erp.com",
      urlPath: "/v1/inventory",
      instruction: "get current inventory levels",
      authentication: AuthType.OAUTH2
    },
    credentials: { access_token: process.env.ERP_TOKEN }
  });

  // Step 2: Transform to internal format
  const transformed = await superglue.transform({
    endpoint: {
      instruction: "convert to warehouse format",
      responseSchema: {
        type: "object",
        properties: {
          warehouse_id: { type: "string" },
          items: {
            type: "array",
            items: {
              type: "object",
              properties: {
                sku: { type: "string" },
                quantity: { type: "integer" },
                location: { type: "string" }
              }
            }
          }
        }
      }
    },
    data: extracted.data
  });

  // Step 3: Save configuration for future runs
  await superglue.upsertTransformation("warehouse-sync", {
    instruction: "convert to warehouse format",
    responseSchema: transformed.config.responseSchema,
    responseMapping: transformed.config.responseMapping
  });

  return transformed.data;
}
```

### 4. Scheduled Sync with Webhooks

```typescript
async function setupScheduledSync() {
  const superglue = new SuperglueClient({ apiKey: process.env.SUPERGLUE_KEY });

  // Run with webhook for async completion notification
  const result = await superglue.call({
    endpoint: {
      urlHost: "https://api.salesforce.com",
      urlPath: "/services/data/v58.0/sobjects/Account",
      instruction: "get all accounts created this week"
    },
    credentials: { access_token: process.env.SFDC_TOKEN },
    options: {
      webhookUrl: "https://myapp.com/webhooks/superglue-complete",
      timeout: 120000,  // 2 minute timeout
      retries: 5
    }
  });

  // Webhook will receive:
  // POST https://myapp.com/webhooks/superglue-complete
  // {
  //   "success": true,
  //   "data": { /* transformed data */ },
  //   "id": "run-uuid"
  // }
}
```

### 5. Error Handling Pattern

```typescript
async function safeCall<T>(
  superglue: SuperglueClient,
  config: ApiInput
): Promise<{ success: boolean; data?: T; error?: string }> {
  try {
    const result = await superglue.call<T>({
      endpoint: config,
      options: {
        retries: 3,
        retryDelay: 1000,
        timeout: 30000
      }
    });

    if (!result.success) {
      return {
        success: false,
        error: result.error
      };
    }

    return {
      success: true,
      data: result.data as T
    };
  } catch (error) {
    return {
      success: false,
      error: error.message
    };
  }
}

// Usage
const result = await safeCall(superglue, config);
if (!result.success) {
  console.error("Sync failed:", result.error);
  // Handle error...
} else {
  console.log("Synced", result.data);
}
```

---

## Browser vs Node.js Considerations

### Browser Environment

**Available Features:**
- `FormData` for file uploads
- Native `fetch` (axios falls back to XHR)
- Local storage for caching API keys
- Limited by CORS policies

**Considerations:**
```typescript
// Browser-specific: File upload
const fileInput = document.querySelector('input[type="file"]');
const result = await superglue.extract({
  endpoint: { instruction: "parse this file" },
  file: fileInput.files[0]  // Only available in browser
});

// Browser security: API key should come from your backend
// Don't embed API keys in frontend code!
const superglue = new SuperglueClient({
  apiKey: await getApiKeyFromBackend()  // Fetch from your auth endpoint
});
```

### Node.js Environment

**Available Features:**
- File system access
- Stream support
- No CORS restrictions
- Environment variables

**Considerations:**
```typescript
// Node.js-specific: File reading
import fs from 'fs';
import FormData from 'form-data';

// For file uploads in Node.js
const fileStream = fs.createReadStream('/path/to/file.csv');
const form = new FormData();
form.append('file', fileStream);

// Environment variables for configuration
const superglue = new SuperglueClient({
  endpoint: process.env.SUPERGLUE_ENDPOINT,
  apiKey: process.env.SUPERGLUE_API_KEY
});

// Larger timeouts for server-side processing
const result = await superglue.call({
  endpoint: config,
  options: {
    timeout: 300000,  // 5 minutes for server-side
    retries: 5
  }
});
```

### Isomorphic Pattern

```typescript
// Shared code that works in both environments
export async function syncData(config: ApiInput) {
  const superglue = new SuperglueClient({
    endpoint: getEndpoint(),
    apiKey: await getApiKey()
  });

  const result = await superglue.call({
    endpoint: config,
    options: {
      // Adaptive timeout based on environment
      timeout: isBrowser() ? 30000 : 300000,
      retries: isBrowser() ? 1 : 5
    }
  });

  return result.data;
}

function isBrowser(): boolean {
  return typeof window !== 'undefined';
}
```

---

## Performance Considerations

### Connection Pooling

Axios handles connection pooling automatically in Node.js:

```typescript
import https from 'https';

// Create custom agent for connection pooling
const agent = new https.Agent({
  keepAlive: true,
  maxSockets: 50,
  maxFreeSockets: 10
});

// Pass to axios (would need axios adapter configuration)
```

### Request Deduplication

```typescript
// Prevent duplicate in-flight requests
const pendingRequests = new Map<string, Promise<any>>();

async function deduplicatedCall(key: string, operation: () => Promise<any>) {
  if (pendingRequests.has(key)) {
    return pendingRequests.get(key);
  }

  const promise = operation().finally(() => {
    pendingRequests.delete(key);
  });

  pendingRequests.set(key, promise);
  return promise;
}
```

### Caching Strategy

```typescript
// Client-side caching of configurations
const configCache = new Map<string, ApiConfig>();

async function getCachedConfig(
  superglue: SuperglueClient,
  id: string
): Promise<ApiConfig> {
  if (configCache.has(id)) {
    return configCache.get(id);
  }

  const config = await superglue.getApi(id);
  configCache.set(id, config);
  return config;
}
```

---

**Document completed:** 2026-03-25
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.superglue/superglue-js/`
