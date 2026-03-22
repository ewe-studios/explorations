---
name: Hophacks SpacetimeDB Workshop
description: Educational workshop materials for learning SpacetimeDB development, featuring a real-time chat application built with TypeScript and React
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/hophacks-spacetimedb-workshop/
---

# Hophacks SpacetimeDB Workshop - Educational Workshop Materials

## Overview

The Hophacks SpacetimeDB Workshop is an **educational resource** designed to teach developers how to build real-time applications with SpacetimeDB. The workshop features a complete chat application built with TypeScript, React, and Vite, demonstrating core SpacetimeDB concepts including tables, reducers, subscriptions, and client-side state synchronization.

Key features:
- **Hands-on workshop** - Step-by-step learning materials
- **Real-time chat app** - Complete working example
- **TypeScript + React** - Modern frontend stack
- **Vite build system** - Fast development and hot reloading
- **SpacetimeDB SDK** - Type-safe client bindings
- **Educational focus** - Designed for hackathons and workshops

## Directory Structure

```
hophacks-spacetimedb-workshop/
├── server-rs/                    # SpacetimeDB module (Rust backend)
│   ├── src/
│   │   └── lib.rs                # Module implementation
│   └── Cargo.toml
├── src/                          # React + TypeScript frontend
│   ├── components/
│   │   ├── Chat.tsx              # Chat component
│   │   ├── MessageList.tsx       # Message display
│   │   ├── MessageInput.tsx      # Message input
│   │   └── UserList.tsx          # Online users
│   ├── hooks/
│   │   └── useSpacetimeDB.ts     # SpacetimeDB hook
│   ├── types/
│   │   └── generated/            # Auto-generated types
│   ├── App.tsx                   # Main application
│   └── main.tsx                  # Entry point
├── public/
│   └── index.html
├── package.json
├── tsconfig.json
├── vite.config.ts
└── README.md
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    React + TypeScript Client                    │
│                    (Vite + SpacetimeDB SDK)                     │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Components              Hooks            Types          │   │
│  │  - Chat                  - useDB          - User         │   │
│  │  - MessageList           - useReducer     - Message      │   │
│  │  - MessageInput          - useState       - AppState     │   │
│  │  - UserList                                             │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                            │
                            │ SpacetimeDB Client SDK
                            │ (WebSocket connection)
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                  SpacetimeDB Module (Rust)                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Tables                   Reducers                       │   │
│  │  - user                   - set_name                     │   │
│  │  - message                - send_message                 │   │
│  │                           - client_connected             │   │
│  │                           - client_disconnected          │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │           In-Memory Database (Automatic Sync)           │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Backend Module (Rust)

### Table Definitions

```rust
// server-rs/src/lib.rs
use spacetimedb::{Identity, ReducerContext, Table, Timestamp};

/// User table - tracks connected clients
#[spacetimedb::table(name = user, public)]
pub struct User {
    #[primary_key]
    pub identity: Identity,
    pub name: Option<String>,
    pub online: bool,
}

/// Message table - stores chat messages
#[spacetimedb::table(name = message, public)]
pub struct Message {
    pub sender: Identity,
    pub sent: Timestamp,
    pub text: String,
}
```

### Reducers (Server Actions)

```rust
/// Set or update user's display name
#[spacetimedb::reducer]
pub fn set_name(ctx: &ReducerContext, name: String) -> Result<(), String> {
    let name = validate_name(name)?;

    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        // Update existing user
        ctx.db.user().identity().update(User {
            name: Some(name),
            ..user
        });
        Ok(())
    } else {
        Err("Cannot set name for unknown user".to_string())
    }
}

/// Send a chat message
#[spacetimedb::reducer]
pub fn send_message(ctx: &ReducerContext, text: String) -> Result<(), String> {
    let text = validate_message(text)?;

    // Insert message into the message table
    // All subscribed clients will receive this update automatically
    ctx.db.message().insert(Message {
        sender: ctx.sender,
        text,
        sent: ctx.timestamp,
    });
    Ok(())
}
```

### Lifecycle Reducers

```rust
/// Called when module is first published
#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    // Initialize module state if needed
}

