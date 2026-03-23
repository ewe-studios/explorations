# Security Deep Dive: Sandboxing and CSP

## Overview

MCP Apps implements a multi-layered security model based on:
1. **Origin isolation** - Different origins at each layer
2. **Iframe sandboxing** - Restricted browser capabilities
3. **Content Security Policy** - Network access control
4. **Permission Policy** - Feature access control
5. **Message validation** - Origin verification on postMessage

## Double-Iframe Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Host (Parent Window)                                       │
│  Origin: https://claude.ai                                │
│                                                             │
│  ┌───────────────────────────────────────────────────────┐ │
│  │  Sandbox Proxy (Outer Iframe)                         │ │
│  │  Origin: https://sandbox.mcp-internal.com            │ │
│  │  Sandbox: allow-scripts allow-same-origin             │ │
│  │                                                       │ │
│  │  ┌─────────────────────────────────────────────────┐ │ │
│  │  │  View (Inner Iframe)                            │ │ │
│  │  │  Origin: Same as Sandbox (via same-origin)      │ │ │
│  │  │  Sandbox: allow-scripts allow-same-origin       │ │ │
│  │  │           allow-forms (optional)                │ │ │
│  │  │  CSP: As declared in _meta.ui.csp               │ │ │
│  │  └─────────────────────────────────────────────────┘ │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Why Double Iframe?

1. **Outer Sandbox Proxy**
   - Different origin from host (security boundary)
   - CSP enforcement via HTTP headers (tamper-proof)
   - Message relay between host and view
   - Creates inner iframe with controlled sandbox

2. **Inner View Iframe**
   - Same origin as sandbox (allows document.write)
   - Runs untrusted HTML/JS
   - Restricted by sandbox attribute
   - CSP limits network access

## Sandbox Attribute

### Outer Iframe (Sandbox Proxy)

```html
<iframe
  sandbox="allow-scripts allow-same-origin"
  src="https://sandbox-proxy.example.com/sandbox.html"
>
```

**Permissions:**
- `allow-scripts` - Required for JavaScript execution
- `allow-same-origin` - Allows access to localStorage, cookies
- NO `allow-forms` - Forms cannot submit (outer frame)
- NO `allow-popups` - Cannot open popups
- NO `allow-top-navigation` - Cannot navigate parent

### Inner Iframe (View)

```html
<iframe
  sandbox="allow-scripts allow-same-origin allow-forms"
  allow="clipboard-write; geolocation"
>
```

**Permissions (configurable):**
- `allow-scripts` - Always required
- `allow-same-origin` - Required for MCP communication
- `allow-forms` - If view needs form submission
- `allow-popups` - Never granted by default

## Content Security Policy (CSP)

### Declaration in Resource Metadata

```typescript
registerAppResource(server, 'dashboard', 'ui://dashboard', {}, async () => ({
  contents: [{
    uri: 'ui://dashboard',
    mimeType: 'text/html;profile=mcp-app',
    text: dashboardHtml,
    _meta: {
      ui: {
        csp: {
          connectDomains: ['https://api.example.com'],
          resourceDomains: ['https://cdn.jsdelivr.net', 'https://*.cloudflare.com'],
          frameDomains: ['https://www.youtube.com'],
          baseUriDomains: ['https://cdn.example.com']
        }
      }
    }
  }]
}));
```

### CSP Directives Mapping

| Meta Field | CSP Directive | Purpose |
|------------|---------------|---------|
| `connectDomains` | `connect-src` | Fetch, XHR, WebSocket |
| `resourceDomains` | `img-src`, `script-src`, `style-src`, `font-src`, `media-src` | Static resources |
| `frameDomains` | `frame-src` | Nested iframes |
| `baseUriDomains` | `base-uri` | Document base URI |

### Default CSP (No Metadata)

If `_meta.ui.csp` is omitted:

```
default-src 'none';
script-src 'self' 'unsafe-inline';
style-src 'self' 'unsafe-inline';
img-src 'self' data:;
media-src 'self' data:;
connect-src 'none';
object-src 'none';
frame-src 'none';
base-uri 'self';
```

### CSP Header Construction

The sandbox proxy constructs CSP headers:

