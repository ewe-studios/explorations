# Plugins Module — Deep-Dive Exploration

**Module:** `plugins/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/plugins/`  
**Files:** 2 TypeScript files  
**Created:** 2026-04-07

---

## 1. Module Overview

The `plugins/` module implements **built-in plugin registry** — managing plugins that ship with the CLI and can be enabled/disabled by users via the `/plugin` UI. Built-in plugins differ from marketplace plugins in that they're compiled into the binary and appear under a "Built-in" section in the plugin manager.

### Core Responsibilities

1. **Plugin Registry** — Built-in plugin management:
   - Registration at startup via `registerBuiltinPlugin()`
   - Plugin ID format: `{name}@builtin`
   - Availability checking via `isAvailable()`

2. **Enable/Disable State** — User-controlled plugin state:
   - User settings take precedence
   - Falls back to `defaultEnabled` (defaults to `true`)
   - Persisted to `enabledPlugins` in settings.json

3. **Skill Extraction** — Plugin-provided skills:
   - Convert `BundledSkillDefinition` to `Command` objects
   - Only skills from enabled plugins returned
   - Source marked as `'bundled'` (not `'builtin'`)

4. **Component Integration** — Multi-component plugins:
   - Skills (bundled skill definitions)
   - Hooks configuration
   - MCP servers

### Key Design Patterns

- **Plugin ID Convention**: `{name}@builtin` suffix distinguishes from marketplace
- **Availability Gating**: `isAvailable()` check omits unavailable plugins
- **Enable State Priority**: user setting > plugin default > true
- **Source Labeling**: Skills use `'bundled'` source for proper analytics

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `builtinPlugins.ts` | ~160 | Built-in plugin registry and management |
| `bundled/*` | varies | Bundled plugin skill definitions |

**Total:** ~160 lines in 1 core file

---

## 3. Key Exports

### Plugin Registry (`builtinPlugins.ts`)

```typescript
// Built-in plugin definition
export type BuiltinPluginDefinition = {
  name: string
  description: string
  version?: string
  defaultEnabled?: boolean
  isAvailable?: () => boolean
  skills?: BundledSkillDefinition[]
  hooks?: HooksSettings
  mcpServers?: MCPServerConfig[]
}

// Loaded plugin with enable state
export type LoadedPlugin = {
  name: string
  manifest: {
    name: string
    description: string
    version?: string
  }
  path: string  // 'builtin' sentinel
  source: string  // '{name}@builtin'
  repository: string  // '{name}@builtin'
  enabled: boolean
  isBuiltin: true
  hooksConfig?: HooksSettings
  mcpServers?: MCPServerConfig[]
}

// Registry functions
export function registerBuiltinPlugin(definition: BuiltinPluginDefinition): void
export function isBuiltinPluginId(pluginId: string): boolean
export function getBuiltinPluginDefinition(name: string): BuiltinPluginDefinition | undefined
export function getBuiltinPlugins(): { enabled: LoadedPlugin[]; disabled: LoadedPlugin[] }
export function getBuiltinPluginSkillCommands(): Command[]
export function clearBuiltinPlugins(): void
```

---

## 4. Line-by-Line Analysis

### 4.1 Plugin Registration (`builtinPlugins.ts` lines 28-32)

```typescript
export function registerBuiltinPlugin(
  definition: BuiltinPluginDefinition,
): void {
  BUILTIN_PLUGINS.set(definition.name, definition)
}
```

**Simple Registry**: Map-based storage keyed by plugin name.

### 4.2 Plugin ID Check (`builtinPlugins.ts` lines 37-39)

```typescript
export function isBuiltinPluginId(pluginId: string): boolean {
  return pluginId.endsWith(`@${BUILTIN_MARKETPLACE_NAME}`)
}
```

**ID Convention**: `@builtin` suffix identifies built-in plugins.

### 4.3 Get Plugin Definition (`builtinPlugins.ts` lines 46-50)

```typescript
export function getBuiltinPluginDefinition(
  name: string,
): BuiltinPluginDefinition | undefined {
  return BUILTIN_PLUGINS.get(name)
}
```

**UI Integration**: Used by `/plugin` UI to show skills/hooks/MCP list without marketplace lookup.

### 4.4 Get All Plugins (`builtinPlugins.ts` lines 57-102)

```typescript
export function getBuiltinPlugins(): {
  enabled: LoadedPlugin[]
  disabled: LoadedPlugin[]
} {
  const settings = getSettings_DEPRECATED()
  const enabled: LoadedPlugin[] = []
  const disabled: LoadedPlugin[] = []

  for (const [name, definition] of BUILTIN_PLUGINS) {
    // Skip unavailable plugins
    if (definition.isAvailable && !definition.isAvailable()) {
      continue
    }

    const pluginId = `${name}@${BUILTIN_MARKETPLACE_NAME}`
    const userSetting = settings?.enabledPlugins?.[pluginId]
    
    // Enabled state: user preference > plugin default > true
    const isEnabled =
      userSetting !== undefined
        ? userSetting === true
        : (definition.defaultEnabled ?? true)

    const plugin: LoadedPlugin = {
      name,
      manifest: {
        name,
        description: definition.description,
        version: definition.version,
      },
      path: BUILTIN_MARKETPLACE_NAME,  // sentinel — no filesystem path
      source: pluginId,
      repository: pluginId,
      enabled: isEnabled,
      isBuiltin: true,
      hooksConfig: definition.hooks,
      mcpServers: definition.mcpServers,
    }

    if (isEnabled) {
      enabled.push(plugin)
    } else {
      disabled.push(plugin)
    }
  }

  return { enabled, disabled }
}
```

