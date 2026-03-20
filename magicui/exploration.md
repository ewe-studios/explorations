---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/magicui
repository: https://github.com/magicui/design
explored_at: 2026-03-20T00:00:00Z
language: TypeScript, React, Tailwind CSS
---

# Project Exploration: MagicUI

## Overview

MagicUI is a React component library built on top of Radix UI primitives and Tailwind CSS. It provides beautiful, animated UI components that can be easily copied into any React project. MagicUI follows the "copy-paste component" philosophy pioneered by shadcn/ui, with a focus on stunning visual effects and animations.

**Key Characteristics:**
- **Copy-Paste Components** - Own your code, no npm dependencies
- **Radix UI Based** - Accessible primitives
- **Tailwind CSS** - Utility-first styling
- **Framer Motion** - Smooth animations
- **Next.js Optimized** - Server Components support
- **TypeScript** - Full type safety

## Repository Structure

```
magicui/
├── app/                        # Next.js app router demo
│   ├── (docs)/                 # Documentation pages
│   │   ├── layout.tsx
│   │   ├── page.tsx
│   │   └── docs/
│   │       ├── page.tsx
│   │       └── [slug]/
│   │           └── page.tsx
│   ├── layout.tsx              # Root layout
│   └── globals.css             # Global styles
│
├── components/
│   ├── ui/                     # Reusable UI components
│   │   ├── button.tsx
│   │   ├── card.tsx
│   │   └── ...
│   ├── magic/                  # MagicUI special components
│   │   ├── animated-list.tsx
│   │   ├── blur-image.tsx
│   │   ├── marquee.tsx
│   │   ├── glow-effect.tsx
│   │   └── ...
│   ├── layout/
│   │   ├── site-header.tsx
│   │   ├── site-footer.tsx
│   │   ├── main-nav.tsx
│   │   ├── mobile-nav.tsx
│   │   └── sidebar-nav.tsx
│   └── docs/
│       ├── mdx-components.tsx  # MDX rendering
│       ├── component-preview.tsx
│       ├── component-installation.tsx
│       ├── copy-button.tsx
│       └── ...
│
├── content/                    # Documentation content
│   └── docs/
│       └── *.mdx
│
├── config/                     # Site configuration
│   ├── site.ts                 # Site metadata
│   └── docs.ts                 # Docs navigation
│
├── lib/                        # Utilities
│   ├── utils.ts                # CN helper
│   └── fonts.ts                # Font configuration
│
├── public/                     # Static assets
├── styles/                     # Additional styles
├── hooks/                      # Custom React hooks
├── registry/                   # Component registry for CLI
│   └── registry.ts             # Component definitions
├── scripts/
│   └── build-registry.ts       # Build component registry
│
├── contentlayer.config.ts      # Contentlayer config
├── components.json             # shadcn/ui config
├── tailwind.config.ts          # Tailwind configuration
├── next.config.js              # Next.js config
├── package.json
└── tsconfig.json
```

## Architecture

### Component Architecture

MagicUI components follow a consistent pattern:

```typescript
// components/magic/animated-list.tsx
"use client";

import { cn } from "@/lib/utils";
import React, { ReactElement, ReactNode, useEffect, useMemo, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";

export interface AnimatedListProps {
  className?: string;
  children: React.ReactNode;
  delay?: number;
}

export const AnimatedList = React.memo(
  ({ className, children, delay = 1000 }: AnimatedListProps) => {
    const [index, setIndex] = useState(0);
    const childrenArray = useMemo(() => React.Children.toArray(children), [children]);

    useEffect(() => {
      const interval = setInterval(() => {
        setIndex((prevIndex) => (prevIndex + 1) % childrenArray.length);
      }, delay);
      return () => clearInterval(interval);
    }, [childrenArray.length, delay]);

    const itemsToShow = useMemo(() => {
      return childrenArray.slice(0, index + 1).reverse();
    }, [index, childrenArray]);

    return (
      <div className={cn("flex flex-col items-center gap-4", className)}>
        <AnimatePresence>
          {itemsToShow.map((item) => (
            <AnimatedListItem key={(item as ReactElement).key}>
              {item}
            </AnimatedListItem>
          ))}
        </AnimatePresence>
      </div>
    );
  },
);

AnimatedList.displayName = "AnimatedList";

export function AnimatedListItem({ children }: { children: ReactNode }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{ duration: 0.3 }}
    >
      {children}
    </motion.div>
  );
}
```

### Key Patterns

1. **"use client" directive** - Components using animations are client components
2. **framer-motion** - Used for all animations
3. **cn utility** - Class merging for Tailwind + dynamic classes
4. **React.memo** - Performance optimization
5. **AnimatePresence** - Exit animations

