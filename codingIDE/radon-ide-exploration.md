# Radon IDE Exploration

## Overview

**Radon IDE** is a commercial IDE extension for VSCode and Cursor that transforms these editors into fully-featured IDEs for React Native and Expo development.

- **Website**: https://ide.swmansion.com
- **Publisher**: Software Mansion
- **License**: Commercial (paid)
- **Platforms**: VSCode, Cursor, Windsurf

**Note**: This directory (`/home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/radon-ide/`) is primarily a mirror/documentation repository for issue tracking. The actual Radon IDE source code is proprietary and not included here.

---

## Repository Contents

```
radon-ide/
├── README.md           # Main documentation
├── packages/
│   └── docs/          # Documentation website source
│       ├── static/
│       │   └── img/   # Documentation images
│       └── docs/      # Markdown documentation
├── .github/           # GitHub workflows
├── LICENSE.txt        # License (proprietary)
└── .gitignore
```

---

## Features

### Element Inspector with Component Hierarchy

Radon provides a visual element inspector that shows the React Native component tree in real-time, allowing developers to:
- Inspect component properties
- View the component hierarchy
- Navigate between parent/child components
- See real-time updates as the app runs

### Debugger Integrated with Source Code

Full debugging support with:
- Breakpoints in source code
- Step-through debugging
- Variable inspection
- Call stack navigation
- Watch expressions

### Logging Console with Jump-to-Source

Enhanced logging console that:
- Shows console.log output from the device
- Provides clickable links to source locations
- Filters and searches logs
- Supports log levels (info, warn, error)

### Device Settings Adjustments

Control simulator/emulator settings directly from the IDE:
- **Theme**: Switch between light/dark mode
- **Text Size**: Adjust system font size
- **Location**: Mock GPS location
- **System Language**: Change device language
- **Accessibility**: Toggle accessibility settings

### Screen Recording and Replays

Built-in screen recording for:
- Recording bug reproductions
- Creating demos
- Sharing with team members
- Automatic replay generation

### Component Preview

Preview individual React Native components in isolation:
- Similar to Storybook functionality
- Quick component testing
- Visual regression testing support

---

## Project Compatibility

Radon IDE works with:
- **React Native**: 0.68+
- **Expo**: SDK 45+
- **Expo Go**: For quick testing
- **Development builds**: For full features

### Supported Platforms

| Platform | Support Level |
|----------|---------------|
| iOS Simulator | Full |
| Android Emulator | Full |
| Physical iOS Device | Full |
| Physical Android Device | Full |
| Web (React Native Web) | Partial |

---

## Installation

### Via VSCode/Cursor Marketplace

1. Open VSCode or Cursor
2. Go to Extensions (Ctrl+Shift+X)
3. Search for "Radon IDE"
4. Click Install

### Direct Installation

```bash
# Download from website
curl -L https://ide.swmansion.com/download/radon-ide.vsix -o radon-ide.vsix

# Install in VSCode
code --install-extension radon-ide.vsix
```

---

## Getting Started

### 1. Launch Radon IDE

After installation:
1. Open a React Native/Expo project
2. Open the Command Palette (Ctrl+Shift+P)
3. Run "Radon IDE: Start"

### 2. Configure Your Project

Radon auto-detects most configurations, but you may need to specify:
- Metro bundler port (default: 8081)
- Platform (iOS/Android)
- Device/Simulator selection

### 3. Start Development

Once connected:
- View logs in the Radon panel
- Use the element inspector
- Set breakpoints and debug
- Adjust device settings

---

## Architecture

### Extension Components

```
┌────────────────────────────────────────────────────┐
│                VSCode / Cursor                      │
│  ┌──────────────────────────────────────────────┐  │
│  │           Radon IDE Extension                 │  │
│  │  ┌────────────┐  ┌────────────┐  ┌────────┐ │  │
│  │  │   Panel    │  │  Debugger  │  │ Device │ │  │
│  │  │   (React)  │  │  Adapter   │  │  API   │ │  │
│  │  └────────────┘  └────────────┘  └────────┘ │  │
│  └──────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────┘
                          │
                          │ Metro Protocol
                          │ Chrome DevTools Protocol
                          ▼
┌────────────────────────────────────────────────────┐
│           React Native / Expo App                   │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐   │
│  │   Metro    │  │  React     │  │   Native   │   │
│  │  Bundler   │  │  Debugger  │  │   Runtime  │   │
│  └────────────┘  └────────────┘  └────────────┘   │
└────────────────────────────────────────────────────┘
```

