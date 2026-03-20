---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/shoelace
repository: https://github.com/shoelace-style/shoelace
explored_at: 2026-03-20T00:00:00Z
language: TypeScript, Lit Web Components
---

# Project Exploration: Shoelace

## Overview

Shoelace is a forward-thinking library of web components built with Lit. It provides a comprehensive collection of high-quality, accessible, and customizable UI components that work across all modern frameworks and frameworks-agnostic environments.

**Key Characteristics:**
- **Web Standards-based** - Built on Custom Elements and Shadow DOM
- **Framework Agnostic** - Works with React, Vue, Angular, Svelte, or vanilla JS
- **Accessible** - WCAG 2.1 AA compliant, ARIA support
- **Customizable** - CSS custom properties, parts, and themes
- **Zero Framework Dependencies** - Only requires Lit (9KB)

## Repository Structure

```
shoelace/
├── src/
│   ├── shoelace.ts               # Main entry - exports all components
│   ├── shoelace-autoloader.ts    # Auto-register components on demand
│   ├── components/               # All web components (50+)
│   │   ├── alert/                # Alert/Toast notifications
│   │   │   └── alert.ts          # SlAlert component
│   │   ├── animated-image/       # GIF-like animations
│   │   ├── animation/            # Animation controller
│   │   ├── avatar/               # User avatar
│   │   ├── badge/                # Status badges
│   │   ├── breadcrumb/           # Navigation breadcrumbs
│   │   ├── breadcrumb-item/      # Individual breadcrumb
│   │   ├── button/               # Button component
│   │   ├── button-group/         # Button group container
│   │   ├── card/                 # Card container
│   │   ├── carousel/             # Image carousel
│   │   ├── carousel-item/        # Carousel slide
│   │   ├── checkbox/             # Checkbox input
│   │   ├── color-picker/         # Color selection
│   │   ├── copy-button/          # Copy to clipboard
│   │   ├── details/              # Expandable details
│   │   ├── dialog/               # Modal dialogs
│   │   ├── divider/              # Visual divider
│   │   ├── drawer/               # Slide-out drawer
│   │   ├── dropdown/             # Dropdown menus
│   │   ├── format-bytes/         # File size formatting
│   │   ├── format-date/          # Date formatting
│   │   ├── format-number/        # Number formatting
│   │   ├── icon/                 # Icon display
│   │   ├── icon-button/          # Icon button
│   │   ├── image-comparer/       # Before/after comparison
│   │   ├── include/              # HTML inclusion
│   │   ├── input/                # Text input
│   │   ├── menu/                 # Menu container
│   │   ├── menu-item/            # Menu item
│   │   ├── menu-label/           # Menu section label
│   │   ├── mutation-observer/    # DOM mutation observer
│   │   ├── option/               # Select option
│   │   ├── popup/                # Popup positioning
│   │   ├── progress-bar/         # Progress bar
│   │   ├── progress-ring/        # Circular progress
│   │   ├── qr-code/              # QR code generator
│   │   ├── radio/                # Radio input
│   │   ├── radio-button/         # Radio button
│   │   ├── radio-group/          # Radio group
│   │   ├── range/                # Range slider
│   │   ├── rating/               # Star rating
│   │   ├── relative-time/        # Relative time display
│   │   ├── resize-observer/      # Element resize observer
│   │   ├── select/               # Select dropdown
│   │   ├── skeleton/             # Loading placeholder
│   │   ├── spinner/              # Loading spinner
│   │   ├── split-panel/          # Resizable split panel
│   │   ├── switch/               # Toggle switch
│   │   ├── tab/                  # Tab button
│   │   ├── tab-group/            # Tab container
│   │   ├── tab-panel/            # Tab content panel
│   │   ├── tag/                  # Tag/chip
│   │   ├── textarea/             # Textarea input
│   │   ├── tooltip/              # Tooltip
│   │   ├── tree/                 # Tree view
│   │   ├── tree-item/            # Tree item
│   │   └── visually-hidden/      # Screen reader only
│   ├── themes/                   # Theme definitions
│   │   └── light.css             # Light theme
│   ├── styles/                   # Shared styles
│   ├── translations/             # i18n translations
│   ├── utilities/                # Utility functions
│   │   ├── animation.ts          # Animation registry
│   │   ├── base-path.ts          # Asset base path
│   │   ├── icon-library.ts       # Icon registration
│   │   └── form.ts               # Form utilities
│   └── events/                   # Custom events
│       └── events.ts             # Event declarations
├── docs/                         # Documentation site
├── apps/                         # Demo applications
├── examples/                     # Usage examples
└── scripts/                      # Build scripts
```

