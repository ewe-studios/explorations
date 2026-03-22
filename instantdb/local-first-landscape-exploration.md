---
name: Local-First Landscape Data
description: Research data and visualization for the local-first software ecosystem, documenting tools, libraries, and projects in the local-first space
type: sub-project
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.wasm/src.InstantDB/local-first-landscape-data/
---

# Local-First Landscape Data - Ecosystem Research

## Overview

This project contains research data and visualization for the **local-first software** ecosystem. It documents and categorizes tools, libraries, frameworks, and projects that enable local-first application development.

## What is Local-First Software?

Local-first software is a paradigm where:
1. **Data lives on the user's device first** - Cloud is secondary
2. **Works offline by default** - Network is optional
3. **Real-time sync when connected** - Collaborative features
4. **User owns their data** - Portability and control
5. **End-to-end encryption** - Privacy by design

## Project Structure

```
local-first-landscape-data/
├── data.js                      # Main data export (JSON/JS)
├── details.md                   # Detailed categorization notes
├── README.md                    # Project documentation
├── logo.light.svg               # Logo for light mode
├── logo.dark.svg                # Logo for dark mode
├── package.json                 # NPM package config
├── pnpm-lock.yaml               # PNPM lockfile
└── .gitignore                   # Git ignore rules
```

## Data Structure

### Main Data Export (data.js)

```javascript
// data.js structure
module.exports = {
  categories: [
    {
      id: 'sync-engines',
      name: 'Sync Engines',
      description: 'Data synchronization and conflict resolution',
      projects: [
        {
          name: 'InstantDB',
          url: 'https://instantdb.com',
          description: 'Real-time database with Firebase-like API',
          language: 'TypeScript/Clojure',
          license: 'MIT',
          features: ['real-time', 'offline-first', 'permissions'],
          maturity: 'production'
        },
        // ... more projects
      ]
    },
    {
      id: 'crdt-libraries',
      name: 'CRDT Libraries',
      description: 'Conflict-free Replicated Data Types',
      projects: [...]
    },
    {
      id: 'storage',
      name: 'Local Storage',
      description: 'Browser and local persistence layers',
      projects: [...]
    },
    {
      id: 'frameworks',
      name: 'Frameworks',
      description: 'Full-stack local-first frameworks',
      projects: [...]
    }
  ]
};
```

### Categories Covered

| Category | Description | Examples |
|----------|-------------|----------|
| Sync Engines | Data synchronization | InstantDB, ElectricSQL, Replicache |
| CRDT Libraries | Conflict-free types | Automerge, Yjs, CRDTs-rs |
| Local Storage | Persistence | IndexedDB, SQLite, RxDB |
| P2P Networks | Decentralized sync | Hypermerge, IPFS, GunDB |
| Frameworks | Full solutions | ElectricSQL, WatermelonDB, RxDB |
| Version Control | Data versioning | Automerge, NBDT, Treeverse |
| Query Engines | Local querying | Dexie, PowerLoom, Datalog |
| Encryption | Security | IronCore, Virgil Security |

## Ecosystem Visualization

### Landscape Categories

```
┌─────────────────────────────────────────────────────────────────────┐
│                    LOCAL-FIRST LANDSCAPE                             │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │ Sync Engines│  │  CRDTs      │  │  Storage    │                 │
│  │             │  │             │  │             │                 │
│  │ • InstantDB │  │ • Automerge │  │ • Dexie     │                 │
│  │ • Electric  │  │ • Yjs       │  │ • SQLite    │                 │
│  │ • Replicache│  │ • CRDTs-rs  │  │ • RxDB      │                 │
│  └─────────────┘  └─────────────┘  └─────────────┘                 │
│                                                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │  P2P        │  │ Frameworks  │  │  Query      │                 │
│  │             │  │             │  │             │                 │
│  │ • Hypermerge│  │ • Watermelon│  │ • Datalog   │                 │
│  │ • IPFS      │  │ • Electric  │  │ • PowerLoom │                 │
│  │ • GunDB     │  │ • Instant   │  │ • RxQuery   │                 │
│  └─────────────┘  └─────────────┘  └─────────────┘                 │
│                                                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │
│  │ Encryption  │  │ Version Ctrl│  │  Tools      │                 │
│  │             │  │             │  │             │                 │
│  │ • IronCore  │  │ • NBDT      │  │ • TileDB    │                 │
│  │ • Virgil    │  │ • Treeverse │  │ • Syncal    │                 │
│  └─────────────┘  └─────────────┘  └─────────────┘                 │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Key Projects Analysis

### Sync Engines

#### InstantDB
- **Language**: TypeScript, Clojure
- **Model**: Triples (EAV)
- **Sync**: WebSocket + WAL tailing
- **Query**: InstaQL (GraphQL-like)
- **Unique**: Firebase-like DX with real-time by default

#### ElectricSQL
- **Language**: TypeScript, Elixir
- **Model**: PostgreSQL tables
- **Sync**: Logical replication + CRDT
- **Query**: SQL (via Postgres)
- **Unique**: Direct PostgreSQL sync to client

#### Replicache
- **Language**: TypeScript, Go
- **Model**: JSON documents
- **Sync**: Custom HTTP-based
- **Query**: Subscription queries
- **Unique**: Built by former Firefox Sync team

### CRDT Libraries

#### Automerge
- **Language**: Rust, TypeScript
- **Type**: Document CRDT
- **Features**: Version history, efficient patches
- **Use Case**: Collaborative editing

#### Yjs
- **Language**: JavaScript, Rust
- **Type**: Shared types CRDT
- **Features**: Undo/redo, awareness protocol
- **Use Case**: Rich text editors

### Local Storage

#### Dexie.js
- **Wrapper**: IndexedDB
- **Features**: Fluent API, TypeScript support
- **Size**: ~7KB gzipped

#### WatermelonDB
- **Wrapper**: SQLite (React Native)
- **Features**: Lazy loading, reactive queries
- **Use Case**: Mobile apps

## Data Model for Landscape

```typescript
interface Project {
  name: string;
  url: string;
  github?: string;
  description: string;
  language: string[];
  license: string;
  features: string[];
  maturity: 'experimental' | 'beta' | 'production';
  lastUpdated?: string;
  stars?: number;
}

