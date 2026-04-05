# Nordcraft Figma Integration Deep Dive

## Overview

The Nordcraft (formerly Toddle) Figma plugin enables designers to export Figma designs as production-ready HTML/Tailwind CSS code that can be directly imported into Nordcraft projects. This deep-dive examines the plugin architecture, conversion pipeline, and design-to-code synchronization patterns.

## Plugin Architecture

### Monorepo Structure

The Figma plugin is organized as a monorepo with multiple packages:

```
figma-plugin/
├── apps/
│   ├── plugin/           # Main Figma plugin
│   │   ├── plugin-src/   # Background script (code.ts -> code.js)
│   │   └── ui-src/       # UI components (main.tsx -> index.html)
│   └── debug/            # Debug web app for UI development
├── packages/
│   ├── backend/          # Core conversion logic
│   ├── plugin-ui/        # Shared UI components
│   ├── types/            # TypeScript type definitions
│   └── eslint-config-custom/
└── turbo.json
```

### Plugin Modes

The plugin operates in two modes:

```typescript
// Standard mode - Full plugin UI
async function standardMode(): Promise<void> {
  figma.showUI(__html__, {
    width: 384,
    height: 706,
    themeColors: false
  })
  
  await initSettings()
  
  // Listen for selection changes
  figma.on('selectionchange', () => {
    safeRun(userPluginSettings)
  })
  
  // Listen for document changes
  figma.on('documentchange', () => {
    // Debounced re-conversion
    safeRun(userPluginSettings)
  })
}

// Codegen mode - Figma Dev Mode integration
async function codegenMode(): Promise<void> {
  await getUserSettings()
  
  figma.codegen.on('generate', async ({ language, node }) => {
    const convertedSelection = convertIntoNodes([node], null)
    
    if (language === 'html') {
      return [
        {
          title: 'Code',
          code: await htmlMain(convertedSelection, settings, true),
          language: 'HTML'
        },
        {
          title: 'Text Styles',
          code: htmlCodeGenTextStyles(settings),
          language: 'HTML'
        }
      ] as CodegenResult[]
    }
  })
}
```

### Plugin Settings

```typescript
interface PluginSettings {
  framework: 'HTML' | 'Flutter' | 'SwiftUI'
  jsx: boolean
  optimizeLayout: boolean
  showLayerNames: boolean
  inlineStyle: boolean
  responsiveRoot: boolean
  flutterGenerationMode: 'snippet' | 'full'
  swiftUIGenerationMode: 'snippet' | 'full'
  roundTailwindValues: boolean
  roundTailwindColors: boolean
  customTailwindColors: boolean
  customTailwindPrefix: string
  embedImages: boolean
}

const defaultPluginSettings: PluginSettings = {
  framework: 'HTML',       // Always HTML for Nordcraft
  jsx: false,
  optimizeLayout: false,
  showLayerNames: false,
  inlineStyle: true,
  responsiveRoot: false,
  flutterGenerationMode: 'snippet',
  swiftUIGenerationMode: 'snippet',
  roundTailwindValues: false,
  roundTailwindColors: false,
  customTailwindColors: false,
  customTailwindPrefix: '',
  embedImages: false
}
```

## Conversion Pipeline

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Figma Selection                           │
│  (User selects frames, groups, or individual layers)        │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              Node to AltNode Conversion                      │
│  - Traverse Figma Node tree                                 │
│  - Create intermediate AltNode representation               │
│  - Apply layout optimizations                              │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│              Style Extraction & Processing                   │
│  - Extract colors, gradients, effects                        │
│  - Convert to CSS/Tailwind values                           │
│  - Generate color variables                                  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  HTML Code Generation                        │
│  - Generate HTML structure                                   │
│  - Apply CSS classes or inline styles                       │
│  - Handle responsive layouts                                 │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Output to Nordcraft                       │
│  - Copy code to clipboard                                    │
│  - Display preview in plugin UI                              │
│  - Show conversion warnings                                  │
└─────────────────────────────────────────────────────────────┘
```

### AltNode Intermediate Representation

The plugin creates an intermediate representation called `AltNodes`:

```typescript
// AltNode types
type AltNode =
  | AltFrame
  | AltGroup
  | AltRect
  | AltText
  | AltEllipse
  | AltLine
  | AltVector
  | AltSlice
  | AltBooleanOperation

interface AltFrame {
  id: string
  name: string
  type: 'FRAME'
  x: number
  y: number
  width: number
  height: number
  rotation: number
  children: AltNode[]
  
