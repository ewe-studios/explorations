---
location: /home/darkvoid/Boxxed/@dev/repo-expolorations/nordcraftengine
source: github.com/nordcraftengine/nordcraft
explored_at: 2026-04-05
prerequisites: Web development basics, HTML/CSS/JavaScript familiarity, Component-based UI concepts
---

# Zero to Nordcraft Developer - Complete Fundamentals

## Table of Contents

1. [What is Nordcraft?](#what-is-nordcraft)
2. [Core Concepts](#core-concepts)
3. [Getting Started](#getting-started)
4. [Dashboard and Projects](#dashboard-and-projects)
5. [The Editor](#the-editor)
6. [Canvas and Navigation](#canvas-and-navigation)
7. [Element Tree and Selection](#element-tree-and-selection)
8. [Styling Elements](#styling-elements)
9. [Components](#components)
10. [Pages and Navigation](#pages-and-navigation)
11. [Variables and Formulas](#variables-and-formulas)
12. [Workflows and Events](#workflows-and-events)
13. [API Integration](#api-integration)
14. [Branches and Publishing](#branches-and-publishing)
15. [Packages](#packages)

## What is Nordcraft?

**Nordcraft** is a visual web development engine that combines visual development with a code foundation. It enables building web applications and websites through a visual interface while generating clean, optimized HTML, CSS, and JavaScript under the hood.

### The Problem Nordcraft Solves

Traditional web development:

```
1. Design in Figma/Sketch
2. Hand off to developers
3. Set up development environment
4. Write HTML/CSS/JavaScript
5. Back-and-forth design reviews
6. Manual optimization and testing
7. Deploy and maintain
```

Nordcraft approach:

```
1. Build visually in the editor
2. Clean code generated automatically
3. No environment setup required
4. Real-time preview and testing
5. Instant design iterations
6. Built-in optimization
7. One-click publishing with branching
```

### Key Features

| Feature | Description |
|---------|-------------|
| **Visual Development** | Build UI visually with drag-and-drop |
| **Code Foundation** | Generates clean HTML/CSS/JavaScript |
| **Component System** | Reusable, composable components |
| **Reactive Data** | Signals-based reactivity system |
| **Server-Side Rendering** | SSR for SEO and performance |
| **Branching** | Git-like version control built-in |
| **Packages** | Share reusable components |
| **API Integration** | Connect to any backend service |

### Nordcraft vs Alternatives

| Platform | Visual | Code Access | SSR | Version Control | Learning Curve |
|----------|--------|-------------|-----|-----------------|----------------|
| **Nordcraft** | Yes | Full | Yes | Built-in branching | Low |
| **Webflow** | Yes | Limited | Partial | Basic | Medium |
| **Framer** | Yes | Limited | Yes | Basic | Low |
| **React + Vercel** | No | Full | Yes | Git | High |
| **Bubble** | Yes | No | No | Basic | Medium |

## Core Concepts

### 1. Visual Development with Code Foundation

Nordcraft combines visual building with clean code generation:
- Visual changes generate optimized code
- Custom code can be added when needed
- No traditional IDE setup required

### 2. Component-Based Architecture

```
┌─────────────────────────────────────────┐
│              Application                │
│  ┌─────────────┐  ┌─────────────────┐  │
│  │  Component  │  │    Component    │  │
│  │  ┌───────┐  │  │  ┌───────────┐  │  │
│  │  │ Child │  │  │  │   Child   │  │  │
│  │  └───────┘  │  │  └───────────┘  │  │
│  └─────────────┘  └─────────────────┘  │
└─────────────────────────────────────────┘
```

### 3. Projects: Apps and Packages

- **Apps**: Full web applications with pages and functionality
- **Packages**: Reusable component/function libraries

### 4. Branches and Version Control

- `main` branch = live version
- Create feature branches for development
- Preview before publishing
- Git-like workflow built-in

### 5. Pages and Components

- **Pages**: URL-based routes with SEO metadata
- **Components**: Reusable UI building blocks

### 6. Reactive Data Flow

```
Variables → Formulas → Workflows → UI Updates
    ↓           ↓           ↓           ↓
  State     Transform   Actions    Render
```

### 7. Server and Client Rendering

- SSR for initial load and SEO
- CSR for interactivity
- Hybrid approach automatically managed

### 8. API Integration

- Configure endpoints visually
- Handle authentication
- Transform and use response data
- Real-time streaming support

## Getting Started

### Requirements

- Modern web browser (Chrome, Firefox, Safari, Edge)
- Nordcraft account (free tier available)
- No local installation required (cloud-based)

### Creating Your First Project

1. **Sign up** at [nordcraft.com](https://nordcraft.com)
2. **Launch Nordcraft** to access the dashboard
3. **Click "Create Project"** in the Start building section
4. **Choose project type**: App or Package
5. **Name your project** and select organization
6. **Start building** in the visual editor

### Project Types

#### Creating an App

```
1. Select "App" as project type
2. Choose starter template (Blank, Blog, Dashboard, etc.)
3. App includes:
   - Default home page
   - Page routing system
   - Component library
   - Data binding capabilities
```

#### Creating a Package

```
1. Select "Package" as project type
2. Define package contents:
   - Components to export
   - Formulas/functions
   - Styles and themes
3. Package can be:
   - Published publicly
   - Installed in other projects
   - Versioned independently
```

## Dashboard and Projects

### Dashboard Sections

```
┌────────────────────────────────────────────┐
│            Nordcraft Dashboard             │
├────────────────────────────────────────────┤
│  Start Building                            │
│  ┌──────────────┐  ┌──────────────┐       │
│  │  New Project │  │  Challenges  │       │
│  └──────────────┘  └──────────────┘       │
├────────────────────────────────────────────┤
│  Recent Activity                           │
│  - Project A (edited 2 hours ago)          │
│  - Project B (edited yesterday)            │
├────────────────────────────────────────────┤
│  All Projects                              │
│  ┌────────────────────────────────────┐   │
│  │ Project Name    │ Type  │ Status   │   │
│  ├────────────────────────────────────┤   │
│  │ My App          │ App   │ Published│   │
│  │ UI Kit          │ Pkg   │ Draft    │   │
│  └────────────────────────────────────┘   │
└────────────────────────────────────────────┘
```

### Project Settings

- **General**: Name, description, icon
- **Domain**: Custom domain configuration
- **Environment Variables**: API keys, secrets
- **Team Members**: Collaboration permissions
- **Branches**: Branch management
- **Publishing**: Deploy settings

## The Editor

### Editor Layout

```
┌─────────────────────────────────────────────────────────────┐
│  Top Bar: Undo/Redo │ Viewport │ Preview │ AI Assistant    │
├──────────┬──────────────────────────────────────┬───────────┤
│          │                                       │           │
│  Left    │             CANVAS                    │   Right   │
│  Panel   │          (Central Workspace)          │   Panel   │
│          │                                       │           │
│  - Tree  │     ┌─────────────────────┐          │  - Data   │
│  - Files │     │   Your Application  │          │  - Style  │
│  - Pkgs  │     │                     │          │  - Events │
│  - Issues│     └─────────────────────┘          │           │
│          │                                       │           │
├──────────┴──────────────────────────────────────┴───────────┤
│  Status Bar: Element Info │ Issues │ Sync Status            │
└─────────────────────────────────────────────────────────────┘
```

### Left Panel Sections

1. **Element Tree**: Hierarchical view of page/component structure
2. **Project Sidebar**: Files, resources, settings
3. **Packages**: Browse and install external packages
4. **Issues Panel**: Project issues and warnings

### Right Panel Sections

1. **Data Panel** (default): Variables, formulas, workflows
2. **Element Panel** (when element selected):
   - Style tab: CSS properties and variants
   - Attributes tab: HTML attributes and classes
   - Events tab: Interaction handlers

## Canvas and Navigation

### Canvas Controls

```kotlin
// Navigation shortcuts
Space + Drag     // Pan canvas
Scroll           // Vertical scroll
Shift + Scroll   // Horizontal scroll
Cmd/Ctrl + 0     // Fit to screen
Cmd/Ctrl + +/-   // Zoom in/out

// Element interaction
Hover            // Highlight element
Click            // Select element
Double-click     // Edit text content
Escape           // Deselect

// Drag and drop
Drag             // Move element
Cmd/Ctrl + Drag  // Force insertion mode
Option/Alt + Drag// Duplicate element
```

### Responsive Testing

```
Canvas resize handles:
┌───────────────────┐
│                   │
│                   │
│       Canvas      │◄── Drag to resize
│                   │
│                   │
└───────────────────┘

Viewport presets:
- Mobile: 375px
- Tablet: 768px
- Desktop: 1440px
- Custom: Any width
```

## Element Tree and Selection

### Tree Structure

```
Page: Home
├── Header (Component)
│   ├── Logo (Image)
│   ├── Nav (Container)
│   │   ├── Link 1 (Text)
│   │   └── Link 2 (Text)
│   └── CTA Button (Button)
├── Hero Section (Container)
│   ├── Heading (Text)
│   ├── Subheading (Text)
│   └── Action Button (Button)
└── Footer (Component)
    └── Copyright (Text)
```

### Selection Techniques

```typescript
// Multi-select
Shift + Click    // Select multiple elements
Cmd/Ctrl + A     // Select all elements in container

// Navigate tree
Arrow Up         // Select parent element
Arrow Down       // Select first child
Arrow Left       // Collapse/parent
Arrow Right      // Expand/first child

// Find element
Cmd/Ctrl + F     // Search elements
Type element name // Quick filter
```

## Styling Elements

### CSS Properties Panel

```
┌─────────────────────────────────┐
│         CSS Properties          │
├─────────────────────────────────┤
│  Size                           │
│  ├─ Width: 100%                 │
│  ├─ Height: auto                │
│  └─ Min/Max dimensions          │
├─────────────────────────────────┤
│  Layout                         │
│  ├─ Display: flex               │
│  ├─ Direction: row              │
│  ├─ Justify: center             │
│  ├─ Align: center               │
│  └─ Gap: 16px                   │
├─────────────────────────────────┤
│  Typography                     │
│  ├─ Font: Inter                 │
│  ├─ Size: 16px                  │
│  ├─ Weight: 400                 │
│  └─ Line-height: 1.5            │
├─────────────────────────────────┤
│  Background                     │
│  ├─ Color: #3B82F6              │
│  ├─ Image: url(...)             │
│  └─ Gradient: linear(...)       │
└─────────────────────────────────┘
```

### Style Variants

```kotlin
// Pseudo-class variants
Default          // Base styles
:hover           // Mouse hover state
:active          // Click/press state
:focus-visible   // Keyboard focus state
:disabled        // Disabled state

// Media query variants
@media (max-width: 768px)   // Mobile
@media (max-width: 1024px)  // Tablet
@media (min-width: 1440px)  // Desktop

// Custom class variants
.selected        // When class applied
.loading         // Loading state
.error           // Error state
```

### Conditional Styles

```typescript
// Style variables for dynamic values
const primaryColor = useStyleVariable('primary', '#3B82F6')
const spacing = useStyleVariable('spacing-md', '16px')

// Apply in styles
Box({
  backgroundColor: primaryColor,
  padding: spacing,
  borderRadius: '8px'
})

// Conditional styling with formulas
const isHovered = useVariable(false)
const backgroundColor = useFormula(
  () => isHovered() ? '#1d4ed8' : '#3B82F6'
)
```

## Components

### What are Components?

Components are reusable, self-contained UI units:

```typescript
// Component structure
Component: Button
├── Props (inputs)
│   ├── label: string
│   ├── variant: 'primary' | 'secondary'
│   ├── disabled: boolean
│   └── onClick: () => void
├── Internal structure
│   └── <button class="btn">{label}</button>
├── Styles
│   ├── Default variant
│   ├── Primary variant
│   └── Secondary variant
└── Events
    └── click → onClick prop
```

### Creating Components

```
1. Select elements to componentize
2. Right-click → "Create Component"
3. Name the component
4. Define props (inputs)
5. Configure style overrides
6. Save to component library
```

### Using Components

```typescript
// Drag from component library
// Or use in code:

import { Button } from './components'

// Component instance
<Button 
  label="Click me" 
  variant="primary"
  disabled={false}
  onClick={() => console.log('Clicked!')}
/>

// Style overrides
<Button 
  label="Custom"
  style={{
    backgroundColor: 'red',
    borderRadius: '12px'
  }}
/>
```

### Component Composition

```typescript
// Card component using other components
Component: Card
├── Header (Component)
│   ├── Icon (Component)
│   └── Title (Text)
├── Body (Container)
│   └── Content (Slot)
└── Footer (Component)
    └── Actions (Container)
        ├── Cancel Button (Component)
        └── Confirm Button (Component)
```

## Pages and Navigation

### Page Structure

```typescript
// Page configuration
Page: User Profile
├── Path: /users/:userId
├── Parameters
│   └── userId: string (required)
├── Query Parameters
│   ├── tab: 'posts' | 'about' | 'settings'
│   └── page: number
├── SEO
│   ├── Title: "{userName} Profile"
│   ├── Description: "..."
│   └── Open Graph image
└── Content
    ├── Header (Component)
    ├── Profile Content (Dynamic)
    └── Footer (Component)
```

### Creating Pages

```
1. Click "+" in page tree
2. Choose page type:
   - Static page: /about
   - Dynamic page: /users/:id
   - Catch-all: /docs/[...slug]
3. Configure URL path
4. Set up parameters
5. Design page content
```

### Navigation Methods

```typescript
// Link component (client-side navigation)
<Link to="/about">About Us</Link>

// Dynamic navigation
<Link to={`/users/${userId}`}>Profile</Link>

// With query params
<Link to="/search?q=nordcraft&page=2">Search</Link>

// Programmatic navigation
const navigate = useNavigate()
navigate('/home')
navigate(-1)  // Go back
navigate('/users/123', { replace: true })
```

## Variables and Formulas

### Variables

```typescript
// Variable types
const count = useVariable(0)           // Number
const name = useVariable('Alice')      // String
const isActive = useVariable(true)     // Boolean
const items = useVariable([])          // Array
const user = useVariable(null)         // Object

// Variable operations
count.set(5)
count.update(n => n + 1)
name.set('Bob')
items.push(newItem)
user.set({ id: 1, name: 'Alice' })
```

### Formulas

```typescript
// Basic formula (derived value)
const doubled = useFormula(() => count() * 2)

// Multi-variable formula
const fullName = useFormula(() => 
  `${firstName()} ${lastName()}`
)

// Conditional formula
const statusColor = useFormula(() => 
  status() === 'active' ? 'green' : 'gray'
)

// Array transformation
const itemNames = useFormula(() => 
  items().map(item => item.name)
)
```

### Formula Categories

```
┌─────────────────────────────────────┐
│         Formula Types               │
├─────────────────────────────────────┤
│  String                             │
│  ├─ Concatenate                     │
│  ├─ Format                          │
│  ├─ Parse                           │
│  └─ Transform                       │
├─────────────────────────────────────┤
│  Number                             │
│  ├─ Arithmetic                      │
│  ├─ Aggregate (sum, avg, etc.)      │
│  └─ Format (currency, etc.)         │
├─────────────────────────────────────┤
│  Boolean                            │
│  ├─ Compare                         │
│  ├─ Logic (and, or, not)            │
│  └─ Conditional                     │
├─────────────────────────────────────┤
│  Array                              │
│  ├─ Filter                          │
│  ├─ Map                             │
│  ├─ Reduce                          │
│  └─ Sort                            │
├─────────────────────────────────────┤
│  Object                             │
│  ├─ Pick/Omit                       │
│  ├─ Merge                           │
│  └─ Transform                       │
└─────────────────────────────────────┘
```

## Workflows and Events

### Events

```typescript
// Mouse events
onClick          // Element clicked
onDoubleClick    // Double click
onMouseEnter     // Cursor enters element
onMouseLeave     // Cursor leaves element
onMouseDown      // Mouse button pressed
onMouseUp        // Mouse button released

// Keyboard events
onKeyDown        // Key pressed down
onKeyUp          // Key released
onKeyPress       // Key pressed (character)

// Form events
onSubmit         // Form submitted
onChange         // Value changed
onFocus          // Element focused
onBlur           // Element lost focus

// Touch events
onTouchStart     // Touch begins
onTouchMove      // Touch moves
onTouchEnd       // Touch ends

// Other events
onLoad           // Element loaded
onScroll         // Element scrolled
onResize         // Window resized
```

### Workflows

```typescript
// Simple workflow
onClick → [
  navigate('/next-page')
]

// Multi-action workflow
onSubmit → [
  validateForm(),
  if (valid) → [
    submitForm(),
    showNotification('Success!'),
    navigate('/thank-you')
  ],
  else → [
    showErrors()
  ]
]

// Async workflow
onClick → [
  setLoading(true),
  try → [
    const data = await fetchData(),
    updateState(data),
    showNotification('Loaded!')
  ],
  catch → [
    showError('Failed to load'),
    logError(error)
  ],
  finally → [
    setLoading(false)
  ]
]
```

### Workflow Actions

```
┌─────────────────────────────────────┐
│         Action Categories           │
├─────────────────────────────────────┤
│  Navigation                         │
│  ├─ Navigate to page                │
│  ├─ Open URL                        │
│  ├─ Go back/forward                 │
│  └─ Scroll to element               │
├─────────────────────────────────────┤
│  State                              │
│  ├─ Set variable                    │
│  ├─ Update variable                 │
│  ├─ Reset variable                  │
│  └─ Toggle variable                 │
├─────────────────────────────────────┤
│  API                                │
│  ├─ Call API                        │
│  ├─ Refresh API data                │
│  └─ Cancel request                  │
├─────────────────────────────────────┤
│  UI                                 │
│  ├─ Show/hide element               │
│  ├─ Open modal                      │
│  ├─ Close modal                     │
│  └─ Show notification               │
├─────────────────────────────────────┤
│  Utilities                          │
│  ├─ Wait/delay                      │
│  ├─ Log to console                  │
│  ├─ Trigger event                   │
│  └─ Run formula                     │
└─────────────────────────────────────┘
```

## API Integration

### Configuring APIs

```typescript
// API endpoint configuration
API: User API
├── Base URL: https://api.example.com
├── Authentication: Bearer Token
│   └── Token: {{secrets.API_KEY}}
├── Endpoints
│   ├── GET /users
│   │   ├── Params: page, limit
│   │   └── Response: User[]
│   ├── GET /users/:id
│   │   ├── Params: id
│   │   └── Response: User
│   └── POST /users
│       ├── Body: CreateUserInput
│       └── Response: User
```

### Calling APIs

```typescript
// Call API in workflow
const loadUsers = workflow('loadUsers', [
  callApi('UserAPI', '/users', {
    method: 'GET',
    params: { page: 1, limit: 10 }
  }),
  onSuccess: (response) => [
    setVariable('users', response.data),
    setVariable('loading', false)
  ],
  onError: (error) => [
    setVariable('error', error.message),
    setVariable('loading', false)
  ]
])

// Use API data in UI
<Repeat each={users()}>
  {(user) => (
    <Text>{user.name}</Text>
  )}
</Repeat>
```

### Server-Side Rendering

```typescript
// Configure API for SSR
API: User API
├── Endpoint: GET /users/:id
├── Server-Side Rendering: Enabled
└── Cache: 60 seconds

// Result:
// - API called during initial page load
// - Data embedded in HTML
// - Fast first contentful paint
// - SEO-friendly
```

## Branches and Publishing

### Branching Workflow

```
main (production)
├── feature/new-header
│   └── Preview: https://feature-new-header.preview.nordcraft.com
├── bugfix/login-issue
│   └── Preview: https://bugfix-login.preview.nordcraft.com
└── redesign/homepage
    └── Preview: https://redesign-home.preview.nordcraft.com
```

### Creating Branches

```
1. Click branch selector (shows "main")
2. Click "Create branch"
3. Name branch: feature/my-feature
4. Optionally branch from: main or another branch
5. Start building
6. Changes isolated to branch
```

### Publishing Changes

```
1. Complete work on branch
2. Preview branch URL to verify
3. Click "Publish" button
4. Review changes summary
5. Confirm publish
6. Branch merged to main
7. Changes live on production domain
```

## Packages

### What are Packages?

```typescript
// Package structure
Package: UI Components
├── Version: 1.0.0
├── Exports
│   ├── Components
│   │   ├── Button
│   │   ├── Card
│   │   └── Modal
│   ├── Formulas
│   │   ├── formatDate
│   │   └── validateEmail
│   └── Styles
│       └── Theme variables
└── Dependencies
    └── None (standalone)
```

### Creating Packages

```
1. Create new Package project
2. Build components/formulas
3. Mark items as "Exported"
4. Configure package metadata
   - Name, version, description
   - Author, license
   - Repository URL
5. Publish package
```

### Installing Packages

```
1. Open Packages panel
2. Search package registry
3. Find desired package
4. Click "Install"
5. Select version
6. Package available in component library
```

### Using Package Components

```typescript
// Import from package
import { Button, Card } from '@nordcraft/ui-components'

// Use in your app
<Card>
  <Button variant="primary">Action</Button>
</Card>

// Package components update automatically
// or pin to specific version
```

## Conclusion

Nordcraft provides:

1. **Visual Development**: Build UI visually with drag-and-drop
2. **Code Foundation**: Generates clean HTML/CSS/JavaScript
3. **Component System**: Reusable, composable components
4. **Reactive Data**: Variables, formulas, and workflows
5. **API Integration**: Connect to any backend service
6. **Server-Side Rendering**: SEO-friendly performance
7. **Branching System**: Git-like version control
8. **Package Ecosystem**: Share and reuse components

## Next Steps

- Deep dive into the editor architecture
- Advanced component patterns
- Styling best practices
- Data binding strategies
- Production deployment guide
