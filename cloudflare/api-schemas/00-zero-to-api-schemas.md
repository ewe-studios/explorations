# Cloudflare API Schemas Documentation

**Last Updated:** 2026-04-05

---

## Overview

The `api-schemas` package contains **OpenAPI schema definitions** for the Cloudflare API. This is a minimal package that provides machine-readable API specifications for Cloudflare's REST API endpoints.

---

## What is OpenAPI?

OpenAPI Specification (OAS) is a standard for describing REST APIs in a machine-readable format (JSON or YAML). It enables:

- **Documentation Generation** - Auto-generated API docs
- **Client SDK Generation** - Generate TypeScript, Python, Go clients
- **Server Validation** - Request/response validation
- **Testing** - Automated API testing from schemas
- **Discovery** - API exploration and understanding

---

## Usage

### Install Package

```bash
npm install @cloudflare/api-schemas
```

### Load Schema

```typescript
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

// Load OpenAPI schema
const schemaPath = resolve(
  __dirname,
  '../node_modules/@cloudflare/api-schemas/openapi.json'
);
const openapiSchema = JSON.parse(readFileSync(schemaPath, 'utf-8'));

console.log(openapiSchema.info.title); // "Cloudflare API"
console.log(openapiSchema.info.version); // API version
```

### Generate TypeScript Types

```bash
# Using openapi-typescript
npx openapi-typescript node_modules/@cloudflare/api-schemas/openapi.json \
  -o cloudflare-api-types.ts
```

### Generate API Client

```bash
# Using openapi-generator
openapi-generator generate \
  -i node_modules/@cloudflare/api-schemas/openapi.json \
  -g typescript \
  -o ./generated-client
```

---

## Schema Structure

```json
{
  "openapi": "3.0.0",
  "info": {
    "title": "Cloudflare API",
    "version": "1.0.0",
    "description": "Cloudflare REST API specification"
  },
  "servers": [
    {
      "url": "https://api.cloudflare.com/client/v4"
    }
  ],
  "paths": {
    "/zones": {
      "get": {
        "operationId": "listZones",
        "summary": "List zones",
        "parameters": [...],
        "responses": {
          "200": {
            "description": "Success",
            "content": {
              "application/json": {
                "schema": {
                  "$ref": "#/components/schemas/ZoneListResponse"
                }
              }
            }
          }
        }
      }
    }
  },
  "components": {
    "schemas": {
      "Zone": {
        "type": "object",
        "properties": {
          "id": { "type": "string" },
          "name": { "type": "string" },
          "status": { "type": "string" }
        }
      },
      "ZoneListResponse": {
        "type": "object",
        "properties": {
          "success": { "type": "boolean" },
          "result": {
            "type": "array",
            "items": { "$ref": "#/components/schemas/Zone" }
          }
        }
      }
    }
  }
}
```

---

## Common Endpoints

### Zones

```yaml
GET /zones:
  summary: List zones
  parameters:
    - name: name
      in: query
      schema: { type: string }
    - name: status
      in: query
      schema: { type: string }
    - name: page
      in: query
      schema: { type: integer }
    - name: per_page
      in: query
      schema: { type: integer }

GET /zones/{zone_id}:
  summary: Get zone details
  parameters:
    - name: zone_id
      in: path
      required: true
      schema: { type: string }

POST /zones:
  summary: Create zone
  requestBody:
    content:
      application/json:
        schema:
          type: object
          properties:
            name: { type: string }
            account:
              type: object
              properties:
                id: { type: string }
            plan:
              type: object
              properties:
                id: { type: string }
```

### DNS Records

```yaml
GET /zones/{zone_id}/dns_records:
  summary: List DNS records
  parameters:
    - name: zone_id
      in: path
      required: true
      schema: { type: string }
    - name: type
      in: query
      schema: { type: string, enum: [A, AAAA, CNAME, TXT, MX, NS, SRV] }
    - name: name
      in: query
      schema: { type: string }
    - name: content
      in: query
      schema: { type: string }

POST /zones/{zone_id}/dns_records:
  summary: Create DNS record
  requestBody:
    content:
      application/json:
        schema:
          type: object
          required: [type, name]
          properties:
            type: { type: string }
            name: { type: string }
            content: { type: string }
            ttl: { type: integer }
            proxied: { type: boolean }
```

### Workers

```yaml
PUT /zones/{zone_id}/workers/routes/{pattern}:
  summary: Create Workers route
  parameters:
    - name: zone_id
      in: path
      required: true
      schema: { type: string }
    - name: pattern
      in: path
      required: true
      schema: { type: string }
  requestBody:
    content:
      application/json:
        schema:
          type: object
          properties:
            script: { type: string }

PUT /accounts/{account_id}/workers/scripts/{script_name}:
  summary: Upload Worker script
  parameters:
    - name: account_id
      in: path
      required: true
      schema: { type: string }
    - name: script_name
      in: path
      required: true
      schema: { type: string }
  requestBody:
    content:
      application/javascript:
        schema: { type: string }
```

---

## Authentication

Cloudflare API uses API tokens or API keys:

```typescript
// Using API Token (recommended)
const headers = {
  'Authorization': `Bearer ${API_TOKEN}`,
  'Content-Type': 'application/json'
};

// Using API Key (legacy)
const headers = {
  'X-Auth-Key': API_KEY,
  'X-Auth-Email': EMAIL,
  'Content-Type': 'application/json'
};
```

---

## Resources

- **Cloudflare API Documentation**: https://developers.cloudflare.com/api/
- **OpenAPI Specification**: https://swagger.io/specification/
- **openapi-typescript**: https://github.com/drwpow/openapi-typescript
- **openapi-generator**: https://github.com/OpenAPITools/openapi-generator

---

## Related Documents

- [Cloudflare Workers API](../workers/00-zero-to-workers.md)
- [Cloudflare AI Gateway](../ai/00-zero-to-cloudflare-ai.md)
- [Cloudflare Tunnel](../cloudflared/00-zero-to-cloudflared.md)
