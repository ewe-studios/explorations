# Interaction: Normalized DOM Events for Accessibility

`@remix-run/interaction` provides normalized DOM event helpers for press, keys, popover, and form interactions — designed for WAI-ARIA compliant components.

Source: `packages/interaction/src/lib/` — 5 files.

## defineInteraction — Core Abstraction

```typescript
export function defineInteraction<T extends string>(
  name: T,
  factory: InteractionFactory,
): T {
  return name
}

export interface Interaction {
  target: EventTarget
  on(target: EventTarget, handlers: Record<string, Function>): void
  off(target: EventTarget, handlers: Record<string, Function>): void
}
```

Each interaction is a named string constant that maps to an event type, plus a factory function that sets up the underlying DOM listeners.

## Press Events

Source: `lib/interactions/press.ts`

```typescript
export let press = defineInteraction('rmx:press', Press)
export let pressDown = defineInteraction('rmx:press-down', Press)
export let pressUp = defineInteraction('rmx:press-up', Press)
export let longPress = defineInteraction('rmx:long-press', Press)
```

### What Press Does

Normalizes pointer and keyboard input:
- **Pointer down** → `pressDown`
- **Pointer up** (after down, without moving off target) → `pressUp`, then `press`
- **Enter/Space keydown** → same as pointer down
- **Hold for 500ms** → `longPress`

```typescript
// Usage
on(button, {
  [press]: (event) => console.log('pressed'),
  [longPress]: (event) => {
    console.log('long pressed')
    event.preventDefault()  // Cancel subsequent press/pressUp
  },
})
```

### Press State Machine

```
idle → pointerdown/keydown → pressing → pointerup/keyup → pressed
                                     ↘ 500ms timeout → long-pressed
```

Aha: The press interaction prevents the default behavior for Enter/Space on buttons, avoiding double-triggering (once from the key event, once from the click event).

## Key Events

Source: `lib/interactions/keys.ts`

```typescript
export let escape = defineInteraction('keydown:Escape', makeKeyInteraction('Escape'))
export let enter = defineInteraction('keydown:Enter', makeKeyInteraction('Enter'))
export let space = defineInteraction('keydown: ', makeKeyInteraction(' '))
export let backspace = defineInteraction('keydown:Backspace', makeKeyInteraction('Backspace'))
export let del = defineInteraction('keydown:Delete', makeKeyInteraction('Delete'))
export let arrowLeft = defineInteraction('keydown:ArrowLeft', makeKeyInteraction('ArrowLeft'))
export let arrowRight = defineInteraction('keydown:ArrowRight', makeKeyInteraction('ArrowRight'))
export let arrowUp = defineInteraction('keydown:ArrowUp', makeKeyInteraction('ArrowUp'))
export let arrowDown = defineInteraction('keydown:ArrowDown', makeKeyInteraction('ArrowDown'))
export let home = defineInteraction('keydown:Home', makeKeyInteraction('Home'))
export let end = defineInteraction('keydown:End', makeKeyInteraction('End'))
export let pageUp = defineInteraction('keydown:PageUp', makeKeyInteraction('PageUp'))
export let pageDown = defineInteraction('keydown:PageDown', makeKeyInteraction('PageDown'))
```

All key interactions prevent the default browser behavior (scrolling for arrow keys, etc.).

## Popover Events

Source: `lib/interactions/popover.ts`

```typescript
export let popoverToggle = defineInteraction('rmx:popover-toggle', Popover)
export let beforePopoverToggle = defineInteraction('rmx:before-popover-toggle', Popover)
```

Dispatches on the **owner** of a popover (the button with `popovertarget`), not the popover element itself.

```typescript
on(button, {
  [popoverToggle](event) {
    console.log(event.newState)  // 'open' or 'closed'
    console.log(event.oldState)  // 'open' or 'closed'
  },
})
```

Uses the native `ToggleEvent` from the Popover API (`event.newState`, `event.oldState`).

## Form Events

Source: `lib/interactions/form.ts`

```typescript
export const formReset = defineInteraction('rmx:form-reset', FormReset)
```

Dispatches on a form element's child when the form is reset:

```typescript
events(someHiddenInput).on([
  formReset(() => {
    // Reset custom component state
    hiddenInput.value = ''
  }),
])
```

Finds the parent form via `target.form` or `target.closest('form')`, then listens for the native `reset` event.

## on() Helper

```typescript
export function on(element: EventTarget, handlers: Record<string, Function>): void
export function events(element: EventTarget): { on(handlers: Interaction[]): void }
```

The `events()` wrapper provides a fluent API for registering multiple interactions on the same element.

## Package Structure

```
interaction/src/
├── index.ts
└── lib/
    ├── interaction.ts         # defineInteraction, on, events
    └── interactions/
        ├── press.ts           # press, pressDown, pressUp, longPress
        ├── keys.ts            # escape, enter, space, arrow keys, etc.
        ├── popover.ts         # popoverToggle, beforePopoverToggle
        └── form.ts            # formReset
```

## Cross-References

- [Component](08-component.md) — Web components use interaction events
