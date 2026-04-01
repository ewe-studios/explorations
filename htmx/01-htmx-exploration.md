---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/src.HTMX/htmx
repository: git@github.com:bigskysoftware/htmx.git
explored_at: 2026-03-30
language: JavaScript
category: Hypermedia-Driven Web, AJAX
---

# HTMX - Exploration

## Overview

HTMX is a **JavaScript library that extends HTML** to enable AJAX, CSS transitions, WebSockets, and Server-Sent Events directly through HTML attributes. It allows you to build modern, dynamic user interfaces without writing JavaScript, following hypermedia-driven architecture principles.

### Key Value Proposition

- **No JavaScript Framework**: Build interactivity with HTML attributes
- **Hypermedia-Driven**: HATEOAS, RESTful architecture
- **Small**: ~14k min+gzipped, zero dependencies
- **Universal Backend Support**: Works with any server language
- **Progressive Enhancement**: Enhances existing HTML
- **Composable**: Mix with other libraries

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    HTMX Architecture                             │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Browser                               │   │
│  │  ┌───────────────────────────────────────────────────┐  │   │
│  │  │              HTMX Library (~14kb)                 │  │   │
│  │  │  - Attribute parser                               │  │   │
│  │  │  - AJAX engine                                    │  │   │
│  │  │  - WebSocket/SSE handler                          │  │   │
│  │  │  - DOM swapper                                    │  │   │
│  │  │  - Event system                                   │  │   │
│  │  └───────────────────────────────────────────────────┘  │   │
│  │                           │                               │   │
│  │                           │ AJAX/WSS/SSE                  │   │
│  └───────────────────────────┼───────────────────────────────┘   │
│                              │                                    │
│                              ▼                                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Server                                │   │
│  │  - Returns HTML fragments (not JSON)                    │   │
│  │  - Any language: Go, Python, Elixir, Rust, etc.         │   │
│  │  - Full server-side rendering                           │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
htmx/
├── src/
│   └── htmx.js           # Core library (~4000 lines)
│
├── www/
│   ├── static/
│   │   ├── img/          # Logo and images
│   │   └── js/           # Documentation site JS
│   └── templates/        # Documentation pages
│
├── dist/
│   ├── htmx.js           # Full version
│   └── htmx.min.js       # Minified version
│
├── extensions/
│   ├── ws.js             # WebSocket extension
│   ├── sse.js            # Server-Sent Events extension
│   ├── json-enc.js       # JSON encoding
│   ├── path-deps.js      # Path dependencies
│   ├── loading-states.js # Loading indicators
│   └── ...               # 20+ official extensions
│
├── test/
│   ├── index.html        # Test runner
│   ├── attributes/       # Attribute-specific tests
│   ├── core/             # Core functionality tests
│   ├── ext/              # Extension tests
│   └── manual/           # Manual tests
│
└── package.json
```

## Core Concepts

### 1. AJAX Attributes

HTMX adds AJAX capability through HTML attributes:

```html
<!-- Basic GET request -->
<button hx-get="/api/users/1">
  Load User
</button>

<!-- POST request -->
<button hx-post="/api/users" hx-swap="outerHTML">
  Create User
</button>

<!-- DELETE request -->
<button hx-delete="/api/users/1" hx-swap="none">
  Delete User
</button>
```

### 2. Content Swapping

HTMX replaces content using various swap strategies:

```html
<!-- outerHTML: Replace entire element (default) -->
<div hx-get="/new" hx-swap="outerHTML">Old</div>

<!-- innerHTML: Replace content inside -->
<div hx-get="/content" hx-swap="innerHTML"></div>

<!-- beforebegin: Insert before element -->
<div hx-get="/banner" hx-swap="beforebegin"></div>

<!-- afterbegin: Insert as first child -->
<ul hx-get="/items" hx-swap="afterbegin"></ul>

<!-- beforeend: Append as last child -->
<ul hx-get="/items" hx-swap="beforeend"></ul>

<!-- afterend: Insert after element -->
<div hx-get="/footer" hx-swap="afterend"></div>
```

### 3. Triggers

Control when requests are issued:

```html
<!-- Default: click -->
<button hx-get="/clicked">Click Me</button>

<!-- Other events -->
<input hx-get="/search" hx-trigger="keyup">
<input hx-get="/search" hx-trigger="keyup[key=='Enter']">

