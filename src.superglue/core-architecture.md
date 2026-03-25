# Superglue Core Architecture Deep Dive

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.superglue/superglue/`

---

## Table of Contents

1. [System Architecture](#system-architecture)
2. [GraphQL API Layer](#graphql-api-layer)
3. [Data Transformation Engine](#data-transformation-engine)
4. [Extract Pipeline](#extract-pipeline)
5. [File Processing System](#file-processing-system)
6. [Caching Architecture](#caching-architecture)
7. [Authentication System](#authentication-system)
8. [Error Handling & Retry Logic](#error-handling--retry-logic)

---

## System Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         CLIENT LAYER                                 │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
│  │  JavaScript SDK │  │   GraphQL CLI   │  │  External Apps  │     │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘     │
└───────────┼────────────────────┼────────────────────┼───────────────┘
            │                    │                    │
            └────────────────────┼────────────────────┘
                                 │
            ┌────────────────────▼────────────────────┐
            │         GRAPHQL API LAYER                │
            │         (Apollo Server)                  │
            │  Port: 3000 (GraphQL)                    │
            │  Port: 3001 (Dashboard)                  │
            └────────────────────┬────────────────────┘
                                 │
            ┌────────────────────▼────────────────────┐
            │           CORE ENGINE                    │
            │  ┌─────────────────────────────────┐    │
            │  │  Resolver Layer                  │    │
            │  │  - callResolver                  │    │
            │  │  - extractResolver               │    │
            │  │  - transformResolver             │    │
            │  └─────────────────────────────────┘    │
            │  ┌─────────────────────────────────┐    │
            │  │  Utils Layer                     │    │
            │  │  - api.ts (HTTP handling)        │    │
            │  │  - extract.ts (Data extraction)  │    │
            │  │  - transform.ts (LLM engine)     │    │
            │  │  - file.ts (File parsing)        │    │
            │  │  - schema.ts (Schema gen)        │    │
            │  │  - tools.ts (JSONata, utils)     │    │
            │  └─────────────────────────────────┘    │
            └────────────────────┬────────────────────┘
                                 │
            ┌────────────────────▼────────────────────┐
            │          DATASTORE LAYER                 │
            │  ┌─────────────┐  ┌─────────────────┐   │
            │  │   Redis     │  │   In-Memory     │   │
            │  │   (Prod)    │  │   (Dev/Test)    │   │
            │  └─────────────┘  └─────────────────┘   │
            └─────────────────────────────────────────┘
                                 │
            ┌────────────────────▼────────────────────┐
            │        EXTERNAL DATA SOURCES             │
            │  REST APIs │ GraphQL │ Files │ Legacy   │
            └─────────────────────────────────────────┘
```

### Request Flow

```
1. Client sends GraphQL mutation (call/extract/transform)
                    │
                    ▼
2. Apollo Server receives request
   - Authentication middleware validates token
   - Telemetry middleware captures request
                    │
                    ▼
3. Resolver executes (callResolver/extractResolver/transformResolver)
   - Checks cache for existing configuration
   - If not found, generates new configuration via LLM
                    │
                    ▼
4. Extract Pipeline processes
   - Makes HTTP request or parses file
   - Handles pagination, decompression
                    │
                    ▼
5. Transform Engine processes
   - Generates JSONata expression (if not cached)
   - Executes transformation
   - Validates against schema
                    │
                    ▼
6. Result returned to client
   - Configuration saved to cache
   - Run result logged
   - Webhook notification (if configured)
```

---

## GraphQL API Layer

### Server Configuration

**File:** `packages/core/index.ts`

```typescript
import { ApolloServer } from '@apollo/server';
import { expressMiddleware } from '@apollo/server/express4';
import cors from 'cors';
import express from 'express';
import http from 'http';

const PORT = process.env.GRAPHQL_PORT || 3000;

const apolloConfig = {
  typeDefs,              // Loaded from api.graphql
  resolvers,             // Imported from graphql/graphql.ts
  introspection: true,
  csrfPrevention: false,
  bodyParserOptions: {
    limit: "1024mb",     // Support large payloads
    type: "application/json"
  },
  plugins: [
    // Landing page plugin
    ApolloServerPluginLandingPageLocalDefault({
      footer: false,
      embed: true,
      document: DEFAULT_QUERY
    }),
    // Telemetry plugin
    {
      requestDidStart: async () => ({
        willSendResponse: async (requestContext) => {
          const errors = requestContext.errors ||
            requestContext?.response?.body?.singleResult?.errors;

          if (errors && errors.length > 0 && telemetryClient) {
            const orgId = requestContext.contextValue.orgId;
            handleQueryError(errors, requestContext.request.query, orgId);
          }
        }
      })
    }
  ],
};
```

### Authentication Middleware

```typescript
const authMiddleware = async (req, res, next) => {
  if(req.path === '/health') {
    return res.status(200).send('OK');
  }

  const token = req.headers?.authorization?.split(" ")?.[1]?.trim() ||
                req.query.token;

  if(!token) {
    return res.status(401).send(getAuthErrorHTML(token));
  }

  const authResult = await authManager.authenticate(token);

  if (!authResult.success) {
    return res.status(401).send(getAuthErrorHTML(token));
  }

  req.orgId = authResult.orgId;
  req.headers["orgId"] = authResult.orgId;
  return next();
};
```

### GraphQL Schema

**File:** `api.graphql`