```typescript
// In sandbox proxy (outer iframe)
function buildCspHeader(csp: McpUiResourceCsp): string {
  const directives: string[] = [];

  // Default to restrictive
  directives.push("default-src 'none'");

  // Scripts and styles from self + inline (required for HTML UIs)
  directives.push("script-src 'self' 'unsafe-inline'");
  directives.push("style-src 'self' 'unsafe-inline'");

  // Images from self + data URIs
  directives.push("img-src 'self' data:");

  // Connect domains (fetch/XHR/WebSocket)
  if (csp.connectDomains?.length) {
    directives.push(`connect-src ${csp.connectDomains.join(' ')}`);
  } else {
    directives.push("connect-src 'none'");
  }

  // Resource domains
  if (csp.resourceDomains?.length) {
    for (const domain of csp.resourceDomains) {
      directives.push(`script-src ${domain}`);
      directives.push(`style-src ${domain}`);
      directives.push(`img-src ${domain}`);
      directives.push(`font-src ${domain}`);
    }
  }

  // Frame domains
  if (csp.frameDomains?.length) {
    directives.push(`frame-src ${csp.frameDomains.join(' ')}`);
  } else {
    directives.push("frame-src 'none'");
  }

  // Base URI domains
  if (csp.baseUriDomains?.length) {
    directives.push(`base-uri ${csp.baseUriDomains.join(' ')}`);
  } else {
    directives.push("base-uri 'self'");
  }

  // Block plugins
  directives.push("object-src 'none'");

  return directives.join('; ');
}
```

### CSP via HTTP Headers vs Meta Tags

**HTTP Headers (Recommended):**
```http
Content-Security-Policy: default-src 'none'; script-src 'self' 'unsafe-inline'...
```

Set by sandbox proxy server based on `?csp=<json>` query parameter:
```
GET /sandbox.html?csp={"connectDomains":["https://api.example.com"]}
```

**Meta Tags (Fallback):**
```html
<meta http-equiv="Content-Security-Policy"
      content="default-src 'none'; script-src 'self' 'unsafe-inline'...">
```

Less secure - can be bypassed by malicious content.

## Permission Policy

### Declaration

```typescript
_meta: {
  ui: {
    permissions: {
      camera?: {},
      microphone?: {},
      geolocation?: {},
      clipboardWrite?: {}
    }
  }
}
```

### Iframe Allow Attribute

The sandbox proxy sets the inner iframe's `allow` attribute:

```html
<iframe
  sandbox="allow-scripts allow-same-origin allow-forms"
  allow="camera; microphone; geolocation; clipboard-write"
>
```

### Feature Detection in Views

Views should NOT assume permissions are granted:

```javascript
// Check for geolocation support
if ('geolocation' in navigator) {
  navigator.geolocation.getCurrentPosition(success, error);
} else {
  // Fallback behavior
}

// Check for camera access
if (navigator.mediaDevices?.getUserMedia) {
  navigator.mediaDevices.getUserMedia({ video: true })
    .catch(err => {
      // Permission denied or not available
    });
}
```

## Origin Validation

### Sandbox Proxy Validation

```typescript
// In sandbox.ts (ext-apps reference implementation)
const ALLOWED_REFERRER_PATTERN = /^http:\/\/(localhost|127\.0\.0\.1)(:|\/|$)/;

if (!document.referrer) {
  throw new Error("No referrer, cannot validate embedding site.");
}

if (!document.referrer.match(ALLOWED_REFERRER_PATTERN)) {
  throw new Error("Embedding domain not allowed in referrer.");
}

const EXPECTED_HOST_ORIGIN = new URL(document.referrer).origin;
```

### Message Source Validation

```typescript
// Relay messages from parent to inner iframe
window.addEventListener("message", async (event) => {
  // Validate messages from parent
  if (event.source === window.parent) {
    if (event.origin !== EXPECTED_HOST_ORIGIN) {
      console.error("[Sandbox] Rejecting message from unexpected origin");
      return;
    }
    // Relay to inner iframe
    inner.contentWindow.postMessage(event.data, "*");
  }

  // Validate messages from inner iframe
  if (event.source === inner.contentWindow) {
    if (event.origin !== OWN_ORIGIN) {
      console.error("[Sandbox] Rejecting message from inner iframe");
      return;
    }
    // Relay to parent
    window.parent.postMessage(event.data, EXPECTED_HOST_ORIGIN);
  }
});
```

### View-Side Validation