## Components

### Marketing Components

| Component | Description |
|-----------|-------------|
| AnimatedList | Staggered list animation |
| BlurImage | Image with blur transition |
| Marquee | Infinite scrolling content |
| GlowEffect | Glowing gradient effect |
| GradientBg | Animated gradient background |
| ParticleBg | Particle animation background |
| Dock | macOS-style dock menu |
| Orbit | Circular orbit animation |
| Sphere | 3D sphere visualization |
| Testimonial | Animated testimonials |
| Bento Grid | Bento box grid layout |
| Metric | Animated metrics display |

### UI Components (via shadcn/ui)

| Component | Description |
|-----------|-------------|
| Button | Button variants |
| Card | Card container |
| Dialog | Modal dialog |
| Input | Form input |
| Label | Form label |
| Select | Dropdown select |
| Tabs | Tab navigation |
| Tooltip | Hover tooltip |
| Avatar | User avatar |
| Badge | Status badge |
| Accordion | Expandable content |
| Alert | Alert notifications |
| Toast | Toast notifications |
| Command | Command palette |
| ScrollArea | Custom scrollbar |
| Separator | Visual divider |
| Switch | Toggle switch |

## Installation

### CLI Installation

MagicUI provides a CLI for installing components:

```bash
# Initialize MagicUI
npx magicui-cli@latest init

# Add specific component
npx magicui-cli@latest add animated-list
npx magicui-cli@latest add marquee
npx magicui-cli@latest add blur-image

# Add to specific path
npx magicui-cli@latest add animated-list -c components/marketing
```

### Manual Installation

```bash
# Install dependencies
npm install framer-motion clsx tailwind-merge

# Copy component file
# 1. Go to magicui.design
# 2. Find component
# 3. Click "Copy Code"
# 4. Paste into your project
```

## Configuration

### components.json

```json
{
  "$schema": "https://magicui.design/schema.json",
  "style": "default",
  "rsc": true,
  "tsx": true,
  "tailwind": {
    "config": "tailwind.config.ts",
    "css": "app/globals.css",
    "baseColor": "slate",
    "cssVariables": true
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils",
    "ui": "@/components/ui",
    "magic": "@/components/magic"
  }
}
```

### tailwind.config.ts

```typescript
import type { Config } from "tailwindcss";

const config: Config = {
  darkMode: ["class"],
  content: [
    "./pages/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./app/**/*.{ts,tsx}",
    "./src/**/*.{ts,tsx}",
  ],
  theme: {
    container: {
      center: true,
      padding: "2rem",
      screens: {
        "2xl": "1400px",
      },
    },
    extend: {
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      keyframes: {
        "accordion-down": {
          from: { height: "0" },
          to: { height: "var(--radix-accordion-content-height)" },
        },
        "accordion-up": {
          from: { height: "var(--radix-accordion-content-height)" },
          to: { height: "0" },
        },
      },
      animation: {
        "accordion-down": "accordion-down 0.2s ease-out",
        "accordion-up": "accordion-up 0.2s ease-out",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};

export default config;
```

## Usage Examples

### Animated List

```tsx
import { AnimatedList } from "@/components/magic/animated-list";

function NotificationFeed() {
  return (
    <AnimatedList>
      <NotificationItem notification={notification1} />
      <NotificationItem notification={notification2} />
      <NotificationItem notification={notification3} />
    </AnimatedList>
  );
}
```

### Marquee

```tsx
import { Marquee } from "@/components/magic/marquee";

function ReviewCarousel() {
  return (
    <Marquee pauseOnHover className="[--duration:20s]">
      {reviews.map((review) => (
        <ReviewCard key={review.id} {...review} />
      ))}
    </Marquee>
  );
}
```

### Blur Image

```tsx
import { BlurImage } from "@/components/magic/blur-image";

function ProductCard() {
  return (
    <BlurImage
      src="/product.jpg"
      alt="Product"
      width={400}
      height={300}
      placeholder="blur"
      blurDataURL="data:image/jpeg;base64,..."
    />
  );
}
```

### Bento Grid

```tsx
import { BentoCard, BentoGrid } from "@/components/magic/bento-grid";

const features = [
  {
    name: "Feature 1",
    description: "Description here",
    href: "#",
    cta: "Learn more",
    className: "col-span-3 lg:col-span-1",
    background: <GradientBackground />,
  },
];

function FeatureGrid() {
  return (
    <BentoGrid>
      {features.map((feature) => (
        <BentoCard key={feature.name} {...feature} />
      ))}
    </BentoGrid>
  );
}
```