```graphql
scalar JSONSchema
scalar JSON
scalar JSONata
scalar DateTime
scalar Upload

interface BaseConfig {
  id: ID!
  version: String
  createdAt: DateTime
  updatedAt: DateTime
}

union ConfigType = ApiConfig | ExtractConfig | TransformConfig

type ApiConfig implements BaseConfig {
  id: ID!
  urlHost: String!
  urlPath: String
  instruction: String!
  method: HttpMethod
  queryParams: JSON
  headers: JSON
  body: String
  documentationUrl: String
  responseSchema: JSONSchema
  responseMapping: JSONata
  authentication: AuthType
  pagination: Pagination
  dataPath: String
}

type ExtractConfig implements BaseConfig {
  id: ID!
  urlHost: String!
  urlPath: String
  instruction: String!
  method: HttpMethod
  queryParams: JSON
  headers: JSON
  body: String
  documentationUrl: String
  decompressionMethod: DecompressionMethod
  authentication: AuthType
  fileType: FileType
  dataPath: String
}

type TransformConfig implements BaseConfig {
  id: ID!
  instruction: String!
  responseSchema: JSONSchema
  responseMapping: JSONata
  confidence: Float
  confidence_reasoning: String
}

type RunResult {
  id: ID!
  success: Boolean!
  data: JSON
  error: String
  startedAt: DateTime!
  completedAt: DateTime!
  config: ConfigType
}

enum AuthType {
  NONE, HEADER, QUERY_PARAM, OAUTH2
}

enum HttpMethod {
  GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS
}

enum FileType {
  CSV, JSON, XML, AUTO
}

enum DecompressionMethod {
  GZIP, DEFLATE, NONE, AUTO, ZIP
}

enum CacheMode {
  ENABLED, DISABLED, READONLY, WRITEONLY
}

enum PaginationType {
  OFFSET_BASED, PAGE_BASED, DISABLED
}

input RequestOptions {
  cacheMode: CacheMode
  timeout: Int
  retries: Int
  retryDelay: Int
  webhookUrl: String
}

input ApiInputRequest @oneOf {
  endpoint: ApiInput
  id: ID
}

type Query {
  listRuns(limit: Int = 10, offset: Int = 0, configId: ID): RunList!
  getRun(id: ID!): RunResult
  listApis(limit: Int = 10, offset: Int = 0): ApiList!
  getApi(id: ID!): ApiConfig
  listTransforms(limit: Int = 10, offset: Int = 0): TransformList!
  getTransform(id: ID!): TransformConfig
  listExtracts(limit: Int = 10, offset: Int = 0): ExtractList!
  getExtract(id: ID!): ExtractConfig
  generateSchema(instruction: String!, responseData: String): JSONSchema!
  getTenantInfo: TenantInfo
}

type Mutation {
  setTenantInfo(email: String, emailEntrySkipped: Boolean): TenantInfo!

  call(
    input: ApiInputRequest!
    payload: JSON
    credentials: JSON
    options: RequestOptions
  ): RunResult!

  extract(
    input: ExtractInputRequest!
    payload: JSON
    credentials: JSON
    options: RequestOptions
  ): RunResult!

  transform(
    input: TransformInputRequest!
    data: JSON!
    options: RequestOptions
  ): RunResult!

  upsertApi(id: ID!, input: JSON!): ApiConfig!
  deleteApi(id: ID!): Boolean!
  updateApiConfigId(oldId: ID!, newId: ID!): ApiConfig!

  upsertExtraction(id: ID!, input: JSON!): ExtractConfig!
  deleteExtraction(id: ID!): Boolean!

  upsertTransformation(id: ID!, input: JSON!): TransformConfig!
  deleteTransformation(id: ID!): Boolean!
}
```

### Resolver Architecture

**File:** `packages/core/graphql/graphql.ts`

```typescript
export const resolvers = {
  Query: {
    listRuns: listRunsResolver,
    getRun: getRunResolver,
    listApis: listApisResolver,
    getApi: getApiResolver,
    listTransforms: listTransformsResolver,
    getTransform: getTransformResolver,
    listExtracts: listExtractsResolver,
    getExtract: getExtractResolver,
    generateSchema: generateSchemaResolver,
    getTenantInfo: getTenantInfoResolver
  },
  Mutation: {
    setTenantInfo: setTenantInfoResolver,
    call: callResolver,
    extract: extractResolver,
    transform: transformResolver,
    upsertApi: upsertApiResolver,
    deleteApi: deleteApiResolver,
    updateApiConfigId: updateApiConfigIdResolver,
    upsertExtraction: upsertExtractResolver,
    deleteExtraction: deleteExtractResolver,
    upsertTransformation: upsertTransformResolver,
    deleteTransformation: deleteTransformResolver,
  },
  JSON: JSONResolver,
  JSONSchema: JSONSchemaResolver,
  JSONata: JSONataResolver,
  Upload: GraphQLUpload,
  ConfigType: {
    __resolveType(obj: any, context: any, info: any) {
      const parentField = info.path.prev.key;

      switch (parentField) {
        case 'call':
          return 'ApiConfig';
        case 'extract':
          return 'ExtractConfig';
        case 'transform':
          return 'TransformConfig';
        default:
          return 'ApiConfig';
      }
    }
  }
};
```

---

## Data Transformation Engine

### LLM-Powered Mapping Generation

**File:** `packages/core/utils/transform.ts`

The core innovation of Superglue is using LLMs to automatically generate data transformation expressions.

#### Flow Diagram

```
┌────────────────────────────────────────────────────────────────┐
│                   prepareTransform()                            │
│                                                                  │
│  1. Validate inputs                                              │
│     - Check responseSchema exists                               │
│     - Check data is not empty                                   │
│                                                                  │
│  2. Check cache (if fromCache enabled)                          │
│     - Generate hash from request + data schema                  │
│     - Look up in datastore                                      │
│     - Return cached config if found                             │
│                                                                  │
│  3. Generate mapping (if not cached)                            │
│     - Call generateMapping() with LLM                           │
│     - Retry up to 5 times on failure                            │
│                                                                  │
│  4. Return TransformConfig                                       │
└────────────────────────────────────────────────────────────────┘
```

#### Implementation

```typescript
export async function prepareTransform(
  datastore: DataStore,
  fromCache: boolean,
  input: TransformInput,
  data: any,
  orgId?: string
): Promise<TransformConfig | null> {

  // 1. Validate inputs
  if(!input?.responseSchema || Object.keys(input.responseSchema).length === 0) {
    return null;
  }

  if(!data || (Array.isArray(data) && data.length === 0)) {
    return null;
  }

  // 2. Check cache
  if(fromCache) {
    const cached = await datastore.getTransformConfigFromRequest(
      input as TransformInput,
      data,
      orgId
    );
    if (cached) return { ...cached, ...input };
  }

  // 3. Generate hash for config ID
  const hash = createHash('md5')
    .update(JSON.stringify({
      request: input,
      payloadKeys: getSchemaFromData(data)
    }))
    .digest('hex');

  // 4. Return early if responseMapping already provided
  if(input.responseMapping) {
    return {
      id: hash,
      createdAt: new Date(),
      updatedAt: new Date(),
      responseMapping: input.responseMapping,
      responseSchema: input.responseSchema,
      ...input
    };
  }

  // 5. Generate mapping via LLM
  const mapping = await generateMapping(
    input.responseSchema,
    data,
    input.instruction
  );

  if(mapping) {
    return {
      id: hash,
      createdAt: new Date(),
      updatedAt: new Date(),
      ...input,
      responseSchema: input.responseSchema,
      responseMapping: mapping.jsonata,
      confidence: mapping.confidence,
      confidence_reasoning: mapping.confidence_reasoning
    };
  }

  return null;
}
```

