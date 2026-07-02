---
title: "04 Vite Plugins - The 23 Custom Plugins That Power Astro"
---

# Vite Plugins: The 23 Custom Plugins That Power Astro

Astro 7.0.4 ships with **23 custom Vite plugins** that collectively transform .astro files, resolve routes, process content, inject scripts and styles, manage the dev server, and orchestrate builds across three Vite environments. Rather than relying on Vite's defaults, Astro assembles a purpose-built plugin pipeline where each plugin handles one narrow concern virtual modules bridge compile-time data to runtime code and the `enforce: 'pre'` / `enforce: 'post'` ordering guarantees that transforms happen in the right sequence. This document explains every plugin, how they are assembled, and how they interact with Vite's environment model.

See [03-component-model.md](03-component-model.md) for how `.astro` files are compiled and [05-build-pipeline.md](05-build-pipeline.md) for how these plugins are invoked during the three-environment build.

## Plugin Assembly: `createVite` and `getAllBuildPlugins`

Astro's Vite configuration is constructed in two phases. The first phase runs during dev and build setup in `createVite`, which produces a base `vite.InlineConfig` with all shared plugins. The second phase adds build-only plugins via `getAllBuildPlugins` in `static-build.ts`.

### Dev/Build Base Plugins (`createVite`)

The function `createVite` at `packages/astro/src/core/create-vite.ts:161` is the single entry point for assembling the base Vite config. It returns a `vite.InlineConfig` whose `plugins` array contains the flattened list of all Astro plugins. The order matters plugins that must run early (like `vitePluginRenderers` with `enforce: 'pre'`) appear first, while fallback plugins (like `configAliasVitePlugin` with `enforce: 'post'`) appear later. Source:

```typescript
// packages/astro/src/core/create-vite.ts:169
const commonConfig: vite.InlineConfig = {
  // ...
  plugins: [
    { name: 'astro:watcher-max-listeners', configureServer(server) { /* ... */ } },
    serializedManifestPlugin({ settings, command, sync }),
    vitePluginRenderers({ settings, routesList, serverIslandsState, command }),
    vitePluginStaticPaths(),
    await astroPluginRoutes({ routesList, settings, logger, fsMod: fs, command }),
    astroVirtualManifestPlugin({ settings }),
    vitePluginEnvironment({ settings, astroPkgsConfig, command }),
    pluginPage({ routesList }),
    pluginPages({ routesList }),
    configAliasVitePlugin({ settings }),
    astroLoadFallbackPlugin({ fs, root: settings.config.root }),
    astroVitePlugin({ settings, logger }),       // vite-plugin-astro
    astroScriptsPlugin({ settings }),
    vitePluginApp(),
    vitePluginFetchable({ settings }),
    command === 'dev' && vitePluginAstroServer({ settings, logger }),
    command === 'dev' && vitePluginAstroServerClient(),
    astroDevCssPlugin({ routesList, command }),
    // ... 15+ more plugins
    astroHmrReloadPlugin(),
    vitePluginChromedevtools({ settings }),
    command === 'dev' && vitePluginDevStatus(),
  ],
  // ...
};
```

User-provided `vite.config.js` plugins and integration-injected plugins are merged after this base config (lines 320-342), with `apply` filtering to exclude build-only plugins during dev and vice versa.

### Build-Only Plugins (`getAllBuildPlugins`)

During the static build, `getAllBuildPlugins` at `packages/astro/src/core/build/plugins/index.ts:19` returns the additional plugins that are only needed for production builds:

```typescript
// packages/astro/src/core/build/plugins/index.ts:19
export function getAllBuildPlugins(
  internals: BuildInternals,
  options: StaticBuildOptions,
): Array<VitePlugin | VitePlugin[]> {
  return [
    pluginComponentEntry(internals),
    pluginAnalyzer(internals),
    pluginInternals(options, internals),
    pluginMiddleware(options, internals),
    vitePluginActionsBuild(options, internals),
    pluginCssTargetLowering(),
    ...pluginCSS(options, internals),
    astroHeadBuildPlugin(internals),
    pluginPrerender(options, internals),
    pluginScripts(internals),
    ...pluginSSR(options, internals),
    pluginNoop(),
    vitePluginSSRAssets(internals),
    pluginChunkImports(options),
  ].filter(Boolean);
}
```

