# OpenUI -- Component Library

The `react-ui` package provides 53 prebuilt React components organized into categories: layout, data display, forms, actions, content, and system. Two built-in libraries are exported: `openuiLibrary` (standalone) and `openuiChatLibrary` (chat-focused with component groups).

**Aha:** Components are defined with Zod v4 schemas via `defineComponent({ name, props, description, component })`. The schema serves three purposes: (1) validation — missing required props are dropped with errors, (2) default values — missing props get schema defaults, (3) positional mapping — the LLM can write `Button("Save", true)` instead of verbose named props because the schema property order defines positional argument order. The Zod schema is compiled to JSON Schema (`library.toJSONSchema()`) which the parser uses as a `ParamMap`.

Source: `openui/packages/react-ui/src/` — component library

## Component Categories

| Category | Components | Purpose |
|----------|-----------|---------|
| **Layout** | Stack, Root, Grid, Tabs, SectionBlock, Accordion, BottomTray | Arrange child components |
| **Data Display** | Table, BarChart, BarChartCondensed, AreaChart, AreaChartCondensed, Charts, Carousel, Card, CardHeader, Tag, TagBlock, CodeBlock, ImageBlock, ImageGallery, ListBlock, ListItem | Visualize data |
| **Forms** | Form, FormControl, Input, TextArea, Select, Slider, DatePicker, Calendar, RadioGroup, RadioItem, CheckBoxGroup, CheckBoxItem, SwitchGroup, SwitchItem | User input |
| **Actions** | Button, Buttons, IconButton, FollowUpBlock, FollowUpItem | Trigger actions |
| **Content** | TextContent, TextCallout, Callout, Label, MarkDownRenderer | Display text |
| **System** | MessageLoading, ToolCall, ToolResult, Artifact, ThemeProvider | Framework internals |
| **Shell** | CopilotShell, Shell | Layout context providers |

## Component Schema Example

```json
{
  "name": "Table",
  "description": "Display data in a tabular format",
  "parameters": {
    "properties": {
      "data": { "type": "array", "description": "Rows to display" },
      "columns": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "header": { "type": "string" },
            "accessor": { "type": "string" }
          }
        }
      },
      "striped": { "type": "boolean", "default": false }
    },
    "required": ["data", "columns"]
  }
}
```

## Component Implementation Pattern

```tsx
// Table.tsx
interface TableProps {
  data: any[];
  columns: ColumnDef[];
  striped?: boolean;
}

export function Table({ data, columns, striped = false }: TableProps) {
  return (
    <table>
      <thead>
        <tr>
          {columns.map(col => <th key={col.header}>{col.header}</th>)}
        </tr>
      </thead>
      <tbody>
        {data.map((row, i) => (
          <tr key={i} className={striped && i % 2 === 1 ? 'striped' : ''}>
            {columns.map(col => <td key={col.accessor}>{row[col.accessor]}</td>)}
          </tr>
        ))}
      </tbody>
    </table>
  );
}
```

## Form Field Handling

Form fields use a wrapped value format:

```typescript
interface FormFieldValue {
  value: any;
  componentType: string;  // 'input', 'select', 'textarea', etc.
}
```

The renderer wraps raw values in `FormFieldValue` objects when rendering form components. The `get`/`set` functions on form fields are handled by the evaluator — `get` returns the current store value, `set` updates the store and triggers re-rendering.

**Aha:** Form fields in OpenUI Lang are two-way bound. When the user changes an input, the store updates, and any expression that depends on that store value re-evaluates. This means a chart showing `data=$sales` automatically updates when the user changes the `$sales` filter via a form. The reactivity is end-to-end: form input → store update → query refetch → chart re-render.

## Chat-Specific Components

The `openuiChatLibrary` includes components optimized for chat interfaces:

- `MessageLoading`: Animated loading indicator while LLM responds
- `ToolCall`: Display tool execution (bash, file read, etc.)
- `ToolResult`: Display tool output
- `Artifact`: Portal-managed artifact display (files, images)
- `FollowUpBlock`: Suggested follow-up questions

These components integrate with the `react-headless` chat store for seamless chat rendering.

## ToolProvider Integration

Components can invoke tools through the ToolProvider:

```typescript
interface ToolProvider {
  callTool(toolName: string, args: Record<string, any>): Promise<any>;
}
```

The ToolProvider can be:
- A simple function map: `{ tools: { search: (q) => ... } }`
- An MCP client: connects to a Model Context Protocol server

When the LLM generates `<Button onClick={@Run(search, $query)}>`, the renderer invokes `toolProvider.callTool('search', { query: ... })`.

## Component Groups

Components are organized in groups for the LLM prompt:

```typescript
const chatLibrary = {
  groups: [
    { name: 'Layout', components: ['Stack', 'Grid', 'Tabs'] },
    { name: 'Data', components: ['Table', 'BarChart', 'AreaChart'] },
    { name: 'Forms', components: ['Form', 'Input', 'Select'] },
    { name: 'Actions', components: ['Button', 'FollowUpBlock'] },
  ]
};
```

The prompt generator uses groups to organize the component list in the LLM's system prompt, making it easier for the LLM to find the right component.

See [React Renderer](06-react-renderer.md) for how components are rendered.
See [Evaluator](05-evaluator.md) for how form bindings work.
See [OpenClaw Plugin](08-openclaw-plugin.md) for server-side tools.