### Mapping Generation with OpenAI

```typescript
const jsonataSchema = {
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "JSONata Expression Schema",
  "type": "object",
  "properties": {
    "jsonata": {
      "type": "string",
      "description": "JSONata expression"
    },
    "confidence": {
      "type": "number",
      "description": "Confidence score 0-100"
    },
    "confidence_reasoning": {
      "type": "string",
      "description": "Reasoning for confidence score"
    }
  },
  "required": ["jsonata", "confidence", "confidence_reasoning"],
  "additionalProperties": false
};

export async function generateMapping(
  schema: any,
  payload: any,
  instruction?: string,
  retry = 0,
  messages?: ChatCompletionMessageParam[]
): Promise<{jsonata: string, confidence: number, confidence_reasoning: string} | null> {

  try {
    const openai = new OpenAI({
      apiKey: process.env.OPENAI_API_KEY,
      baseURL: process.env.OPENAI_API_BASE_URL,
    });

    // Build prompt
    const userPrompt = `
Given a source data and structure, create a jsonata expression in JSON FORMAT.

Target Schema:
${JSON.stringify(schema, null, 2)}

${instruction ? `Instruction: ${instruction}` : ''}

Source Data Structure:
${JSON.stringify(toJsonSchema(payload, {required: true, arrays: {mode: 'first'}}), null, 2)}

Source Data Sample:
${JSON.stringify(sample(payload, 2), null, 2).slice(0, 30000)}
`;

    // Initialize messages if first call
    if(!messages) {
      messages = [
        {role: "system", content: PROMPT_MAPPING},
        {role: "user", content: userPrompt}
      ];
    }

    // Temperature increases with retries to explore different solutions
    const temperature = String(process.env.OPENAI_MODEL).startsWith("o")
      ? undefined
      : Math.min(retry * 0.1, 1);

    const completion = await openai.chat.completions.create({
      model: process.env.OPENAI_MODEL,
      temperature,
      messages,
      response_format: {
        type: "json_schema",
        json_schema: {
          name: "required_format",
          schema: jsonataSchema,
        }
      },
    });

    const assistantResponse = String(completion.choices[0].message.content);
    messages.push({role: "assistant", content: assistantResponse});

    const content = JSON.parse(assistantResponse);
    console.log("generated mapping", content?.jsonata);

    // Validate the generated expression
    const transformation = await applyJsonataWithValidation(
      payload,
      content.jsonata,
      schema
    );

    if(!transformation.success) {
      console.log("validation failed", String(transformation?.error).substring(0, 100));
      throw new Error(`Validation failed: ${transformation.error}`);
    }

    console.log("validation succeeded");
    return content;

  } catch (error) {
    // Retry up to 5 times with error feedback
    if(retry < 5) {
      messages.push({role: "user", content: error.message});
      return generateMapping(schema, payload, instruction, retry + 1, messages);
    }
    console.error('Error generating mapping:', String(error));
  }

  return null;
}
```

### Data Sampling

For large payloads, only a sample is sent to the LLM:

```typescript
export function sample(value: any, sampleSize = 10): any {
  if (Array.isArray(value)) {
    const arrLength = value.length;
    if (arrLength <= sampleSize) {
      return value.map(item => sample(item, sampleSize));
    }
    // Stratified sampling
    const step = Math.floor(arrLength / sampleSize);
    return Array.from({ length: sampleSize }, (_, i) =>
      sample(value[i * step], sampleSize)
    );
  }

  if (value && typeof value === 'object') {
    return Object.entries(value).reduce((acc, [key, val]) => ({
      ...acc,
      [key]: sample(val, sampleSize)
    }), {});
  }

  return value;
}
```

### JSONata Execution Engine

**File:** `packages/core/utils/tools.ts`

```typescript
import jsonata from "jsonata";

export async function applyJsonata(data: any, expr: string): Promise<any> {
  try {
    const expression = superglueJsonata(expr);
    const result = await expression.evaluate(data);
    return result;
  } catch (error) {
    throw new Error(`Mapping transformation failed: ${error.message}`);
  }
}

export function superglueJsonata(expr: string) {
  const expression = jsonata(expr);

  // Register custom functions
  expression.registerFunction("max", (arr: any[]) => {
    if(Array.isArray(arr)) {
      return Math.max(...arr);
    }
    return arr;
  });

  expression.registerFunction("min", (arr: any[]) => {
    if(Array.isArray(arr)) {
      return Math.min(...arr);
    }
    return arr;
  });

  expression.registerFunction("number", (value: string) =>
    parseFloat(value)
  );

  expression.registerFunction("substring", (str: string, start: number, end?: number) =>
    String(str).substring(start, end)
  );

  expression.registerFunction("replace", (obj: any, pattern: string, replacement: string) => {
    if(Array.isArray(obj)) {
      return obj.map(item => String(item).replace(pattern, replacement));
    }
    if(typeof obj === "object") {
      return Object.fromEntries(
        Object.entries(obj).map(([key, value]) =>
          [key, String(value).replace(pattern, replacement)]
        )
      );
    }
    return String(obj).replace(pattern, replacement);
  });

  expression.registerFunction("toDate", (date: string | number) => {
    try {
      // Handle numeric timestamps
      if (typeof date === 'number' || /^\d+$/.test(date)) {
        const timestamp = typeof date === 'number' ? date : parseInt(date, 10);
        const millisTimestamp = timestamp < 10000000000
          ? timestamp * 1000  // Seconds to milliseconds
          : timestamp;
        return new Date(millisTimestamp).toISOString();
      }

      // Handle MM/DD/YYYY format
      const match = String(date).match(
        /^(\d{2})\/(\d{2})\/(\d{4})(?:\s+(\d{2}):(\d{2}):(\d{2}))?$/
      );
      if (match) {
        const [_, month, day, year, hours="00", minutes="00", seconds="00"] = match;
        const isoDate = `${year}-${month}-${day}T${hours}:${minutes}:${seconds}.000Z`;
        return new Date(isoDate).toISOString();
      }

      return new Date(date).toISOString();
    } catch (e) {
      throw new Error(`Invalid date: ${e.message}`);
    }
  });

  expression.registerFunction("dateMax", (dates: string[]) =>
    dates.reduce((max, curr) => new Date(max) > new Date(curr) ? max : curr)
  );

  expression.registerFunction("dateMin", (dates: string[]) =>
    dates.reduce((min, curr) => new Date(min) < new Date(curr) ? min : curr)
  );

  expression.registerFunction("dateDiff", (
    date1: string,
    date2: string,
    unit: string = 'days'
  ) => {
    const d1 = new Date(date1);
    const d2 = new Date(date2);
    const diff = Math.abs(d1.getTime() - d2.getTime());

    switch(unit.toLowerCase()) {
      case 'seconds': return Math.floor(diff / 1000);
      case 'minutes': return Math.floor(diff / (1000 * 60));
      case 'hours': return Math.floor(diff / (1000 * 60 * 60));
      case 'days': return Math.floor(diff / (1000 * 60 * 60 * 24));
      default: return diff;
    }
  });

  return expression;
}
```

