---
location: /home/darkvoid/Boxxed/@formulas/src.UIFrameworks/headlessui
repository: https://github.com/tailwindlabs/headlessui
explored_at: 2026-03-20T00:00:00Z
language: TypeScript, React, Vue
---

# Project Exploration: Headless UI

## Overview

Headless UI is a library of completely unstyled, fully accessible UI components designed to integrate perfectly with Tailwind CSS. Created by Tailwind Labs, it provides the building blocks for custom component libraries without imposing any visual design decisions.

**Key Characteristics:**
- **Unstyled** - No default styles, complete design freedom
- **Accessible** - WAI-ARIA compliant, keyboard navigation
- **Framework Support** - React and Vue versions
- **Tailwind Integration** - Works seamlessly with Tailwind CSS
- **Type Safe** - Full TypeScript support

## Repository Structure

```
headlessui/
├── packages/
│   ├── @headlessui-react/        # React implementation
│   │   ├── src/
│   │   │   ├── components/       # All React components
│   │   │   │   ├── accordion/
│   │   │   │   │   └── accordion.tsx
│   │   │   │   ├── alert-dialog/
│   │   │   │   │   └── alert-dialog.tsx
│   │   │   │   ├── avatar/
│   │   │   │   │   └── avatar.tsx
│   │   │   │   ├── button/
│   │   │   │   │   └── button.tsx
│   │   │   │   ├── checkbox/
│   │   │   │   │   └── checkbox.tsx
│   │   │   │   ├── combobox/
│   │   │   │   │   └── combobox.tsx
│   │   │   │   ├── description-list/
│   │   │   │   │   └── description-list.tsx
│   │   │   │   ├── dialog/
│   │   │   │   │   └── dialog.tsx
│   │   │   │   ├── disclosure/
│   │   │   │   │   └── disclosure.tsx
│   │   │   │   ├── field/
│   │   │   │   │   └── field.tsx
│   │   │   │   ├── focus-trap/
│   │   │   │   │   └── focus-trap.tsx
│   │   │   │   ├── label/
│   │   │   │   │   └── label.tsx
│   │   │   │   ├── listbox/
│   │   │   │   │   └── listbox.tsx
│   │   │   │   ├── menu/
│   │   │   │   │   └── menu.tsx
│   │   │   │   ├── popover/
│   │   │   │   │   └── popover.tsx
│   │   │   │   ├── portal/
│   │   │   │   │   └── portal.tsx
│   │   │   │   ├── radio-group/
│   │   │   │   │   └── radio-group.tsx
│   │   │   │   ├── select/
│   │   │   │   │   └── select.tsx
│   │   │   │   ├── switch/
│   │   │   │   │   └── switch.tsx
│   │   │   │   ├── tab/
│   │   │   │   │   └── tab.tsx
│   │   │   │   ├── table/
│   │   │   │   │   └── table.tsx
│   │   │   │   ├── tabs/
│   │   │   │   │   └── tabs.tsx
│   │   │   │   ├── toggle/
│   │   │   │   │   └── toggle.tsx
│   │   │   │   └── transitions/
│   │   │   │       └── transition.tsx
│   │   │   ├── hooks/            # React hooks
│   │   │   │   ├── use-disclosure.ts
│   │   │   │   ├── use-focus-trap.ts
│   │   │   │   ├── use-id.ts
│   │   │   │   ├── use-inert.ts
│   │   │   │   ├── use-outside-click.ts
│   │   │   │   └── ...
│   │   │   ├── internal/         # Internal utilities
│   │   │   │   ├── dom.ts
│   │   │   │   ├── owner.ts
│   │   │   │   ├── platform.ts
│   │   │   │   └── render.ts
│   │   │   ├── index.ts          # Main exports
│   │   │   └── index.test.ts     # Tests
│   │   └── package.json
│   │
│   ├── @headlessui-vue/          # Vue implementation
│   │   ├── src/
│   │   │   ├── components/       # Vue components
│   │   │   ├── hooks/            # Composition API hooks
│   │   │   └── index.ts
│   │   └── package.json
│   │
│   └── @headlessui-tailwindcss/  # Tailwind plugin
│       ├── src/
│       │   └── index.ts
│       └── package.json
│
├── playgrounds/
│   ├── playground-react/         # React development playground
│   └── playground-vue/           # Vue development playground
│
├── scripts/                      # Build and release scripts
└── jest/                         # Jest configuration
```