interface Category {
  id: string;
  name: string;
  description: string;
  projects: Project[];
}

interface Landscape {
  version: string;
  lastUpdated: string;
  categories: Category[];
  tags: string[];
}
```

## Usage Patterns

### As NPM Package

```javascript
// Import in your application
const landscape = require('local-first-landscape-data');

// Get all sync engines
const syncEngines = landscape.categories.find(
  c => c.id === 'sync-engines'
);

// Filter by maturity
const productionTools = syncEngines.projects.filter(
  p => p.maturity === 'production'
);
```

### Visualization Component

```tsx
// React component example
function LandscapeGrid({ data }) {
  return (
    <div className="grid">
      {data.categories.map(cat => (
        <CategoryCard key={cat.id} category={cat} />
      ))}
    </div>
  );
}

function CategoryCard({ category }) {
  return (
    <div className="card">
      <h3>{category.name}</h3>
      <p>{category.description}</p>
      <ul>
        {category.projects.map(project => (
          <li key={project.name}>
            <a href={project.url}>{project.name}</a>
            <span className="tags">
              {project.features.map(f => (
                <span className="tag" key={f}>{f}</span>
              ))}
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
```

## Research Methodology

### Inclusion Criteria

Projects are included if they:
1. Enable offline-first functionality
2. Provide synchronization capabilities
3. Support local data ownership
4. Have public documentation
5. Are actively maintained (updated within 12 months)

### Exclusion Criteria

Projects are excluded if they:
1. Require constant connectivity
2. Don't support data portability
3. Are proprietary without API access
4. Have been abandoned (no updates in 18+ months)

## Ecosystem Trends (2026)

### Growing Areas

1. **CRDT-based collaboration** - More real-time editing tools
2. **SQLite + WASM** - Full databases in browsers
3. **Edge computing** - Sync at the edge (Cloudflare, Deno)
4. **AI + local-first** - Local LLMs with synced context

### Maturing Areas

1. **Conflict resolution** - Better automatic merge strategies
2. **Encryption** - End-to-end encrypted sync
3. **Query optimization** - Efficient local querying
4. **Mobile support** - React Native, Flutter integration

### Emerging Patterns

1. **Hybrid sync models** - CRDT + operational transforms
2. **Incremental materialization** - Partial data loading
3. **Schema evolution** - Handling breaking changes
4. **Observability** - Debugging sync issues

## Related Resources

### Papers and Research

- "Local-First Software" (Ink & Switch, 2019)
- "CRDTs: A Comprehensive Guide" (Martin Kleppmann)
- "The Local-First Software Movement" (ACM Queue)

### Communities

- Local-First Slack (inkandswitch.com/local-first)
- r/localfirst (Reddit)
- Local-First Discord

### Conferences

- Local-First Summit (annual)
- Strange Loop talks
- React Conf (local-first tracks)

## Implementation Considerations

### Choosing a Sync Engine

| Factor | Questions to Ask |
|--------|------------------|
| Data model | Does it match your schema? |
| Query language | SQL vs custom vs NoSQL? |
| Conflict strategy | CRDT vs last-write-wins? |
| Encryption | E2E required? |
| Offline support | Full or partial? |
| Scalability | Max users/data size? |
| Cost | Open source vs commercial? |

### Migration Strategy

```
1. Audit existing data model
2. Choose target local-first stack
3. Design sync schema
4. Build migration scripts
5. Test with subset of users
6. Gradual rollout
7. Monitor sync metrics
8. Deprecate old system
```

## Metrics and Benchmarks

### Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Query latency (local) | <10ms | IndexedDB/SQLite |
| Sync latency | <500ms | Network dependent |
| Conflict rate | <1% | Well-designed schemas |
| Offline duration | Unlimited | Full functionality |
| Bundle size | <50KB | Client library |

### Success Metrics

- Time to interactive (TTI)
- Offline success rate
- Sync conflict resolution rate
- User-perceived latency
- Data loss incidents (should be 0)

## Future Directions

### Near Term (2026-2027)

- Better WASM-based CRDTs
- Standardized sync protocols
- Improved developer tooling
- More E2E encrypted options

### Long Term (2028+)

- OS-level sync primitives
- Universal data portability
- Decentralized identity integration
- AI-assisted conflict resolution