### Schema Validation

```typescript
import { Validator } from "jsonschema";

export async function applyJsonataWithValidation(
  data: any,
  expr: string,
  schema: any
): Promise<TransformResult> {
  try {
    const result = await applyJsonata(data, expr);

    // Check for empty results
    if(result === null || result === undefined ||
       result?.length === 0 || Object.keys(result).length === 0) {
      return { success: false, error: "Result is empty" };
    }

    // Validate against schema
    const validator = new Validator();
    const optionalSchema = addNullableToOptional(schema);
    const validation = validator.validate(result, optionalSchema);

    if (!validation.valid) {
      return {
        success: false,
        error: validation.errors
          .map(e => `${e.stack}. Source: ${JSON.stringify(e.instance)}`)
          .join('\n')
          .slice(0, 5000)
      };
    }

    return { success: true, data: result };
  } catch (error) {
    return { success: false, error: error.message };
  }
}

function addNullableToOptional(schema: any): any {
  if (!schema || typeof schema !== 'object') return schema;

  const newSchema = { ...schema };

  if (schema.type === 'object' && schema.properties) {
    const required = new Set(schema.required || []);
    newSchema.properties = Object.entries(schema.properties).reduce(
      (acc, [key, value]) => ({
        ...acc,
        [key]: !required.has(key)
          ? makeNullable(value)
          : addNullableToOptional(value)
      }),
      {}
    );
  }

  if (schema.type === 'array' && schema.items) {
    newSchema.items = addNullableToOptional(schema.items);
  }

  return newSchema;
}

function makeNullable(schema: any): any {
  if (!schema || typeof schema !== 'object') return schema;

  const newSchema = { ...schema };

  if (Array.isArray(schema.type)) {
    if (!schema.type.includes('null')) {
      newSchema.type = [...schema.type, 'null'];
    }
  } else if (schema.type) {
    newSchema.type = [schema.type, 'null'];
  }

  if (schema.properties) {
    newSchema.properties = Object.entries(schema.properties).reduce(
      (acc, [key, value]) => ({
        ...acc,
        [key]: makeNullable(value)
      }),
      {}
    );
  }

  if (schema.items) {
    newSchema.items = makeNullable(schema.items);
  }

  return newSchema;
}
```

---

## Extract Pipeline

### API Configuration Generation

**File:** `packages/core/utils/api.ts`

```typescript
export async function prepareEndpoint(
  endpointInput: ApiInput,
  payload: any,
  credentials: any,
  lastError: string | null = null,
  previousMessages: OpenAI.Chat.ChatCompletionMessageParam[] = []
): Promise<{ config: ApiConfig; messages: OpenAI.Chat.ChatCompletionMessageParam[] }> {

  const currentTime = new Date();

  let apiCallConfig: Partial<ApiConfig> = {
    ...endpointInput,
    createdAt: currentTime,
    updatedAt: currentTime,
    id: crypto.randomUUID()
  };

  // Fetch documentation if available
  const documentation = await getDocumentation(
    apiCallConfig.documentationUrl ||
    composeUrl(apiCallConfig.urlHost, apiCallConfig.urlPath),
    apiCallConfig.headers,
    apiCallConfig.queryParams,
    apiCallConfig?.urlPath
  );

  const availableVars = [
    ...Object.keys(payload || {}),
    ...Object.keys(credentials || {})
  ];

  const computedApiCallConfig = await generateApiConfig(
    apiCallConfig,
    documentation,
    availableVars,
    lastError,
    previousMessages
  );

  return computedApiCallConfig;
}
```

### HTTP Request Execution