<!-- With debounce -->
<input hx-get="/search" hx-trigger="keyup changed delay:500ms">

<!-- With throttle -->
<button hx-post="/save" hx-trigger="click throttle:1s">Save</button>

<!-- Load on page load -->
<div hx-get="/init" hx-trigger="load once"></div>

<!-- From different element -->
<input type="text" id="search-input">
<div hx-get="/search" hx-trigger="keyup from:#search-input"></div>
```

### 4. Targets

Specify where response should be swapped:

```html
<!-- Target self (default) -->
<button hx-get="/update">Update Me</button>

<!-- Target different element -->
<button hx-get="/update" hx-target="#content">Update Content</button>

<!-- Target closest parent -->
<button hx-get="/update" hx-target="closest tr">Update Row</button>

<!-- Target next sibling -->
<button hx-get="/details" hx-target="next div">Show Details</button>

<!-- Target previous sibling -->
<button hx-get="/summary" hx-target="previous div">Show Summary</button>
```

### 5. Indicators

Show loading state:

```html
<style>
  .htmx-indicator { display: none; }
  .htmx-request .htmx-indicator { display: inline; }
  .htmx-request.htmx-indicator { display: inline; }
</style>

<button hx-get="/slow" hx-indicator="#loading">
  Click Me
</button>
<span id="loading" class="htmx-indicator">
  Loading...
</span>
```

## Request/Response Cycle

### Request Flow

```
1. User triggers event (click, keyup, etc.)
         ↓
2. HTMX intercepts event
         ↓
3. Creates AJAX request with headers:
   - HX-Request: true
   - HX-Trigger: <element-id>
   - HX-Trigger-Name: <element-name>
   - HX-Target: <target-id>
   - HX-Current-URL: <current-url>
         ↓
4. Server receives request
         ↓
5. Server returns HTML fragment
```

### Response Headers

Server can control behavior via headers:

```
HX-Location: /redirect-url      # Client-side redirect
HX-Push-Url: /new-url           # Push to history
HX-Redirect: /server-redirect   # Server redirect
HX-Refresh: true                # Full page refresh
HX-Reswap: innerHTML            # Change swap method
HX-Retarget: #other             # Change target element
HX-Trigger: event-name          # Trigger client event
```

## Forms

### Basic Form

```html
<form hx-post="/api/users" hx-swap="outerHTML">
  <input type="text" name="name" placeholder="Name">
  <input type="email" name="email" placeholder="Email">
  <button type="submit">Create User</button>
</form>
```

### Form with Validation

```html
<form hx-post="/api/users" hx-swap="none">
  <!-- Inline field validation -->
  <input
    type="email"
    name="email"
    hx-post="/validate/email"
    hx-trigger="blur"
    hx-target="#email-error"
    placeholder="Email"
  >
  <div id="email-error" style="color: red;"></div>

  <button type="submit">Submit</button>

  <!-- Error display -->
  <div id="form-errors"></div>
</form>
```

### Server Validation Response

```html
<!-- Valid: Return empty div -->
<div></div>

<!-- Invalid: Return error message -->
<div style="color: red;">Email already taken</div>
```

## Patterns

### Click to Edit

```html
<!-- Display Mode -->
<span id="user-1">
  John Doe
  <button hx-get="/api/users/1/edit" hx-target="#user-1">
    Edit
  </button>
</span>

<!-- Server returns Edit Mode -->
<span id="user-1">
  <form hx-put="/api/users/1" hx-target="#user-1" hx-swap="outerHTML">
    <input type="text" name="name" value="John Doe" autofocus>
    <button type="submit">Save</button>
    <button type="button" hx-get="/api/users/1/view" hx-target="#user-1">
      Cancel
    </button>
  </form>
</span>
```

### Infinite Scroll

```html
<div id="items" hx-get="/items?page=2" hx-trigger="revealed" hx-swap="beforeend">
  <!-- Items load as user scrolls -->
</div>
```

### Delete Row

```html
<table>
  <tr id="user-1">
    <td>John Doe</td>
    <td>
      <button
        hx-delete="/api/users/1"
        hx-target="#user-1"
        hx-swap="outerHTML"
        hx-confirm="Are you sure?"
      >
        Delete
      </button>
    </td>
  </tr>
</table>

<!-- Server returns empty to remove row -->
```

### Batch Operations

```html
<div id="todos">
  <label><input type="checkbox" name="ids" value="1"> Task 1</label>
  <label><input type="checkbox" name="ids" value="2"> Task 2</label>
  <label><input type="checkbox" name="ids" value="3"> Task 3</label>