  // Layout properties
  layoutMode: 'NONE' | 'HORIZONTAL' | 'VERTICAL'
  primaryAxisAlignItems: 'MIN' | 'MAX' | 'CENTER' | 'SPACE_BETWEEN'
  counterAxisAlignItems: 'MIN' | 'MAX' | 'CENTER' | 'BASELINE'
  paddingLeft: number
  paddingRight: number
  paddingTop: number
  paddingBottom: number
  itemSpacing: number
  
  // Style properties
  fills: Paint[]
  strokes: Stroke[]
  effects: Effect[]
  clipsContent: boolean
  background: Paint[]
}

interface AltText {
  id: string
  name: string
  type: 'TEXT'
  x: number
  y: number
  width: number
  height: number
  
  // Text properties
  characters: string
  style: TextStyle
  
  // Font properties
  fontFamily: string
  fontWeight: number
  fontSize: number
  lineHeight: number
  letterSpacing: number
  textAlignHorizontal: 'LEFT' | 'RIGHT' | 'CENTER' | 'JUSTIFIED'
  textAlignVertical: 'TOP' | 'BOTTOM' | 'CENTER'
}
```

### Node Conversion

Converting Figma nodes to AltNodes:

```typescript
function convertNodesToAltNodes(
  nodes: ReadonlyArray<SceneNode>,
  parent: AltNode | null
): AltNode[] {
  const altNodes: AltNode[] = []
  
  for (const node of nodes) {
    const altNode = convertNodeToAltNode(node, parent)
    if (altNode) {
      altNodes.push(altNode)
      
      // Recursively convert children
      if ('children' in node && node.children) {
        altNode.children = convertNodesToAltNodes(
          node.children,
          altNode
        )
      }
    }
  }
  
  return altNodes
}

function convertNodeToAltNode(
  node: SceneNode,
  parent: AltNode | null
): AltNode | null {
  // Skip hidden nodes
  if (!node.visible) {
    return null
  }
  
  switch (node.type) {
    case 'FRAME':
    case 'COMPONENT':
    case 'INSTANCE':
      return convertFrame(node, parent)
    
    case 'RECTANGLE':
      return convertRect(node, parent)
    
    case 'TEXT':
      return convertText(node, parent)
    
    case 'ELLIPSE':
      return convertEllipse(node, parent)
    
    case 'LINE':
      return convertLine(node, parent)
    
    case 'VECTOR':
      // Vectors are not fully supported
      addWarning(`Vector "${node.name}" cannot be converted`)
      return null
    
    case 'GROUP':
      // Groups are converted to frames
      return convertGroup(node, parent)
    
    default:
      addWarning(`Unsupported node type: ${node.type}`)
      return null
  }
}
```

### Layout Optimization

The plugin optimizes layouts during conversion:

```typescript
interface LayoutOptimization {
  // Detect flexbox patterns
  detectFlexbox(frame: AltFrame): FlexProperties
  
  // Convert absolute positioning to flex
  convertAbsoluteToFlex(children: AltNode[]): AltNode[]
  
  // Merge nested frames with single children
  mergeSingleChildFrames(node: AltNode): AltNode
  
  // Detect and apply responsive constraints
  applyConstraints(node: AltNode, parent: AltNode): void
}

// Auto-layout detection
function detectFlexbox(frame: AltFrame): FlexProperties {
  // Check if frame has auto-layout
  if (frame.layoutMode !== 'NONE') {
    return {
      display: 'flex',
      flexDirection: frame.layoutMode === 'HORIZONTAL' 
        ? 'row' 
        : 'column',
      justifyContent: mapAxisAlign(frame.primaryAxisAlignItems),
      alignItems: mapAxisAlign(frame.counterAxisAlignItems),
      gap: frame.itemSpacing,
      padding: {
        top: frame.paddingTop,
        right: frame.paddingRight,
        bottom: frame.paddingBottom,
        left: frame.paddingLeft
      }
    }
  }
  
  // Detect flex pattern from constraints
  const children = frame.children
  if (children.length === 0) return null
  
  // Analyze child constraints for flex pattern
  const hasHorizontalStretch = children.some(
    child => child.constraints.horizontal === 'LEFT_RIGHT'
  )
  const hasVerticalStretch = children.some(
    child => child.constraints.vertical === 'TOP_BOTTOM'
  )
  
  if (hasHorizontalStretch && !hasVerticalStretch) {
    return { flexDirection: 'column', alignItems: 'stretch' }
  }
  
  return null
}
```

## Style Conversion

### Color Extraction

Extracting and converting colors:

```typescript
interface ExtractedColor {
  figmaColor: Color
  cssColor: string
  tailwindClass?: string
  variableName?: string
  usageCount: number
}