## Architecture

### Component Architecture

Shoelace components are built with Lit, extending `LitElement`:

```typescript
import { LitElement, html, css } from 'lit';
import { property, state } from 'lit/decorators.js';

@customElement('sl-button')
export default class SlButton extends LitElement {
  static styles = css`
    :host {
      display: inline-block;
    }

    .button {
      /* Component styles in Shadow DOM */
    }
  `;

  @property({ type: String }) variant: 'default' | 'primary' | 'success' = 'default';
  @property({ type: Boolean }) disabled = false;
  @property({ type: Boolean }) outline = false;
  @property({ type: Boolean }) pill = false;
  @property({ type: Boolean }) circle = false;

  render() {
    return html`
      <button class="button" ?disabled=${this.disabled}>
        <slot></slot>
      </button>
    `;
  }
}
```

### Component Anatomy

Each component follows a consistent pattern:

```
Component/
├── component.ts          # Main component class
├── component.styles.ts   # CSS-in-JS styles
└── component.test.ts     # Web Test Runner tests
```

### Shadow DOM Encapsulation

Shoelace uses Shadow DOM for true encapsulation:

```typescript
@customElement('sl-input')
export default class SlInput extends LitElement {
  static shadowRootOptions = { ...LitElement.shadowRootOptions, delegatesFocus: true };

  static styles = [
    componentStyles,
    css`
      :host {
        display: block;
      }

      :host([disabled]) {
        pointer-events: none;
        opacity: 0.5;
      }
    `
  ];
}
```

## Components (50+)

### Form Controls

| Component | Tag | Description |
|-----------|-----|-------------|
| Button | `<sl-button>` | Button with variants |
| Input | `<sl-input>` | Text input with validation |
| Textarea | `<sl-textarea>` | Multi-line text input |
| Select | `<sl-select>` | Dropdown select |
| Checkbox | `<sl-checkbox>` | Checkbox input |
| Radio | `<sl-radio>` | Radio input |
| Radio Group | `<sl-radio-group>` | Radio button group |
| Switch | `<sl-switch>` | Toggle switch |
| Range | `<sl-range>` | Range slider |
| Color Picker | `<sl-color-picker>` | Color selection |

### Navigation

| Component | Tag | Description |
|-----------|-----|-------------|
| Breadcrumb | `<sl-breadcrumb>` | Breadcrumb navigation |
| Breadcrumb Item | `<sl-breadcrumb-item>` | Single breadcrumb |
| Tab Group | `<sl-tab-group>` | Tab container |
| Tab | `<sl-tab>` | Tab button |
| Tab Panel | `<sl-tab-panel>` | Tab content |

### Overlays

| Component | Tag | Description |
|-----------|-----|-------------|
| Dialog | `<sl-dialog>` | Modal dialog |
| Drawer | `<sl-drawer>` | Slide-out drawer |
| Dropdown | `<sl-dropdown>` | Dropdown menu |
| Popup | `<sl-popup>` | Positioned popup |
| Tooltip | `<sl-tooltip>` | Hover tooltip |
| Alert | `<sl-alert>` | Toast notification |

### Display

| Component | Tag | Description |
|-----------|-----|-------------|
| Avatar | `<sl-avatar>` | User avatar |
| Badge | `<sl-badge>` | Status badge |
| Card | `<sl-card>` | Content card |
| Icon | `<sl-icon>` | Icon display |
| Icon Button | `<sl-icon-button>` | Icon button |
| Image | `<sl-image-comparer>` | Before/after comparison |
| Progress Bar | `<sl-progress-bar>` | Linear progress |
| Progress Ring | `<sl-progress-ring>` | Circular progress |
| Skeleton | `<sl-skeleton>` | Loading placeholder |
| Spinner | `<sl-spinner>` | Loading spinner |