These plugins are assembled into the final Vite build config in `static-build.ts` via `createViteBuildConfig`. The build plugins handle concerns like SSR chunk generation, prerendering, manifest injection, and CSS extraction tasks that are irrelevant during dev.

## Plugin Ordering: `enforce` and Hook Execution Order

Vite plugins support an `enforce` property that controls when they run relative to other plugins. Astro uses all three positions:

| `enforce` value | When it runs | Astro plugins using it |
|---|---|---|
| `'pre'` | Before user plugins, before default Vite plugins | `astro:build` (vite-plugin-astro), `astro:markdown`, `astro:routes`, `astro:tsconfig-alias-css`, `astro:static-paths`, `astro:head-metadata`, `astro:load-fallback-hmr`, `astro:build:css-hmr` |
| (default/none) | After `'pre'` plugins, at normal priority | Most plugins (scripts, pages, renderers, environment, etc.) |
| `'post'` | After user plugins, after default Vite plugins | `@astro/plugin-pages`, `@astro/plugin-page`, `astro:tsconfig-alias`, `astro:scripts:page-ssr`, `astro:hmr-reload`, `astro:load-fallback` |

The `astro:build` plugin (the core `.astro` transformer) uses `enforce: 'pre'` so that `.astro` files are transformed before any other plugin sees them. The `@astro/plugin-pages` uses `enforce: 'post'` so it can resolve virtual page modules after the underlying `.astro` files have been transformed.

**Key insight:** The `astro:hmr-reload` plugin uses both `enforce: 'post'` AND `hotUpdate: { order: 'post' }` to guarantee it runs as the very last plugin in the hot update pipeline. This ensures that if no other plugin handles a hot update, Astro triggers a full browser reload as a fallback behavior.

## The Three Vite Environments

Astro 7.0.4 uses Vite's environment API with four named environments defined in `ASTRO_VITE_ENVIRONMENT_NAMES` at `packages/astro/src/core/constants.ts:102`:

```typescript
// packages/astro/src/core/constants.ts:102
export const ASTRO_VITE_ENVIRONMENT_NAMES = {
  ssr: 'ssr',         // On-demand server-rendered pages
  client: 'client',   // Browser JS (hydration scripts, islands)
  astro: 'astro',     // Runnable dev environment for module loading
  prerender: 'prerender',  // Static page generation during build
} as const;
```

Each plugin can restrict which environments it operates in using `applyToEnvironment`. This is critical because many plugins should only run in SSR/prerender (e.g., route discovery, page scripts) while others should only run in client (e.g., CSS HMR, dev toolbar). The helper `isAstroServerEnvironment` at `packages/astro/src/environments.ts:4` checks if an environment is one of `ssr`, `prerender`, or `astro`.

### Plugin-Environment Interaction

The following diagram shows how the 23 plugins distribute across environments:

```mermaid
flowchart TD
    subgraph Dev Server
        direction TB
        A[vite-plugin-astro-server<br/>ssr only] --> B[vite-plugin-hmr-reload<br/>ssr/prerender/astro]
        C[vite-plugin-head<br/>dev metadata propagation] --> D[vite-plugin-overlay<br/>client only]
        E[vite-plugin-chromedevtools<br/>all envs, middleware] --> F[vite-plugin-dev-status<br/>all envs, middleware]
    end

    subgraph All Environments
        direction TB
        G[vite-plugin-astro<br/>core transformer] --> H[vite-plugin-scripts<br/>all envs]
        I[vite-plugin-environment<br/>configEnvironment hook] --> J[vite-plugin-config-alias<br/>all envs]
    end

    subgraph SSR + Prerender
        direction TB
        K[vite-plugin-pages<br/>virtual page modules] --> L[vite-plugin-page<br/>per-component modules]
        M[vite-plugin-routes<br/>virtual:astro:routes] --> N[vite-plugin-renderers<br/>framework SSR]
        O[vite-plugin-markdown<br/>.md transformation] --> P[vite-plugin-html<br/>.html transform]
    end

    subgraph Client Only
        direction TB
        Q[vite-plugin-astro<br/>astro:build:css-hmr] --> R[vite-plugin-css<br/>dev CSS collection]
    end

    subgraph Build Only
        direction TB
        S[pluginSSR] --> T[pluginPrerender]
        U[pluginCSS] --> V[pluginScripts]
        W[astroHeadBuildPlugin] --> X[pluginMiddleware]
    end

    Dev Server --> All Environments
    All Environments --> SSR + Prerender
    SSR + Prerender --> Build Only
```