function retrieveGenericSolidUIColors(
  framework: Framework
): ExtractedColor[] {
  const colors: Map<string, ExtractedColor> = new Map()
  
  // Traverse all nodes and extract solid UI colors
  function extractColors(node: AltNode): void {
    if ('fills' in node && node.fills) {
      for (const fill of node.fills) {
        if (fill.type === 'SOLID') {
          const colorKey = colorToKey(fill.color)
          const existing = colors.get(colorKey)
          
          if (existing) {
            existing.usageCount++
          } else {
            colors.set(colorKey, {
              figmaColor: fill.color,
              cssColor: rgbToCss(fill.color),
              tailwindClass: findNearestTailwind(fill.color),
              usageCount: 1
            })
          }
        }
      }
    }
    
    // Recursively extract from children
    if ('children' in node) {
      for (const child of node.children) {
        extractColors(child)
      }
    }
  }
  
  // Extract gradients
  function extractGradients(node: AltNode): void {
    if ('fills' in node && node.fills) {
      for (const fill of node.fills) {
        if (fill.type === 'GRADIENT_LINEAR') {
          // Process gradient
        }
      }
    }
  }
  
  return Array.from(colors.values())
    .sort((a, b) => b.usageCount - a.usageCount)
}

// Color conversion utilities
function rgbToCss(color: Color): string {
  const r = Math.round(color.r * 255)
  const g = Math.round(color.g * 255)
  const b = Math.round(color.b * 255)
  const a = color.a ?? 1
  
  if (a === 1) {
    return `rgb(${r}, ${g}, ${b})`
  }
  return `rgba(${r}, ${g}, ${b}, ${a})`
}

function findNearestTailwind(color: Color): string {
  const targetHex = rgbToHex(color)
  
  // Find nearest Tailwind color
  let nearestColor: string | null = null
  let nearestDistance = Infinity
  
  for (const [tailwindClass, hex] of TAILWIND_COLORS) {
    const distance = colorDistance(targetHex, hex)
    if (distance < nearestDistance) {
      nearestDistance = distance
      nearestColor = tailwindClass
    }
  }
  
  // Only return if close enough
  return nearestDistance < THRESHOLD ? nearestColor : undefined
}
```

### Text Style Conversion

```typescript
interface TextStyle {
  fontFamily: string
  fontWeight: number
  fontSize: number
  lineHeight: number
  letterSpacing: number
  textAlign: 'left' | 'right' | 'center' | 'justify'
  textTransform: 'none' | 'uppercase' | 'lowercase' | 'capitalize'
  textDecoration: 'none' | 'underline' | 'line-through'
}

function convertTextStyle(style: TextStyle): CSSProperties {
  return {
    'font-family': style.fontFamily,
    'font-weight': convertFontWeight(style.fontWeight),
    'font-size': `${style.fontSize}px`,
    'line-height': style.lineHeight / style.fontSize,
    'letter-spacing': `${style.letterSpacing}px`,
    'text-align': style.textAlign,
    'text-transform': style.textTransform,
    'text-decoration': style.textDecoration
  }
}

function convertFontWeight(weight: number): string {
  // Figma uses numeric weights (100-900)
  const weightMap: Record<number, string> = {
    100: '100',
    200: '200',
    300: '300',
    400: '400',
    500: '500',
    600: '600',
    700: '700',
    800: '800',
    900: '900'
  }
  
  return weightMap[weight] || '400'
}
```

### Effect Conversion (Shadows)

```typescript
interface ShadowEffect {
  type: 'DROP_SHADOW' | 'INNER_SHADOW'
  color: Color
  offset: Vector
  radius: number
  spread: number
  blendMode: BlendMode
}

function convertShadow(effect: ShadowEffect): string {
  const color = rgbToCss(effect.color)
  const offsetX = effect.offset.x
  const offsetY = effect.offset.y
  const blur = effect.radius
  const spread = effect.spread || 0
  
  if (effect.type === 'INNER_SHADOW') {
    return `inset ${offsetX}px ${offsetY}px ${blur}px ${spread}px ${color}`
  }
  
  return `${offsetX}px ${offsetY}px ${blur}px ${spread}px ${color}`
}