**Availability Check**: `isAvailable()` gates plugins that shouldn't appear (e.g., platform-specific).

**Enable State Priority**: 
1. User setting in `enabledPlugins`
2. Plugin's `defaultEnabled` 
3. `true` (default)

### 4.5 Get Skill Commands (`builtinPlugins.ts` lines 108-121)

```typescript
export function getBuiltinPluginSkillCommands(): Command[] {
  const { enabled } = getBuiltinPlugins()
  const commands: Command[] = []

  for (const plugin of enabled) {
    const definition = BUILTIN_PLUGINS.get(plugin.name)
    if (!definition?.skills) continue
    for (const skill of definition.skills) {
      commands.push(skillDefinitionToCommand(skill))
    }
  }

  return commands
}
```

**Enabled Only**: Skills from disabled plugins not returned.

### 4.6 Skill Definition to Command (`builtinPlugins.ts` lines 132-159)

```typescript
function skillDefinitionToCommand(definition: BundledSkillDefinition): Command {
  return {
    type: 'prompt',
    name: definition.name,
    description: definition.description,
    hasUserSpecifiedDescription: true,
    allowedTools: definition.allowedTools ?? [],
    argumentHint: definition.argumentHint,
    whenToUse: definition.whenToUse,
    model: definition.model,
    disableModelInvocation: definition.disableModelInvocation ?? false,
    userInvocable: definition.userInvocable ?? true,
    contentLength: 0,
    // 'bundled' not 'builtin' — 'builtin' in Command.source means hardcoded
    // slash commands (/help, /clear). Using 'bundled' keeps these skills in
    // the Skill tool's listing, analytics name logging, and prompt-truncation
    // exemption. The user-toggleable aspect is tracked on LoadedPlugin.isBuiltin.
    source: 'bundled',
    loadedFrom: 'bundled',
    hooks: definition.hooks,
    context: definition.context,
    agent: definition.agent,
    isEnabled: definition.isEnabled ?? (() => true),
    isHidden: !(definition.userInvocable ?? true),
    progressMessage: 'running',
    getPromptForCommand: definition.getPromptForCommand,
  }
}
```

**Source Labeling**: Uses `'bundled'` (not `'builtin'`) to:
- Keep skills in Skill tool's listing
- Enable analytics name logging
- Maintain prompt-truncation exemption

**'builtin' Reserved**: `Command.source === 'builtin'` means hardcoded slash commands like `/help`, `/clear`.

---

## 5. Integration Points

### 5.1 With `utils/settings/settings.js`

| Component | Integration |
|-----------|-------------|
| `getBuiltinPlugins()` | Uses `getSettings_DEPRECATED()` for user preferences |

### 5.2 With `skills/bundledSkills.ts`

| Component | Integration |
|-----------|-------------|
| `skillDefinitionToCommand()` | Uses `BundledSkillDefinition` type |

### 5.3 With `types/plugin.js`

| Component | Integration |
|-----------|-------------|
| `builtinPlugins.ts` | Uses `BuiltinPluginDefinition`, `LoadedPlugin` types |

---

## 6. Data Flow

### 6.1 Plugin Registration Flow

```
Startup
    │
    ▼
initBuiltinPlugins() called
    │
    ├──► registerBuiltinPlugin({name: 'plugin-name', ...})
    │    └──► BUILTIN_PLUGINS.set(name, definition)
    │
    ▼
Registry populated
```

### 6.2 Plugin Loading Flow

```
/plugin UI opens
    │
    ▼
getBuiltinPlugins()
    │
    ├──► Get user settings (enabledPlugins)
    ├──► Iterate BUILTIN_PLUGINS map
    ├──► Check isAvailable()
    ├──► Determine isEnabled (user > default > true)
    │
    ▼
Return {enabled, disabled} arrays
```

### 6.3 Skill Extraction Flow

```
Skill loading
    │
    ▼
getBuiltinPluginSkillCommands()
    │
    ├──► getBuiltinPlugins() → enabled plugins only
    ├──► For each enabled plugin:
    │    └──► Convert skills to Command objects
    │
    ▼
Return Command[] array
```

---

## 7. Key Patterns

### 7.1 Plugin ID Format

```
{name}@builtin

Examples:
- my-plugin@builtin
- another-plugin@builtin
```

**Why**: Distinguishes from marketplace plugins (`{name}@{marketplace}`).

### 7.2 Availability Gating

```typescript
isAvailable?: () => boolean

// Example: platform-specific plugin
isAvailable: () => process.platform === 'darwin'
```

**Purpose**: Hide plugins that aren't applicable to current environment.

### 7.3 Enable State Resolution

```
User setting (enabledPlugins['plugin@builtin'])
    ↓ (if undefined)
Plugin default (defaultEnabled)
    ↓ (if undefined)
true (default)
```

---

## 8. Summary

The `plugins/` module provides **built-in plugin management**:

1. **Registry Pattern** — Simple Map-based registration
2. **Enable/Disable State** — User-controlled with sensible defaults
3. **Skill Integration** — Converts bundled skill definitions to commands
4. **Availability Gating** — Hide platform-specific or environment-dependent plugins

**Key Design Decisions**:
- **@builtin suffix** clearly identifies built-in plugins
- **'bundled' source** keeps skills in proper analytics/truncation categories
- **Availability check** before loading prevents showing inapplicable plugins
- **User preference priority** respects explicit user choices

---

**Last Updated:** 2026-04-07  
**Status:** Complete — 1 of 2 files analyzed (bundled plugin definitions vary)
