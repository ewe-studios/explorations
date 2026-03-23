# Fresh Plugins Registry Exploration

## Overview

The Fresh Plugins Registry is a centralized metadata system for discovering, installing, and managing Fresh editor plugins. It provides structured information about available plugins, language packs, and themes.

**Location**: `/home/darkvoid/Boxxed/@formulas/src.rust/src.CodingIDE/fresh-plugins-registry/`

---

## Directory Structure

```
fresh-plugins-registry/
├── plugins.json           # Main plugin registry
├── languages.json         # Language pack registry
├── themes.json            # Theme registry
├── blocklist.json         # Blocked/malicious plugins
├── schemas/
│   └── registry.schema.json  # JSON Schema for validation
├── README.md              # Registry documentation
└── update-schemas.sh      # Schema update script
```

---

## Registry Files

### 1. plugins.json

Main plugin registry containing metadata for all available plugins.

```json
{
  "$schema": "./schemas/registry.schema.json",
  "schema_version": 1,
  "updated": "2026-01-25T13:00:00Z",
  "packages": {
    "calculator": {
      "description": "In-editor calculator with expression evaluation",
      "repository": "https://github.com/sinelaw/fresh-plugins#calculator",
      "author": "Fresh Editor Team",
      "license": "MIT",
      "keywords": ["calculator", "math", "utility"],
      "latest_version": "1.0.0",
      "fresh_min_version": "0.1.0"
    },
    "color-highlighter": {
      "description": "Highlights color codes (hex, rgb, hsl) with their actual colors",
      "repository": "https://github.com/sinelaw/fresh-plugins#color-highlighter",
      "author": "Fresh Editor Team",
      "license": "MIT",
      "keywords": ["color", "css", "highlighting", "preview"],
      "latest_version": "1.0.0",
      "fresh_min_version": "0.1.0"
    },
    "todo-highlighter": {
      "description": "Highlights TODO, FIXME, HACK, and other comment annotations",
      "repository": "https://github.com/sinelaw/fresh-plugins#todo-highlighter",
      "author": "Fresh Editor Team",
      "license": "MIT",
      "keywords": ["todo", "fixme", "highlighting", "annotations"],
      "latest_version": "1.0.0",
      "fresh_min_version": "0.1.0"
    },
    "amp": {
      "description": "Amp AI coding agent integration for Fresh Editor",
      "repository": "https://github.com/sinelaw/fresh-plugins#amp",
      "author": "Fresh Editor Team",
      "license": "Apache-2.0",
      "keywords": ["amp", "ai", "coding", "agent", "sourcegraph"],
      "latest_version": "0.1.0",
      "fresh_min_version": "0.1.0"
    }
  }
}
```

#### Package Metadata Fields

| Field | Type | Description |
|-------|------|-------------|
| `description` | string | Human-readable description |
| `repository` | string | URL to source code/repository |
| `author` | string | Plugin author/maintainer |
| `license` | string | SPDX license identifier |
| `keywords` | string[] | Search/discovery keywords |
| `latest_version` | string | Semantic version string |
| `fresh_min_version` | string | Minimum Fresh version required |

---

### 2. languages.json

Registry of language packs for syntax highlighting and language-specific features.

```json
{
  "$schema": "./schemas/registry.schema.json",
  "schema_version": 1,
  "languages": {
    "elixir": {
      "name": "Elixir",
      "extensions": [".ex", ".exs"],
      "scopeName": "source.elixir",
      "grammar": "https://github.com/elixir-editors/elixir-tmbundle",
      "highlights": true
    },
    "hare": {
      "name": "Hare",
      "extensions": [".ha"],
      "scopeName": "source.hare",
      "grammar": "https://git.sr.ht/~ecs/hare-tmbundle",
      "highlights": true
    },
    "solidity": {
      "name": "Solidity",
      "extensions": [".sol"],
      "scopeName": "source.solidity",
      "grammar": "https://github.com/juanfranblanco/vscode-solidity",
      "highlights": true
    },
    "templ": {
      "name": "Templ",
      "extensions": [".templ"],
      "scopeName": "source.templ",
      "grammar": "https://github.com/a-h/templ",
      "highlights": true
    },
    "zenc": {
      "name": "Zenc",
      "extensions": [".zenc"],
      "scopeName": "source.zenc",
      "grammar": "https://github.com/sinelaw/zenc",
      "highlights": true
    }
  }
}
```