## The 40+ Vite Plugins, In Detail

### 1. vite-plugin-astro: Core .astro Transformation

**Source:** `packages/astro/src/vite-plugin-astro/index.ts`

This is the heart of Astro. The factory function `astro()` returns **three plugins** that work together:

**`astro:build:css-hmr`** (`enforce: 'pre'`) runs only in the `client` environment. It intercepts requests for Astro CSS virtual modules (`?astro&type=style`) and registers dependency watch files so that CSS changes from `.astro` components properly trigger HMR.

**`astro:build`** (`enforce: 'pre'`) is the main transformer. It implements:
- `configEnvironment`: Sets up `resolve.conditions` per environment, adding the `'astro'` condition for proper package exports resolution.
- `configResolved`: Initializes the `compile` function closure over the resolved Vite config and dev toolbar preferences.
- `buildStart`: Creates a fresh `astroFileToCompileMetadata` map, shared via a WeakMap keyed by config across multiple builds. This map stores compiled script/style data that the virtual module loaders retrieve.
- `load`: Handles virtual module requests for `?astro&type=script` and `?astro&type=style`. For scripts, it returns either inline code or an `import` for external scripts. For styles, it returns the compiled CSS code with `vite.cssScopeTo` metadata for tree-shaking. In SSR environments, script virtual modules return only a comment stub.
- `transform`: Transforms `.astro` files using the Rust-based compiler. In the `client` environment, it returns an error-throwing stub (Astro components cannot render in the browser). In SSR/prerender environments, it returns the compiled JS code with metadata about hydrated components, scripts, and head propagation.
- `handleHotUpdate`: Delegates to the HMR handler for smart invalidation.

**`astro:build:normal`** provides a `resolveId` fallback for unresolved Astro virtual module requests, ensuring that things like `Foo.astro?astro&type=style&index=0&lang.css` resolve correctly even when Vite's default resolver fails.

**Virtual module patterns used:**
- `?astro&type=script&index=N&lang.ts` for scripts extracted from `.astro` frontmatter
- `?astro&type=style&index=N&lang.css` for CSS from `<style>` blocks in `.astro` components

### 2. vite-plugin-astro-server: Dev Server Middleware

**Source:** `packages/astro/src/vite-plugin-astro-server/plugin.ts`

The `astro:server` plugin is **dev-only** (`applyToEnvironment` restricts to `ssr`). It implements `configureServer` to:

1. Create handlers for both the `ssr` and `prerender` environments using `createViteLoader` and `createAstroServerApp`.
2. Register an `unhandledRejection` listener that routes errors to the appropriate handler and broadcasts them via WebSocket.
3. Push middleware onto the Vite dev server stack (in order of execution):
   - `secFetchMiddleware`: Validates request `Host` header against `server.allowedHosts` config for security
   - `routeGuardMiddleware`: Prevents serving files outside `srcDir`/`publicDir`
   - `trailingSlashMiddleware`: Enforces trailing slash configuration
   - `baseMiddleware`: Handles the `base` path prefix
4. If prerender handling is enabled in core, adds a prerender request handler that checks routes and serves prerendered pages.
5. Adds the main `astroDevHandler` middleware that handles all SSR requests via the SSR environment's runner.

This plugin is the bridge between Vite's dev server and Astro's request handling pipeline. Without it, the dev server would serve raw files instead of rendered pages.

### 3. vite-plugin-pages: Virtual Page Manifest

