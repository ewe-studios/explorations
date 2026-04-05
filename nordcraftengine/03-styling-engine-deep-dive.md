# Nordcraft Styling Engine Deep Dive

## Overview

Nordcraft's styling engine provides a visual interface for CSS authoring while generating optimized, production-ready styles. This deep-dive examines the technical implementation of the styling system, including CSS variables, conditional styles, responsive variants, and the CSS editor.

## Style Data Model

### Node Style Structure

Each element or component node contains style information:

```typescript
interface ElementNodeModel {
  style: NodeStyleModel           // Base styles
  variants?: StyleVariant[]       // Conditional style variants
  'style-variables'?: Array<{     // Dynamic style variables
    category: StyleTokenCategory
    name: string
    formula: Formula
    unit?: string
  }>
  classes: Record<string, {       // CSS class bindings
    formula?: Formula
  }>
}

// Style properties as key-value pairs
type NodeStyleModel = Record<string, string>

// Example style object
const buttonStyles: NodeStyleModel = {
  'display': 'flex',
  'align-items': 'center',
  'justify-content': 'center',
  'padding': '12px 24px',
  'background-color': '#007bff',
  'color': '#ffffff',
  'border-radius': '6px',
  'font-size': '16px',
  'font-weight': '600'
}
```

### Style Variant Structure

Variants define conditional styles:

```typescript
interface StyleVariant {
  id?: string
  className?: string            // Generated class name
  
  // Pseudo-class conditions
  hover?: boolean
  focus?: boolean
  focusWithin?: boolean
  active?: boolean
  disabled?: boolean
  
  // Structural conditions
  firstChild?: boolean
  lastChild?: boolean
  evenChild?: boolean
  empty?: boolean
  
  // Responsive conditions
  mediaQuery?: MediaQuery
  breakpoint: 'small' | 'medium' | 'large'
  
  // Starting style for view transitions
  startingStyle?: boolean
  
  // The actual styles
  style: NodeStyleModel
}

interface MediaQuery {
  'min-width'?: string
  'max-width'?: string
  'min-height'?: string
  'max-height'?: string
}
```

## CSS Variable System

### Style Variable Definition

Style variables enable dynamic, formula-driven styles:

```typescript
interface StyleVariable {
  category: StyleTokenCategory
  name: string
  formula: Formula
  unit?: string
}

type StyleTokenCategory = 
  | 'color'
  | 'spacing'
  | 'typography'
  | 'border'
  | 'shadow'
  | 'custom'

// Example: Dynamic background based on state
const dynamicBackground: StyleVariable = {
  category: 'color',
  name: 'button-bg',
  formula: {
    op: 'if',
    args: [
      { type: 'variable', name: 'isSelected' },  // Condition
      { type: 'static', value: '#0056b3' },      // Then (darker)
      { type: 'static', value: '#007bff' }       // Else (default)
    ]
  },
  unit: undefined  // No unit for colors
}

// Example: Dynamic width with unit
const dynamicWidth: StyleVariable = {
  category: 'spacing',
  name: 'container-width',
  formula: {
    type: 'variable',
    name: 'width'
  },
  unit: 'px'  // Append 'px' to numeric value
}
```

### Style Variable Evaluation

Style variables evaluate formulas and generate CSS custom properties:

```typescript
function evaluateStyle_variables(
  styleVariables: StyleVariable[],
  context: FormulaContext
): Record<string, string> {
  const cssVariables: Record<string, string> = {}
  
  for (const variable of styleVariables) {
    const value = evaluateFormula(variable.formula, context)
    
    // Apply unit if specified
    const finalValue = variable.unit 
      ? `${value}${variable.unit}`
      : String(value)
    
    // Generate CSS custom property name
    const cssVarName = `--${toKebabCase(variable.name)}`
    cssVariables[cssVarName] = finalValue
  }
  
  return cssVariables
}

// Using style variables in styles
const stylesWithVariables: NodeStyleModel = {
  'background-color': 'var(--button-bg)',
  'width': 'var(--container-width)',
  'color': 'var(--text-color)'
}
```

