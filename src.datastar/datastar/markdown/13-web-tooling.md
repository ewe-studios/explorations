# Datastar -- Web Tooling

Datastar ships IDE support for VS Code and IntelliJ-based editors, plus an SDK schema for language server authors.

## VS Code Extension

Location: `tools/vscode-extension/`

The VS Code extension provides:
- Syntax highlighting for Datastar attributes in 22+ template languages
- Autocomplete snippets for all `data-*` attributes
- Custom attribute support via `datastar.customAttributes` setting

### Supported Languages

The extension injects a TextMate grammar into these language scopes:

| Language ID | File Extensions | Examples |
|------------|----------------|----------|
| html | .html | Plain HTML |
| php | .php | PHP templates |
| twig | .twig | Symfony, Craft CMS |
| blade | .blade.php | Laravel |
| vue | .vue | Vue SFC |
| svelte | .svelte | Svelte |
| astro | .astro | Astro |
| templ | .templ | Go Templ |
| erb | .erb | Ruby on Rails |
| ejs | .ejs | Embedded JS |
| handlebars | .hbs | Handlebars |
| mustache | .mustache | Mustache |
| liquid | .liquid | Shopify, Jekyll |
| pug | .pug | Pug templates |
| razor | .cshtml | ASP.NET |
| django-html | .html | Django |
| jinja | .j2 | Jinja2 |
| gohtml | .gohtml | Go templates |
| jsp | .jsp | Java Server Pages |
| aspnetcorerazor | .razor | ASP.NET Core |
| nunjucks | .njk | Nunjucks |
| edge | .edge | AdonisJS Edge |

### Snippets

All snippets are defined in `src/data-attributes.json` and provide tab-completion for:

```
data-bind → data-bind:value
data-on:click → data-on:click="$doSomething()"
data-show → data-show="$condition"
data-text → data-text="$value"
data-class → data-class:active="$isActive"
data-effect → data-effect="$console.log($state)"
data-signals → data-signals="{ key: value }"
data-init → data-init="$initialize()"
// ... all attribute plugins
```

### Custom Attributes

Users can register custom attribute names in VS Code settings:

```json
{
  "datastar.customAttributes": ["my-plugin", "custom-action"]
}
```

These appear in autocomplete alongside the built-in attributes.

## IntelliJ Plugin

Location: `tools/intellij-plugin/`

A Gradle-based IntelliJ plugin built on the JetBrains Plugin SDK. Provides the same syntax highlighting and autocomplete as the VS Code extension for IntelliJ-based IDEs (WebStorm, PhpStorm, IDEA, etc.).

Build system:

```
tools/intellij-plugin/
├── build.gradle.kts          # Gradle build configuration
├── settings.gradle.kts       # Gradle settings
├── gradle.properties         # Plugin metadata
├── src/                      # Plugin source
├── test.html                 # Test file for highlighting
├── test.css                  # Test CSS injection
└── schema.json               # Attribute schema
```

## SDK Schema

Location: `sdk/datastar-sdk-config-v1.schema.json`

A JSON Schema that describes the Datastar attribute surface for third-party tool authors:

```json
{
  "version": "1.0.1",
  "datastarKey": "datastar",
  "defaults": {
    "booleans": {
      "elementsUseViewTransitions": false,
      "patchSignalsOnlyIfMissing": false
    },
    "durations": {
      "sseRetryDuration": 1000
    }
  },
  "enums": {
    "ElementPatchMode": {
      "values": ["remove", "outer", "inner", "replace", "prepend", "append", "before", "after"]
    },
    "EventType": {
      "values": [
        { "name": "STARTED", "value": "datastar-fetch-started" },
        { "name": "FINISHED", "value": "datastar-fetch-finished" },
        { "name": "ERROR", "value": "datastar-fetch-error" },
        { "name": "RETRYING", "value": "datastar-fetch-retrying" },
        { "name": "RETRIES_FAILED", "value": "datastar-fetch-retries-failed" }
      ]
    },
    "Namespace": {
      "values": ["html", "svg", "mathml"]
    }
  },
  "datalineLiterals": ["selector", "mode", "elements", "useViewTransition", "namespace", "signals", "onlyIfMissing"]
}
```