### Data Display

| Component | Tag | Description |
|-----------|-----|-------------|
| Format Bytes | `<sl-format-bytes>` | File size display |
| Format Date | `<sl-format-date>` | Date formatting |
| Format Number | `<sl-format-number>` | Number formatting |
| Relative Time | `<sl-relative-time>` | "5 minutes ago" |
| QR Code | `<sl-qr-code>` | QR code generation |
| Rating | `<sl-rating>` | Star rating |
| Tag | `<sl-tag>` | Tag/chip |

### Layout

| Component | Tag | Description |
|-----------|-----|-------------|
| Divider | `<sl-divider>` | Visual separator |
| Split Panel | `<sl-split-panel>` | Resizable panels |
| Details | `<sl-details>` | Expandable content |
| Tree | `<sl-tree>` | Tree view |
| Tree Item | `<sl-tree-item>` | Tree node |
| Carousel | `<sl-carousel>` | Image carousel |

### Utilities

| Component | Tag | Description |
|-----------|-----|-------------|
| Animation | `<sl-animation>` | Animation controller |
| Animated Image | `<sl-animated-image>` | GIF/WebP animation |
| Copy Button | `<sl-copy-button>` | Copy to clipboard |
| Include | `<sl-include>` | HTML inclusion |
| Mutation Observer | `<sl-mutation-observer>` | DOM observer |
| Resize Observer | `<sl-resize-observer>` | Element observer |
| Visually Hidden | `<sl-visually-hidden>` | Screen reader only |
| Menu | `<sl-menu>` | Menu container |
| Menu Item | `<sl-menu-item>` | Menu option |
| Menu Label | `<sl-menu-label>` | Menu section |
| Option | `<sl-option>` | Select option |
| Button Group | `<sl-button-group>` | Button group |

## Styling System

### CSS Custom Properties

```css
sl-button {
  --button-font-family: var(--sl-font-sans);
  --button-font-size: var(--sl-font-size-medium);
  --button-font-weight: var(--sl-font-weight-semibold);
  --letter-spacing: var(--sl-letter-spacing-normal);
  --line-height: var(--sl-line-height-normal);

  /* Colors */
  --color-primary: var(--sl-color-primary-500);
  --color-primary-hover: var(--sl-color-primary-600);

  /* Sizing */
  --height: 2.375rem;
  --padding: 0 1.25rem;
}
```

### CSS Parts

Shoelace components expose `::part()` for styling internal elements:

```css
sl-button::part(base) {
  background: blue;
}

sl-input::part(input) {
  border-color: red;
}

sl-dialog::part(panel) {
  border-radius: 12px;
}
```

### Themes

Shoelace includes built-in themes:

```html
<!-- Light theme (default) -->
<link rel="stylesheet" href="shoelace/dist/themes/light.css" />

<!-- Dark theme -->
<link rel="stylesheet" href="shoelace/dist/themes/dark.css" />
```

## Usage

### Direct HTML

```html
<!DOCTYPE html>
<html>
<head>
  <script type="module" src="shoelace/dist/shoelace.js"></script>
  <link rel="stylesheet" href="shoelace/dist/themes/light.css" />
</head>
<body>
  <sl-button variant="primary">Click me</sl-button>
  <sl-input label="Email" type="email"></sl-input>
</body>
</html>
```

### Auto-loader

```html
<script type="module" src="shoelace/cdn/shoelace-autoloader.js"></script>
<!-- Components auto-register when used -->
```

### React

```tsx
import '@shoelace-style/shoelace/dist/components/button/button.js';
import { SlButton } from '@shoelace-style/shoelace/dist/react';

function App() {
  return <SlButton variant="primary">Hello</SlButton>;
}
```

### Vue

```vue
<template>
  <sl-button variant="primary">Hello</sl-button>
</template>

<script setup>
import '@shoelace-style/shoelace/dist/components/button/button.js';
</script>
```