#### Language Pack Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Display name |
| `extensions` | string[] | File extensions |
| `scopeName` | string | TextMate scope name |
| `grammar` | string | Grammar repository URL |
| `highlights` | boolean | Syntax highlighting support |

---

### 3. themes.json

Registry of color themes for the editor.

```json
{
  "$schema": "./schemas/registry.schema.json",
  "schema_version": 1,
  "themes": {
    "default-dark": {
      "name": "Default Dark",
      "type": "dark",
      "author": "Fresh Editor Team",
      "colors": {
        "background": "#1e1e1e",
        "foreground": "#d4d4d4",
        "accent": "#007acc"
      },
      "preview": "https://getfresh.dev/themes/default-dark.png"
    },
    "default-light": {
      "name": "Default Light",
      "type": "light",
      "author": "Fresh Editor Team",
      "colors": {
        "background": "#ffffff",
        "foreground": "#333333",
        "accent": "#0066cc"
      },
      "preview": "https://getfresh.dev/themes/default-light.png"
    },
    "monokai": {
      "name": "Monokai",
      "type": "dark",
      "author": "Wimer Hazenberg",
      "colors": {
        "background": "#272822",
        "foreground": "#f8f8f2",
        "accent": "#a6e22e"
      },
      "preview": "https://getfresh.dev/themes/monokai.png"
    },
    "gruvbox": {
      "name": "Gruvbox Dark",
      "type": "dark",
      "author": "Pavel Pertsev",
      "colors": {
        "background": "#282828",
        "foreground": "#ebdbb2",
        "accent": "#d79921"
      },
      "preview": "https://getfresh.dev/themes/gruvbox.png"
    },
    "nord": {
      "name": "Nord",
      "type": "dark",
      "author": "Arctic Ice Studio",
      "colors": {
        "background": "#2e3440",
        "foreground": "#d8dee9",
        "accent": "#88c0d0"
      },
      "preview": "https://getfresh.dev/themes/nord.png"
    }
  }
}
```

#### Theme Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Theme display name |
| `type` | string | "dark" or "light" |
| `author` | string | Theme creator |
| `colors` | object | Key UI colors |
| `preview` | string | Screenshot URL |

---

### 4. blocklist.json

Registry of blocked/malicious plugins.

```json
{
  "$schema": "./schemas/registry.schema.json",
  "schema_version": 1,
  "blocked_plugins": [
    {
      "name": "malicious-plugin",
      "reason": "Attempts to access filesystem directly",
      "date_added": "2024-01-01",
      "reported_by": "security@getfresh.dev"
    }
  ],
  "blocked_authors": [
    {
      "author": "bad-actor",
      "reason": "Multiple security violations",
      "date_added": "2024-01-01"
    }
  ]
}
```

#### Blocklist Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Plugin name (for plugin blocks) |
| `author` | string | Author name (for author blocks) |
| `reason` | string | Reason for blocking |
| `date_added` | string | ISO 8601 date |
| `reported_by` | string | Reporter contact |

---

## JSON Schema

