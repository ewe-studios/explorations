---
location: /home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.GedWeb/utm-dev/pkg/icons/
explored_at: 2026-03-19T12:00:00Z
package: pkg/icons
---

# Deep Dive: Icon Generation (pkg/icons/)

## Overview

The `pkg/icons` package generates platform-specific application icons from a single source image (`icon-source.png`). It handles:

- Android adaptive icons (multiple densities)
- iOS app icons (multiple sizes)
- macOS ICNS bundles
- Windows ICO and MSIX icons

## Configuration

### Config Structure

```go
type Config struct {
    InputPath  string  // Source icon (icon-source.png)
    OutputPath string  // Output directory
    Platform   string  // Target platform
}
```

**Platforms:**
- `android` - Android adaptive icons
- `ios` - iOS app icons
- `macos` - macOS ICNS bundle
- `windows-msix` - Windows MSIX package icons
- `windows-ico` - Windows ICO file

### ProjectConfig Structure

```go
type ProjectConfig struct {
    ProjectPath string  // Project directory
    Platform    string  // Target platform
}
```

## Icon Generation Flow

### High-Level Flow

```
1. Ensure source icon exists (icon-source.png)
2. Create output directories
3. Generate platform-specific sizes
4. Create platform-specific format (XML, JSON, ICNS, ICO)
```

### GenerateForProject()

```go
func GenerateForProject(cfg ProjectConfig) error {
    // Get or create source icon
    sourceIconPath, err := EnsureSourceIcon(cfg.ProjectPath)
    if err != nil {
        return fmt.Errorf("failed to ensure source icon: %w", err)
    }

    // Determine output path
    var outputPath string
    switch cfg.Platform {
    case "android":
        outputPath = filepath.Join(cfg.ProjectPath, constants.BuildDir)
    case "ios", "macos":
        outputPath = filepath.Join(cfg.ProjectPath, constants.BuildDir, "Assets.xcassets")
    case "windows", "windows-msix":
        outputPath = filepath.Join(cfg.ProjectPath, constants.BuildDir)
    }

    // Generate icons
    return Generate(Config{
        InputPath:  sourceIconPath,
        OutputPath: outputPath,
        Platform:   cfg.Platform,
    })
}
```

## Platform-Specific Generation

### Android Icons

**Output Structure:**
```
project/build/
├── res/
│   ├── mipmap-mdpi/
│   │   └── ic_launcher.png (48x48)
│   ├── mipmap-hdpi/
│   │   └── ic_launcher.png (72x72)
│   ├── mipmap-xhdpi/
│   │   └── ic_launcher.png (96x96)
│   ├── mipmap-xxhdpi/
│   │   └── ic_launcher.png (144x144)
│   ├── mipmap-xxxhdpi/
│   │   └── ic_launcher.png (192x192)
│   └── mipmap-anydpi-v26/
│       └── ic_launcher.xml (adaptive icon definition)
└── icon.png (fallback)
```

**Implementation:**
```go
func generateAndroidIcons(inputPath, outputPath string) error {
    // Load source image
    src, err := loadIcon(inputPath)
    if err != nil {
        return err
    }

    // Generate all densities
    sizes := map[string]int{
        "mipmap-mdpi":   48,
        "mipmap-hdpi":   72,
        "mipmap-xhdpi":  96,
        "mipmap-xxhdpi": 144,
        "mipmap-xxxhdpi": 192,
    }

    for dir, size := range sizes {
        dirPath := filepath.Join(outputPath, "res", dir)
        os.MkdirAll(dirPath, 0755)

        resized := resizeImage(src, size, size)
        savePNG(resized, filepath.Join(dirPath, "ic_launcher.png"))
    }

    // Generate adaptive icon XML
    generateAdaptiveIconXML(outputPath)

    return nil
}
```

**Adaptive Icon XML:**
```xml
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@mipmap/ic_launcher_background"/>
    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>
</adaptive-icon>
```

### iOS Icons

**Output Structure:**
```
project/build/Assets.xcassets/
├── AppIcon.appiconset/
│   ├── Contents.json
│   ├── icon-20x20@2x.png
│   ├── icon-20x20@3x.png
│   ├── icon-29x29@2x.png
│   ├── icon-29x29@3x.png
│   ├── icon-40x40@2x.png
│   ├── icon-40x40@3x.png
│   ├── icon-60x60@2x.png
│   ├── icon-60x60@3x.png
│   ├── icon-76x76.png
│   ├── icon-76x76@2x.png
│   ├── icon-83.5x83.5@2x.png
│   └── icon-1024x1024.png (App Store)
└── Contents.json
```

**Contents.json:**
```json
{
  "images": [
    {"size": "20x20", "idiom": "iphone", "scale": "2x", "filename": "icon-20x20@2x.png"},
    {"size": "20x20", "idiom": "iphone", "scale": "3x", "filename": "icon-20x20@3x.png"},
    {"size": "29x29", "idiom": "iphone", "scale": "2x", "filename": "icon-29x29@2x.png"},
    {"size": "29x29", "idiom": "iphone", "scale": "3x", "filename": "icon-29x29@3x.png"},
    ...
  ],
  "info": {
    "version": 1,
    "author": "utm-dev"
  }
}
```

### macOS Icons (ICNS)

**Output:** `.icns` file containing multiple icon sizes

**Implementation:**
```go
func generateICNS(inputPath, outputPath string) error {
    src, err := png.Decode(openFile(inputPath))
    if err != nil {
        return err
    }

    // Create ICNS structure
    icnsData := createICNSData(src)

    // Write ICNS file
    err = icns.Encode(writeFile(outputPath, "icon.icns"), icnsData)
    return err
}
```