## Components

### Form Components

| Component | React | Vue | Description |
|-----------|-------|-----|-------------|
| Checkbox | `<Checkbox>` | `<Checkbox>` | Checkbox input with label |
| Field | `<Field>` | `<Field>` | Form field wrapper |
| Label | `<Label>` | `<Label>` | Accessible label |
| Description | `<Description>` | `<Description>` | Field description |
| Radio Group | `<RadioGroup>` | `<RadioGroup>` | Radio button group |
| Select | `<Select>` | `<Select>` | Native select styling |
| Switch | `<Switch>` | `<Switch>` | Toggle switch |
| Combobox | `<Combobox>` | `<Combobox>` | Searchable select |
| Listbox | `<Listbox>` | `<Listbox>` | Custom select dropdown |

### Overlay Components

| Component | React | Vue | Description |
|-----------|-------|-----|-------------|
| Dialog | `<Dialog>` | `<Dialog>` | Modal dialog |
| Alert Dialog | `<AlertDialog>` | `<AlertDialog>` | Confirmation dialog |
| Panel | `<Dialog.Panel>` | `<Dialog.Panel>` | Dialog content panel |
| Title | `<Dialog.Title>` | `<Dialog.Title>` | Dialog title |
| Description | `<Dialog.Description>` | `<Dialog.Description>` | Dialog description |
| Portal | `<Portal>` | `<Portal>` | Teleport to body |
| Popover | `<Popover>` | `<Popover>` | Popover menu |
| Disclosure | `<Disclosure>` | `<Disclosure>` | Collapsible content |

### Navigation Components

| Component | React | Vue | Description |
|-----------|-------|-----|-------------|
| Menu | `<Menu>` | `<Menu>` | Dropdown menu |
| Menu Button | `<Menu.Button>` | `<Menu.Button>` | Menu trigger |
| Menu Items | `<Menu.Items>` | `<Menu.Items>` | Menu items container |
| Menu Item | `<Menu.Item>` | `<Menu.Item>` | Individual menu item |
| Tab Group | `<Tab.Group>` | `<Tab.Group>` | Tab container |
| Tab List | `<Tab.List>` | `<Tab.List>` | Tab button list |
| Tab | `<Tab>` | `<Tab>` | Individual tab |
| Tab Panel | `<Tab.Panel>` | `<Tab.Panel>` | Tab content panel |
| Accordion | `<Accordion>` | `<Accordion>` | Collapsible sections |

### Data Display

| Component | React | Vue | Description |
|-----------|-------|-----|-------------|
| Avatar | `<Avatar>` | `<Avatar>` | User avatar image |
| Avatar Image | `<Avatar.Image>` | `<Avatar.Image>` | Avatar image |
| Avatar Badge | `<Avatar.Badge>` | `<Avatar.Badge>` | Status badge |
| Table | `<Table>` | `<Table>` | Accessible table |

### Utility Components

| Component | React | Vue | Description |
|-----------|-------|-----|-------------|
| Focus Trap | `<FocusTrap>` | `<FocusTrap>` | Trap focus inside |
| Transition | `<Transition>` | `<Transition>` | Enter/leave animations |
| Transition Child | `<Transition.Child>` | `<Transition.Child>` | Transition wrapper |
| Toggle | `<Toggle>` | `<Toggle>` | Boolean toggle |

## Architecture

### Component Pattern

Headless UI components use a compound component pattern:

```tsx
// Simplified example
interface MenuComponent {
  (props: MenuProps): JSX.Element;
  Button: typeof MenuButton;
  Items: typeof MenuItems;
  Item: typeof MenuItem;
}

const MenuRoot: MenuComponent = (props) => {
  const [open, setOpen] = useState(false);

  return (
    <MenuContext.Provider value={{ open, setOpen }}>
      {props.children}
    </MenuContext.Provider>
  );
};

MenuRoot.Button = MenuButton;
MenuRoot.Items = MenuItems;
MenuRoot.Item = MenuItem;

export const Menu = Object.assign(MenuRoot, {
  Button: MenuButton,
  Items: MenuItems,
  Item: MenuItem,
});
```

