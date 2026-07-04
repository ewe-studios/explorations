# Build & Configuration

## electron-vite Config

Source: `electron.vite.config.ts` — 11 lines.

```ts
export default defineConfig({
  main: { plugins: [externalizeDepsPlugin()] },
  preload: { plugins: [externalizeDepsPlugin()] },
  renderer: {},
})
```

The `externalizeDepsPlugin` tells Vite to treat `package.json` dependencies as external (not bundled) for the main and preload processes. This is standard for Electron — Node.js modules are available natively.

## electron-builder Config

Source: `package.json` — `build` field.

```json
{
  "build": {
    "appId": "com.universal-timer.app",
    "productName": "Universal Timer",
    "mac": {
      "icon": "resources/icon.icns",
      "category": "public.app-category.productivity",
      "target": "dmg"
    },
    "files": ["out/**/*"],
    "extraResources": [{ "from": "resources", "to": "resources" }]
  }
}
```

Build output:
- **`npm run dist`** → builds to `out/` then creates a `.dmg` installer
- **App ID**: `com.universal-timer.app`
- **Icon**: `resources/icon.icns`

## Resources

| File | Purpose |
|------|---------|
| `resources/icon.icns` | App icon for macOS |
| `resources/tray-icon.png` | Default tray icon |
| `resources/tray-icons/*.png` | State-specific tray icons (idle, running, paused) |
| `resources/sound-start.wav` | Sound played when timer starts |
| `resources/sound-pause.wav` | Sound played when timer pauses |
| `resources/timer-*.png` | Timer state icons |

## TypeScript Configs

| File | Purpose |
|------|---------|
| `tsconfig.json` | Root config |
| `tsconfig.node.json` | Node.js process (main + preload) |
| `tsconfig.web.json` | Renderer process |

## PRD

Source: `time-tracker-prd.md` — Product Requirements Document defining the app's features, target platform, and design goals.

## Cross-References

- [Overview](00-overview.md) — Architecture summary
- [Features](04-features.md) — Pin, hotkeys, export/import
