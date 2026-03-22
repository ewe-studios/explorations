---
name: Instant React Repro
description: React reproduction example application demonstrating InstantDB integration patterns with Vite and TypeScript
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.InstantDB/instant-react-repro/
---

# Instant React Repro - React Example Application

## Overview

A Vite-based React application that serves as a reproduction/example of how to integrate InstantDB into a modern React frontend. This project demonstrates the practical usage patterns of InstantDB in a real application context.

## Project Structure

```
instant-react-repro/
├── src/
│   ├── App.tsx                    # Main application component
│   ├── main.tsx                   # Entry point with InstantDB init
│   ├── components/                # UI components (if present)
│   └── hooks/                     # Custom React hooks (if present)
├── public/                        # Static assets
├── instant.perms.ts               # Permission rules definition
├── instant.schema.ts              # Schema/type definitions
├── package.json                   # Dependencies and scripts
├── vite.config.ts                 # Vite bundler configuration
├── tsconfig.json                  # TypeScript configuration
└── .env                           # Environment variables (APP_ID)
```

## Key Configuration Files

### Package.json

```json
{
  "name": "instant-react-repro",
  "version": "0.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "lint": "eslint ."
  },
  "dependencies": {
    "@instantdb/react": "^0.x.x",
    "react": "^18.x.x",
    "react-dom": "^18.x.x"
  },
  "devDependencies": {
    "@types/react": "^18.x.x",
    "@types/react-dom": "^18.x.x",
    "@vitejs/plugin-react": "^4.x.x",
    "typescript": "^5.x.x",
    "vite": "^5.x.x"
  }
}
```

### Vite Configuration

```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    open: true
  },
  build: {
    target: 'esnext',
    minify: 'esbuild'
  }
});
```

### InstantDB Schema

```typescript
// instant.schema.ts
import { i } from "@instantdb/react";

// Define your data schema with TypeScript types
const schema = i.schema({
  entities: {
    users: i.entity<{
      name: string;
      email: string;
      avatarUrl?: string;
    }>(),
    posts: i.entity<{
      title: string;
      content: string;
      published: boolean;
    }>(),
    comments: i.entity<{
      text: string;
    }>()
  },
  links: {
    postAuthor: {
      forward: { on: "posts", has: "one", label: "author" }
    },
    postComments: {
      forward: { on: "posts", has: "many", label: "comments" }
    },
    commentAuthor: {
      forward: { on: "comments", has: "one", label: "author" }
    }
  }
});

export type AppSchema = typeof schema;
export default schema;
```

### Permission Rules

```typescript
// instant.perms.ts
import { i } from "@instantdb/react";

const perms = i.permissions({
  users: {
    bind: "user == data",
    allow: {
      view: "true",  // Public read
      create: "auth != null",  // Authenticated users
      update: "auth.id == user.id",  // Own profile only
      delete: "auth.id == user.id"
    }
  },
  posts: {
    bind: "post == data",
    allow: {
      view: "true",
      create: "auth != null",
      update: "auth.id == post.authorId",
      delete: "auth.id == post.authorId"
    }
  },
  comments: {
    bind: "comment == data",
    allow: {
      view: "true",
      create: "auth != null",
      update: "auth.id == comment.authorId",
      delete: "auth.id == comment.authorId"
    }
  }
});

export default perms;
```

## Application Patterns

### Initialization

```typescript
// src/main.tsx
import React from 'react';
import ReactDOM from 'react-dom/client';
import { init } from '@instantdb/react';
import schema from '../instant.schema';
import App from './App';

// Initialize InstantDB with schema
const db = init({
  appId: import.meta.env.VITE_INSTANT_APP_ID,
  schema,
});

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App db={db} />
  </React.StrictMode>
);
```

### Query Pattern with useQuery

```typescript
// src/App.tsx
import { useQuery, tx, id } from '@instantdb/react';

function PostList() {
  // Real-time query with loading and error states
  const { isLoading, error, data } = useQuery({
    posts: {
      author: {},  // Nested relation
      comments: {  // One-to-many relation
        author: {}
      }
    }
  });

  if (isLoading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div>
      {data.posts.map(post => (
        <PostCard key={post.id} post={post} />
      ))}
    </div>
  );
}
```

### Transaction Pattern

```typescript
function CreatePost({ authorId }: { authorId: string }) {
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');

  const handleCreate = () => {
    db.transact(
      tx.posts[id()].update({
        title,
        content,
        published: true,
        author: { id: authorId }  // Link to author
      })
    );
    setTitle('');
    setContent('');
  };

  return (
    <form onSubmit={(e) => { e.preventDefault(); handleCreate(); }}>
      <input value={title} onChange={e => setTitle(e.target.value)} />
      <textarea value={content} onChange={e => setContent(e.target.value)} />
      <button type="submit">Create Post</button>
    </form>
  );
}
```

### Presence Pattern (Typing Indicators)

```typescript
function ChatRoom({ roomId }: { roomId: string }) {
  // Set own presence
  useEffect(() => {
    db.presence.set({ roomId, typing: false });
    return () => db.presence.set({ roomId, typing: false });
  }, [roomId]);

  // Subscribe to others' presence
  const { data: peers } = db.presence.useSubscribe();

  const typingUsers = Object.values(peers || {})
    .filter(p => p.roomId === roomId && p.typing);

  return (
    <div>
      {typingUsers.length > 0 && (
        <div>{typingUsers.length} typing...</div>
      )}
    </div>
  );
}
```