```typescript
export async function callEndpoint(
  endpoint: ApiConfig,
  payload: Record<string, any>,
  credentials: Record<string, any>,
  options: RequestOptions
): Promise<any> {
  const allVariables = { ...payload, ...credentials };

  let allResults = [];
  let page = 1;
  let offset = 0;
  let hasMore = true;
  let loopCounter = 0;

  // Pagination loop (max 500 iterations)
  while (hasMore && loopCounter <= 500) {
    // Generate pagination variables
    let paginationVars = {};
    if (endpoint.pagination?.type === PaginationType.PAGE_BASED) {
      paginationVars = { page, limit: endpoint.pagination?.pageSize || 50 };
      page++;
    } else if (endpoint.pagination?.type === PaginationType.OFFSET_BASED) {
      paginationVars = { offset, limit: endpoint.pagination?.pageSize || 50 };
      offset += endpoint.pagination?.pageSize || 50;
    } else {
      hasMore = false;
    }

    const requestVars = { ...paginationVars, ...allVariables };

    // Validate all variables are available
    const invalidVars = validateVariables(endpoint, Object.keys(requestVars));
    if (invalidVars.length > 0) {
      throw new Error(
        `The following variables are not defined: ${invalidVars.join(', ')}`
      );
    }

    // Build request with variable substitution
    const headers = Object.fromEntries(
      Object.entries(endpoint.headers || {})
        .map(([key, value]) => [key, replaceVariables(value, requestVars)])
    );

    // Process Basic Auth headers
    const processedHeaders = {};
    for (const [key, value] of Object.entries(headers)) {
      if (key.toLowerCase() === 'authorization' &&
          typeof value === 'string' &&
          value.startsWith('Basic ')) {
        processedHeaders[key] = convertBasicAuthToBase64(value);
      } else {
        processedHeaders[key] = value;
      }
    }

    const queryParams = Object.fromEntries(
      Object.entries(endpoint.queryParams || {})
        .map(([key, value]) => [key, replaceVariables(value, requestVars)])
    );

    const body = endpoint.body ? replaceVariables(endpoint.body, requestVars) : "";
    const url = replaceVariables(
      composeUrl(endpoint.urlHost, endpoint.urlPath),
      requestVars
    );

    const axiosConfig: AxiosRequestConfig = {
      method: endpoint.method,
      url: url,
      headers: processedHeaders,
      data: body,
      params: queryParams,
      timeout: options?.timeout || 60000,
    };

    console.log(`${endpoint.method} ${url}`);
    const response = await callAxios(axiosConfig, options);

    // Validate response
    if(![200, 201, 204].includes(response?.status) || response.data?.error) {
      const error = JSON.stringify(
        response?.data?.error || response.data?.errors || response?.data
      );

      let message = `${endpoint.method} ${url} failed with status ${response.status}.
Response: ${String(error).slice(0, 200)}
Headers: ${JSON.stringify(headers)}
Body: ${JSON.stringify(body)}
Params: ${JSON.stringify(queryParams)}`;

      // Special handling for rate limits
      if (response.status === 429) {
        const retryAfter = response.headers['retry-after']
          ? `Retry-After: ${response.headers['retry-after']}`
          : 'No Retry-After header provided';

        message = `Rate limit exceeded. ${retryAfter}. Maximum wait time of 60s exceeded. ${message}`;
      }

      throw new Error(`API call failed with status ${response.status}. ${message}`);
    }

    // Check for HTML error responses
    if (typeof response.data === 'string' &&
        (response.data.slice(0, 100).trim().toLowerCase().startsWith('<!doctype html') ||
         response.data.slice(0, 100).trim().toLowerCase().startsWith('<html'))) {
      throw new Error(
        `Received HTML response instead of expected JSON data from ${url}.
This usually indicates an error page or invalid endpoint.
Response: ${response.data.slice(0, 2000)}`
      );
    }

    // Extract data at specified path
    let responseData = response.data;
    if (endpoint.dataPath) {
      const pathParts = endpoint.dataPath.split('.');
      for (const part of pathParts) {
        responseData = responseData[part] || responseData;
      }
    }

    // Collect paginated results
    if (Array.isArray(responseData)) {
      if(responseData.length < endpoint.pagination?.pageSize) {
        hasMore = false;
      }

      if(JSON.stringify(responseData) !== JSON.stringify(allResults)) {
        allResults = allResults.concat(responseData);
      } else {
        hasMore = false;
      }
    } else if(responseData && allResults.length == 0) {
      allResults.push(responseData);
      hasMore = false;
    } else {
      hasMore = false;
    }

    loopCounter++;
  }

  return {
    data: allResults?.length == 1 ? allResults[0] : allResults
  };
}
```

### Variable Substitution

```typescript
export function replaceVariables(
  template: string,
  variables: Record<string, any>
): string {
  if (!template) return "";

  const variableNames = Object.keys(variables);
  const pattern = new RegExp(`\\{(${variableNames.join('|')})(?:\\.(\\w+))*\\}`, 'g');

  return String(template).replace(pattern, (match, path) => {
    const parts = path.split('.');
    let value = variables;

    for (const part of parts) {
      if (value === undefined || value === null) {
        return match; // Keep original if path is invalid
      }
      value = value[part];
    }

    if (value === undefined || value === null) {
      return match;
    }

    if(Array.isArray(value)) {
      return JSON.stringify(value);
    }

    return String(value);
  });
}

function validateVariables(generatedConfig: any, vars: string[]) {
  vars = [...vars, "page", "limit", "offset"];

  const findTemplateVars = (str: string) => {
    if (!str) return [];
    const matches = str.match(/\{(\w+)\}/g) || [];
    return matches.map(match => match.slice(1, -1));
  };

  const varMatches = [
    generatedConfig.urlPath,
    ...Object.values(generatedConfig.queryParams || {}),
    ...Object.values(generatedConfig.headers || {}),
    generatedConfig.body
  ].flatMap(value => findTemplateVars(String(value)));

  const invalidVars = varMatches.filter(v => !vars.includes(v));
  return invalidVars;
}
```

### Retry Logic with Rate Limit Handling

```typescript
export async function callAxios(
  config: AxiosRequestConfig,
  options: RequestOptions
) {
  let retryCount = 0;
  const maxRetries = options?.retries || 0;
  const delay = options?.retryDelay || 1000;
  const maxRateLimitWaitMs = 60 * 1000; // 60s max
  let rateLimitRetryCount = 0;
  let totalRateLimitWaitTime = 0;

  // Don't send body for GET, HEAD, DELETE, OPTIONS
  if(["GET", "HEAD", "DELETE", "OPTIONS"].includes(config.method!)) {
    config.data = undefined;
  }

  do {
    try {
      const response = await axios({
        ...config,
        validateStatus: null, // Don't throw on any status
      });

      if (response.status === 429) {
        let waitTime = 0;

        if (response.headers['retry-after']) {
          const retryAfter = response.headers['retry-after'];
          if (/^\d+$/.test(retryAfter)) {
            waitTime = parseInt(retryAfter, 10) * 1000;
          } else {
            const retryDate = new Date(retryAfter);
            waitTime = retryDate.getTime() - Date.now();
          }
        } else {
          // Exponential backoff with jitter
          waitTime = Math.min(
            Math.pow(2, rateLimitRetryCount) * 1000 + Math.random() * 1000,
            10000
          );
        }

        // Check if we've exceeded max wait time
        if (totalRateLimitWaitTime + waitTime > maxRateLimitWaitMs) {
          console.log(
            `Rate limit retry would exceed maximum wait time of ${maxRateLimitWaitMs}ms`
          );
          return response;
        }

        console.log(`Rate limited (429). Waiting ${waitTime}ms before retry.`);
        await new Promise(resolve => setTimeout(resolve, waitTime));

        totalRateLimitWaitTime += waitTime;
        rateLimitRetryCount++;
        continue;
      }

      return response;
    } catch (error) {
      if (retryCount >= maxRetries) throw error;
      retryCount++;
      await new Promise(resolve => setTimeout(resolve, delay * retryCount));
    }
  } while (retryCount < maxRetries || rateLimitRetryCount > 0);
}
```

