# Hero Components

Attention-grabbing hero section components sourced from real company landing pages.

Source: `src/components/` — 4 hero components.

## AsciiHero (101 lines)

Renders ASCII art as an animated hero element. Uses `useAsciiField` hook for the animation.

```tsx
export function AsciiHero({ text, speed, className }) {
  const { ascii } = useAsciiField(text, { speed })
  return <pre className={className}>{ascii}</pre>
}
```

### useAsciiField Hook (265 lines)

The most complex hook. Generates ASCII art animations from text input.

Aha: The hook pre-computes ASCII frames and cycles through them at the specified speed. The 265 lines handle character mapping, frame generation, animation timing, and cleanup.

## MockIDE (170 lines)

A mock code editor/IDE display for showing code snippets in a realistic-looking editor window.

Features:
- Syntax highlighting simulation
- Line numbers
- Window chrome (traffic light dots)
- Tab bar
- Cursor blink

## Prompt (268 lines)

An AI prompt display component showing a conversational AI interaction.

Features:
- User prompt bubble
- AI response with streaming text
- Typing indicator
- Timestamp

### useTypewriter Hook (92 lines)

```tsx
export function useTypewriter(text: string, speed: number = 50) {
  const [displayed, setDisplayed] = useState('')
  useEffect(() => {
    let i = 0
    const interval = setInterval(() => {
      setDisplayed(text.slice(0, ++i))
      if (i >= text.length) clearInterval(interval)
    }, speed)
    return () => clearInterval(interval)
  }, [text, speed])
  return displayed
}
```

Character-by-character reveal at configurable speed.

## PromptHero (91 lines)

A hero section wrapping the Prompt component with additional styling and layout.

## Cross-References

- [Overview](00-overview.md) — All components
- [Background Effects](02-background-effects.md) — Animated backgrounds
- [Interactive Components](03-interactive-components.md) — User interactions