// Multiple shadows
function convertShadows(effects: Effect[]): string {
  const shadows = effects
    .filter((e): e is ShadowEffect => 
      e.type === 'DROP_SHADOW' || e.type === 'INNER_SHADOW'
    )
    .map(convertShadow)
  
  return shadows.join(', ')
}
```

## HTML Code Generation

### HTML Builder Pattern

```typescript
interface HTMLBuilder {
  // Node conversion
  buildNode(node: AltNode): string
  
  // Style generation
  buildStyles(node: AltNode): string
  
  // Layout handling
  buildLayout(node: AltNode): string
  
  // Children rendering
  buildChildren(node: AltNode): string
}

class HTMLDefaultBuilder implements HTMLBuilder {
  buildNode(node: AltNode): string {
    const tag = this.getTagForNode(node)
    const styles = this.buildStyles(node)
    const children = this.buildChildren(node)
    
    if (children) {
      return `<${tag}${styles}>${children}</${tag}>`
    }
    
    return `<${tag}${styles} />`
  }
  
  getTagForNode(node: AltNode): string {
    switch (node.type) {
      case 'TEXT':
        return 'p'
      case 'FRAME':
      case 'GROUP':
        return 'div'
      case 'RECT':
        return 'div'
      case 'ELLIPSE':
        return 'div'  // With border-radius: 50%
      default:
        return 'div'
    }
  }
  
  buildStyles(node: AltNode): string {
    const styles = this.extractStyles(node)
    const classes = this.generateClasses(styles)
    
    if (inlineStyle) {
      const styleString = this.stylesToInline(styles)
      return ` style="${styleString}"`
    }
    
    return ` class="${classes}"`
  }
}
```

### Layout Conversion

```typescript
function buildAutoLayout(frame: AltFrame): CSSProperties {
  const styles: CSSProperties = {}
  
  // Display
  styles.display = 'flex'
  
  // Flex direction
  styles.flexDirection = frame.layoutMode === 'HORIZONTAL' 
    ? 'row' 
    : 'column'
  
  // Justify content (primary axis)
  styles.justifyContent = mapAxisAlign(
    frame.primaryAxisAlignItems
  )
  
  // Align items (counter axis)
  styles.alignItems = mapAxisAlign(
    frame.counterAxisAlignItems
  )
  
  // Gap
  if (frame.itemSpacing > 0) {
    styles.gap = `${frame.itemSpacing}px`
  }
  
  // Padding
  if (frame.paddingTop > 0) {
    styles.paddingTop = `${frame.paddingTop}px`
  }
  if (frame.paddingRight > 0) {
    styles.paddingRight = `${frame.paddingRight}px`
  }
  if (frame.paddingBottom > 0) {
    styles.paddingBottom = `${frame.paddingBottom}px`
  }
  if (frame.paddingLeft > 0) {
    styles.paddingLeft = `${frame.paddingLeft}px`
  }
  
  return styles
}

function mapAxisAlign(
  align: 'MIN' | 'MAX' | 'CENTER' | 'SPACE_BETWEEN' | 'BASELINE'
): string {
  switch (align) {
    case 'MIN':
      return 'flex-start'
    case 'MAX':
      return 'flex-end'
    case 'CENTER':
      return 'center'
    case 'SPACE_BETWEEN':
      return 'space-between'
    case 'BASELINE':
      return 'baseline'
    default:
      return 'flex-start'
  }
}
```

### Responsive Constraints

```typescript
interface Constraints {
  horizontal: 'LEFT' | 'RIGHT' | 'LEFT_RIGHT' | 'CENTER' | 'SCALE'
  vertical: 'TOP' | 'BOTTOM' | 'TOP_BOTTOM' | 'CENTER' | 'SCALE'
}