/// Called when a client connects
#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) {
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        // Returning user - set online status
        ctx.db.user().identity().update(User {
            online: true,
            ..user
        });
    } else {
        // New user - create record
        ctx.db.user().insert(User {
            name: None,
            identity: ctx.sender,
            online: true,
        });
    }
}

/// Called when a client disconnects
#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(ctx: &ReducerContext) {
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        ctx.db.user().identity().update(User {
            online: false,
            ..user
        });
    }
}
```

## Frontend (TypeScript + React)

### SpacetimeDB Hook

```typescript
// src/hooks/useSpacetimeDB.ts
import { useEffect, useState } from 'react';
import { SpacetimeDB, Identity } from 'spacetimedb';

// Import auto-generated types from the module
import { User, Message } from '../types/generated';

interface AppState {
  users: User[];
  messages: Message[];
  currentUser?: User;
}

export function useSpacetimeDB(
  host: string,
  moduleName: string
): AppState & {
  setName: (name: string) => Promise<void>;
  sendMessage: (text: string) => Promise<void>;
  isConnected: boolean;
} {
  const [db, setDb] = useState<SpacetimeDB | null>(null);
  const [state, setState] = useState<AppState>({
    users: [],
    messages: [],
  });
  const [isConnected, setIsConnected] = useState(false);

  useEffect(() => {
    // Connect to SpacetimeDB
    const connect = async () => {
      const newDb = await SpacetimeDB.connect(host, moduleName);

      // Subscribe to tables
      newDb.user.subscribe({
        onInsert: (user) => {
          setState(prev => ({
            ...prev,
            users: [...prev.users, user],
          }));
        },
        onDelete: (user) => {
          setState(prev => ({
            ...prev,
            users: prev.users.filter(u => u.identity !== user.identity),
          }));
        },
        onUpdate: (oldUser, newUser) => {
          setState(prev => ({
            ...prev,
            users: prev.users.map(u =>
              u.identity === newUser.identity ? newUser : u
            ),
          }));
        },
      });

      newDb.message.subscribe({
        onInsert: (message) => {
          setState(prev => ({
            ...prev,
            messages: [...prev.messages, message],
          }));
        },
      });

      setDb(newDb);
      setIsConnected(true);
    };

    connect();
  }, [host, moduleName]);

  const setName = async (name: string) => {
    if (!db) throw new Error('Not connected');
    await db.reducers.set_name(name);
  };

  const sendMessage = async (text: string) => {
    if (!db) throw new Error('Not connected');
    await db.reducers.send_message(text);
  };

  return {
    ...state,
    setName,
    sendMessage,
    isConnected,
  };
}
```

### Chat Component

```typescript
// src/components/Chat.tsx
import React, { useState } from 'react';
import { useSpacetimeDB } from '../hooks/useSpacetimeDB';
import { MessageList } from './MessageList';
import { MessageInput } from './MessageInput';
import { UserList } from './UserList';

interface ChatProps {
  host: string;
  moduleName: string;
}

export function Chat({ host, moduleName }: ChatProps) {
  const {
    users,
    messages,
    currentUser,
    setName,
    sendMessage,
    isConnected,
  } = useSpacetimeDB(host, moduleName);

  const [nameInput, setNameInput] = useState('');

  const handleSetName = () => {
    if (nameInput.trim()) {
      setName(nameInput.trim());
      setNameInput('');
    }
  };

  if (!isConnected) {
    return <div>Connecting to SpacetimeDB...</div>;
  }

  return (
    <div className="chat-container">
      <div className="sidebar">
        <h3>Online Users ({users.filter(u => u.online).length})</h3>
        <UserList users={users} />
      </div>

      <div className="main">
        {!currentUser?.name ? (
          <div className="name-form">
            <input
              type="text"
              value={nameInput}
              onChange={(e) => setNameInput(e.target.value)}
              placeholder="Enter your name"
              onKeyPress={(e) => e.key === 'Enter' && handleSetName()}
            />
            <button onClick={handleSetName}>Join Chat</button>
          </div>
        ) : (
          <>
            <MessageList messages={messages} users={users} />
            <MessageInput onSend={sendMessage} />
          </>
        )}
      </div>
    </div>
  );
}
```

### Message List Component

```typescript
// src/components/MessageList.tsx
import React from 'react';
import { Message, User } from '../types/generated';
import { Identity } from 'spacetimedb';