**Source:** `packages/astro/src/vite-plugin-pages/pages.ts`

The `@astro/plugin-pages` plugin (`enforce: 'post'`) creates a virtual module `virtual:astro:pages` that exports a `pageMap` Map from component paths to lazy import functions. It filters routes based on the current environment:

- **SSR environment**: Only includes routes where `prerender === false` (on-demand pages)
- **Prerender environment**: Only includes routes where `prerender === true` (static pages)
- **Other environments**: Includes all routes

```typescript
// packages/astro/src/vite-plugin-pages/pages.ts:36
export function pluginPages({ routesList }: PagesPluginOptions): VitePlugin {
  return {
    name: '@astro/plugin-pages',
    enforce: 'post',
    applyToEnvironment(environment) {
      return environment.name === 'ssr' || environment.name === 'prerender';
    },
    // resolveId + load for virtual:astro:pages
  };
}
```

This environment-based filtering is a key optimization: each build only includes the pages it actually needs, reducing memory and bundle size.

### 4. vite-plugin-routes: Route Discovery and Virtual Module

**Source:** `packages/astro/src/vite-plugin-routes/index.ts`

The `astro:routes` plugin creates the `virtual:astro:routes` virtual module, which exports a `routes` array of deserialized route info. It:

- **Dev mode**: Watches for file adds/changes/deletes in `srcDir` and recalculates routes. On changes, it invalidates the virtual module in both `ssr` and `prerender` environments and sends `astro:routes-updated` via HMR.
- **Build mode**: Filters routes per environment prerender gets prerenderable routes, SSR gets on-demand routes.
- **Transform hook**: Attaches `pageOptions.prerender` metadata to page/endpoint modules, enabling downstream plugins to know whether a page should be prerendered.
- **HMR hook**: Updates the prerender option on pages when their source changes, so `export const prerender = true` toggles are reflected immediately.

### 5. vite-plugin-markdown: Markdown Processing

**Source:** `packages/astro/src/vite-plugin-markdown/index.ts`

The `astro:markdown` plugin (`enforce: 'pre'`) transforms `.md` and `.markdown` files into Astro-compatible modules using the `load` hook (not `transform`, because Vite transforms `import.meta.env` references before the transform hook, and markdown processing needs the raw file content).

It:
- Lazily initializes a `MarkdownRenderer` from the configured markdown processor
- Parses frontmatter and renders markdown to HTML
- Handles local and remote image paths for the `astro:assets` pipeline
- Exports `frontmatter`, `file`, `url`, `rawContent()`, `compiledContent()`, and `getHeadings()` for content collection integration
- Wraps the content in a `createComponent` that optionally uses a `layout` from frontmatter

### 6. vite-plugin-css: CSS Bundling and Dev Collection

**Source:** `packages/astro/src/vite-plugin-css/index.ts`

The CSS plugin provides **two dev-only plugins** for CSS collection:

**`virtual:astro:dev-css`** (and `virtual:astro:dev-css:*` per-route): Walks the Vite module graph depth-first from a page component to collect all imported CSS modules, preserving import order. It uses `ensureModulesLoaded` to eagerly fetch dynamically imported components whose modules may be registered in the graph but not yet transformed, ensuring their CSS is visible during the walk. The plugin stops traversal at `?astroPropagatedAssets` modules to prevent CSS bleeding between content collection entries.

**`virtual:astro:dev-css-all`**: Creates a map from component names to functions that import their per-route CSS modules. Used by Cloudflare's `optimizeDeps` config.

**CSS tree-shaking via `vite.cssScopeTo`**: When `vite-plugin-astro` loads a style virtual module (via `?astro&type=style`), it attaches `meta.vite.cssScopeTo: [filename, 'default']` to the returned module. This tells Vite that the CSS is scoped to the default export of that `.astro` file if the component's default export is never imported, the CSS is tree-shaken away. Global styles (with `isGlobal: true`) skip this metadata and are always included.

**Virtual module IDs** at `packages/astro/src/vite-plugin-css/const.ts`:
- `virtual:astro:dev-css` / `\0virtual:astro:dev-css`
- `virtual:astro:dev-css:*` / `\0virtual:astro:dev-css:*` (per-route)
- `virtual:astro:dev-css-all` / `\0virtual:astro:dev-css-all`

