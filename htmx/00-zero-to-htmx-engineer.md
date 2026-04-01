---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/htmx
explored_at: 2026-03-30
prerequisites: Basic HTML knowledge, Web fundamentals (HTTP, AJAX)
---

# Zero to HTMX Engineer - Complete Fundamentals

## Table of Contents

1. [What is HTMX?](#what-is-htmx)
2. [Why HTMX?](#why-htmx)
3. [Installation](#installation)
4. [Your First HTMX Request](#your-first-htmx-request)
5. [HTTP Methods](#http-methods)
6. [Content Swapping](#content-swapping)
7. [Triggers & Events](#triggers--events)
8. [Forms & Validation](#forms--validation)
9. [WebSockets & SSE](#websockets--sse)

## What is HTMX?

HTMX is a **JavaScript library that extends HTML** to enable AJAX, CSS transitions, WebSockets, and Server-Sent Events directly through HTML attributes. It allows you to build modern, dynamic user interfaces without writing JavaScript.

### The Problem HTMX Solves

**Traditional Web Development:**
```
HTML → Static, boring
JavaScript → Write fetch(), event listeners, DOM manipulation
React/Vue → Build tools, npm, bundlers, state management
Complexity: Two languages, build pipelines, framework lock-in
```

**HTMX Approach:**
```
HTML + HTMX attributes → Dynamic, interactive
Server returns HTML fragments → No JSON parsing
Simplicity: HTML is the interface AND the logic
```

### Key Concepts

| Term | Definition |
|------|------------|
| **hx-get** | Issue GET request on trigger |
| **hx-post** | Issue POST request on trigger |
| **hx-swap** | How to swap response into DOM |
| **hx-trigger** | What event triggers the request |
| **hx-target** | Where to swap the response |
| **hx-indicator** | Show loading indicator |

## Why HTMX?

### Benefits

1. **No JavaScript Framework**: Build interactivity with HTML attributes
2. **Server-Rendered**: Full HTML from server, SEO-friendly
3. **Small**: ~14k min+gzipped, no dependencies
4. **Progressive Enhancement**: Works without JS, enhances with it
5. **Composable**: Mix with any backend (Python, Go, Elixir, etc.)
6. **Hypermedia-Driven**: HATEOAS, RESTful architecture

### When to Use HTMX

**Good fit:**
- Dynamic forms with validation
- Infinite scroll / pagination
- Inline editing
- Real-time notifications
- Partial page updates
- Click-to-edit patterns

**Not recommended:**
- Offline-first apps
- Heavy client-side logic
- Mobile apps (native/React Native)
- Games or canvas-heavy UIs

## Installation

### CDN (Simplest)

```html
<script src="https://unpkg.com/htmx.org@2.0.2"></script>
```

### npm Install

```bash
npm install htmx.org --save
```

```javascript
// In your JS bundle
import 'htmx.org'
```

### Verify Installation

```html
<!DOCTYPE html>
<html>
<head>
  <script src="https://unpkg.com/htmx.org@2.0.2"></script>
</head>
<body>
  <!-- If this button makes AJAX request, HTMX works! -->
  <button hx-get="/clicked" hx-swap="outerHTML">
    Click Me
  </button>
</body>
</html>
```

## Your First HTMX Request

### Basic GET Request

```html
<!-- Button issues GET request, replaces itself -->
<button hx-get="/clicked" hx-swap="outerHTML">
  Click Me
</button>
```

**Server response:**
```html
<button hx-get="/clicked-again" hx-swap="outerHTML">
  Clicked! Click again
</button>
```

### What Happens:

1. User clicks button
2. HTMX issues GET to `/clicked`
3. Server returns HTML fragment
4. HTMX replaces button with response

### With Loading Indicator

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

## HTTP Methods

### GET Request

```html
<button hx-get="/api/users/1">
  Load User
</button>
```

### POST Request

```html
<button hx-post="/api/users" hx-swap="outerHTML">
  Create User
</button>
```

### PUT Request

```html
<button hx-put="/api/users/1" hx-swap="outerHTML">
  Update User
</button>
```

### DELETE Request

```html
<button hx-delete="/api/users/1" hx-swap="outerHTML">
  Delete User
</button>
```

### PATCH Request

```html
<button hx-patch="/api/users/1" hx-swap="outerHTML">
  Patch User
</button>
```

## Content Swapping

### outerHTML (Default)

```html
<!-- Replace entire element -->
<button hx-get="/new" hx-swap="outerHTML">
  Replace Me
</button>
```

### innerHTML

```html
<!-- Replace content inside element -->
<div hx-get="/content" hx-swap="innerHTML">
  <!-- Content replaced, div stays -->
</div>
```

### beforebegin / afterbegin / beforeend / afterend

```html
<!-- Insert at specific position -->
<ul hx-get="/items" hx-swap="beforeend">
  <!-- New items appended -->
</ul>

<div hx-get="/banner" hx-swap="beforebegin">
  <!-- Banner inserted before -->
</div>
```

### delete

```html
<!-- Remove element after delay -->
<button hx-delete="/api/item/1" hx-swap="delete" hx-swap-oob="true">
  Delete
</button>
```

### none

```html
<!-- No swap, just trigger event -->
<button hx-post="/api/track" hx-swap="none">
  Track Click
</button>
```

## Triggers & Events

### Basic Triggers

```html
<!-- Click (default) -->
<button hx-get="/clicked">Click Me</button>

<!-- Different event -->
<input hx-get="/search" hx-trigger="keyup">

<!-- Specific key -->
<input hx-get="/search" hx-trigger="keyup[key=='Enter']">

<!-- With delay (debounce) -->
<input hx-get="/search" hx-trigger="keyup changed delay:500ms">

<!-- Throttle -->
<button hx-post="/save" hx-trigger="click throttle:1s">
  Save
</button>

<!-- Once -->
<div hx-get="/init" hx-trigger="load once">
  Load once on page load
</div>
```

### hx-trigger Modifiers

| Modifier | Description |
|----------|-------------|
| `changed` | Only if value changed |
| `delay:500ms` | Debounce |
| `throttle:1s` | Throttle |
| `from:body` | Listen on different element |
| `target:.input` | Listen on specific selector |
| `once` | Only trigger once |

### Custom Events

```html
<!-- Listen for custom event -->
<div hx-get="/refresh" hx-trigger="content-updated from:body">
  Content
</div>

<!-- Trigger from JavaScript -->
<script>
  document.body.dispatchEvent(new CustomEvent('content-updated'))
</script>
```

## Forms & Validation

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
  <div>
    <input
      type="email"
      name="email"
      hx-post="/validate/email"
      hx-trigger="blur"
      hx-target="#email-error"
      placeholder="Email"
    >
    <div id="email-error" style="color: red;"></div>
  </div>

  <button type="submit">Submit</button>
</form>
```

### Inline Edit Pattern

```html
<!-- Display mode -->
<span id="user-name-1">
  John Doe
  <button hx-get="/api/users/1/edit" hx-target="#user-name-1">
    Edit
  </button>
</span>

<!-- Edit mode (server returns this) -->
<span id="user-name-1">
  <form hx-put="/api/users/1" hx-target="#user-name-1" hx-swap="outerHTML">
    <input type="text" name="name" value="John Doe" autofocus>
    <button type="submit">Save</button>
    <button type="button" hx-get="/api/users/1/view" hx-target="#user-name-1">
      Cancel
    </button>
  </form>
</span>
```

## WebSockets & SSE

### WebSockets Extension

```html
<!-- Include extension -->
<script src="https://unpkg.com/htmx.org@2.0.2"></script>
<script src="https://unpkg.com/htmx.org/dist/ext/ws.js"></script>

<!-- Connect to WebSocket -->
<div hx-ext="ws" ws-connect="/chat">
  <!-- Messages arrive via WebSocket -->
  <div ws-send>
    <input name="message" placeholder="Type...">
    <button>Send</button>
  </div>

  <div id="messages"></div>
</div>
```

### Server-Sent Events

```html
<script src="https://unpkg.com/htmx.org/dist/ext/sse.js"></script>

<div hx-ext="sse" sse-connect="/notifications">
  <!-- Subscribe to event stream -->
  <div sse-swap="new-message">
    Waiting for messages...
  </div>

  <div sse-swap="user-joined">
    Waiting for users...
  </div>
</div>
```

### Server Response (SSE)

```
data: <div>New message from Alice!</div>
event: new-message

data: <div>Bob joined the room</div>
event: user-joined
```

---

**Next Steps:**
- [01-htmx-exploration.md](./01-htmx-exploration.md) - Full architecture
- [02-htmx-extensions-deep-dive.md](./02-htmx-extensions-deep-dive.md) - Extensions system
- [03-htmx-vs-liveview-deep-dive.md](./03-htmx-vs-liveview-deep-dive.md) - Comparison with LiveView