```typescript
// In the View (inner iframe)
window.addEventListener('message', (event) => {
  // Only accept from sandbox proxy
  if (event.origin !== window.location.origin) {
    return; // Ignore messages from unexpected origins
  }

  // Process message
  handleMcpMessage(event.data);
});
```

## Security Self-Test

The sandbox proxy verifies its own isolation:

```typescript
// In sandbox.ts
try {
  window.top!.alert("If you see this, the sandbox is not setup securely.");
  throw "FAIL";
} catch (e) {
  if (e === "FAIL") {
    throw new Error("The sandbox is not setup securely.");
  }
  // Expected: SecurityError confirms proper sandboxing
}
```

This test ensures `window.top` is inaccessible - if it throws SecurityError, the sandbox is working correctly.

## Threat Model

### Threat 1: Malicious View Content

**Attack:** Server serves malicious JavaScript in UI resource

**Mitigations:**
- Sandboxed iframe cannot access host DOM
- Cannot navigate parent window
- Cannot open popups
- CSP limits network exfiltration
- No access to cookies/storage from other origins

### Threat 2: Data Exfiltration

**Attack:** View tries to send data to attacker's server

**Mitigations:**
- CSP `connect-src` blocks undeclared domains
- `img-src` blocks image-based exfiltration
- `frame-src` blocks nested iframe attacks
- Host can audit CSP declarations

### Threat 3: Clickjacking

**Attack:** Transparent overlay tricks user interaction

**Mitigations:**
- Single-origin isolation
- Host controls iframe positioning
- No `allow-top-navigation`
- Host can implement additional protections

### Threat 4: Privilege Escalation

**Attack:** View escapes sandbox to access host

**Mitigations:**
- Double-iframe architecture
- Origin validation on all messages
- Security self-test on sandbox load
- `postMessage` only communication channel

### Threat 5: SSRF (Server-Side Request Forgery)

**Attack:** External URL resource fetches internal resources

**Mitigations:**
- TypeScript SDK validates URLs (blocks private IPs, localhost)
- Response size limits
- Timeout on fetches
- Server developers should use URL allowlists

## Production Checklist

### For Server Developers

- [ ] Declare all required `connectDomains`
- [ ] Declare all required `resourceDomains`
- [ ] Use wildcard subdomains where appropriate: `https://*.example.com`
- [ ] Specify `prefersBorder` for consistent appearance
- [ ] Set `domain` for stable OAuth/CORS origins
- [ ] Minimize `permissions` - request only what's needed
- [ ] Validate external URLs before using `externalUrl` content type

### For Host Developers

- [ ] Implement double-iframe sandbox
- [ ] Validate referrer origins
- [ ] Parse and enforce CSP from metadata
- [ ] Set sandbox attributes correctly
- [ ] Validate message origins
- [ ] Implement security self-test
- [ ] Log CSP configurations for audit
- [ ] Handle tool visibility correctly

### For View Developers

- [ ] Use feature detection for permissions
- [ ] Validate message origins
- [ ] Handle CSP restrictions gracefully
- [ ] Fallback when features unavailable
- [ ] Don't assume network access
- [ ] Test in restrictive CSP environment

## Example: Secure Map Server

```typescript
const mapUI = await createUIResource({
  uri: 'ui://map/viewer',
  encoding: 'text',
  content: {
    type: 'rawHtml',
    htmlString: `
      <!DOCTYPE html>
      <html>
      <head>
        <meta http-equiv="Content-Security-Policy"
              content="default-src 'self';
                       connect-src https://api.mapbox.com;
                       img-src https://*.mapbox.com data:;">
        <script src="https://api.mapbox.com/mapbox-gl-js/v2.0.0/mapbox-gl.js"></script>
      </head>
      <body>
        <div id="map"></div>
        <script>
          // Map viewer code using Mapbox API
        </script>
      </body>
      </html>
    `
  },
  _meta: {
    ui: {
      csp: {
        connectDomains: ['https://api.mapbox.com'],
        resourceDomains: ['https://api.mapbox.com', 'https://*.mapbox.com']
      }
    }
  }
});
```

## Related Documentation

- [CSP and CORS Guide](https://apps.extensions.modelcontextprotocol.io/api/documents/CSPandCORS.html)
- [Authorization Patterns](./authorization.md)
- [Specification: Security Implications](https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/2026-01-26/apps.mdx#security-implications)