## Styling

### Utility Function

```typescript
// lib/utils.ts
import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

### CSS Variables

```css
/* app/globals.css */
@layer base {
  :root {
    --background: 0 0% 100%;
    --foreground: 222.2 84% 4.9%;
    --card: 0 0% 100%;
    --card-foreground: 222.2 84% 4.9%;
    --popover: 0 0% 100%;
    --popover-foreground: 222.2 84% 4.9%;
    --primary: 222.2 47.4% 11.2%;
    --primary-foreground: 210 40% 98%;
    --secondary: 210 40% 96.1%;
    --secondary-foreground: 222.2 47.4% 11.2%;
    --muted: 210 40% 96.1%;
    --muted-foreground: 215.4 16.3% 46.9%;
    --accent: 210 40% 96.1%;
    --accent-foreground: 222.2 47.4% 11.2%;
    --destructive: 0 84.2% 60.2%;
    --destructive-foreground: 210 40% 98%;
    --border: 214.3 31.8% 91.4%;
    --input: 214.3 31.8% 91.4%;
    --ring: 222.2 84% 4.9%;
    --radius: 0.5rem;
  }

  .dark {
    --background: 222.2 84% 4.9%;
    --foreground: 210 40% 98%;
    /* ... dark mode values */
  }
}
```

## Documentation Site

### MDX Components

```tsx
// components/docs/mdx-components.tsx
import * as React from "react";
import { MDXComponents } from "mdx/types";

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return {
    ...components,
    // Custom component overrides
    h1: ({ className, ...props }) => (
      <h1 className={cn("text-4xl font-bold", className)} {...props} />
    ),
    pre: ({ className, ...props }) => (
      <pre className={cn("bg-muted p-4 rounded-lg", className)} {...props} />
    ),
    code: ({ className, ...props }) => (
      <code className={cn("bg-muted px-1 py-0.5 rounded", className)} {...props} />
    ),
  };
}
```

### Component Preview

```tsx
// components/docs/component-preview.tsx
"use client";

import { cn } from "@/lib/utils";

interface ComponentPreviewProps {
  name: string;
  className?: string;
}

export function ComponentPreview({ name, className }: ComponentPreviewProps) {
  const Component = React.useMemo(() => {
    return React.lazy(() => import(`@/components/magic/${name}`));
  }, [name]);

  return (
    <div className={cn("relative", className)}>
      <React.Suspense fallback={<div>Loading...</div>}>
        <Component />
      </React.Suspense>
    </div>
  );
}
```

## Dependencies

### Core Dependencies

| Package | Purpose |
|---------|---------|
| `react` | UI framework |
| `next` | React framework |
| `tailwindcss` | Styling |
| `framer-motion` | Animations |
| `clsx` | Class utilities |
| `tailwind-merge` | Class merging |

### Radix UI Dependencies

| Package | Purpose |
|---------|---------|
| `@radix-ui/react-dialog` | Dialog primitive |
| `@radix-ui/react-dropdown-menu` | Dropdown menu |
| `@radix-ui/react-tooltip` | Tooltip |
| `@radix-ui/react-tabs` | Tabs |
| `@radix-ui/react-accordion` | Accordion |
| `@radix-ui/react-avatar` | Avatar |
| `@radix-ui/react-icons` | Icons |

### Documentation Dependencies

| Package | Purpose |
|---------|---------|
| `contentlayer` | MDX content |
| `next-contentlayer` | Contentlayer for Next.js |
| `rehype-pretty-code` | Code syntax highlighting |
| `shiki` | Syntax highlighter |
| `geist` | Font family |
| `next-themes` | Theme switching |

## Key Insights

1. **Copy-Paste Philosophy** - You own the code, no runtime dependencies.

2. **Composable** - Components are designed to be composed together.

3. **Animation-First** - Framer Motion built into every component.

4. **Radix Primitives** - Built on accessible foundations.

5. **Tailwind Native** - Full Tailwind CSS integration.

6. **TypeScript Ready** - Full type inference.

7. **RSC Compatible** - Works with React Server Components.

8. **Themeable** - CSS variables for easy theming.

## Open Considerations

1. **Versioning** - How are component updates handled?

2. **Dependency Management** - What happens when Radix updates?

3. **Customization Limits** - How extensible are components?

4. **Performance** - Impact of Framer Motion at scale?

5. **Accessibility** - How accessible are the animated components?

6. **Mobile Support** - Touch-friendly animations?

7. **SSR Compatibility** - How do animations work with SSR?

8. **Bundle Size** - Impact of copying many components?
