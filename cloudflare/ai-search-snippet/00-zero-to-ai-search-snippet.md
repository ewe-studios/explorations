# Zero to Cloudflare AI Search Snippet: Complete Guide

**Last Updated:** 2026-04-05

---

## Table of Contents

1. [Introduction](#introduction)
2. [What is AI Search Snippet?](#what-is-ai-search-snippet)
3. [Architecture](#architecture)
4. [Installation](#installation)
5. [Quick Start](#quick-start)
6. [Components](#components)
7. [API Reference](#api-reference)
8. [Customization](#customization)
9. [Framework Integration](#framework-integration)
10. [API Server Requirements](#api-server-requirements)

---

## Introduction

Cloudflare AI Search Snippet is a **production-ready, self-contained TypeScript Web Component library** providing search and chat interfaces with streaming support. It's zero-dependency, fully customizable, and framework-agnostic.

```bash
npm install @cloudflare/ai-search-snippet
```

---

## What is AI Search Snippet?

### Features

| Feature | Description |
|---------|-------------|
| **Zero Dependencies** | Self-contained, everything bundled |
| **Framework Agnostic** | Native Web Components work everywhere |
| **Streaming Support** | Real-time streaming responses |
| **Accessible** | WCAG 2.1 AA compliant with ARIA |
| **Dark Mode** | Automatic theme switching |
| **Tiny Bundle** | < 50KB gzipped |
| **TypeScript** | Full type definitions included |
| **XSS Protection** | HTML sanitization built-in |

### Components

| Component | Tag | Description |
|-----------|-----|-------------|
| SearchBar | `<search-bar-snippet>` | Search input with results dropdown |
| SearchModal | `<search-modal-snippet>` | Modal search with Cmd/Ctrl+K |
| ChatBubble | `<chat-bubble-snippet>` | Floating chat bubble overlay |
| ChatPage | `<chat-page-snippet>` | Full-page chat with history |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    AI Search Snippet                         │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  Components Layer                    │   │
│  │  - search-bar-snippet.ts                             │   │
│  │  - search-modal-snippet.ts                           │   │
│  │  - chat-bubble-snippet.ts                            │   │
│  │  - chat-page-snippet.ts                              │   │
│  └─────────────────────────────────────────────────────┘   │
│                            ↑                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   API Layer                          │   │
│  │  - ai-search.ts (streaming client)                   │   │
│  │  - Base Client abstraction                           │   │
│  └─────────────────────────────────────────────────────┘   │
│                            ↑                                │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                  Styles Layer                        │   │
│  │  - theme.ts (CSS variables)                          │   │
│  │  - search.ts, chat.ts (component styles)             │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## Installation

### npm/yarn

```bash
npm install @cloudflare/ai-search-snippet
# or
yarn add @cloudflare/ai-search-snippet
```

### CDN

```html
<script type="module" src="https://<hash>/search.ai.cloudflare.com/search-snippet.es.js"></script>
```

---

## Quick Start

### Basic Search Bar

```html
<!-- Import the library -->
<script type="module" src="https://<hash>/search-snippet.es.js"></script>

<!-- Search bar with results -->
<search-bar-snippet
  api-url="https://<hash>/search.ai.cloudflare.com/"
  placeholder="Search..."
  max-results="10"
></search-bar-snippet>
```

### Modal Search with Keyboard Shortcut

```html
<search-modal-snippet
  api-url="https://<hash>/search.ai.cloudflare.com/"
  placeholder="Search documentation..."
  shortcut="k"
  max-results="10"
></search-modal-snippet>
```

### Chat Bubble

```html
<chat-bubble-snippet
  api-url="https://<hash>/search.ai.cloudflare.com/"
  placeholder="Ask a question..."
></chat-bubble-snippet>
```

---

## Components

### SearchBarSnippet

```html
<search-bar-snippet
  api-url="https://api.example.com"
  placeholder="Search..."
  max-results="10"
  debounce-ms="300"
  show-url="false"
  theme="auto"
></search-bar-snippet>
```

**JavaScript API:**

```typescript
const searchBar = document.querySelector('search-bar-snippet');
await searchBar.search('query');
```

### SearchModalSnippet

```html
<search-modal-snippet
  api-url="https://api.example.com"
  placeholder="Search..."
  shortcut="k"
  use-meta-key="true"
  max-results="10"
></search-modal-snippet>
```

**JavaScript API:**

```typescript
const modal = document.querySelector('search-modal-snippet');
modal.open();
modal.close();
modal.search('query');
const results = modal.getResults();
const isOpen = modal.isModalOpen();
```

### ChatBubbleSnippet

```html
<chat-bubble-snippet
  api-url="https://api.example.com"
  placeholder="Type a message..."
></chat-bubble-snippet>
```

**JavaScript API:**

```typescript
const chatBubble = document.querySelector('chat-bubble-snippet');
await chatBubble.sendMessage('Hello!');
const messages = chatBubble.getMessages();
chatBubble.clearChat();
```

### ChatPageSnippet

```html
<chat-page-snippet
  api-url="https://api.example.com"
  placeholder="Type a message..."
></chat-page-snippet>
```

**JavaScript API:**

```typescript
const chatPage = document.querySelector('chat-page-snippet');
await chatPage.sendMessage('Hello!');
const messages = chatPage.getMessages();
const sessions = chatPage.getSessions();
const current = chatPage.getCurrentSession();
```

---

## API Reference

### Common Attributes

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `api-url` | string | `localhost:3000` | API endpoint URL |
| `placeholder` | string | Component-specific | Input placeholder |
| `theme` | `light \| dark \| auto` | `auto` | Color scheme |
| `hide-branding` | boolean | `false` | Hide "Powered by" |

### Search Attributes

| Attribute | Type | Default | Description |
|-----------|------|---------|-------------|
| `max-results` | number | `10` | Max search results |
| `debounce-ms` | number | `300` | Input debounce delay |
| `show-url` | boolean | `false` | Show URL in results |

### Events

```javascript
// Common events
component.addEventListener('ready', () => console.log('Ready'));
component.addEventListener('error', (e) => console.error(e.detail.error));

// Modal events
modal.addEventListener('open', () => console.log('Opened'));
modal.addEventListener('close', () => console.log('Closed'));
modal.addEventListener('result-select', (e) => console.log(e.detail.result));

// Chat events
chat.addEventListener('message', (e) => console.log(e.detail.message));
```

---

## Customization

### CSS Variables

```css
search-bar-snippet,
search-modal-snippet,
chat-bubble-snippet,
chat-page-snippet {
  /* Primary Colors */
  --search-snippet-primary-color: #2563eb;
  --search-snippet-primary-hover: #0f51dfff;
  
  /* Background & Surface */
  --search-snippet-background: #ffffff;
  --search-snippet-surface: #f8f9fa;
  --search-snippet-hover-background: #f1f3f5;
  
  /* Text */
  --search-snippet-text-color: #212529;
  --search-snippet-text-secondary: #6c757d;
  
  /* Border & Focus */
  --search-snippet-border-color: #dee2e6;
  --search-snippet-focus-ring: #0066cc40;
  
  /* Typography */
  --search-snippet-font-family: -apple-system, BlinkMacSystemFont, "Segoe UI";
  --search-snippet-font-size-base: 14px;
  --search-snippet-line-height: 1.5;
  
  /* Spacing */
  --search-snippet-spacing-xs: 4px;
  --search-snippet-spacing-sm: 8px;
  --search-snippet-spacing-md: 12px;
  --search-snippet-spacing-lg: 16px;
  
  /* Shadows */
  --search-snippet-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  --search-snippet-shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.2);
  
  /* Border Radius */
  --search-snippet-border-radius: 18px;
  
  /* Transitions */
  --search-snippet-transition: 200ms ease;
}
```

### Dark Theme Example

```css
search-bar-snippet {
  --search-snippet-primary-color: #4dabf7;
  --search-snippet-background: #1a1b1e;
  --search-snippet-text-color: #c1c2c5;
  --search-snippet-border-color: #373a40;
}
```

### Chat Bubble Specific

```css
chat-bubble-snippet {
  --chat-bubble-button-size: 60px;
  --chat-bubble-button-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
  --chat-bubble-button-bottom: 20px;
  --chat-bubble-button-right: 20px;
  --chat-bubble-button-z-index: 9999;
}
```

---

## Framework Integration

### React

```tsx
import { useEffect, useRef } from 'react';
import '@cloudflare/ai-search-snippet';

function ChatWidget() {
  const ref = useRef<HTMLElement>(null);

  useEffect(() => {
    const chat = ref.current;
    
    const handleMessage = (e: CustomEvent) => {
      console.log('Message:', e.detail);
    };

    chat?.addEventListener('message', handleMessage as EventListener);
    
    return () => {
      chat?.removeEventListener('message', handleMessage as EventListener);
    };
  }, []);

  return (
    <chat-bubble-snippet
      ref={ref}
      api-url="https://api.example.com"
      placeholder="Ask a question..."
    />
  );
}
```

### Vue

```vue
<template>
  <chat-bubble-snippet
    :api-url="apiUrl"
    placeholder="Ask a question..."
    @message="handleMessage"
    @error="handleError"
  />
</template>

<script setup>
import { ref } from 'vue';
import '@cloudflare/ai-search-snippet';

const apiUrl = ref('https://api.example.com');

const handleMessage = (event) => {
  console.log('Message:', event.detail.message);
};

const handleError = (event) => {
  console.error('Error:', event.detail.error);
};
</script>
```

---

## API Server Requirements

### Search Endpoint

**POST** `/search`

**Request:**
```json
{
  "query": "search query",
  "max_results": 10,
  "filters": {}
}
```

**Response:**
```json
{
  "results": [
    {
      "id": "result-1",
      "title": "Result Title",
      "snippet": "Result description...",
      "url": "https://example.com",
      "metadata": {}
    }
  ],
  "total": 42
}
```

### Chat Endpoint (Streaming)

**POST** `/ask`

**Request:**
```json
{
  "query": "user message",
  "generate_mode": "summarize",
  "prev": [
    {
      "role": "user",
      "content": "previous message",
      "timestamp": 1234567890
    }
  ]
}
```

**Response:** Streaming via `ReadableStream`

---

## Development

### Build from Source

```bash
# Install dependencies
npm install

# Development mode
npm run dev

# Build for production
npm run build

# Lint and format
npm run lint
npm run format
```

### Project Structure

```
ai-search-snippet/
├── src/
│   ├── api/
│   │   ├── index.ts              # Base Client
│   │   └── ai-search.ts          # AISearchClient
│   ├── components/
│   │   ├── search-bar-snippet.ts
│   │   ├── search-modal-snippet.ts
│   │   ├── chat-bubble-snippet.ts
│   │   ├── chat-page-snippet.ts
│   │   └── chat-view.ts
│   ├── styles/
│   │   ├── theme.ts              # CSS variables
│   │   ├── search.ts             # Search styles
│   │   └── chat.ts               # Chat styles
│   ├── types/
│   │   └── index.ts              # TypeScript defs
│   ├── utils/
│   │   └── index.ts              # Utilities
│   └── main.ts                   # Entry point
├── dist/                          # Build output
├── package.json
└── vite.config.ts
```

---

## Browser Support

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

---

## Related Documents

- [Cloudflare AI Deep Dive](../ai/01-workers-ai-infrastructure-deep-dive.md)
- [AI Gateway Deep Dive](../ai/02-ai-gateway-deep-dive.md)