**Icon Sizes in ICNS:**
- 16x16
- 32x32
- 64x64
- 128x128
- 256x256
- 512x512
- 1024x1024

### Windows Icons

**MSIX Format:**
```
project/build/
├── Images/
│   ├── StoreLogo.scale-100.png (50x50)
│   ├── Square44x44Logo.scale-100.png
│   ├── Square150x150Logo.scale-100.png
│   └── Wide310x150Logo.scale-100.png
└── Package.appxmanifest (references icons)
```

**ICO Format:**
```
project/build/
└── icon.ico (multi-size icon file)
```

**ICO Contains:**
- 16x16
- 32x32
- 48x48
- 256x256

## Source Icon Generation

### EnsureSourceIcon()

```go
func EnsureSourceIcon(appDir string) (string, error) {
    sourceIconPath := filepath.Join(appDir, "icon-source.png")

    if _, err := os.Stat(sourceIconPath); os.IsNotExist(err) {
        // Generate test icon if missing
        if err := GenerateTestIcon(sourceIconPath); err != nil {
            return "", fmt.Errorf("failed to generate source icon: %w", err)
        }
    }

    return sourceIconPath, nil
}
```

### GenerateTestIcon()

Creates a simple 1024x1024 blue square for testing:

```go
func GenerateTestIcon(outputPath string) error {
    img := image.NewRGBA(image.Rect(0, 0, 1024, 1024))
    blue := color.RGBA{0, 0, 255, 255}

    for x := 0; x < 1024; x++ {
        for y := 0; y < 1024; y++ {
            img.Set(x, y, blue)
        }
    }

    f, err := os.Create(outputPath)
    defer f.Close()

    return png.Encode(f, img)
}
```

## Image Processing

### resizeImage()

Uses bilinear interpolation via `github.com/nfnt/resize`:

```go
import "github.com/nfnt/resize"

func resizeImage(src image.Image, width, height int) image.Image {
    return resize.Resize(uint(width), uint(height), src, resize.Lanczos3)
}
```

### loadIcon()

```go
func loadIcon(path string) (image.Image, error) {
    f, err := os.Open(path)
    if err != nil {
        return nil, err
    }
    defer f.Close()

    return png.Decode(f)
}
```

## Constants

Icon sizes and paths defined in `pkg/constants/directories.go`:

```go
const (
    BuildDir = ".build"
    BinDir   = ".bin"
)
```

## Dependencies

### External Libraries

| Library | Purpose |
|---------|---------|
| `github.com/nfnt/resize` | Image resizing |
| `github.com/JackMordaunt/icns` | ICNS generation |
| `golang.org/x/image` | Image encoding/decoding |

## Usage in Build Flow

### cmd/build.go Integration

```go
func buildMacOS(proj *project.GioProject, platform string, opts BuildOptions) error {
    // Generate icons before build
    if !opts.SkipIcons {
        if err := generateIcons(proj.RootDir, "macos"); err != nil {
            cache.RecordBuild(..., false)
            return fmt.Errorf("failed to generate icons: %w", err)
        }
    }

    // gogio uses generated icons
    iconPath := proj.Paths().SourceIcon
    gogioCmd := exec.Command("gogio", "-icon", iconPath, ...)
}
```

### generateIcons() Helper

```go
func generateIcons(appDir, platform string) error {
    sourceIconPath, err := icons.EnsureSourceIcon(appDir)
    if err != nil {
        return err
    }

    var outputPath string
    switch platform {
    case "android":
        outputPath = filepath.Join(appDir, constants.BuildDir)
    case "ios", "macos":
        outputPath = filepath.Join(appDir, constants.BuildDir, "Assets.xcassets")
    case "windows":
        platform = "windows-msix"
        outputPath = filepath.Join(appDir, constants.BuildDir)
    }

    return icons.Generate(icons.Config{
        InputPath:  sourceIconPath,
        OutputPath: outputPath,
        Platform:   platform,
    })
}
```

## Design Decisions

### 1. Single Source Image

**Why:** Simplifies designer workflow - one 1024x1024 PNG for all platforms.

**Trade-off:** Less control over platform-specific optimizations.

### 2. Build Directory Output

Icons go to `.build/` not source directory.

**Why:**
- Generated artifacts separate from source
- Easy to clean (`rm -rf .build`)
- gogio expects icons in build directory

### 3. Auto-Generated Test Icon

**Why:**
- New projects work immediately
- No blocking on designer assets
- Consistent testing experience

### 4. Platform-Specific Formats

Each platform gets its native format:
- Android: PNG + XML
- iOS: PNG + JSON
- macOS: ICNS
- Windows: ICO/MSIX

## Error Handling

```go
func Generate(cfg Config) error {
    switch cfg.Platform {
    case "android":
        return generateAndroidIcons(cfg.InputPath, cfg.OutputPath)
    case "ios":
        return generateIOSIcons(cfg.InputPath, cfg.OutputPath)
    case "macos":
        return generateICNS(cfg.InputPath, cfg.OutputPath)
    case "windows-msix":
        return generateWindowsIcons(cfg.InputPath, cfg.OutputPath)
    case "windows-ico":
        return generateICO(cfg.InputPath, cfg.OutputPath)
    default:
        return fmt.Errorf("unsupported platform: %s", cfg.Platform)
    }
}
```

## Future Enhancements

1. **Foreground/Background Separation:** Android adaptive icon support
2. **Rounded Corners:** Automatic corner application for iOS
3. **Icon Preview:** Generate preview grid for designers
4. **SVG Support:** Accept SVG as source format
5. **Theme Variants:** Light/dark mode icon generation