### 7. vite-plugin-html: HTML Generation

**Source:** `packages/astro/src/vite-plugin-html/index.ts`

A minimal plugin that:
- Removes Vite's built-in `vite:build-html` plugin in the `options` hook to prevent conflicts
- Transforms `.html` files via the `transform` handler

### 8. vite-plugin-scripts: Page Script Injection

**Source:** `packages/astro/src/vite-plugin-scripts/index.ts`

The `astro:scripts` plugin manages three virtual script modules injected by integrations via `injectScript()`:

| Virtual Module | Stage | Purpose |
|---|---|---|
| `astro:scripts/before-hydration.js` | `before-hydration` | Runs before island hydration |
| `astro:scripts/page.js` | `page` | Runs on every page in the browser |
| `astro:scripts/page-ssr.js` | `page-ssr` | Runs on every page during SSR |

The `resolveId` hook passes through these IDs unchanged. The `load` hook concatenates all scripts for the matching stage. The `buildStart` hook emits `before-hydration.js` as a Rollup chunk when applicable, ensuring it appears as a separate output file.

Note the comment in the source: these plugins cannot use the `\0` virtual module convention because they need to be ESM-importable from actual code where `\0` prefixes would not resolve correctly.

### 9. vite-plugin-renderers: Framework Renderer Integration

**Source:** `packages/astro/src/vite-plugin-renderers/index.ts`

The `astro:plugin-renderers` plugin (`enforce: 'pre'`) creates the `virtual:astro:renderers` module that imports each configured renderer's server entrypoint (React, Vue, Svelte, Solid, etc.) and exports a `renderers` array with the SSR implementations merged in.

During a fully static build (SSR environment with no islands and no non-prerendered routes), it returns an empty array, skipping renderer imports entirely. This is a build-time optimization: if nothing needs hydration, framework renderers are never loaded.

### 10. vite-plugin-static-paths: Static Asset Handling

**Source:** `packages/astro/src/vite-plugin-static-paths/index.ts`

The `astro:static-paths` plugin (`enforce: 'pre'`) provides a virtual module `astro:static-paths` that:
- In the `prerender` environment: Re-exports the real `StaticPaths` class from `astro/runtime/prerender/static-paths.js`
- In all other environments: Returns a no-op class with an empty `getAll()` method