### Communication Protocols

1. **Metro Bundler API**: For bundle information
2. **Chrome DevTools Protocol**: For debugging
3. **React Native Debug Protocol**: For element inspection
4. **Custom Native Module**: For device settings

---

## License and Pricing

### Free Trial

- 14-day free trial
- Full feature access
- No credit card required

### Paid Plans

| Plan | Price | Features |
|------|-------|----------|
| Individual | $15/month | Single developer license |
| Team | $50/month | Up to 5 developers |
| Enterprise | Custom | Unlimited developers, priority support |

### License Activation

1. Purchase license from https://ide.swmansion.com/pricing
2. Receive license key via email
3. Open VSCode/Cursor
4. Open Radon IDE panel
5. Enter license key

---

## Comparison with Alternatives

| Feature | Radon IDE | React Native Debugger | Flipper |
|---------|-----------|----------------------|---------|
| Element Inspector | ✅ | ✅ | ✅ |
| Debugger Integration | ✅ | ✅ | ✅ |
| Device Settings | ✅ | ❌ | Partial |
| Screen Recording | ✅ | ❌ | ❌ |
| Component Preview | ✅ | ❌ | ❌ |
| Jump-to-Source Logs | ✅ | ❌ | ❌ |
| Cost | Paid | Free | Free |
| Platform | VSCode/Cursor | Standalone | Standalone |

---

## Troubleshooting

### Common Issues

#### "Cannot connect to device"
1. Ensure Metro bundler is running
2. Check that device is connected
3. Verify correct port configuration

#### "Debugger not attaching"
1. Restart the app in debug mode
2. Check Chrome DevTools connection
3. Clear watchman roots: `watchman watch-del-all`

#### "Element inspector not working"
1. Ensure React Native dev menu is enabled
2. Check that element inspector is activated
3. Restart VSCode/Cursor

---

## Resources

### Documentation
- [Getting Started](https://ide.swmansion.com/docs/getting-started/installation)
- [Feature Guide](https://ide.swmansion.com/docs/guides/feature-highlight)
- [Troubleshooting](https://ide.swmansion.com/docs/guides/troubleshooting)

### Community
- [GitHub Issues](https://github.com/sinelaw/radon-ide/issues)
- [Discord](https://discord.gg/swmansion)
- [Twitter](https://twitter.com/swmansion)

---

## Development

### Building from Source (Documentation Site Only)

This repository only contains the documentation site source code:

```bash
cd packages/docs
npm install
npm run start
```

The actual extension source is proprietary and not available in this repository.

---

## Related Projects

### By Software Mansion

- **React Native Reanimated**: Animation library
- **React Native Gesture Handler**: Native gesture handling
- **React Native Screens**: Native screen primitives
- **Expo Modules**: Expo module infrastructure

### Similar Tools

- **Expo DevTools**: Web-based development tools
- **Flipper**: Mobile app debugger by Meta
- **React Native Debugger**: Standalone debugger

---

## Notes for Rust Implementation

If building a similar IDE extension in Rust:

### Potential Architecture

```rust
// Hypothetical Rust IDE extension
pub struct IdeExtension {
    debugger: DebuggerAdapter,
    device: DeviceController,
    inspector: ElementInspector,
}

// Use tower-lsp for LSP-like protocol
use tower_lsp::{LanguageServer, lsp_types::*};

// Use tokio for async communication
use tokio::sync::mpsc;

// Use serde for message serialization
use serde::{Serialize, Deserialize};
```

### Key Considerations

1. **VSCode Extension API**: Requires TypeScript/JavaScript
2. **Native Modules**: Rust for performance-critical parts
3. **Protocol Handling**: Chrome DevTools Protocol parsing
4. **Device Communication**: USB/network protocols

---

## Related Documents

- [Fresh Editor](fresh-exploration.md) - Open-source terminal editor
- [Rockies](rockies-exploration.md) - WASM-based game
- [WASM Analysis](wasm-web-editor-analysis.md) - Web editor feasibility