### registry.schema.json

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "Fresh Plugin Registry",
  "description": "Schema for Fresh editor plugin registry",
  "type": "object",
  "required": ["schema_version", "updated"],
  "properties": {
    "$schema": {
      "type": "string",
      "description": "JSON Schema reference"
    },
    "schema_version": {
      "type": "integer",
      "minimum": 1,
      "description": "Registry schema version"
    },
    "updated": {
      "type": "string",
      "format": "date-time",
      "description": "Last update timestamp (ISO 8601)"
    },
    "packages": {
      "type": "object",
      "additionalProperties": {
        "$ref": "#/definitions/PackageInfo"
      }
    }
  },
  "definitions": {
    "PackageInfo": {
      "type": "object",
      "required": ["description", "latest_version"],
      "properties": {
        "description": { "type": "string" },
        "repository": { "type": "string", "format": "uri" },
        "author": { "type": "string" },
        "license": { "type": "string" },
        "keywords": {
          "type": "array",
          "items": { "type": "string" }
        },
        "latest_version": { "type": "string" },
        "fresh_min_version": { "type": "string" }
      }
    }
  }
}
```

---

## Update Mechanism

### update-schemas.sh

```bash
#!/bin/bash
# update-schemas.sh - Update registry schemas

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Fetch latest schema from upstream
curl -sL https://getfresh.dev/schemas/registry.schema.json \
    -o schemas/registry.schema.json

# Validate all registry files
for file in *.json; do
    if [ "$file" != "update-schemas.sh" ]; then
        echo "Validating $file..."
        # Use ajv or similar JSON Schema validator
        # ajv validate -s schemas/registry.schema.json -d "$file"
    fi
done

echo "Schema update complete!"
```

---

## Registry API (Future)

A potential HTTP API for plugin discovery:

### Endpoints

```
GET /api/v1/plugins          # List all plugins
GET /api/v1/plugins/:name    # Get plugin details
GET /api/v1/languages        # List language packs
GET /api/v1/themes           # List themes
GET /api/v1/search?q=:query  # Search plugins
POST /api/v1/plugins         # Submit new plugin (auth required)
```

### Response Format

```json
{
  "success": true,
  "data": { ... },
  "meta": {
    "total": 100,
    "page": 1,
    "per_page": 20
  }
}
```

---

## Integration with Fresh Editor

### Plugin Discovery

Fresh editor can fetch the registry to enable plugin installation:

```rust
// Hypothetical plugin discovery code
pub async fn fetch_registry() -> Result<PluginRegistry> {
    let response = ureq::get("https://getfresh.dev/registry/plugins.json")
        .call()?;
    let registry: PluginRegistry = response.into_json()?;
    Ok(registry)
}

pub fn search_plugins(query: &str, registry: &PluginRegistry) -> Vec<&PluginInfo> {
    registry.packages.values()
        .filter(|p| p.keywords.iter().any(|k| k.contains(query))
               || p.description.contains(query))
        .collect()
}
```

### Plugin Installation Flow

```
1. User runs: plugin install calculator
2. Fresh fetches registry
3. Finds "calculator" package
4. Downloads from repository URL
5. Validates signature/hash
6. Installs to ~/.config/fresh/plugins/
7. Reloads plugin system
```

---

## Security Considerations

### Plugin Verification

```json
{
  "packages": {
    "calculator": {
      "name": "calculator",
      "latest_version": "1.0.0",
      "checksum": "sha256:abc123...",
      "signature": "PGP signature here",
      "public_key": "https://getfresh.dev/keys/fresh-editor.pub"
    }
  }
}
```

### Trust Model

1. **Registry-signed**: Packages signed by Fresh Editor team
2. **Community-verified**: Community-reviewed plugins
3. **Unverified**: User-installed from external sources

---

## Version Compatibility

### SemVer Enforcement

```typescript
// Plugin compatibility check
function isCompatible(pluginMinVersion: string, editorVersion: string): boolean {
    const [pMajor, pMinor, pPatch] = pluginMinVersion.split('.').map(Number);
    const [eMajor, eMinor, ePatch] = editorVersion.split('.').map(Number);

    if (eMajor < pMajor) return false;
    if (eMajor > pMajor) return true;
    if (eMinor < pMinor) return false;
    return true;
}
```

---

## Related Documents

- [Fresh Plugins](fresh-plugins-exploration.md) - Plugin system exploration
- [Fresh Editor](fresh-exploration.md) - Main editor exploration
- [Rust Revision](rust-revision.md) - Rust reproduction guide
