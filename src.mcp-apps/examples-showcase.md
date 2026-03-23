# Examples Showcase

## Overview

The MCP Apps ecosystem includes 25+ example applications demonstrating various use cases and techniques. This document catalogs the examples with their key features and implementation details.

## Source Locations

**ext-apps examples:**
`/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MCP/ext-apps/examples`

**mcp-ui examples:**
`/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.AICoders/src.MCP/mcp-ui/examples`

## Complex Applications

### map-server

**Description:** Interactive 3D globe viewer using CesiumJS

**Key Features:**
- Real-time 3D rendering
- Globe navigation
- Location search
- Multiple map styles

**CSP Configuration:**
```typescript
csp: {
  connectDomains: ['https://api.mapbox.com'],
  resourceDomains: [
    'https://api.mapbox.com',
    'https://*.mapbox.com',
    'https://api.cesium.com'
  ]
}
```

**Techniques:**
- Large asset loading
- WebGL in sandboxed iframe
- Mouse/touch interaction

### threejs-server

**Description:** Interactive 3D scene renderer

**Key Features:**
- Three.js integration
- Custom 3D scenes
- Animation support
- User interaction

**Use Cases:**
- Product visualization
- Data visualization
- Educational content

### shadertoy-server

**Description:** Real-time GLSL shader renderer

**Key Features:**
- Shader compilation
- Real-time rendering
- Parameter adjustment
- Shader sharing

**Techniques:**
- WebGL shader programming
- Uniform passing from host
- Frame-by-frame rendering

### sheet-music-server

**Description:** ABC notation to sheet music converter

**Key Features:**
- ABC notation parsing
- Sheet music rendering
- Playback support
- Transposition

**CSP Configuration:**
```typescript
csp: {
  connectDomains: [],  // No external APIs
  resourceDomains: ['https://cdn.jsdelivr.net']  // ABC.js library
}
```

### wiki-explorer-server

**Description:** Wikipedia link graph visualization

**Key Features:**
- D3.js force-directed graph
- Wikipedia API integration
- Interactive exploration
- Node/link styling

**Techniques:**
- Graph visualization
- API data fetching
- Dynamic layout

## Data Visualization

### cohort-heatmap-server

**Description:** Customer retention heatmap

**Features:**
- Chart.js integration
- Cohort analysis
- Color-coded cells
- Tooltip displays

**Data Flow:**
1. Server calculates cohorts
2. UI renders heatmap
3. User hovers for details

### scenario-modeler-server

**Description:** SaaS business projections

**Features:**
- Financial modeling
- Scenario comparison
- Interactive sliders
- Projection charts

**Components:**
- MetricCard - KPI display
- ProjectionChart - Graph visualization
- ScenarioSelector - A/B testing

### budget-allocator-server

**Description:** Interactive budget allocation

**Features:**
- Drag-and-drop allocation
- Real-time calculation
- Constraint validation
- Visual feedback

### customer-segmentation-server

**Description:** Scatter chart with clustering

**Features:**
- Multi-dimensional data
- K-means clustering
- Interactive filtering
- Segment analysis

**Techniques:**
- Large dataset rendering
- Clustering algorithms
- Color coding

## Real-Time Applications

### system-monitor-server

**Description:** Real-time OS metrics

**Features:**
- CPU usage graph
- Memory usage
- Network activity
- Auto-refresh

**Techniques:**
- Polling server metrics
- Real-time chart updates
- Threshold alerts

### transcript-server

**Description:** Live speech transcription

**Features:**
- Streaming audio
- Real-time transcription
- Speaker identification
- Timestamp display

**Use Cases:**
- Meeting transcription
- Interview notes
- Voice memos

### video-resource-server

**Description:** Binary video via MCP resources

**Features:**
- Video streaming
- Chunked loading
- Format detection
- Playback controls

**Techniques:**
- Binary resource handling
- Blob URL creation
- Media element integration

## Utilities

### pdf-server

**Description:** Interactive PDF viewer

**Features:**
- PDF.js integration
- Chunked loading
- Page navigation
- Search functionality
- Zoom controls

**Techniques:**
- Large file handling
- Progressive rendering
- Virtual scrolling

**Plugin System:**
Includes `plugin/CONNECTORS.md` for extending PDF sources.

### qr-server

**Description:** QR code generator

**Features:**
- Text/URL encoding
- Custom styling
- Size options
- Download support

**Implementation:** Python-based
```python
from mcp_ui_server import create_ui_resource

qr_resource = create_ui_resource({
    "uri": "ui://qr/generate",
    "content": {
        "type": "rawHtml",
        "htmlString": generate_qr_html(data)
    }
})
```

### say-server

**Description:** Text-to-speech demo

**Features:**
- Multiple voices
- Speed control
- Pitch adjustment
- Audio playback

**Implementation:** Python-based

### integration-server

**Description:** Integration testing example

**Features:**
- Test cases
- Assertion helpers
- Debug output
- Error handling

**Use Cases:**
- CI/CD testing
- Development debugging
- Regression testing

## Starter Templates

### basic-server-react

**Purpose:** React server template