---

## File Processing System

**File:** `packages/core/utils/file.ts`

### Decompression

```typescript
import { gunzip, inflate } from 'zlib';
import { promisify } from 'util';
import * as unzipper from 'unzipper';

export async function decompressData(
  compressed: Buffer,
  method: DecompressionMethod
): Promise<Buffer> {
  const gunzipAsync = promisify(gunzip);
  const inflateAsync = promisify(inflate);

  const signature = compressed.slice(0, 4).toString('hex');

  // ZIP signature: PK (504b)
  if (method == DecompressionMethod.ZIP ||
      method == DecompressionMethod.AUTO && signature.startsWith('504b')) {
    console.log("Decompressing with zip");
    return await decompressZip(compressed);
  }
  // GZIP signature: 1f8b
  else if (method == DecompressionMethod.GZIP ||
           method == DecompressionMethod.AUTO && signature.startsWith('1f8b')) {
    console.log("Decompressing with gzip");
    const buffer = await gunzipAsync(compressed);
    return buffer;
  }
  // DEFLATE signature: 1f9d
  else if(method == DecompressionMethod.DEFLATE ||
          method == DecompressionMethod.AUTO && signature.startsWith('1f9d')) {
    console.log("Decompressing with deflate");
    const buffer = await inflateAsync(compressed);
    return buffer;
  }

  return compressed;
}

async function decompressZip(buffer: Buffer): Promise<Buffer> {
  try {
    const zipStream = await unzipper.Open.buffer(buffer);

    // Check if it's an Excel file
    const isExcel = zipStream.files.some(f =>
      f.path === '[Content_Types].xml' ||
      f.path.startsWith('xl/') ||
      f.path.endsWith('.xlsb') ||
      f.path.includes('xl/worksheets/sheet') ||
      f.path.includes('xl/binData/')
    );

    if (isExcel) {
      return buffer; // Return as-is for Excel processing
    }

    const firstFile = zipStream.files?.[0];
    const fileStream = firstFile.stream();
    const chunks: Buffer[] = [];

    for await (const chunk of fileStream) {
      chunks.push(Buffer.from(chunk));
    }

    return Buffer.concat(chunks);
  } catch (error) {
    console.error("Error decompressing zip.", error);
    throw "Error decompressing zip: " + error;
  }
}
```

### File Type Detection

```typescript
async function detectFileType(buffer: Buffer): Promise<FileType> {
  // Check for Excel signature (XLSX files are ZIP files)
  const xlsxSignature = buffer.slice(0, 4).toString('hex');
  if (xlsxSignature === '504b0304') {
    try {
      XLSX.read(buffer, { type: 'buffer' });
      return FileType.EXCEL;
    } catch {
      // Continue with other detection
    }
  }

  const sample = buffer.slice(0, 1024).toString('utf8');
  const trimmedLine = sample.trim();

  if (trimmedLine.startsWith('{') || trimmedLine.startsWith('[')) {
    return FileType.JSON;
  } else if (trimmedLine.startsWith('<?xml') || trimmedLine.startsWith('<')) {
    return FileType.XML;
  } else {
    return FileType.CSV;
  }
}
```

### CSV Parsing with Delimiter Detection

```typescript
import Papa from 'papaparse';
import { Readable } from 'stream';

async function parseCSV(buffer: Buffer): Promise<any> {
  const results: any[] = [];
  const metadata: any[] = [];

  // First pass: detect headers from sample
  const sampleSize = Math.min(buffer.length, 32768);
  const sample = buffer.slice(0, sampleSize);
  const { headerValues, headerRowIndex, delimiter } =
    await detectCSVHeaders(sample);

  let currentLine = -1;

  return new Promise((resolve, reject) => {
    Papa.parse(Readable.from(buffer), {
      header: false,
      skipEmptyLines: false,
      delimiter: delimiter,
      step: (result: {data: any[]}, parser) => {
        try {
          currentLine++;

          // Store metadata rows (before header row)
          if(currentLine <= headerRowIndex) {
            if(result.data == null ||
               result.data?.filter(Boolean).length == 0 ||
               currentLine == headerRowIndex) return;
            metadata.push(result?.data);
            return;
          }

          // Skip empty data rows
          if(result.data == null ||
             result.data.map((value: any) => value?.trim()).filter(Boolean).length == 0)
            return;

          // Create object from row
          const dataObject: { [key: string]: any } = {};
          for(let i = 0; i < headerValues.length; i++) {
            dataObject[headerValues[i]] = result.data[i];
          }
          results.push(dataObject);
        } catch(error) {
          console.error("Error parsing CSV", error);
          parser.abort();
        }
      },
      complete: () => {
        console.log('Finished parsing CSV');
        if(metadata.length > 0) {
          resolve({ data: results, metadata });
        } else {
          resolve(results);
        }
      },
      error: (error) => {
        console.error('Failed parsing CSV');
        reject(error);
      },
    });
  });
}

function detectDelimiter(buffer: Buffer): string {
  const sampleSize = Math.min(buffer.length, 32768);
  const sample = buffer.slice(0, sampleSize).toString('utf8');

  const delimiters = [',', '|', '\t', ';', ':'];
  const counts = delimiters.map(delimiter => ({
    delimiter,
    count: countUnescapedDelimiter(sample, delimiter)
  }));

  return counts.reduce((prev, curr) =>
    curr.count > prev.count ? curr : prev
  ).delimiter;
}

function countUnescapedDelimiter(text: string, delimiter: string): number {
  let count = 0;
  let inQuotes = false;
  let prevChar = '';

  for (let i = 0; i < text.length; i++) {
    const currentChar = text[i];

    if (currentChar === '"' && prevChar !== '\\') {
      inQuotes = !inQuotes;
    }
    else if (currentChar === delimiter && !inQuotes) {
      count++;
    }

    prevChar = currentChar;
  }

  return count;
}
```