## Events

Shoelace components emit custom events:

```typescript
// Event types
interface SlChangeEvent { detail: { value: string } }
interface SlInputEvent { detail: { value: string } }
interface SlBlurEvent { }
interface SlFocusEvent { }

// Usage
document.querySelector('sl-input').addEventListener('sl-input', (e) => {
  console.log(e.detail.value);
});

// React
<sl-input onSlInput={(e) => console.log(e.detail.value)} />
```

### Available Events

| Event | Component | Description |
|-------|-----------|-------------|
| `sl-change` | Forms | Value changed |
| `sl-input` | Forms | Input event |
| `sl-blur` | Forms | Lost focus |
| `sl-focus` | Forms | Gained focus |
| `sl-show` | Dialog/Drawer | Shown |
| `sl-after-show` | Dialog/Drawer | Animation complete |
| `sl-hide` | Dialog/Drawer | Hidden |
| `sl-after-hide` | Dialog/Drawer | Animation complete |
| `sl-select` | Select | Item selected |
| `sl-clear` | Select | Selection cleared |

## Form Integration

Shoelace components work with native forms:

```html
<form action="/submit" method="POST">
  <sl-input name="email" label="Email" required></sl-input>
  <sl-select name="country" label="Country">
    <sl-option value="us">United States</sl-option>
    <sl-option value="ca">Canada</sl-option>
  </sl-select>
  <sl-checkbox name="terms">I agree</sl-checkbox>
  <sl-button type="submit">Submit</sl-button>
</form>
```

### Form Utilities

```typescript
import { serialize, getFormControls } from '@shoelace-style/shoelace/dist/utilities/form';

const form = document.querySelector('form');
const data = serialize(form); // FormData-like object
const controls = getFormControls(form); // Array of form controls
```

## Accessibility

- **WCAG 2.1 AA** compliant
- **ARIA** roles and attributes
- **Keyboard navigation** support
- **Focus management** for overlays
- **Screen reader** tested

```html
<!-- Accessible dialog -->
<sl-dialog label="Terms of Service" aria-describedby="terms-content">
  <p id="terms-content">Please read our terms...</p>
  <sl-button slot="footer" variant="primary">Accept</sl-button>
</sl-dialog>
```

## Animation System

Shoelace has a built-in animation registry:

```typescript
import { setDefaultAnimation, setAnimationConfig } from '@shoelace-style/shoelace/dist/utilities/animation';

// Override default animations
setDefaultAnimation('dialog.show', {
  keyframes: [
    { opacity: 0, transform: 'scale(0.9)' },
    { opacity: 1, transform: 'scale(1)' }
  ],
  options: { duration: 200 }
});

// Disable animations
setAnimationConfig('disable');
```

## Internationalization

```typescript
import { registerTranslation } from '@shoelace-style/shoelace/dist/utilities/base-path';
import { es } from '@shoelace-style/shoelace/dist/translations/es.js';

registerTranslation(es);

// Component uses translation
<sl-select label="País">
  <sl-option value="es">España</sl-option>
</sl-select>
```

## Key Insights

1. **True Framework Agnostic** - Works everywhere because it's just web components.

2. **Shadow DOM Encapsulation** - Styles don't leak; components are truly isolated.

3. **Progressive Enhancement** - Works without JavaScript for basic functionality.

4. **Composable** - Components can be composed together naturally.

5. **Design System Ready** - CSS custom properties make theming easy.

6. **Zero Runtime Framework** - No React/Vue/Angular needed.

7. **Accessible by Default** - ARIA, keyboard, screen reader support built-in.

8. **Custom Elements Manifest** - IDE support via `custom-elements.json`.

## Open Considerations

1. **Performance at Scale** - How does it perform with 100+ components?

2. **Hydration** - How does SSR work with Shoelace?

3. **Code Splitting** - How does the auto-loader handle chunking?

4. **Mobile Performance** - Touch optimization details?

5. **SEO** - How do search engines handle web components?

6. **Server-Side Rendering** - What SSR frameworks are supported?
