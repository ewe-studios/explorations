# Interactive Components

Components that respond to user interaction.

## BeforeAfter (110 lines)

Before/after slider comparison. User drags a divider to reveal the difference between two images.

## Button (93 lines)

Animated button with hover effects, click animations, and variant styles.

## Popover (169 lines)

Animated popover that positions itself relative to a trigger element.

## ChatBubble (55 lines)

Animated chat bubble with typing indicator.

## ChatFAB (127 lines)

Floating action button for chat widgets.

## GlassCard (86 lines)

Glassmorphism card with backdrop blur.

## PricingCard (109 lines)

Pricing card with hover animations and CTA button.

## GradientText (36 lines)

Text with animated gradient color cycling.

## QuestText (123 lines)

RPG/quest-style text animation with character-by-character reveal and sound effects.

## Rotator (76 lines)

3D text rotator/carousel with perspective transforms.

## SlippyWords (149 lines)

Sliding word animation where words slide in and out of view.

## useTokenStream Hook (83 lines)

Simulates streaming tokens (like AI chat responses) with configurable speed and burst patterns.

## StatCounter (44 lines)

Animated number counter using `useCounter` hook.

## StatusDot (25 lines)

Animated status indicator (pulsing dot).

## EyebrowPill (24 lines)

Small eyebrow label component.

## CommunityBadge (39 lines)

Community/member badge.

## StickyBanner (25 lines)

Sticky banner that stays visible while scrolling.

## TokenStream (59 lines)

Visual component for streaming token display.

## WaitlistForm (92 lines)

Waitlist signup form with validation.

## Temperature (451 lines)

Temperature/thermometer visualization — the largest single component.

## cn Utility (25 lines)

```ts
import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
```

## Cross-References

- [Overview](00-overview.md) — All components
- [Hero Components](01-hero-components.md) — Hero sections
- [Logo Walls](04-logo-walls.md) — Logo displays