### XML Parsing with SAX

```typescript
import sax from 'sax';

async function parseXML(buffer: Buffer): Promise<any[]> {
  const results: any = {};
  let currentElement: any = null;
  const elementStack: any[] = [];

  return new Promise((resolve, reject) => {
    const parser = sax.createStream(true);

    parser.on('opentag', (node) => {
      const newElement: any = {};
      if (currentElement) {
        elementStack.push(currentElement);
      }
      currentElement = newElement;
    });

    parser.on('text', (text) => {
      if (!currentElement || text?.trim()?.length == 0) return;

      if(Object.keys(currentElement)?.length > 0) {
        currentElement["__text"] = text.trim();
      }
      else if(Array.isArray(currentElement)) {
        currentElement.push(text.trim());
      }
      else if(typeof currentElement === "string") {
        currentElement = [currentElement, text.trim()];
      }
      else {
        currentElement = text.trim();
      }
    });

    parser.on('closetag', (tagName) => {
      let parentElement = elementStack.pop();
      if(parentElement == null) {
        parentElement = results;
      }

      if (currentElement) {
        if(!parentElement[tagName]) {
          parentElement[tagName] = currentElement;
        }
        else if(Array.isArray(parentElement[tagName])) {
          parentElement[tagName].push(currentElement);
        }
        else {
          // Convert to array when multiple elements with same name
          parentElement[tagName] = [parentElement[tagName], currentElement];
        }
      }
      currentElement = parentElement;
    });

    parser.on('error', (error) => {
      console.error('Failed converting XML to JSON:', error);
      reject(error);
    });

    parser.on('end', async () => {
      console.log('Finished parsing XML');
      resolve(results);
    });

    const readStream = Readable.from(buffer);
    readStream.pipe(parser);
  });
}
```

### Excel Parsing

```typescript
import * as XLSX from 'xlsx';

async function parseExcel(buffer: Buffer): Promise<{ [sheetName: string]: any[] }> {
  const workbook = XLSX.read(buffer, {
    type: 'buffer',
    cellDates: true
  });

  const result: { [sheetName: string]: any[] } = {};

  for (const sheetName of workbook.SheetNames) {
    const worksheet = workbook.Sheets[sheetName];

    // Get all rows
    const rawRows = XLSX.utils.sheet_to_json<any>(worksheet, {
      raw: false,
      header: 1,
      defval: null,
      blankrows: true
    });

    if (!rawRows?.length) {
      result[sheetName] = [];
      continue;
    }

    // Find header row from first 20 rows
    const headerRowIndex = rawRows
      .slice(0, 20)
      .reduce((maxIndex, row, currentIndex, rows) =>
        (row.length > rows[maxIndex]?.length || 0) ? currentIndex : maxIndex
      , 0);

    // Get headers
    const headers = rawRows[headerRowIndex].map((header: any, index: number) =>
      header ? String(header).trim() : `Column ${index + 1}`
    );

    // Process data rows
    const processedRows = rawRows.slice(headerRowIndex + 1).map((row: any) => {
      const obj: { [key: string]: any } = {};
      headers.forEach((header: string, index: number) => {
        if (header && row[index] !== undefined) {
          obj[header] = row[index];
        }
      });
      return obj;
    });

    result[sheetName] = processedRows;
  }

  return result;
}
```

---

## Caching Architecture

**File:** `packages/core/datastore/redis.ts`

### Redis Key Structure

```typescript
export class RedisService implements DataStore {
  private redis: RedisClientType;
  private readonly RUN_PREFIX = 'run:';
  private readonly API_PREFIX = 'api:';
  private readonly EXTRACT_PREFIX = 'extract:';
  private readonly TRANSFORM_PREFIX = 'transform:';
  private readonly TTL = 60 * 60 * 24 * 90; // 90 days

  private getKey(prefix: string, id: string, orgId: string): string {
    return `${orgId ? `${orgId}:` : ''}${prefix}${id}`;
  }

  private getPattern(prefix: string, orgId?: string): string {
    return `${orgId ? `${orgId}:` : ''}${prefix}*`;
  }
}
```

### Hash-Based Configuration Lookup

```typescript
import { createHash } from 'crypto';

private generateHash(data: any): string {
  return createHash('md5').update(JSON.stringify(data)).digest('hex');
}

async function getApiConfigFromRequest(
  request: ApiInput,
  payload: any,
  orgId?: string
): Promise<ApiConfig | null> {
  if(!request) return null;

  // Hash request + data schema
  const hash = this.generateHash({
    request,
    payloadKeys: getSchemaFromData(payload)
  });

  const key = this.getKey(this.API_PREFIX, hash, orgId);
  const data = await this.redis.get(key);

  return parseWithId(data, hash);
}

async function saveApiConfig(
  request: ApiInput,
  payload: any,
  config: ApiConfig,
  orgId?: string
): Promise<ApiConfig> {
  if(!request) return null;

  const hash = this.generateHash({
    request,
    payloadKeys: getSchemaFromData(payload)
  });

  const key = this.getKey(this.API_PREFIX, hash, orgId);
  config.id = hash;

  await this.redis.set(key, JSON.stringify(config));
  return config;
}
```

### Run Result Storage

```typescript
async function createRun(run: RunResult, orgId?: string): Promise<RunResult> {
  if(!run) return null;

  // Key format: {orgId}:run:{configId}:{runId}
  const key = this.getKey(
    this.RUN_PREFIX,
    `${run.config?.id}:${run.id}`,
    orgId
  );

  await this.redis.set(key, JSON.stringify(run), {
    EX: this.TTL  // 90 day TTL
  });

  return run;
}

async function listRuns(
  limit: number = 10,
  offset: number = 0,
  configId?: string,
  orgId?: string
): Promise<{ items: RunResult[], total: number }> {
  // Different pattern based on whether configId is specified
  const pattern = configId
    ? this.getRunsByConfigPattern(this.RUN_PREFIX, configId, orgId)
    : this.getPattern(this.RUN_PREFIX, orgId);

  const keys = await this.redis.keys(pattern);
  const total = keys.length;

  if (total === 0) {
    return { items: [], total: 0 };
  }

  const runs = await Promise.all(
    keys.map(async (key) => {
      const data = await this.redis.get(key);
      const runId = key.split(':').pop()!;
      return parseWithId(data, runId);
    })
  );

  const validRuns = runs
    .filter((run): run is RunResult => run !== null)
    .sort((a, b) => (b.startedAt?.getTime() ?? 0) - (a.startedAt?.getTime() ?? 0));

  return {
    items: validRuns.slice(offset, offset + limit),
    total
  };
}
```

