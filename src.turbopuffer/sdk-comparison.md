# SDK Comparison: Turbopuffer Language Bindings

## Comprehensive Analysis of Python, TypeScript, Go, and Java SDKs

This document provides a detailed comparison of all four Turbopuffer SDK implementations, analyzing API design patterns, connection handling, serialization, and language-specific considerations.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Python SDK Deep Dive](#python-sdk-deep-dive)
3. [TypeScript SDK Deep Dive](#typescript-sdk-deep-dive)
4. [Go SDK Deep Dive](#go-sdk-deep-dive)
5. [Java SDK Deep Dive](#java-sdk-deep-dive)
6. [API Design Pattern Comparison](#api-design-pattern-comparison)
7. [Connection Pooling and Retries](#connection-pooling-and-retries)
8. [Serialization Formats](#serialization-formats)
9. [Error Handling Patterns](#error-handling-patterns)
10. [Pagination Patterns](#pagination-patterns)
11. [Recommendations](#recommendations)

---

## Architecture Overview

### Common Foundation: Stainless Codegen

All four SDKs are generated using [Stainless](https://www.stainless.com/), ensuring API consistency across languages.

**Shared Characteristics:**
- Same API endpoints and methods
- Consistent error types (adapted to language conventions)
- Automatic retry logic
- Pagination helpers
- Type safety (where language supports it)

**Architecture Diagram:**
```
┌─────────────────────────────────────────────────────────────────┐
│                    Application Code                              │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                      SDK Client                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Resources     │  │   Pagination    │  │   Error Types   │  │
│  │   (Namespaces)  │  │   (Auto-paging) │  │   (Typed)       │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                     HTTP Layer                                   │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   Retry Logic   │  │   Timeouts      │  │   Middleware    │  │
│  │   (exponential) │  │   (configurable)│  │   (logging)     │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                  HTTP Client (Language-specific)                 │
│  Python: httpx    │  TS: fetch     │  Go: net/http  │  Java: OkHttp │
└─────────────────────────────────────────────────────────────────┘
```

---

## Python SDK Deep Dive

### Installation and Setup

```bash
# Standard installation
pip install turbopuffer

# With optional fast JSON encoding (requires C binaries)
pip install --pre turbopuffer[fast]
```

### Client Configuration

```python
import os
from turbopuffer import Turbopuffer, AsyncTurbopuffer

# Synchronous client
client = Turbopuffer(
    region="gcp-us-central1",
    api_key=os.environ.get("TURBOPUFFER_API_KEY"),
    base_url="https://gcp-us-central1.turbopuffer.com",  # auto-derived
    timeout=60.0,  # seconds
    max_retries=2,
)

# Asynchronous client
async_client = AsyncTurbopuffer(
    region="gcp-us-central1",
    api_key=os.environ.get("TURBOPUFFER_API_KEY"),
)
```

### Key Features

**1. Pydantic Models for Type Safety:**
```python
from turbopuffer.types.namespace_query_response import NamespaceQueryResponse

response = client.namespace("products").query(...)
# response is a Pydantic model with:
# - Autocomplete in IDEs
# - Runtime validation
# - .model_dump() for dict conversion
# - .model_dump_json() for JSON
```

**2. Handling Null vs. Missing:**
```python
response = client.namespace("products").query(...)

# Check if field was present in response
if response.next_cursor is None:
    if 'next_cursor' not in response.model_fields_set:
        print("Field was not present in JSON")
    else:
        print("Field was explicitly null")
```

**3. Custom HTTP Client:**
```python
import httpx
from turbopuffer import Turbopuffer, DefaultHttpxClient

# Custom proxy configuration
client = Turbopuffer(
    region="gcp-us-central1",
    http_client=DefaultHttpxClient(
        proxy="http://proxy.example.com:8080",
        limits=httpx.Limits(max_connections=100),
    ),
)
```

**4. Streaming Responses:**
```python
# Eager (default)
response = client.namespace("products").query(...)

# Streaming (context manager required)
with client.with_streaming_response.namespace("products").query(...) as response:
    print(response.headers.get("X-Custom-Header"))
    for line in response.iter_lines():
        print(line)
```

### Transport Options

The Python SDK supports multiple HTTP transports:

```python
# httpx (default)
from turbopuffer.lib.transport import Transport

# urllib3 fallback
from turbopuffer.lib.transport_urllib3 import Transport

# aiohttp for async
from turbopuffer.lib.transport_aiohttp import Transport
```

---

## TypeScript SDK Deep Dive

### Installation

```bash
npm install @turbopuffer/turbopuffer
```

### Client Configuration

```typescript
import Turbopuffer from '@turbopuffer/turbopuffer';

const client = new Turbopuffer({
  region: 'gcp-us-central1',
  apiKey: process.env['TURBOPUFFER_API_KEY'],
  timeout: 60000,  // milliseconds
  maxRetries: 2,
});
```

### Key Features

**1. API Promise Wrapper:**
```typescript
// All methods return APIPromise<T>
const results = client.namespace('products').query({
  vector: [0.1, 0.2, 0.3],
  top_k: 10,
});

// Access raw response
const rawResponse = await results.asResponse();
console.log(rawResponse.headers.get('X-Request-ID'));

// Get parsed data with response
const { data, response } = await results.withResponse();
```

**2. Async Iteration for Pagination:**
```typescript
// Auto-pagination with for-await
const allResults = [];
for await (const item of client.namespaces({ prefix: 'products' })) {
  allResults.push(item);
}

// Manual pagination
let page = await client.namespaces({ prefix: 'products' });
while (page.hasNextPage()) {
  page = await page.getNextPage();
}
```

**3. Custom Fetch:**
```typescript
import Turbopuffer from '@turbopuffer/turbopuffer';

// Bring your own fetch implementation
const client = new Turbopuffer({
  fetch: myCustomFetch,
});

// Or configure fetch options
const client = new Turbopuffer({
  fetchOptions: {
    // RequestInit options
    dispatcher: myProxyAgent,  // For proxy support with undici
  },
});
```

**4. Logging:**
```typescript
// Via environment
// export TURBOPUFFER_LOG=debug

// Or programmatically
import Turbopuffer from '@turbopuffer/turbopuffer';
import pino from 'pino';

const logger = pino();
const client = new Turbopuffer({
  logger: logger.child({ name: 'Turbopuffer' }),
  logLevel: 'debug',
});
```

### Runtime Support

The TypeScript SDK supports multiple runtimes:

| Runtime | Support |
|---------|---------|
| Node.js 20+ | ✅ Full |
| Bun 1.0+ | ✅ Full |
| Deno v1.28+ | ✅ Full |
| Cloudflare Workers | ✅ Full |
| Vercel Edge Runtime | ✅ Full |
| Browsers (modern) | ✅ Full |

---

## Go SDK Deep Dive

### Installation

```bash
go get github.com/turbopuffer/turbopuffer-go
```

### Client Configuration

```go
import (
    "github.com/turbopuffer/turbopuffer-go"
    "github.com/turbopuffer/turbopuffer-go/option"
)

client := turbopuffer.NewClient(
    option.WithAPIKey("tpuf_A1..."),
    option.WithRegion("gcp-us-central1"),
)
```

### Key Features

**1. Omitzero Semantics (Go 1.24+):**
```go
// Optional fields use param.Opt[T]
type NamespaceWriteParams struct {
    DistanceMetric DistanceMetric `json:",omitzero"`
    UpsertRows     []RowParam     `json:",omitzero"`
}

// Set optional field
params := turbopuffer.NamespaceWriteParams{
    DistanceMetric: turbopuffer.DistanceMetricCosineDistance,
    UpsertRows:     []turbopuffer.RowParam{...},
}

// Check if field is omitted
if param.IsOmitted(params.DistanceMetric) {
    // Field not set
}
```

**2. Null Handling:**
```go
// Send null for optional field
params.Name = param.Null[string]()

// Send null for struct
params.Point = param.NullStruct[Point]()

// Check if null
param.IsNull(params.Name)  // true
```

**3. Response Field Validity:**
```go
type Animal struct {
    Name   string `json:"name,nullable"`
    Owners int    `json:"owners"`
    JSON   struct {
        Name  respjson.Field
        Owner respjson.Field
    } `json:"-"`
}

// Check if field was present
if res.JSON.Name.Valid() {
    // Field was in response
}

// Get raw JSON value
raw := res.JSON.Name.Raw()
if raw == respjson.Null {
    // Was explicitly null
} else if raw == respjson.Omitted {
    // Was not present
}
```

**4. Auto-pagination:**
```go
// Auto-paging iterator
iter := client.NamespacesAutoPaging(context.TODO(), turbopuffer.NamespacesParams{
    Prefix: turbopuffer.String("products"),
})
for iter.Next() {
    namespace := iter.Current()
    fmt.Printf("%+v\n", namespace)
}
if err := iter.Err(); err != nil {
    panic(err)
}

// Manual pagination
page, err := client.Namespaces(context.TODO(), params)
for page != nil {
    for _, ns := range page.Namespaces {
        fmt.Printf("%+v\n", ns)
    }
    page, err = page.GetNextPage()
}
```

**5. Middleware:**
```go
func Logger(req *http.Request, next option.MiddlewareNext) (*http.Response, error) {
    start := time.Now()
    log.Printf("Request: %s %s", req.Method, req.URL)

    res, err := next(req)

    log.Printf("Response: %d in %v", res.StatusCode, time.Since(start))
    return res, err
}

client := turbopuffer.NewClient(
    option.WithMiddleware(Logger),
)
```

---

## Java SDK Deep Dive

### Installation

**Maven:**
```xml
<dependency>
  <groupId>com.turbopuffer</groupId>
  <artifactId>turbopuffer-java</artifactId>
  <version>0.1.0-beta.12</version>
</dependency>
```

**Gradle:**
```kotlin
implementation("com.turbopuffer:turbopuffer-java:0.1.0-beta.12")
```

### Client Configuration

```java
import com.turbopuffer.client.TurbopufferClient;
import com.turbopuffer.client.okhttp.TurbopufferOkHttpClient;

// From environment variables
TurbopufferClient client = TurbopufferOkHttpClient.fromEnv();

// Or manual configuration
TurbopufferClient client = TurbopufferOkHttpClient.builder()
    .apiKey("tpuf_A1...")
    .baseUrl("https://gcp-us-central1.turbopuffer.com")
    .build();
```

### Key Features

**1. Builder Pattern:**
```java
NamespaceWriteParams params = NamespaceWriteParams.builder()
    .namespace("products")
    .distanceMetric(DistanceMetric.COSINE_DISTANCE)
    .addUpsertRow(RowParam.builder()
        .id("item_123")
        .addVector(0.1f)
        .addVector(0.2f)
        .putAttribute("name", "Red boots")
        .build())
    .build();
```

**2. Immutable Objects with toBuilder():**
```java
// Modify existing params
NamespaceWriteParams modified = params.toBuilder()
    .namespace("updated")
    .build();

// Original params unchanged
```

**3. Async Execution:**
```java
// Sync client
NamespaceWriteResponse response = client.namespaces().write(params);

// Async via method call
CompletableFuture<NamespaceWriteResponse> future =
    client.async().namespaces().write(params);

// Or dedicated async client
TurbopufferClientAsync asyncClient = TurbopufferOkHttpClientAsync.fromEnv();
CompletableFuture<NamespaceWriteResponse> future =
    asyncClient.namespaces().write(params);
```

**4. Raw Response Access:**
```java
HttpResponseFor<List<DocumentRowWithScore>> response =
    client.namespaces().withRawResponse().query(params);

int statusCode = response.statusCode();
Headers headers = response.headers();
List<DocumentRowWithScore> parsed = response.parse();
```

**5. Response Validation:**
```java
// Per-request validation
NamespaceWriteResponse response = client.namespaces().write(
    params,
    RequestOptions.builder().responseValidation(true).build()
);

// Client-wide default
TurbopufferClient client = TurbopufferOkHttpClient.builder()
    .responseValidation(true)
    .build();
```

**6. Custom HTTP Client:**
```java
import okhttp3.OkHttpClient;

OkHttpClient customClient = new OkHttpClient.Builder()
    .proxy(new Proxy(Proxy.Type.HTTP, new InetSocketAddress("proxy", 8080)))
    .build();

TurbopufferClient client = TurbopufferOkHttpClient.builder()
    .okHttpClient(customClient)
    .build();
```

### Artifact Structure

The Java SDK is split into multiple artifacts:

| Artifact | Purpose |
|----------|---------|
| `turbopuffer-java-core` | Core SDK logic, HTTP client agnostic |
| `turbopuffer-java-client-okhttp` | OkHttp client implementation |
| `turbopuffer-java` | Convenience artifact combining both |

---

## API Design Pattern Comparison

### Client Instantiation

| Language | Pattern | Environment |
|----------|---------|-------------|
| Python | `Turbopuffer(api_key=...)` | `TURBOPUFFER_API_KEY` |
| TypeScript | `new Turbopuffer({ apiKey })` | `TURBOPUFFER_API_KEY` |
| Go | `NewClient(option.WithAPIKey(...))` | `TURBOPUFFER_API_KEY` |
| Java | `TurbopufferOkHttpClient.fromEnv()` | `TURBOPUFFER_API_KEY` |

### Error Handling

**Python:**
```python
try:
    client.namespace("products").query(...)
except turbopuffer.APIConnectionError as e:
    print(f"Connection failed: {e.__cause__}")
except turbopuffer.RateLimitError as e:
    print(f"Rate limited: {e.status_code}")
except turbopuffer.APIStatusError as e:
    print(f"API error: {e.status_code}")
```

**TypeScript:**
```typescript
try {
    await client.namespace('products').query(...);
} catch (err) {
    if (err instanceof Turbopuffer.APIError) {
        console.log(`API error: ${err.status}`);
        console.log(`Error type: ${err.name}`);
    }
}
```

**Go:**
```go
_, err := client.Namespace("products").Query(ctx, params)
if err != nil {
    var apierr *turbopuffer.Error
    if errors.As(err, &apierr) {
        log.Printf("API error: %d", apierr.StatusCode)
    }
    log.Fatal(err)
}
```

**Java:**
```java
try {
    client.namespaces().write(params);
} catch (TurbopufferServiceException e) {
    if (e instanceof RateLimitException) {
        System.out.println("Rate limited");
    } else if (e instanceof BadRequestException) {
        System.out.println("Bad request: " + e.getMessage());
    }
}
```

### Error Type Mapping

| HTTP Status | Python | TypeScript | Go | Java |
|-------------|--------|------------|-----|------|
| 400 | `BadRequestError` | `BadRequestError` | `*Error` | `BadRequestException` |
| 401 | `AuthenticationError` | `AuthenticationError` | `*Error` | `UnauthorizedException` |
| 403 | `PermissionDeniedError` | `PermissionDeniedError` | `*Error` | `PermissionDeniedException` |
| 404 | `NotFoundError` | `NotFoundError` | `*Error` | `NotFoundException` |
| 429 | `RateLimitError` | `RateLimitError` | `*Error` | `RateLimitException` |
| 5xx | `InternalServerError` | `InternalServerError` | `*Error` | `InternalServerException` |

---

## Connection Pooling and Retries

### Retry Configuration

**Default Behavior (all SDKs):**
- Max retries: 2
- Retry on: Connection errors, 408, 409, 429, >=500
- Backoff: Exponential

**Python:**
```python
client = Turbopuffer(
    max_retries=5,
    timeout=httpx.Timeout(30.0),
)

# Per-request override
client.with_options(max_retries=0).namespace("products").query(...)
```

**TypeScript:**
```typescript
const client = new Turbopuffer({
  maxRetries: 5,
  timeout: 30000,
});

// Per-request override
await client.namespaces({}, { maxRetries: 0 });
```

**Go:**
```go
client := turbopuffer.NewClient(
    option.WithMaxRetries(5),
)

// Per-request override
client.Namespaces(ctx, params, option.WithMaxRetries(0))
```

**Java:**
```java
TurbopufferClient client = TurbopufferOkHttpClient.builder()
    .maxRetries(5)
    .build();

// Per-request override
client.namespaces().write(params, RequestOptions.builder().maxRetries(0).build());
```

### Connection Pooling

**Python (httpx):**
```python
from turbopuffer import DefaultHttpxClient
import httpx

client = Turbopuffer(
    http_client=DefaultHttpxClient(
        limits=httpx.Limits(
            max_connections=100,
            max_keepalive_connections=20,
        ),
    ),
)
```

**TypeScript (undici for Node.js):**
```typescript
import * as undici from 'undici';

const proxyAgent = new undici.ProxyAgent('http://proxy:8080');

const client = new Turbopuffer({
  fetchOptions: {
    dispatcher: proxyAgent,
  },
});
```

**Go (net/http):**
```go
import "net/http"

client := turbopuffer.NewClient(
    option.WithHTTPClient(&http.Client{
        Transport: &http.Transport{
            MaxIdleConns:        100,
            MaxIdleConnsPerHost: 10,
            IdleConnTimeout:     45 * time.Second,
        },
    }),
)
```

**Java (OkHttp):**
```java
OkHttpClient httpClient = new OkHttpClient.Builder()
    .connectionPool(new ConnectionPool(100, 5, TimeUnit.MINUTES))
    .build();

TurbopufferClient client = TurbopufferOkHttpClient.builder()
    .okHttpClient(httpClient)
    .build();
```

---

## Serialization Formats

### Request Serialization

**Python (Pydantic):**
```python
# Model to JSON
params = NamespaceQueryParams(vector=[0.1, 0.2], top_k=10)
json_data = params.model_dump_json()

# Dict to JSON
json_data = json.dumps({"vector": [0.1, 0.2], "top_k": 10})
```

**TypeScript:**
```typescript
// Native JSON
const json = JSON.stringify({ vector: [0.1, 0.2], top_k: 10 });
```

**Go (encoding/json with omitzero):**
```go
// Struct to JSON (automatic via json.Marshal)
params := NamespaceQueryParams{Vector: []float32{0.1, 0.2}}
jsonBytes, _ := json.Marshal(params)
```

**Java (Jackson):**
```java
// Object to JSON (automatic via Jackson)
ObjectMapper mapper = new ObjectMapper();
String json = mapper.writeValueAsString(params);
```

### Response Deserialization

**Python:**
```python
response = NamespaceQueryResponse.model_validate_json(json_data)
vector = response.vector  # Type-safe access
```

**TypeScript:**
```typescript
const response = await client.namespace('products').query(params);
const vector = response.vector;  // Typed
```

**Go:**
```go
var response NamespaceQueryResponse
json.Unmarshal(jsonBytes, &response)
```

**Java:**
```java
NamespaceQueryResponse response = client.namespaces().query(params);
List<Float> vector = response.getVector();
```

---

## Pagination Patterns

### Auto-Pagination

**Python:**
```python
# Sync iteration
for namespace in client.namespaces(prefix="products"):
    print(namespace.id)

# Async iteration
async for namespace in async_client.namespaces(prefix="products"):
    print(namespace.id)
```

**TypeScript:**
```typescript
for await (const namespace of client.namespaces({ prefix: 'products' })) {
    console.log(namespace.id);
}
```

**Go:**
```go
iter := client.NamespacesAutoPaging(ctx, turbopuffer.NamespacesParams{
    Prefix: turbopuffer.String("products"),
})
for iter.Next() {
    fmt.Printf("%+v\n", iter.Current())
}
```

**Java:**
```java
// As Iterable
for (NamespaceSummary ns : client.namespaces().list(params).autoPager()) {
    System.out.println(ns);
}

// As Stream
client.namespaces().list(params).autoPager().stream()
    .limit(50)
    .forEach(System.out::println);
```

### Manual Pagination

**Python:**
```python
page = client.namespaces(prefix="products")
while page:
    for ns in page.namespaces:
        print(ns.id)
    if page.has_next_page():
        page = page.get_next_page()
```

**TypeScript:**
```typescript
let page = await client.namespaces({ prefix: 'products' });
while (page.hasNextPage()) {
    page = await page.getNextPage();
}
```

**Go:**
```go
page, err := client.Namespaces(ctx, params)
for page != nil && err == nil {
    for _, ns := range page.Namespaces {
        fmt.Printf("%+v\n", ns)
    }
    page, err = page.GetNextPage()
}
```

**Java:**
```java
NamespaceListPage page = client.namespaces().list(params);
while (page != null) {
    for (NamespaceSummary ns : page.namespaces()) {
        System.out.println(ns);
    }
    page = page.getNextPage().orElse(null);
}
```

---

## Recommendations

### Choosing an SDK

**Python:**
- Best for: Data science, ML workflows, scripting
- Pros: Pydantic validation, async support, rich ecosystem
- Cons: GIL limits parallelism

**TypeScript:**
- Best for: Web applications, serverless, edge computing
- Pros: Universal runtime support, native async, small bundle size
- Cons: JavaScript ecosystem complexity

**Go:**
- Best for: High-performance services, CLI tools
- Pros: Excellent concurrency, compiled performance, simple deployment
- Cons: Less ergonomic API (omitzero, param wrappers)

**Java:**
- Best for: Enterprise applications, existing JVM ecosystems
- Pros: Mature ecosystem, excellent async (CompletableFuture), strong typing
- Cons: Verbosity, higher memory usage

### Best Practices

**1. Connection Management:**
```python
# Python: Use context manager
with Turbopuffer() as client:
    ...

# Java: Reuse client (don't create per-request)
private static final TurbopufferClient CLIENT = TurbopufferOkHttpClient.fromEnv();
```

**2. Error Handling:**
```typescript
// TypeScript: Always check error types
try {
    await client.namespace('products').query(...);
} catch (err) {
    if (err instanceof Turbopuffer.RateLimitError) {
        await sleep(exponentialBackoff(retryCount));
    }
}
```

**3. Pagination:**
```go
// Go: Handle iterator errors
iter := client.NamespacesAutoPaging(ctx, params)
for iter.Next() {
    process(iter.Current())
}
if err := iter.Err(); err != nil {
    log.Fatal(err)
}
```

**4. Logging:**
```java
// Java: Enable debug logging for troubleshooting
System.setProperty("TURBOPUFFER_LOG", "debug");
```

---

## Summary Table

| Feature | Python | TypeScript | Go | Java |
|---------|--------|------------|-----|------|
| HTTP Client | httpx | fetch | net/http | OkHttp |
| Async Support | ✅ | ✅ | ❌ | ✅ |
| Type Safety | Pydantic | TypeScript | omitzero | Builder |
| Null Handling | `model_fields_set` | `null` | `param.Opt[T]` | `JsonField<T>` |
| Pagination | Iterator | for-await | AutoPaging | Iterable/Stream |
| Middleware | ✅ | ✅ | ✅ | ✅ |
| Streaming | ✅ | ✅ | ❌ | ✅ |
| Custom HTTP | ✅ | ✅ | ✅ | ✅ |