This schema enables:
- LSP servers to validate `data-*` attribute values
- Code generators to produce type-safe bindings
- Documentation generators to enumerate all valid attributes

## Build Pipeline

The library is built with TypeScript + esbuild:

```
library/src/**/*.ts  →  bundles/datastar.js
                       bundles/datastar-core.js
                       bundles/datastar-aliased.js
```

TypeScript configuration (`tsconfig.json`):

| Setting | Value | Purpose |
|---------|-------|---------|
| target | ES2021 | Modern JS features |
| module | ESNext | Native ES modules |
| lib | ES2021, DOM, DOM.Iterable | Browser APIs |
| moduleResolution | bundler | Path alias resolution |
| strict | true | Full type checking |
| noUnusedLocals | true | No dead code |
| noUnusedParameters | true | No unused params |

The `ALIAS` global is injected at build time via esbuild's `define` option:

```typescript
// globals.d.ts
declare const ALIAS: string | null
```

```javascript
// esbuild config
define: { ALIAS: '"myapp"' }  // → data-myapp-* attributes
```

## Library Structure

```
library/
├── tsconfig.json              # TypeScript configuration
├── src/                       # Source code
│   ├── engine/                # Core engine
│   ├── plugins/               # All plugins
│   ├── utils/                 # Shared utilities
│   └── bundles/               # Entry points
├── dist/                      # Compiled output
│   ├── datastar.js            # Full bundle (~12 KiB)
│   ├── datastar-core.js       # Engine only
│   └── datastar-aliased.js    # Custom prefix
└── package.json               # Dependencies (esbuild, typescript)
```

See [Architecture](01-architecture.md) for the full module graph.
See [Overview](00-overview.md) for a high-level introduction.

## Build Pipeline

```mermaid
flowchart LR
    subgraph "Source"
        TS["library/src/**/*.ts"]
        PKG["package.json"]
        TSCFG["tsconfig.json"]
    end

    subgraph "esbuild"
        BUILD["esbuild.build()"]
        DEFINE["define: { ALIAS: '...' }"]
    end

    subgraph "Output"
        FULL["datastar.js<br/>11.80 KiB — full bundle"]
        CORE["datastar-core.js<br/>engine only"]
        ALIAS["datastar-aliased.js<br/>custom prefix"]
    end

    subgraph "Extensions"
        VSCODE["VS Code Extension<br/>TextMate grammar + snippets"]
        INTELLIJ["IntelliJ Plugin<br/>Gradle + schema.json"]
        SDK["SDK Schema<br/>datastar-sdk-config-v1.schema.json"]
    end

    TS --> BUILD
    PKG --> BUILD
    TSCFG --> BUILD
    BUILD --> DEFINE
    BUILD --> FULL
    BUILD --> CORE
    DEFINE --> ALIAS

    TS -.grammar-> VSCODE
    SDK -.schema-> INTELLIJ
    SDK -.schema-> VSCODE
```

## IDE Extension Architecture

```mermaid
flowchart TB
    subgraph "VS Code Extension"
        TM[TextMate Grammar<br/>datastar.tmLanguage.json]
        SN[Snippets<br/>data-attributes.json]
        CFG[Contributes<br/>package.json]
        SET[Settings<br/>datastar.customAttributes]
    end

    subgraph "IntelliJ Plugin"
        GRADLE[build.gradle.kts]
        SCHEMA[schema.json]
        PLUGIN[src/main/kotlin/]
    end

    subgraph "SDK Schema"
        V[version: "1.0.1"]
        D[defaults]
        E[enums: PatchMode, EventType, Namespace]
        DL[datalineLiterals]
    end

    TM -->|injected into| HTML[22+ language scopes]
    SN -->|tab-completion| ATTR[data-bind, data-on:click, ...]
    CFG -->|contributes| VSCODE[VS Code IDE]
    SET -->|user config| CUSTOM[custom attribute names]

    GRADLE -->|builds| IDE[IntelliJ/WebStorm/PhpStorm]
    SCHEMA -->|validates| PLUGIN
    V --> SDK[Third-party LSP]
    E --> SDK
    DL --> SDK
```