interface MessageListProps {
  messages: Message[];
  users: User[];
}

export function MessageList({ messages, users }: MessageListProps) {
  const getUser = (identity: Identity): User | undefined => {
    return users.find(u => u.identity === identity);
  };

  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp);
    return date.toLocaleTimeString();
  };

  return (
    <div className="message-list">
      {messages.map((message, index) => {
        const user = getUser(message.sender);
        const displayName = user?.name || 'Anonymous';

        return (
          <div key={index} className="message">
            <div className="message-header">
              <span className="username">{displayName}</span>
              <span className="timestamp">
                {formatTimestamp(message.sent)}
              </span>
            </div>
            <div className="message-text">{message.text}</div>
          </div>
        );
      })}
    </div>
  );
}
```

## Vite Configuration

```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    open: true,
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
  },
  // Generate SpacetimeDB types from module
  define: {
    __SPACETIMEDB_MODULE__: '"quickstart-chat"',
  },
});
```

## Type Generation

```bash
# Generate TypeScript types from SpacetimeDB module
npx spacetimedb-sdk generate \
  --output src/types/generated \
  --module quickstart-chat
```

### Generated Types

```typescript
// src/types/generated/index.ts
import { Identity, Timestamp } from 'spacetimedb';

export interface User {
  identity: Identity;
  name: Option<String>;
  online: boolean;
}

export interface Message {
  sender: Identity;
  sent: Timestamp;
  text: string;
}

export interface Reducers {
  set_name: (name: string) => Promise<Result<void, string>>;
  send_message: (text: string) => Promise<Result<void, string>>;
}
```

## Workshop Exercises

### Exercise 1: Connect to SpacetimeDB

```typescript
// Task: Connect to the SpacetimeDB module
const db = await SpacetimeDB.connect('http://localhost:3000', 'chat');
console.log('Connected!');
```

### Exercise 2: Subscribe to Tables

```typescript
// Task: Subscribe to the user table
db.user.subscribe({
  onInsert: (user) => console.log('User joined:', user),
  onDelete: (user) => console.log('User left:', user),
  onUpdate: (oldUser, newUser) => console.log('User updated:', newUser),
});
```

### Exercise 3: Call a Reducer

```typescript
// Task: Set the user's name
await db.reducers.set_name('Alice');

// Task: Send a message
await db.reducers.send_message('Hello, world!');
```

### Exercise 4: Build a Feature

```typescript
// Task: Add a "typing indicator" feature
// 1. Add a typing_indicator table to the module
// 2. Add a set_typing reducer
// 3. Create a TypingIndicator component
// 4. Show when users are typing
```

## Running the Workshop

### Prerequisites

```bash
# Install SpacetimeDB CLI
curl -sSf https://install.spacetimedb.com | sh

# Install Node.js dependencies
npm install

# Install Rust (for module development)
rustup install stable
```

### Start SpacetimeDB

```bash
# Start local SpacetimeDB instance
spacetime start
```

### Publish the Module

```bash
# Navigate to server directory
cd server-rs

# Publish to local SpacetimeDB
spacetime publish quickstart-chat
```

### Run the Frontend

```bash
# In the workshop directory
npm run dev

# Open browser to http://localhost:3000
```

## Related Documents

- [SpacetimeDB Cookbook](./spacetimedb-cookbook-exploration.md) - More patterns
- [JWKS](./jwks-exploration.md) - Authentication
- [Blackholio](./blackholio-exploration.md) - Multiplayer game example

## Sources

- Source: `/home/darkvoid/Boxxed/@formulas/src.rust/src.SpacetimeDB/hophacks-spacetimedb-workshop/`
- SpacetimeDB Docs: https://spacetimedb.com/docs
- Workshop Template: https://spacetimedb.com/docs/sdks/typescript/quickstart