function applyConstraints(
  node: AltNode,
  parent: AltFrame
): CSSProperties {
  const constraints = node.constraints
  const styles: CSSProperties = {}
  
  // Horizontal constraints
  switch (constraints.horizontal) {
    case 'LEFT_RIGHT':
      // Stretch horizontally
      styles.width = '100%'
      break
    
    case 'CENTER':
      // Center horizontally
      styles.marginLeft = 'auto'
      styles.marginRight = 'auto'
      break
    
    case 'SCALE':
      // Scale with parent
      styles.width = `${(node.width / parent.width) * 100}%`
      break
    
    case 'RIGHT':
      // Stick to right
      styles.marginLeft = 'auto'
      break
  }
  
  // Vertical constraints
  switch (constraints.vertical) {
    case 'TOP_BOTTOM':
      // Stretch vertically
      styles.height = '100%'
      break
    
    case 'CENTER':
      // Center vertically
      styles.marginTop = 'auto'
      styles.marginBottom = 'auto'
      break
    
    case 'SCALE':
      // Scale with parent
      styles.height = `${(node.height / parent.height) * 100}%`
      break
    
    case 'BOTTOM':
      // Stick to bottom
      styles.marginTop = 'auto'
      break
  }
  
  return styles
}
```

### Full HTML Preview Generation

```typescript
async function generateHTMLPreview(
  nodes: AltNode[],
  settings: PluginSettings,
  code: string
): Promise<string> {
  const colors = retrieveGenericSolidUIColors(settings.framework)
  const gradients = retrieveGenericGradients(settings.framework)
  
  // Generate color variables
  const colorVariables = colors
    .map(c => `  --color-${c.variableName}: ${c.cssColor};`)
    .join('\n')
  
  // Generate full HTML preview
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Figma Preview</title>
  <style>
    :root {
${colorVariables}
    }
    
    * {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }
    
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      padding: 24px;
    }
  </style>
</head>
<body>
  ${code}
</body>
</html>`
}
```

## Plugin UI

### Preview Component

```typescript
interface PluginUIProps {
  code: string
  htmlPreview: string
  colors: ExtractedColor[]
  gradients: ExtractedGradient[]
  warnings: ConversionWarning[]
  settings: PluginSettings
}

function PluginUI({ 
  code, 
  htmlPreview, 
  colors, 
  gradients,
  warnings,
  settings 
}: PluginUIProps) {
  const [activeTab, setActiveTab] = useState<'code' | 'preview' | 'colors'>('code')
  
  return (
    <div className="plugin-ui">
      <Tabs activeTab={activeTab} onChange={setActiveTab}>
        <Tab id="code" label="Code" />
        <Tab id="preview" label="Preview" />
        <Tab id="colors" label="Colors" />
      </Tabs>
      
      <TabPanel id="code">
        <CodeEditor 
          language="html"
          value={code}
          onCopy={() => copyToClipboard(code)}
        />
      </TabPanel>
      
      <TabPanel id="preview">
        <PreviewFrame html={htmlPreview} />
      </TabPanel>
      
      <TabPanel id="colors">
        <ColorGrid colors={colors} />
        <GradientGrid gradients={gradients} />
      </TabPanel>
      
      {warnings.length > 0 && (
        <WarningsPanel warnings={warnings} />
      )}
    </div>
  )
}
```

### Conversion Warnings

```typescript
interface ConversionWarning {
  message: string
  nodeId?: string
  nodeName?: string
  severity: 'info' | 'warning' | 'error'
}

const warningMessages: ConversionWarning[] = []

function addWarning(message: string): void {
  warningMessages.push({
    message,
    severity: 'warning'
  })
}

// Common warnings
function checkUnsupportedFeatures(node: SceneNode): void {
  if (node.type === 'VECTOR') {
    addWarning(`Vector illustrations ("${node.name}") are not supported`)
  }
  
  if (node.type === 'POLYGON' || node.type === 'STAR') {
    addWarning(`Polygon and Star shapes ("${node.name}") are not supported`)
  }
  
  if ('fills' in node) {
    for (const fill of node.fills) {
      if (fill.type === 'IMAGE') {
        addWarning(`Image fills ("${node.name}") may not export correctly`)
      }
      if (fill.type === 'VIDEO') {
        addWarning(`Video fills ("${node.name}") are not supported`)
      }
    }
  }
  
  if ('strokes' in node && node.strokes.length > 0) {
    addWarning(`Stroke styles ("${node.name}") may not export exactly as designed`)
  }
}
```

## Design-to-Nordcraft Workflow

### Import Process