## Conditional Styles

### Pseudo-class Variants

Pseudo-class-based conditional styling:

```typescript
// Hover variant
const hoverVariant: StyleVariant = {
  id: 'hover-1',
  className: 'button_hover_abc123',
  hover: true,
  style: {
    'background-color': '#0056b3',
    'transform': 'translateY(-2px)',
    'box-shadow': '0 4px 12px rgba(0,0,0,0.15)'
  }
}

// Focus variant
const focusVariant: StyleVariant = {
  id: 'focus-1',
  className: 'button_focus_def456',
  focus: true,
  style: {
    'outline': '2px solid #007bff',
    'outline-offset': '2px'
  }
}

// Combined pseudo-classes
const hoverFocusVariant: StyleVariant = {
  id: 'hover-focus-1',
  hover: true,
  focus: true,
  style: {
    'background-color': '#004094'
  }
}

// CSS output
/*
.button:hover {
  background-color: #0056b3;
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
}

.button:focus {
  outline: 2px solid #007bff;
  outline-offset: 2px;
}

.button:hover:focus {
  background-color: #004094;
}
*/
```

### Class-based Styles

Conditional styles triggered by class bindings:

```typescript
// Element with class binding
const elementWithClasses: ElementNodeModel = {
  type: 'element',
  tag: 'div',
  classes: {
    'selected': {
      formula: {
        type: 'variable',
        name: 'isSelected'
      }
    },
    'disabled': {
      formula: {
        op: 'not',
        args: [{ type: 'variable', name: 'isEnabled' }]
      }
    }
  },
  variants: [
    {
      id: 'selected-style',
      className: 'selected',  // Matches class name
      style: {
        'background-color': '#e3f2fd',
        'border-color': '#2196f3'
      }
    },
    {
      id: 'disabled-style',
      className: 'disabled',
      style: {
        'opacity': '0.5',
        'pointer-events': 'none'
      }
    }
  ]
}

// Class evaluation
function evaluateClasses(
  classes: Record<string, { formula?: Formula }>,
  context: FormulaContext
): Set<string> {
  const activeClasses = new Set<string>()
  
  for (const [className, binding] of Object.entries(classes)) {
    if (!binding.formula) {
      // Static class
      activeClasses.add(className)
      continue
    }
    
    const result = evaluateFormula(binding.formula, context)
    if (result) {
      activeClasses.add(className)
    }
  }
  
  return activeClasses
}
```

### Responsive Breakpoints

Media query-based responsive styles:

```typescript
// Breakpoint definitions
const BREAKPOINTS = {
  small: { maxWidth: '640px' },
  medium: { minWidth: '641px', maxWidth: '1024px' },
  large: { minWidth: '1025px' }
}

// Responsive variant
const mobileVariant: StyleVariant = {
  id: 'mobile-1',
  className: 'container_mobile_xyz789',
  breakpoint: 'small',
  mediaQuery: {
    'max-width': '640px'
  },
  style: {
    'flex-direction': 'column',
    'padding': '16px',
    'width': '100%'
  }
}

// Tablet variant
const tabletVariant: StyleVariant = {
  id: 'tablet-1',
  breakpoint: 'medium',
  mediaQuery: {
    'min-width': '641px',
    'max-width': '1024px'
  },
  style: {
    'flex-direction': 'row',
    'padding': '24px',
    'width': '90%'
  }
}

// Desktop variant
const desktopVariant: StyleVariant = {
  id: 'desktop-1',
  breakpoint: 'large',
  mediaQuery: {
    'min-width': '1025px'
  },
  style: {
    'flex-direction': 'row',
    'padding': '32px',
    'width': '80%',
    'max-width': '1200px'
  }
}

// Generated CSS
/*
@media (max-width: 640px) {
  .container {
    flex-direction: column;
    padding: 16px;
    width: 100%;
  }
}

@media (min-width: 641px) and (max-width: 1024px) {
  .container {
    flex-direction: row;
    padding: 24px;
    width: 90%;
  }
}

@media (min-width: 1025px) {
  .container {
    flex-direction: row;
    padding: 32px;
    width: 80%;
    max-width: 1200px;
  }
}
*/
```