**Structure:**
```
basic-server-react/
├── src/
│   ├── mcp-app.tsx    # View component
│   └── mcp-app.module.css
├── main.ts            # Entry point
├── server.ts          # MCP server
├── package.json
└── vite.config.ts
```

**Features:**
- React 18
- TypeScript
- Vite build
- CSS modules

### basic-server-vue

**Purpose:** Vue 3 server template

**Features:**
- Vue 3 Composition API
- TypeScript
- Vite build
- Scoped CSS

### basic-server-svelte

**Purpose:** Svelte server template

**Features:**
- Svelte 4
- TypeScript
- Vite build
- Scoped styles

### basic-server-preact

**Purpose:** Preact (lightweight React) template

**Features:**
- Preact (3KB alternative)
- React-compatible API
- Fast bundle size

### basic-server-solid

**Purpose:** SolidJS server template

**Features:**
- SolidJS signals
- Fine-grained reactivity
- No virtual DOM

### basic-server-vanillajs

**Purpose:** Vanilla JavaScript template

**Features:**
- No framework
- Minimal dependencies
- Educational reference

**Use Cases:**
- Learning MCP Apps
- Minimal bundle size
- Custom architecture

## Demo Applications

### debug-server

**Purpose:** Debugging and development aid

**Features:**
- Message logging
- State inspection
- Error simulation
- Performance metrics

**Use Cases:**
- Protocol debugging
- Development testing
- Issue reproduction

### basic-host

**Purpose:** Reference host implementation

**Features:**
- Full MCP Apps support
- Sandbox proxy
- CSP enforcement
- Origin validation

**Structure:**
```
basic-host/
├── src/
│   ├── index.tsx       # Host entry
│   ├── sandbox.ts      # Sandbox proxy
│   ├── implementation.ts
│   ├── theme.ts
│   └── host-styles.ts
├── serve.ts
├── vite.config.ts
└── package.json
```

## mcp-ui Examples

### server

**Purpose:** Full-featured TypeScript server

**Deployment:** Cloudflare Workers

**Endpoints:**
- HTTP Streaming: `https://remote-mcp-server-authless.idosalomon.workers.dev/mcp`
- SSE: `https://remote-mcp-server-authless.idosalomon.workers.dev/sse`

**Features:**
- Multiple tools with UI
- Resource handlers
- Error handling
- Production-ready

### typescript-server-demo

**Purpose:** Simple TypeScript demonstration

**Features:**
- Basic tool registration
- UI resource creation
- Minimal setup

### python-server-demo

**Purpose:** Python SDK demonstration

**Features:**
- `mcp-ui-server` package usage
- Python HTML generation
- Integration examples

### ruby-server-demo

**Purpose:** Ruby SDK demonstration

**Features:**
- `mcp_ui_server` gem usage
- Ruby HTML generation
- Bundler integration

### external-url-demo

**Purpose:** External URL content type

**Features:**
- URL fetching
- `<base>` tag injection
- CSP auto-configuration

### mcp-apps-demo

**Purpose:** MCP Apps compliance demonstration

**Features:**
- Full MCP Apps protocol
- JSON-RPC communication
- Capability negotiation

## Running Examples

### Local Development (ext-apps)

```bash
# Clone and install
git clone https://github.com/modelcontextprotocol/ext-apps.git
cd ext-apps
npm install

# Run all examples
npm start

# Access at http://localhost:8080
```

### Individual Server (MCP Client)

```json
{
  "mcpServers": {
    "map": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-map", "--stdio"]
    },
    "threejs": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-threejs", "--stdio"]
    }
  }
}
```

### Python Servers

```bash
# Using uv
uv run qr-server

# Using pip
pip install -e qr-server
python -m qr_server
```

## Example Patterns

### Pattern 1: Data Fetching

```typescript
// Server fetches data, UI renders
app.ontoolinput = async (input) => {
  const data = await fetch('/api/data', {
    method: 'POST',
    body: JSON.stringify(input.arguments)
  });
  const result = await data.json();
  renderVisualization(result);
};
```

### Pattern 2: Interactive Controls

```typescript
// UI controls trigger tool calls
<button onclick="refreshData()">
  Refresh
</button>

<script>
function refreshData() {
  app.callServerTool({
    name: 'fetch_data',
    arguments: {}
  });
}
</script>
```

### Pattern 3: Streaming Updates

```typescript
// Partial results streamed to UI
app.ontoolinputpartial = async (input) => {
  appendToOutput(input.arguments.partialResult);
};

app.ontoolinput = async (input) => {
  setFinalResult(input.arguments);
};
```

### Pattern 4: Follow-up Messages

```typescript
// UI sends follow-up to conversation
<button onclick="askQuestion()">
  Ask Follow-up
</button>

<script>
async function askQuestion() {
  await app.sendMessage({
    role: 'user',
    content: [{ type: 'text', text: 'Explain this' }]
  });
}
</script>
```

## Related Resources

- [ext-apps Examples](https://github.com/modelcontextprotocol/ext-apps/tree/main/examples)
- [mcp-ui Examples](https://github.com/idosal/mcp-ui/tree/main/examples)
- [Quickstart Guide](https://apps.extensions.modelcontextprotocol.io/api/documents/Quickstart.html)
