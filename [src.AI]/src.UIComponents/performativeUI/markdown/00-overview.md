# performativeUI: AI-Native Landing Page Components

27 React components for startup landing pages. Tongue firmly in cheek.

**Source:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.UIComponents/performativeUI/`

## Components (27 exports)

| Category | Components |
|----------|-----------|
| Atoms | Sparkle, GradientText, StatusDot, QuestText |
| Primitives | Button, StickyBanner, EyebrowPill |
| Heroes | Rotator, WordRoll, PromptHero, Prompt, AsciiHero, Goldeneye |
| Backgrounds | Aurora, NodeGraphBackground, FloatingSparkles |
| Surfaces | GlassCard, MockIDE |
| Conversation | ChatBubble, TokenStream, WibblingSpinner, ChatFAB |
| Social Proof | LogoMarquee, LogoRow, SlippyWords, StatCounter, CommunityBadge |
| Menus | Temperature |
| Pricing | PricingCard, BeforeAfter, WaitlistForm, Popover |
| Footers | BigBack |

## Hooks (4 exports)

| Hook | Purpose |
|------|---------|
| `useTypewriter` | Typewriter effect animation |
| `useCounter` | Animated counter animation |
| `useTokenStream` | Token streaming animation |
| `useAsciiField` | ASCII art field animation |

## Utility

```typescript
export { cn, type ClassValue } from "./utils/cn"  // clsx + tailwind-merge
```

## Installation

```bash
npm install performative-ui
import 'performative-ui/styles.css'  // Auto-includes all component styles
```

## Usage

```tsx
import { Button, GradientText, ChatBubble, TokenStream, useTypewriter } from 'performative-ui'
```