```typescript
// 1. Export from Figma
async function exportFromFigma(): Promise<ExportResult> {
  const selection = figma.currentPage.selection
  
  if (selection.length === 0) {
    throw new Error('Please select elements to export')
  }
  
  // Convert to AltNodes
  const altNodes = convertNodesToAltNodes(selection, null)
  
  // Generate code
  const settings = await getUserSettings()
  const code = await convertToCode(altNodes, settings)
  
  // Generate preview
  const htmlPreview = await generateHTMLPreview(
    altNodes,
    settings,
    code
  )
  
  // Extract colors
  const colors = retrieveGenericSolidUIColors(settings.framework)
  const gradients = retrieveGenericGradients(settings.framework)
  
  return {
    code,
    htmlPreview,
    colors,
    gradients,
    warnings: [...warningMessages]
  }
}

// 2. Import to Nordcraft
function importToNordcraft(exportResult: ExportResult): Component {
  const { code, colors } = exportResult
  
  // Parse HTML structure
  const nodes = parseHTMLToNodes(code)
  
  // Create color variables
  const styleVariables = colors.map(color => ({
    category: 'color' as StyleTokenCategory,
    name: color.variableName,
    formula: { type: 'static', value: color.cssColor }
  }))
  
  // Create component
  const component: Component = {
    name: generateComponentName(),
    nodes: {
      root: nodes[0]
    },
    'style-variables': styleVariables,
    attributes: {},
    variables: {},
    formulas: {},
    workflows: {},
    apis: {},
    events: []
  }
  
  return component
}
```

### Synchronization Patterns

For keeping Figma designs and Nordcraft components in sync:

```typescript
interface SyncState {
  figmaFileKey: string
  figmaNodeId: string
  lastSyncTime: number
  nordcraftComponentId: string
  nordcraftBranch: string
}

// Sync workflow
async function syncFromFigma(
  syncState: SyncState
): Promise<SyncResult> {
  // Fetch latest from Figma API
  const figmaResponse = await fetch(
    `https://api.figma.com/v1/files/${syncState.figmaFileKey}/nodes/${syncState.figmaNodeId}`,
    {
      headers: {
        'X-Figma-Token': FIGMA_API_TOKEN
      }
    }
  )
  
  const figmaNode = await figmaResponse.json()
  
  // Convert to AltNodes
  const altNodes = convertNodesToAltNodes([figmaNode], null)
  
  // Generate updated code
  const code = await convertToCode(altNodes, DEFAULT_SETTINGS)
  
  // Parse and update Nordcraft component
  const updatedNodes = parseHTMLToNodes(code)
  
  return {
    success: true,
    updatedNodes,
    changesDetected: true
  }
}
```

## Limitations and Workarounds

### Known Limitations

```typescript
const LIMITATIONS = {
  // Unsupported features
  unsupported: [
    'Vector illustrations / paths',
    'Images (exported as placeholders)',
    'Line/Star/Polygon shapes',
    'Video fills',
    'Blur effects (limited support)',
    '3D transforms'
  ],
  
  // Tailwind-specific limitations
  tailwind: [
    'Maximum width of 384px (w-full used for larger)',
    'Custom Tailwind config not applied',
    'Some utility classes may differ from project config'
  ],
  
  // Layout limitations
  layout: [
    'Complex absolute positioning may use insets',
    'Mixed constraint directions may not translate perfectly',
    'Nested auto-layout with different directions requires manual adjustment'
  ]
}

// Workaround detection
function suggestWorkarounds(node: AltNode): string[] {
  const workarounds: string[] = []
  
  if (node.type === 'VECTOR') {
    workarounds.push(
      'Consider using an SVG export or simplifying to basic shapes'
    )
  }
  
  if (node.type === 'GROUP' && hasComplexPositioning(node)) {
    workarounds.push(
      'Wrap elements in a Frame with auto-layout for better positioning'
    )
  }
  
  if (hasImageFill(node)) {
    workarounds.push(
      'Export images separately and add as background-image in Nordcraft'
    )
  }
  
  return workarounds
}
```

## Summary

The Nordcraft Figma plugin provides:

1. **Intermediate Representation**: AltNode system for layout optimization before code generation
2. **Style Extraction**: Automatic color palette and gradient extraction
3. **Layout Conversion**: Auto-layout to flexbox, constraints to responsive CSS
4. **HTML Generation**: Production-ready HTML with inline styles or Tailwind classes
5. **Preview System**: Live HTML preview within the plugin UI
6. **Warning System**: Clear communication of unsupported features and workarounds
7. **Dual Mode**: Standard plugin mode and Figma Dev Mode integration
8. **Monorepo Architecture**: Shared packages for backend logic and UI components
9. **Codegen Support**: Native integration with Figma's code generation API
10. **Sync Patterns**: Potential workflows for design-to-code synchronization

This integration enables designers to export production-ready components from Figma that can be directly imported into Nordcraft, bridging the gap between design and development.