### Date Parsing Helper

```typescript
function parseWithId(data: string, id: string): any {
  if(!data) return null;

  const parsed = typeof data === 'string' ? JSON.parse(data) : data;

  return {
    ...parsed,
    ...(parsed.startedAt && { startedAt: new Date(parsed.startedAt) }),
    ...(parsed.completedAt && { completedAt: new Date(parsed.completedAt) }),
    ...(parsed.createdAt && { createdAt: new Date(parsed.createdAt) }),
    ...(parsed.updatedAt && { updatedAt: new Date(parsed.updatedAt) }),
    ...(parsed.config && {
      config: parseWithId(parsed.config, parsed.config.id)
    }),
    id: id
  };
}
```

---

## Authentication System

### API Key Management

**File:** `packages/core/auth/localKeyManager.ts`

```typescript
export class LocalKeyManager {
  async authenticate(token: string): Promise<{ success: boolean; orgId: string }> {
    const isValid = token === process.env.AUTH_TOKEN;
    return {
      success: isValid,
      orgId: isValid ? 'default-org' : ''
    };
  }
}
```

### Supabase Key Manager

**File:** `packages/core/auth/supabaseKeyManager.ts`

```typescript
export class SupabaseKeyManager {
  async authenticate(token: string): Promise<{ success: boolean; orgId: string }> {
    // Validate token against Supabase
    const { data, error } = await supabase
      .from('api_keys')
      .select('org_id, is_valid')
      .eq('key', token)
      .single();

    if (error || !data || !data.is_valid) {
      return { success: false, orgId: '' };
    }

    return { success: true, orgId: data.org_id };
  }
}
```

### Credential Masking

```typescript
export function maskCredentials(
  message: string,
  credentials?: Record<string, string>
): string {
  if (!credentials) {
    return message;
  }

  let maskedMessage = message;
  Object.entries(credentials).forEach(([key, value]) => {
    if (value && value.length > 0) {
      // Use global flag to replace all occurrences
      const regex = new RegExp(value, 'g');
      maskedMessage = maskedMessage.replace(regex, `{masked_${key}}`);
    }
  });
  return maskedMessage;
}
```

---

## Error Handling & Retry Logic

### Call Resolver Error Flow

**File:** `packages/core/graphql/resolvers/call.ts`

```typescript
export const callResolver = async (
  _: any,
  { input, payload, credentials, options }: {
    input: ApiInputRequest;
    payload: any;
    credentials?: Record<string, string>;
    options: RequestOptions;
  },
  context: Context,
  info: GraphQLResolveInfo
) => {
  const startedAt = new Date();
  const callId = uuidv4() as string;

  let preparedEndpoint: ApiConfig;
  let messages: OpenAI.Chat.ChatCompletionMessageParam[] = [];
  let lastError: string | null = null;
  let retryCount = 0;

  const readCache = options
    ? options.cacheMode === CacheMode.ENABLED ||
      options.cacheMode === CacheMode.READONLY
    : true;

  try {
    // Retry loop for API calls
    do {
      try {
        // Try cache first
        if(readCache && !lastError) {
          preparedEndpoint = await context.datastore.getApiConfig(input.id, context.orgId) ||
            await context.datastore.getApiConfigFromRequest(input.endpoint, payload, context.orgId);
        }

        // Generate new config if not cached or on error
        if(!preparedEndpoint || lastError) {
          const result = await prepareEndpoint(
            preparedEndpoint || input.endpoint,
            payload,
            credentials,
            lastError,
            messages
          );
          preparedEndpoint = result.config;
          messages = result.messages;  // Keep conversation history for LLM
        }

        if(!preparedEndpoint) {
          throw new Error(
            "Did not find a valid endpoint configuration."
          );
        }

        response = await callEndpoint(preparedEndpoint, payload, credentials, options);

        if(!response.data) {
          response = null;
          throw new Error("No data returned from API.");
        }
      } catch (error) {
        console.log(`API call failed. ${error?.message}`);

        // Capture telemetry
        telemetryClient?.captureException(
          maskCredentials(error.message, credentials),
          context.orgId,
          {
            preparedEndpoint: preparedEndpoint,
            retryCount: retryCount,
          }
        );

        lastError = error?.message || JSON.stringify(error || {});
      }
      retryCount++;
    } while (!response && retryCount < 5);

    if(!response) {
      throw new Error(
        `API call failed after ${retryCount} retries. Last error: ${lastError}`
      );
    }

    // ... rest of successful flow

  } catch (error) {
    const maskedError = maskCredentials(error.message, credentials);

    // Notify webhook of failure
    if (options?.webhookUrl) {
      await notifyWebhook(
        options.webhookUrl,
        callId,
        false,
        undefined,
        error.message
      );
    }

    const result = {
      id: callId,
      success: false,
      error: maskedError,
      config: preparedEndpoint,
      startedAt,
      completedAt: new Date(),
    };

    telemetryClient?.captureException(maskedError, context.orgId, {
      preparedEndpoint: preparedEndpoint,
      messages: messages,
      result: result
    });

    context.datastore.createRun(result, context.orgId);
    return result;
  }
};
```

### Webhook Notification

**File:** `packages/core/utils/webhook.ts`

```typescript
export async function notifyWebhook(
  webhookUrl: string,
  callId: string,
  success: boolean,
  data?: any,
  error?: string
): Promise<void> {
  try {
    await axios.post(webhookUrl, {
      id: callId,
      success,
      data: success ? data : undefined,
      error: error || undefined,
    });
  } catch (webhookError) {
    console.error('Webhook notification failed:', webhookError);
    // Don't throw - webhook failure shouldn't fail the main operation
  }
}
```

---

**Document completed:** 2026-03-25
**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.superglue/superglue/`