## Style Generation Pipeline

### Class Name Generation

Deterministic class names from style hashes:

```typescript
import { hash, generateAlphabeticName } from './hash'

function getClassName(object: any): string {
  return generateAlphabeticName(hash(JSON.stringify(object)))
}

function toValidClassName(
  input: string,
  escapeSpecialCharacters: boolean = false
): string {
  // Replace invalid characters with hyphens
  let className = input
    .trim()
    .replace(/\s+/g, '-')
  
  if (escapeSpecialCharacters) {
    className = className.replace(/[^a-zA-Z0-9-_]/g, (match) => `\\${match}`)
  }
  
  // Ensure doesn't start with number
  if (/^[^a-zA-Z]/.test(className)) {
    className = `_${className}`
  }
  
  return className
}

// Example usage
const styleHash = hash(JSON.stringify({
  'background-color': '#007bff',
  'color': '#ffffff'
}))
const className = generateAlphabeticName(styleHash)
// Result: "button_a1b2c3"
```

### CSS Generation

Converting style models to CSS strings:

```typescript
function generateCSS(
  className: string,
  style: NodeStyleModel,
  variant?: StyleVariant
): string {
  const selector = buildSelector(className, variant)
  const properties = Object.entries(style)
    .map(([property, value]) => `  ${toKebabCase(property)}: ${value};`)
    .join('\n')
  
  return `${selector} {\n${properties}\n}`
}

function buildSelector(
  className: string,
  variant?: StyleVariant
): string {
  if (!variant) {
    return `.${className}`
  }
  
  const parts: string[] = [`.${className}`]
  
  // Pseudo-classes
  if (variant.hover) parts.push(':hover')
  if (variant.focus) parts.push(':focus')
  if (variant.active) parts.push(':active')
  if (variant.disabled) parts.push(':disabled')
  
  // Structural pseudo-classes
  if (variant.firstChild) parts.push(':first-child')
  if (variant.lastChild) parts.push(':last-child')
  if (variant.evenChild) parts.push(':nth-child(even)')
  if (variant.empty) parts.push(':empty')
  
  // Class-based
  if (variant.className && !variant.mediaQuery) {
    parts.push(`.${variant.className}`)
  }
  
  return parts.join('')
}

function generateVariantCSS(
  className: string,
  variant: StyleVariant
): string {
  const baseSelector = buildSelector(className, variant)
  const properties = Object.entries(variant.style)
    .map(([prop, val]) => `  ${toKebabCase(prop)}: ${val};`)
    .join('\n')
  
  // Wrap in media query if responsive
  if (variant.mediaQuery) {
    const mediaCondition = buildMediaCondition(variant.mediaQuery)
    return `@media ${mediaCondition} {\n  ${baseSelector} {\n${properties}\n  }\n}`
  }
  
  return `${baseSelector} {\n${properties}\n}`
}

function buildMediaCondition(mediaQuery: MediaQuery): string {
  const conditions: string[] = []
  
  if (mediaQuery['min-width']) {
    conditions.push(`(min-width: ${mediaQuery['min-width']})`)
  }
  if (mediaQuery['max-width']) {
    conditions.push(`(max-width: ${mediaQuery['max-width']})`)
  }
  if (mediaQuery['min-height']) {
    conditions.push(`(min-height: ${mediaQuery['min-height']})`)
  }
  if (mediaQuery['max-height']) {
    conditions.push(`(max-height: ${mediaQuery['max-height']})`)
  }
  
  return conditions.join(' and ')
}
```

### Full Style Pipeline

Complete style generation from component to CSS:

```typescript
interface StyleGenerationResult {
  className: string
  css: string
  styleVariables: Record<string, string>
}

function generateComponentStyles(
  node: ElementNodeModel | ComponentNodeModel
): StyleGenerationResult {
  // Generate base class name
  const className = getClassName(node.style)
  
  // Generate CSS variables from style-variables
  const styleVariables = node['style-variables']
    ? evaluate_style_variables(node['style-variables'])
    : {}
  
  // Generate base CSS
  const cssParts: string[] = []
  
  // CSS custom properties
  if (Object.keys(styleVariables).length > 0) {
    const varCss = generateCSSVariables(className, styleVariables)
    cssParts.push(varCss)
  }
  
  // Base styles
  cssParts.push(generateCSS(className, node.style))
  
  // Variant styles
  if (node.variants) {
    for (const variant of node.variants) {
      cssParts.push(generateVariantCSS(className, variant))
    }
  }
  
  return {
    className,
    css: cssParts.join('\n\n'),
    styleVariables
  }
}

function generateCSSVariables(
  className: string,
  variables: Record<string, string>
): string {
  const properties = Object.entries(variables)
    .map(([name, value]) => `  ${name}: ${value};`)
    .join('\n')
  
  return `.${className} {\n${properties}\n}`
}
```

## CSS Editor

### Visual Style Panel

The style panel provides a visual interface for CSS properties:

```typescript
interface StylePanelProps {
  node: ElementNodeModel | ComponentNodeModel
  onUpdate: (updates: Partial<NodeStyleModel>) => void
}

// Property groups
const STYLE_PROPERTY_GROUPS = {
  size: ['width', 'height', 'min-width', 'max-width', 'min-height', 'max-height'],
  layout: ['display', 'flex-direction', 'justify-content', 'align-items', 'gap'],
  position: ['position', 'top', 'right', 'bottom', 'left', 'z-index'],
  text: ['font-family', 'font-size', 'font-weight', 'line-height', 'letter-spacing'],
  background: ['background-color', 'background-image', 'gradient'],
  border: ['border', 'border-radius', 'border-width', 'border-color'],
  shadow: ['box-shadow', 'text-shadow'],
  effects: ['opacity', 'filter', 'backdrop-filter'],
  transform: ['transform', 'transform-origin'],
  transition: ['transition', 'animation'],
  advanced: []  // Any unlisted properties
}

// Property editor component
function StylePropertyEditor({
  property,
  value,
  onChange
}: {
  property: string
  value: string
  onChange: (value: string) => void
}) {
  switch (property) {
    case 'display':
      return <Select
        value={value}
        onChange={onChange}
        options={['flex', 'block', 'inline', 'grid', 'none']}
      />
    
    case 'background-color':
      return <ColorPicker value={value} onChange={onChange} />
    
    case 'font-size':
      return <UnitInput
        value={value}
        onChange={onChange}
        units={['px', 'em', 'rem', '%']}
      />
    
    default:
      return <TextInput value={value} onChange={onChange} />
  }
}
```

### CSS Code Editor

Direct CSS code editing mode:

```typescript
interface CSSEditorProps {
  className: string
  variants: StyleVariant[]
  onChange: (variants: StyleVariant[]) => void
}

function CSSEditor({ className, variants, onChange }: CSSEditorProps) {
  // Convert variants to CSS string
  const cssCode = useMemo(() => {
    let css = `.${className} {\n`
    
    // Base styles
    const baseVariant = variants.find(v => 
      !v.hover && !v.focus && !v.mediaQuery
    )
    if (baseVariant) {
      css += formatCSSProperties(baseVariant.style)
    }
    css += '}\n'
    
    // Variant styles
    for (const variant of variants) {
      if (variant === baseVariant) continue
      
      css += `\n.${className}${getVariantSelector(variant)} {\n`
      css += formatCSSProperties(variant.style)
      css += '}\n'
    }
    
    return css
  }, [className, variants])
  
  // Parse CSS string back to variants
  const parseCSS = (cssString: string): StyleVariant[] => {
    const variants: StyleVariant[] = []
    
    // Parse each rule
    const ruleRegex = /(\.?[\w-]+)([^\{]+)?\s*\{([^}]+)\}/g
    let match: RegExpExecArray | null
    
    while ((match = ruleRegex.exec(cssString)) !== null) {
      const [, selector, pseudo, properties] = match
      const parsedProperties = parseCSSProperties(properties)
      
      const variant: StyleVariant = {
        id: generateId(),
        className: selector.replace('.', ''),
        style: parsedProperties,
        ...parsePseudoClasses(pseudo || '')
      }
      
      variants.push(variant)
    }
    
    return variants
  }
  
  return (
    <CodeEditor
      language="css"
      value={cssCode}
      onChange={(newCode) => {
        const newVariants = parseCSS(newCode)
        onChange(newVariants)
      }}
    />
  )
}
```

## Component Style Overrides

### Instance-specific Styling

Styling component instances from the outside:

```typescript
// Component instance with style overrides
const cardInstance: ComponentNodeModel = {
  type: 'component',
  name: 'Card',
  style: {
    // Override root element styles
    'background-color': '#f8f9fa',
    'border': '1px solid #dee2e6'
  },
  variants: [
    {
      hover: true,
      style: {
        'box-shadow': '0 8px 24px rgba(0,0,0,0.12)',
        'transform': 'translateY(-4px)'
      }
    }
  ]
}

// Style override application
function applyComponentStyleOverrides(
  component: Component,
  instance: ComponentNodeModel,
  element: HTMLElement
): void {
  // Apply instance styles to root
  if (instance.style) {
    for (const [property, value] of Object.entries(instance.style)) {
      element.style.setProperty(property, value)
    }
  }
  
  // Generate and apply variant classes
  if (instance.variants) {
    for (const variant of instance.variants) {
      const variantClass = getClassName(variant.style)
      element.classList.add(variantClass)
      
      // Inject variant CSS if not already present
      if (!document.getElementById(variantClass)) {
        injectVariantCSS(variantClass, variant)
      }
    }
  }
}
```

### Style Inheritance and Cascade

Understanding style precedence:

```typescript
// Style cascade order
const STYLE_CASCADE_ORDER = [
  'component-default',     // Component's default styles
  'instance-style',        // Instance-specific inline styles
  'instance-variants',     // Instance variant styles (in order)
  'global-styles'          // Global CSS overrides
]

// Calculate final styles with cascade
function calculateFinalStyles(
  component: Component,
  instance: ComponentNodeModel,
  activePseudoClasses: Set<string>
): NodeStyleModel {
  const finalStyles: NodeStyleModel = {}
  
  // 1. Start with component defaults
  const rootElement = component.nodes['root'] as ElementNodeModel
  if (rootElement?.style) {
    Object.assign(finalStyles, rootElement.style)
  }
  
  // 2. Apply instance inline styles
  if (instance.style) {
    Object.assign(finalStyles, instance.style)
  }
  
  // 3. Apply matching variants (order matters!)
  if (instance.variants) {
    for (const variant of instance.variants) {
      if (variantMatches(variant, activePseudoClasses)) {
        Object.assign(finalStyles, variant.style)
      }
    }
  }
  
  return finalStyles
}

function variantMatches(
  variant: StyleVariant,
  activePseudoClasses: Set<string>
): boolean {
  // Check pseudo-class requirements
  if (variant.hover && !activePseudoClasses.has('hover')) return false
  if (variant.focus && !activePseudoClasses.has('focus')) return false
  if (variant.active && !activePseudoClasses.has('active')) return false
  
  // All requirements met
  return true
}
```

## Theme System

### CSS Custom Properties for Theming

Theme definition using CSS variables:

```typescript
interface ThemeDefinition {
  colors: Record<string, string>
  spacing: Record<string, string>
  typography: Record<string, string>
  shadows: Record<string, string>
  breakpoints: Record<string, string>
}

const DEFAULT_THEME: ThemeDefinition = {
  colors: {
    'primary': '#007bff',
    'primary-hover': '#0056b3',
    'secondary': '#6c757d',
    'success': '#28a745',
    'danger': '#dc3545',
    'background': '#ffffff',
    'surface': '#f8f9fa',
    'text': '#212529',
    'text-muted': '#6c757d'
  },
  spacing: {
    'xs': '4px',
    'sm': '8px',
    'md': '16px',
    'lg': '24px',
    'xl': '32px'
  },
  typography: {
    'font-family': 'Inter, system-ui, sans-serif',
    'font-size-base': '16px',
    'font-size-sm': '14px',
    'font-size-lg': '18px',
    'font-weight-normal': '400',
    'font-weight-bold': '700'
  },
  shadows: {
    'sm': '0 1px 2px rgba(0,0,0,0.05)',
    'md': '0 4px 12px rgba(0,0,0,0.1)',
    'lg': '0 8px 24px rgba(0,0,0,0.15)'
  }
}

// Theme CSS generation
function generateThemeCSS(theme: ThemeDefinition): string {
  const cssProperties: string[] = []
  
  // Colors
  for (const [name, value] of Object.entries(theme.colors)) {
    cssProperties.push(`  --color-${toKebabCase(name)}: ${value};`)
  }
  
  // Spacing
  for (const [name, value] of Object.entries(theme.spacing)) {
    cssProperties.push(`  --spacing-${name}: ${value};`)
  }
  
  // Typography
  for (const [name, value] of Object.entries(theme.typography)) {
    cssProperties.push(`  --${toKebabCase(name)}: ${value};`)
  }
  
  // Shadows
  for (const [name, value] of Object.entries(theme.shadows)) {
    cssProperties.push(`  --shadow-${name}: ${value};`)
  }
  
  return `:root {\n${cssProperties.join('\n')}\n}`
}
```

### Using Theme Variables in Styles

Referencing theme variables:

```typescript
// Component using theme variables
const themedButton: ElementNodeModel = {
  type: 'element',
  tag: 'button',
  style: {
    'background-color': 'var(--color-primary)',
    'color': 'var(--color-background)',
    'padding': 'var(--spacing-sm) var(--spacing-lg)',
    'font-size': 'var(--font-size-base)',
    'font-weight': 'var(--font-weight-bold)',
    'box-shadow': 'var(--shadow-md)',
    'border-radius': 'var(--border-radius-sm)'
  },
  variants: [
    {
      hover: true,
      style: {
        'background-color': 'var(--color-primary-hover)',
        'box-shadow': 'var(--shadow-lg)'
      }
    }
  ]
}
```

## Style Copy and Transfer

### Copy Styles Feature

Transferring styles between elements:

```typescript
interface CopiedStyles {
  style: NodeStyleModel
  variants: StyleVariant[]
  styleVariables: StyleVariable[]
}

function copyStyles(node: ElementNodeModel): CopiedStyles {
  return {
    style: { ...node.style },
    variants: node.variants?.map(v => ({ ...v })) || [],
    styleVariables: node['style-variables']?.map(sv => ({ ...sv })) || []
  }
}

function pasteStyles(
  targetNode: ElementNodeModel,
  copiedStyles: CopiedStyles
): ElementNodeModel {
  return {
    ...targetNode,
    style: { ...copiedStyles.style },
    variants: copiedStyles.variants.map(v => ({
      ...v,
      id: generateId(),  // New IDs for pasted variants
      className: undefined  // Regenerate class names
    })),
    'style-variables': copiedStyles.styleVariables
  }
}
```

## Summary

The Nordcraft styling engine provides:

1. **Visual Style Panel**: Logical property groups with appropriate editors
2. **CSS Code Editor**: Direct CSS authoring with live synchronization
3. **Style Variables**: Formula-driven dynamic styles with CSS custom properties
4. **Conditional Styles**: Pseudo-classes, class-based conditions, media queries
5. **Responsive Breakpoints**: Mobile-first responsive design with visual controls
6. **Component Style Overrides**: Instance-specific styling without modifying component definition
7. **Theme System**: CSS custom properties for consistent theming
8. **Deterministic Class Generation**: Hash-based class names for optimization
9. **Style Copy/Paste**: Transfer complete styling between elements

This architecture enables both visual and code-based styling workflows while generating optimized, production-ready CSS.