</div>

<button
  hx-post="/todos/complete"
  hx-include="[name='ids']"
  hx-target="#todos"
>
  Complete Selected
</button>
```

## Extensions

### WebSocket Extension

```html
<script src="/ext/ws.js"></script>

<div hx-ext="ws" ws-connect="/chat">
  <div ws-send>
    <input name="message" placeholder="Type...">
    <button>Send</button>
  </div>

  <div id="messages" ws-swap="message"></div>
</div>
```

### Server-Sent Events

```html
<script src="/ext/sse.js"></script>

<div hx-ext="sse" sse-connect="/notifications">
  <div sse-swap="new-message"></div>
  <div sse-swap="user-joined"></div>
  <div sse-swap="user-left"></div>
</div>
```

### Client-Side Templates

```html
<script src="/ext/client-side-templates.js"></script>

<div
  hx-get="/api/users"
  hx-ext="client-side-templates"
  mustache-template="user-template"
>
</div>

<template id="user-template">
  <div>{{name}} - {{email}}</div>
</template>
```

### Path Dependencies

```html
<script src="/ext/path-deps.js"></script>

<!-- This element refreshes when /api/users is modified -->
<div hx-ext="path-deps" hx-trigger="path:/api/users" hx-get="/users/list">
  User List
</div>

<!-- This button triggers the refresh -->
<button hx-post="/api/users" hx-path-deps>
  Create User
</button>
```

## Events

HTMX fires events during request lifecycle:

```javascript
// Listen for HTMX events
document.body.addEventListener('htmx:configRequest', (evt) => {
  // Before request is sent
  evt.detail.headers['X-Custom-Header'] = 'value'
})

document.body.addEventListener('htmx:afterRequest', (evt) => {
  // After request completes
  console.log('Request completed:', evt.detail.xhr.status)
})

document.body.addEventListener('htmx:beforeSwap', (evt) => {
  // Before content is swapped
  if (evt.detail.xhr.status === 422) {
    evt.detail.shouldSwap = true
    evt.detail.isError = false
  }
})

// Custom event triggering
document.body.addEventListener('content-updated', () => {
  // React to content changes
})
```

## Testing

### Automated Tests

```html
<!-- test/attributes/hx-get.html -->
<script>
  describe('hx-get tests', function() {
    beforeEach(function() {
      this.server = makeServer()
      clearWorkArea()
    })

    it('issues GET request on click', function() {
      this.server.respondWith('GET', '/test', 'Clicked!')

      var btn = make('<button hx-get="/test">Click</button>')
      btn.click()

      this.server.respond()
      btn.innerHTML.should.equal('Clicked!')
    })
  })
</script>
```

### Manual Testing

```html
<!-- test/manual/index.html -->
<!DOCTYPE html>
<html>
<head>
  <script src="/src/htmx.js"></script>
</head>
<body>
  <h1>Manual Test: Click to Edit</h1>
  <div id="user-1">
    John Doe
    <button hx-get="/api/users/1/edit" hx-target="#user-1">Edit</button>
  </div>
</body>
</html>
```

## Production Considerations

### Performance

- **Bundle Size**: 14k min+gz, smaller than most JS frameworks
- **No Virtual DOM**: Direct DOM manipulation
- **Event Delegation**: Single listener at document level
- **Lazy Loading**: Extensions loaded on demand

### Security

```html
<!-- CSRF Protection -->
<meta name="csrf-token" content="{{ csrf_token }}">
<script>
  document.body.addEventListener('htmx:configRequest', (evt) => {
    evt.detail.headers['X-CSRF-Token'] = document.querySelector('meta[name="csrf-token"]').content
  })
</script>
```

### Caching

```html
<!-- Cache control via headers -->
Cache-Control: no-cache, no-store, must-revalidate

<!-- Or cache specific responses -->
Cache-Control: public, max-age=300
```

---

## Related Deep Dives

- [00-zero-to-htmx-engineer.md](./00-zero-to-htmx-engineer.md) - Fundamentals
- [02-htmx-extensions-deep-dive.md](./02-htmx-extensions-deep-dive.md) - Extensions system
- [03-htmx-vs-liveview-deep-dive.md](./03-htmx-vs-liveview-deep-dive.md) - Comparison with LiveView