### Usage Pattern

```tsx
import { Menu, Transition } from '@headlessui/react'
import { Fragment } from 'react'

function MyMenu() {
  return (
    <Menu as="div" className="relative">
      <Menu.Button>Options</Menu.Button>

      <Transition
        as={Fragment}
        enter="transition ease-out duration-100"
        enterFrom="transform opacity-0 scale-95"
        enterTo="transform opacity-100 scale-100"
        leave="transition ease-in duration-75"
        leaveFrom="transform opacity-100 scale-100"
        leaveTo="transform opacity-0 scale-95"
      >
        <Menu.Items>
          <Menu.Item>
            {({ active }) => (
              <button className={active ? 'bg-blue-500' : ''}>
                Edit
              </button>
            )}
          </Menu.Item>
        </Menu.Items>
      </Transition>
    </Menu>
  )
}
```

### Render Props

Components use render props for maximum flexibility:

```tsx
<Listbox value={selected} onChange={setSelected}>
  <Listbox.Button>
    {({ value }) => <span>{value.name}</span>}
  </Listbox.Button>

  <Listbox.Options>
    {options.map((option) => (
      <Listbox.Option
        key={option.id}
        value={option}
        disabled={option.disabled}
      >
        {({ active, selected }) => (
          <div className={active ? 'bg-blue-500' : ''}>
            {option.name}
            {selected && <span>✓</span>}
          </div>
        )}
      </Listbox.Option>
    ))}
  </Listbox.Options>
</Listbox>
```

## Key Components Deep Dive

### Combobox (Searchable Select)

```tsx
import { Combobox } from '@headlessui/react'
import { useState } from 'react'

function SearchableSelect() {
  const [query, setQuery] = useState('')
  const [selected, setSelected] = useState(null)

  const filteredOptions = query === ''
    ? options
    : options.filter((option) =>
        option.name.toLowerCase().includes(query.toLowerCase())
      )

  return (
    <Combobox value={selected} onChange={setSelected}>
      <Combobox.Input
        onChange={(event) => setQuery(event.target.value)}
      />
      <Combobox.Options>
        {filteredOptions.map((option) => (
          <Combobox.Option key={option.id} value={option}>
            {({ active, selected }) => (
              <div className={active ? 'bg-blue-500' : ''}>
                {option.name}
              </div>
            )}
          </Combobox.Option>
        ))}
      </Combobox.Options>
    </Combobox>
  )
}
```

### Dialog (Modal)

```tsx
import { Dialog, Transition } from '@headlessui/react'
import { Fragment, useState } from 'react'

function Modal() {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <>
      <button onClick={() => setIsOpen(true)}>Open</button>

      <Transition appear show={isOpen} as={Fragment}>
        <Dialog onClose={() => setIsOpen(false)}>
          <Transition.Child
            enter="ease-out duration-300"
            enterFrom="opacity-0"
            enterTo="opacity-100"
            leave="ease-in duration-200"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <div className="fixed inset-0 bg-black/25" />
          </Transition.Child>

          <div className="fixed inset-0 overflow-y-auto">
            <div className="flex min-h-full items-center justify-center p-4">
              <Transition.Child
                enter="ease-out duration-300"
                enterFrom="opacity-0 scale-95"
                enterTo="opacity-100 scale-100"
                leave="ease-in duration-200"
                leaveFrom="opacity-100 scale-100"
                leaveTo="opacity-0 scale-95"
              >
                <Dialog.Panel className="bg-white p-6 rounded-lg">
                  <Dialog.Title>Dialog Title</Dialog.Title>
                  <p>Dialog content</p>
                  <button onClick={() => setIsOpen(false)}>Close</button>
                </Dialog.Panel>
              </Transition.Child>
            </div>
          </div>
        </Dialog>
      </Transition>
    </>
  )
}
```

### Tabs