### Topic Pattern (Broadcasting)

```typescript
function CollaborativeCursor({ roomId }: { roomId: string }) {
  // Publish cursor position
  const handleMouseMove = (e: MouseEvent) => {
    db.topic(`cursor-${roomId}`).publish({
      x: e.clientX,
      y: e.clientY,
      userId: currentUser.id
    });
  };

  // Subscribe to cursor updates
  db.topic(`cursor-${roomId}`).subscribe((updates) => {
    // Update remote cursors in real-time
    updateCursorPositions(updates);
  });
}
```

## TypeScript Integration

### Type-Safe Queries

```typescript
// Derived types from schema
type PostsQuery = {
  posts: Array<{
    id: string;
    title: string;
    content: string;
    published: boolean;
    author?: { id: string; name: string };
    comments: Array<{
      id: string;
      text: string;
      author?: { id: string; name: string };
    }>;
  }>;
};

// Type-safe query helper
function usePostsQuery() {
  return useQuery<PostsQuery>({
    posts: {
      author: {},
      comments: { author: {} }
    }
  });
}
```

### Optimistic Update Helpers

```typescript
// Custom hook for optimistic mutations
function useOptimisticTransaction() {
  const transact = useTransact();

  return useCallback(async <T>(
    txBuilder: () => Transaction,
    optimistics?: () => void
  ) => {
    // Apply optimistic update
    if (optimistics) optimistics();

    // Execute transaction
    const result = await transact(txBuilder());

    // Handle rollback if needed
    if (result.error) {
      // Rollback logic
    }

    return result;
  }, [transact]);
}
```

## Build and Deployment

### Development Server

```bash
# Install dependencies
npm install

# Start dev server with HMR
npm run dev

# Opens at http://localhost:3000
```

### Production Build

```bash
# Type check and build
npm run build

# Preview production build
npm run preview

# Output: ./dist/ with optimized bundles
```

### Environment Configuration

```bash
# .env (development)
VITE_INSTANT_APP_ID=your-dev-app-id

# .env.production
VITE_INSTANT_APP_ID=your-production-app-id
```

## Common Patterns and Solutions

### 1. Pagination

```typescript
function usePaginatedPosts(pageSize = 10) {
  const [cursor, setCursor] = useState<string | null>(null);
  const [allPosts, setAllPosts] = useState([]);

  const { data } = useQuery({
    posts: {
      $: { where: cursor ? { id: { gt: cursor } } : {} }
    }
  });

  useEffect(() => {
    if (data?.posts) {
      setAllPosts(prev => [...prev, ...data.posts]);
    }
  }, [data]);

  const loadMore = () => {
    const lastPost = allPosts[allPosts.length - 1];
    if (lastPost) setCursor(lastPost.id);
  };

  return { posts: allPosts, loadMore, hasMore: data?.posts.length === pageSize };
}
```

### 2. Form with Validation

```typescript
function CreatePostForm({ authorId }: { authorId: string }) {
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validate = () => {
    const newErrors: Record<string, string> = {};
    if (!title.trim()) newErrors.title = 'Required';
    if (content.length < 10) newErrors.content = 'Min 10 chars';
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    try {
      await db.transact(
        tx.posts[id()].update({
          title: title.trim(),
          content: content.trim(),
          author: { id: authorId }
        })
      );
      // Success - form will reset
    } catch (err) {
      setErrors({ submit: 'Failed to create post' });
    }
  };
}
```

### 3. Offline Indicator

```typescript
function ConnectionStatus() {
  const [isOnline, setIsOnline] = useState(navigator.onLine);

  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  return (
    <div className={isOnline ? 'online' : 'offline'}>
      {isOnline ? 'Connected' : 'Working offline'}
    </div>
  );
}
```

## Integration with Other Libraries

### React Router

```typescript
import { createBrowserRouter } from 'react-router-dom';

const router = createBrowserRouter([
  {
    path: '/',
    element: <App />,
    children: [
      { path: 'posts/:id', element: <PostDetail /> },
      { path: 'users/:id', element: <UserProfile /> }
    ]
  }
]);
```

### Zustand for Local State

```typescript
import { create } from 'zustand';

// Local UI state (not synced)
const useUIStore = create((set) => ({
  sidebarOpen: true,
  toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen }))
}));
```

## Performance Considerations

### Query Optimization

```typescript
// Good: Specific query with filters
const { data } = useQuery({
  posts: {
    $: { where: { published: true, authorId: userId } }
  }
});

// Avoid: Over-fetching
const { data } = useQuery({
  posts: {},  // Fetches ALL posts
  users: {},  // Fetches ALL users
  comments: {} // Fetches ALL comments
});
```

### Memoization

```typescript
// Memoize expensive derived data
const formattedPosts = useMemo(() => {
  return posts.map(p => ({
    ...p,
    formattedDate: new Date(p.createdAt).toLocaleDateString()
  }));
}, [posts]);
```

## Testing Patterns

### Unit Test with Mock

```typescript
// __tests__/PostList.test.tsx
import { render, screen } from '@testing-library/react';
import { PostList } from '../PostList';

// Mock InstantDB
jest.mock('@instantdb/react', () => ({
  useQuery: () => ({
    isLoading: false,
    error: null,
    data: { posts: [{ id: '1', title: 'Test' }] }
  })
}));

test('renders posts', () => {
  render(<PostList />);
  expect(screen.getByText('Test')).toBeInTheDocument();
});
```