This allows adapters to use `StaticPaths` from their runtime (e.g., Cloudflare's workerd) to collect all prerenderable paths without polluting non-prerender builds.

### 11. vite-plugin-config-alias: tsconfig Path Aliasing

**Source:** `packages/astro/src/vite-plugin-config-alias/index.ts`

Provides **two plugins** as a fallback for tsconfig `paths` and `baseUrl` that Vite's native `resolve.tsconfigPaths` does not handle in Astro's pipeline:

**`astro:tsconfig-alias-css`** (`enforce: 'pre'`): Transforms CSS files to resolve aliases in `@import` and `url()` statements. Only supports `@import "..."`, `@import url("...")`, and quoted `url("...")` explicitly documented as deprecated and limited to avoid scope creep.

**`astro:tsconfig-alias`** (`enforce: 'post'`): Uses `resolveId` to resolve JS/TS/Astro import specifiers against tsconfig path aliases. The `enforce: 'post'` ensures it only kicks in after all other resolution attempts have failed.

The plugin is marked as **deprecated** and will be removed once Vite handles all remaining alias paths natively.

### 12. vite-plugin-environment: Multi-Environment Setup

**Source:** `packages/astro/src/vite-plugin-environment/index.ts`

The `astro:environment` plugin implements `configEnvironment` to configure each Vite environment with:
- `resolve.dedupe: ['astro']`: Ensures all packages use the same Astro version
- `resolve.noExternal`: Always includes `astro`, `astro/components`, `@nanostores/preact`, and `@fontsource/*`
- `resolve.external`: During dev, adds `prismjs/components/index.js` and `string-width` (CJS deps that Vite can't transform in dev)
- `optimizeDeps.entries`: Configures dependency pre-bundling entries per environment (client includes dev toolbar entrypoints)

### 13. vite-plugin-integrations-container: Integration Bridge

**Source:** `packages/astro/src/vite-plugin-integrations-container/index.ts`

The `astro:integration-container` plugin:
- Runs `runHookServerSetup` during dev to let integrations configure the Vite dev server
- Resolves integration-injected route entrypoints during `buildStart`, using the SSR environment's plugin container in dev (to avoid racing against the client dep optimizer) and `this.resolve()` during build

### 14. vite-plugin-fileurl: File URL Handling

**Source:** `packages/astro/src/vite-plugin-fileurl/index.ts`

A tiny plugin (`enforce: 'pre'`) that intercepts `file://` import IDs, strips the prefix, and delegates resolution to Vite. This allows modules imported via `file:///absolute/path` to resolve correctly within Vite's module graph.

### 15. vite-plugin-head: Head Content Collection and Injection

**Source:** `packages/astro/src/vite-plugin-head/index.ts`

Provides **two plugins**:

**`astro:head-metadata`** (dev only, `enforce: 'pre'`): Tracks `<head>` content and propagation metadata across the module graph. It:
- Creates a virtual module `virtual:astro:component-metadata` that serializes `containsHead` and `propagation` metadata for all modules
- Runs propagation on `resolveId` and `transform`: when a module with `propagation: 'self'` or `propagation: 'in-tree'` is imported, it walks up the importer graph and marks all ancestors
- Invalidates the virtual module on file changes

**`astro:head-metadata-build`** (build only): Runs in `generateBundle` to compute head propagation from the final bundle. It identifies seeds from `?astroPropagatedAssets` query params (content render entries) and `propagation: 'self'` modules, builds an importer graph, and marks all ancestors with `propagation: 'in-tree'`.

### 16. vite-plugin-hmr-reload: Enhanced HMR

**Source:** `packages/astro/src/vite-plugin-hmr-reload/index.ts`

The `astro:hmr-reload` plugin (`enforce: 'post'`, `hotUpdate: { order: 'post' }`) is the **last plugin** in the hot update chain. It:
- Detects SSR-only module updates (modules that exist in SSR but not in the client module graph)
- Triggers a full browser reload when SSR-only modules change (mimicking Vite 5 behavior)
- Handles style modules specially: when CSS changes, it invalidates all per-route dev CSS virtual modules so the next SSR request serves updated inline styles
- Returns `[]` to prevent Vite's default HMR propagation when modules are handled, avoiding unnecessary reloads

### 17. vite-plugin-overlay: Error Overlay

**Source:** `packages/astro/src/vite-plugin-overlay/index.ts`

The `astro:server-client` plugin (client environment only) transforms Vite's client script (`vite/dist/client/client.mjs`) to replace Vite's built-in error overlay with Astro's custom overlay via `patchOverlay(code)`. This ensures that errors displayed in the browser match Astro's error formatting and styling.

### 18. vite-plugin-load-fallback: Unresolved Module Fallback

**Source:** `packages/astro/src/vite-plugin-load-fallback/index.ts`

Only active when a **custom filesystem** is provided (not the default `node:fs`). Provides two plugins:

**`astro:load-fallback`** (`enforce: 'post'`): Tries to resolve imports from the custom filesystem. If a parent module imports a relative path, it checks if the file exists on the custom FS and returns it with an `?astroFallbackFlag` query parameter.

**`astro:load-fallback-hmr`** (`enforce: 'pre'`): Wraps `context.read` in the HMR handler to check the custom filesystem first before falling back to Vite's default read.

### 19. vite-plugin-utils: Shared Utilities

**Source:** `packages/astro/src/vite-plugin-utils/index.ts`

Not a Vite plugin itself this is a **utility module** exported as a pseudo-plugin directory. It provides:
- `getFileInfo(id, config)`: Extracts file ID and URL from a module ID, handling page paths and trailing slash configuration
- `normalizeFilename(filename, root)`: Normalizes various module ID formats (`/@fs/...`, `./...`, `/...`) to absolute paths with forward slashes, with case-insensitive root checking for Windows/macOS
- `cleanUrl(url)`: Strips query strings and hash fragments
- `specialQueriesRE` / `hasSpecialQueries(id)`: Detects `?url`, `?raw`, and `?direct` queries that should skip transformation

### 20. vite-plugin-adapter-config: Adapter-Specific Configuration

**Source:** `packages/astro/src/vite-plugin-adapter-config/index.ts`

The `astro:adapter-config` plugin:
- `config` hook: When the adapter has `entrypointResolution: 'auto'` and a `serverEntrypoint`, it sets the SSR environment's build input to the adapter's entrypoint
- Provides `virtual:astro:adapter-config/client`: Returns `internalFetchHeaders` from the adapter config in client environments, or an empty object in SSR environments

### 21. vite-plugin-chromedevtools: Chrome DevTools Workspace

**Source:** `packages/astro/src/vite-plugin-chromedevtools/index.ts`

The `astro:chromedevtools` plugin registers a dev server middleware that responds to `/.well-known/appspecific/com.chrome.devtools.json` with a Chrome DevTools workspace configuration. When `experimental.chromeDevtoolsWorkspace` is enabled, it generates a UUID and writes `chrome-workspace.json` to the cache directory, enabling Chrome DevTools to map the running dev server to local source files for direct editing.

### 22. vite-plugin-dev-status: Dev Server Health Check

**Source:** `packages/astro/src/vite-plugin-dev-status/index.ts`

The `astro:dev-status` plugin adds a `/_astro/status` middleware endpoint that returns `{ "ok": true }`. This is a simple health check that external tools (load balancers, test harnesses, process managers) can use to verify the dev server is running and responsive.

### 23. vite-plugin-app: App Entrypoint Handling

**Source:** `packages/astro/src/vite-plugin-app/index.ts`

Provides **two plugins**:

**`astro:app`**: Resolves `virtual:astro:app` to export `createApp` from either `astro/app/entrypoint/dev` (during dev) or `astro/app/entrypoint/prod` (during build). This virtual module is the main entry point for creating an Astro `App` instance.

**`astro:server-app`**: Resolves `astro:server-app` to the local `createAstroServerApp.js` module, used by the dev server to create a server-side app instance with the controller and loader.

## Virtual Module Patterns

Astro uses a consistent pattern for virtual modules: an unresolved ID (sometimes prefixed with `virtual:`) and a resolved ID (prefixed with `\0` to mark it as virtual to Rollup). The `resolveId` hook maps from unresolved to resolved, and the `load` hook provides the code.

| Virtual Module ID | Resolved ID | Plugin | Purpose |
|---|---|---|---|
| `virtual:astro:pages` | `\0virtual:astro:pages` | vite-plugin-pages | Page map export |
| `virtual:astro:page:*` | `\0virtual:astro:page:*` | vite-plugin-page | Per-component module |
| `virtual:astro:routes` | `\0virtual:astro:routes` | vite-plugin-routes | Route list export |
| `virtual:astro:renderers` | `\0virtual:astro:renderers` | vite-plugin-renderers | Framework renderers |
| `astro:static-paths` | `\0astro:static-paths` | vite-plugin-static-paths | StaticPaths class |
| `astro:scripts/before-hydration.js` | (passthrough) | vite-plugin-scripts | Before-hydration scripts |
| `astro:scripts/page.js` | (passthrough) | vite-plugin-scripts | Page scripts |
| `astro:scripts/page-ssr.js` | (passthrough) | vite-plugin-scripts | SSR page scripts |
| `virtual:astro:app` | `\0virtual:astro:app` | vite-plugin-app | App entrypoint |
| `astro:server-app` | (resolved file) | vite-plugin-app | Server app factory |
| `virtual:astro:component-metadata` | `\0virtual:astro:component-metadata` | vite-plugin-head | Head metadata |
| `virtual:astro:dev-css` | `\0virtual:astro:dev-css` | vite-plugin-css | Dev CSS collector |
| `virtual:astro:dev-css:*` | `\0virtual:astro:dev-css:*` | vite-plugin-css | Per-route dev CSS |
| `virtual:astro:dev-css-all` | `\0virtual:astro:dev-css-all` | vite-plugin-css | All dev CSS map |
| `virtual:astro:adapter-config/client` | `\0virtual:astro:adapter-config/client` | vite-plugin-adapter-config | Adapter client config |

Plus the query-based virtual modules from `vite-plugin-astro`:
- `*.astro?astro&type=script&index=N&lang.ts` for extracted scripts
- `*.astro?astro&type=style&index=N&lang.css` for extracted styles

## CSS Scoping and Tree-Shaking

The CSS tree-shaking mechanism in Astro is elegant. Here is the flow:

1. **Compilation**: When `vite-plugin-astro` transforms a `.astro` file, the Rust compiler extracts `<style>` blocks and stores them in `compileMetadata.css[]`.

2. **Virtual module load**: When Vite requests `Foo.astro?astro&type=style&index=0&lang.css`, the `load` hook in `astro:build` returns the CSS code with `meta.vite.cssScopeTo: [filename, 'default']`.

3. **Tree-shaking**: Vite sees the `cssScopeTo` metadata and knows this CSS is only needed if the `default` export of `filename` is used. If no module imports `Foo.astro`'s default export, the CSS is removed from the bundle.

4. **Global styles**: Styles marked as `isGlobal: true` (e.g., styles with the `is:global` directive) skip the `cssScopeTo` metadata and are always included.

```typescript
// packages/astro/src/vite-plugin-astro/index.ts:173-184
return {
  code: result.code,
  meta: result.isGlobal
    ? undefined
    : {
        vite: {
          cssScopeTo: [filename, 'default'],
        },
      },
};
```

## Aha Moment: The Cross-Build Metadata WeakMap

**Aha:** The `astroFileToCompileMetadataWeakMap` in `vite-plugin-astro/index.ts:27` is a clever cross-build communication channel. Astro performs multiple Vite builds in sequence (prerender, SSR, client), and the hoisted scripts analyzer from one build needs compile metadata from earlier builds. Instead of passing state through global variables or files, Astro uses a `WeakMap<AstroConfig, Map<string, CompileMetadata>>` keyed by the config object. Since the config object is the same across all builds in a single run, the WeakMap provides automatic lifetime management: when the config is garbage collected, the metadata maps are freed too. This avoids memory leaks while enabling the cross-build communication that hoisted scripts analysis requires.

## Plugin Pipeline Overview

The complete plugin pipeline for a `.astro` page request during dev:

```mermaid
sequenceDiagram
    participant Browser
    participant ViteServer
    participant astro:server as vite-plugin-astro-server
    participant astro:build as vite-plugin-astro (astro:build)
    participant astro:routes as vite-plugin-routes
    participant Rust as Rust Compiler
    participant SSR as SSR Runner

    Browser->>ViteServer: GET /blog/hello
    ViteServer->>astro:server: middleware handles request
    astro:server->>astro:routes: import virtual:astro:routes
    astro:routes-->>astro:server: route list
    astro:server->>astro:build: import /src/pages/blog/[slug].astro
    astro:build->>Rust: compile(source, filename)
    Rust-->>astro:build: { code, css[], scripts[], metadata }
    astro:build-->>astro:server: transformed JS module
    Note over astro:build: css[] stored in compileMetadata
    astro:server->>SSR: execute rendered module
    SSR-->>astro:server: HTML response
    astro:server-->>ViteServer: response + headers
    ViteServer-->>Browser: rendered HTML
```

## Cross-References

- [03-component-model.md](03-component-model.md): How `.astro` files are compiled by the Rust compiler and how scripts/styles are extracted
- [05-build-pipeline.md](05-build-pipeline.md): How `getAllBuildPlugins` and `createViteBuildConfig` orchestrate the three-environment build
- [06-ssr-runtime.md](06-ssr-runtime.md): How the `App` class (from `vite-plugin-app`) handles requests at runtime
- [07-integration-api.md](07-integration-api.md): How integrations inject scripts (`astro:scripts/*`) and routes that flow through these plugins