```tsx
import { Tab } from '@headlessui/react'

function Tabs() {
  return (
    <Tab.Group>
      <Tab.List>
        <Tab>Tab 1</Tab>
        <Tab>Tab 2</Tab>
        <Tab disabled>Tab 3</Tab>
      </Tab.List>
      <Tab.Panels>
        <Tab.Panel>Content 1</Tab.Panel>
        <Tab.Panel>Content 2</Tab.Panel>
        <Tab.Panel>Content 3</Tab.Panel>
      </Tab.Panels>
    </Tab.Group>
  )
}
```

### Focus Trap

```tsx
import { FocusTrap } from '@headlessui/react'

function Sidebar() {
  return (
    <FocusTrap>
      <nav className="sidebar">
        {/* Focus stays inside until explicitly released */}
        <a href="/home">Home</a>
        <a href="/about">About</a>
        <a href="/contact">Contact</a>
      </nav>
    </FocusTrap>
  )
}
```

## Accessibility Features

### Keyboard Navigation

All components support full keyboard navigation:

- **Tab/Shift+Tab** - Navigate between focusable elements
- **Enter/Space** - Activate buttons, checkboxes
- **Arrow keys** - Navigate menus, tabs, lists
- **Escape** - Close dialogs, menus, popovers
- **Home/End** - Jump to first/last item

### ARIA Attributes

Components automatically manage ARIA:

```tsx
// Dialog renders:
<div
  role="dialog"
  aria-modal="true"
  aria-labelledby="headlessui-dialog-title-1"
  aria-describedby="headlessui-dialog-description-2"
>
  <h2 id="headlessui-dialog-title-1">Title</h2>
  <p id="headlessui-dialog-description-2">Description</p>
</div>
```

### Focus Management

- Focus is trapped in modals
- Focus returns to trigger when closed
- Disabled elements are skipped
- Focus visible indicators

## Tailwind CSS Integration

### @headlessui/tailwindcss Plugin

```bash
npm install @headlessui/tailwindcss
```

```js
// tailwind.config.js
const headlessuiPlugin = require('@headlessui/tailwindcss')

module.exports = {
  content: [
    './src/**/*.{js,jsx,ts,tsx}',
  ],
  plugins: [
    headlessuiPlugin,
  ],
}
```

### Styling State

```tsx
<Listbox>
  <Listbox.Button
    className="
      data-[hover]:bg-blue-500
      data-[focus]:ring-2
      data-[checked]:bg-green-500
      data-[disabled]:opacity-50
    "
  />
</Listbox>
```

## Hooks

### useDisclosure

```tsx
import { useDisclosure } from '@headlessui/react'

function MyComponent() {
  const { open, open: toggle, close } = useDisclosure()

  return (
    <>
      <button onClick={toggle}>Toggle</button>
      {open && <Modal onClose={close} />}
    </>
  )
}
```

### useOutsideClick

```tsx
import { useOutsideClick } from '@headlessui/react'

function Dropdown() {
  const ref = useRef(null)

  useOutsideClick(ref, () => {
    // Close when clicking outside
    setOpen(false)
  })

  return <div ref={ref}>...</div>
}
```

### useFocusTrap

```tsx
import { useFocusTrap } from '@headlessui/react'

function Modal() {
  const containerRef = useRef(null)
  useFocusTrap(containerRef)

  return <div ref={containerRef}>...</div>
}
```

## Key Insights

1. **Headless by Design** - No opinions on visual design, complete control.

2. **Accessibility First** - WAI-ARIA patterns built-in, keyboard navigation.

3. **Compound Components** - Flexible API using context and render props.

4. **Type Safe** - Full TypeScript support with inferred types.

5. **Framework Agnostic Within React/Vue** - Same API for both frameworks.

6. **Transition Support** - Built-in enter/leave animation support.

7. **Focus Management** - Automatic focus trapping and restoration.

8. **Server Rendering** - Compatible with Next.js Nuxt.js.

## Open Considerations

1. **Svelte Support** - Is there a Svelte version planned?

2. **Solid.js Support** - Any plans for Solid.js?

3. **Custom Hooks** - What other hooks could be exposed?

4. **Animation Libraries** - Integration with Framer Motion?

5. **Mobile Touch** - Touch gesture support?

6. **Theming** - Best practices for theming headless components?

7. **Performance** - How does it compare to Radix UI?

8. **Testing** - Testing utilities for headless components?
