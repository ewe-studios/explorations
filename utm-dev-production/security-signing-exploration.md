# utm-dev Production - Security & Code Signing Exploration

## Overview

This document explores code signing, notarization, and security strategies for production apps built with utm-dev across all target platforms.

## Architecture

### Code Signing Layers

```
┌─────────────────────────────────────────────────────────────────┐
│                    Application Layer                            │
│  Gio App (Go + WebView)                                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ signed with
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Platform Signing Layer                       │
│  macOS: codesign + notarytool                                   │
│  iOS:   codesign + provisioning profiles                        │
│  Android: apksigner + zipalign                                  │
│  Windows: signtool (Authenticode)                               │
│  Linux:   (optional) gpg signing                                │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ validated by
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Platform Verification                        │
│  Gatekeeper (macOS), Play Protect (Android), SmartScreen (Win) │
└─────────────────────────────────────────────────────────────────┘
```

## macOS Code Signing & Notarization

### Signing Requirements

| Requirement | Description | Tool |
|-------------|-------------|------|
| **Developer ID** | Apple Developer account ($99/year) | App Store Connect |
| **Code Signing** | Sign app with Developer ID Application | `codesign` |
| **Notarization** | Apple security scan | `notarytool` |
| **Stapling** | Attach notarization ticket | `stapler` |

### Signing Workflow

```bash
# 1. Sign the application
codesign --deep --force --verify --verbose \
  --sign "Developer ID Application: Your Name (TEAM_ID)" \
  --options runtime \
  --entitlements entitlements.plist \
  MyApp.app

# 2. Create DMG for notarization
hdiutil create -fs APFS -srcfolder MyApp.app -volname MyApp myapp.dmg

# 3. Submit for notarization
xcrun notarytool submit myapp.dmg \
  --apple-id "your@email.com" \
  --team-id "TEAM_ID" \
  --password-profile "MyProfile" \
  --wait

# 4. Staple the notarization ticket
xcrun stapler staple MyApp.app

# 5. Verify
spctl --assess --type install --context context:primary-signature --verbose=2 MyApp.app
```

### Entitlements File

```xml
<!-- entitlements.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- Hardened Runtime -->
    <key>com.apple.security.cs.allow-jit</key>
    <true/>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.cs.disable-executable-page-protection</key>
    <true/>

    <!-- Network -->
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.network.server</key>
    <true/>

    <!-- Files -->
    <key>com.apple.security.files.user-selected.read-write</key>
    <true/>
    <key>com.apple.security.files.bookmarks.app-scope</key>
    <true/>

    <!-- Automation -->
    <key>com.apple.security.automation.apple-events</key>
    <true/>
</dict>
</plist>
```

### utm-dev Integration

```go
// pkg/signer/macos.go
package signer

import (
    "context"
    "fmt"
    "os/exec"
    "path/filepath"
    "time"
)

type MacOSSigner struct {
    developerID   string
    teamID        string
    appleID       string
    passwordProfile string
}

func (s *MacOSSigner) Sign(appPath string, entitlementsPath string) error {
    // Run codesign
    cmd := exec.Command("codesign",
        "--deep", "--force", "--verify", "--verbose",
        "--sign", s.developerID,
        "--options", "runtime",
        "--entitlements", entitlementsPath,
        appPath,
    )
    return cmd.Run()
}

func (s *MacOSSigner) Notarize(dmgPath string) error {
    ctx := context.Background()

    // Submit for notarization
    cmd := exec.CommandContext(ctx, "xcrun", "notarytool", "submit",
        dmgPath,
        "--apple-id", s.appleID,
        "--team-id", s.teamID,
        "--password-profile", s.passwordProfile,
        "--wait",
    )
    return cmd.Run()
}

func (s *MacOSSigner) Staple(appPath string) error {
    cmd := exec.Command("xcrun", "stapler", "staple", appPath)
    return cmd.Run()
}
```

### Keychain Setup

```bash
# Store notarization credentials in keychain
xcrun notarytool store-credentials "MyProfile" \
  --apple-id "your@email.com" \
  --team-id "TEAM_ID" \
  --password "app-specific-password"
```

## iOS Code Signing

### Signing Requirements

| Requirement | Description |
|-------------|-------------|
| **Apple Developer** | Required for device testing ($99/year) |
| **Provisioning Profile** | Links App ID, certificates, devices |
| **Signing Certificate** | iOS Development or Distribution |
| **UDID Registration** | Device must be registered for dev builds |

### Provisioning Profile Types

| Type | Use Case | Distribution |
|------|----------|--------------|
| **Development** | Testing on registered devices | Xcode/Device |
| **Ad Hoc** | Beta testing (up to 100 devices) | Direct download |
| **App Store** | Production release | App Store only |
| **Enterprise** | Internal company distribution | In-house only |

### utm-dev Integration

```go
// pkg/signer/ios.go
package signer

type IOSSigner struct {
    provisioningProfilePath string
    signingCertificate      string
    entitlementsPath        string
}

func (s *IOSSigner) SignApp(appPath string, bundleID string) error {
    // 1. Embed provisioning profile
    profileDest := filepath.Join(appPath, "embedded.mobileprovision")
    if err := copyFile(s.provisioningProfilePath, profileDest); err != nil {
        return fmt.Errorf("failed to embed provisioning profile: %w", err)
    }

    // 2. Codesign
    cmd := exec.Command("codesign",
        "--deep", "--force", "--verbose",
        "--sign", s.signingCertificate,
        "--entitlements", s.entitlementsPath,
        appPath,
    )
    return cmd.Run()
}

func (s *IOSSigner) GenerateEntitlements(bundleID string, capabilities []string) ([]byte, error) {
    entitlements := `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>` + bundleID + `</string>
`
    // Add capabilities
    for _, cap := range capabilities {
        entitlements += fmt.Sprintf("    <key>%s</key>\n    <true/>\n", cap)
    }

    entitlements += `</dict></plist>`
    return []byte(entitlements), nil
}
```

### Fastlane Integration

```ruby
# Fastfile for iOS signing
platform :ios do
  desc "Build and sign iOS app"
  lane :build_signed do |options|
    # Get certificates
    match(
      type: "appstore",
      app_identifier: options[:bundle_id],
      username: ENV["APPLE_ID"]
    )

    # Build with utm-dev
    sh "utm-dev build ios #{options[:app_path]}"

    # Sign
    sign_app(
      ipa: "build/app.ipa",
      certificate: ENV["SIGH_ID"]
    )
  end
end
```

## Android Signing

### Keystore Setup

```bash
# Generate release keystore
keytool -genkey -v -keystore release.keystore \
  -alias release \
  -keyalg RSA -keysize 2048 -validity 10000

# Configure in app
export ANDROID_KEYSTORE_PATH=~/keystores/release.keystore
export ANDROID_KEYSTORE_PASSWORD=your_password
export ANDROID_KEY_ALIAS=release
export ANDROID_KEY_PASSWORD=your_key_password
```

### APK Signing Workflow

```bash
# 1. Build unsigned APK
gogio -target android -o app-unsigned.apk ./...

# 2. Align the APK
zipalign -p -v 4 app-unsigned.apk app-aligned.apk

# 3. Sign with apksigner
apksigner sign --ks release.keystore \
  --ks-key-alias release \
  --out app-release.apk \
  app-aligned.apk

# 4. Verify signature
apksigner verify --verbose app-release.apk
```

### AAB (App Bundle) Signing

```bash
# Build App Bundle
gogio -target android -bundle -o app.aab ./...

# Sign the bundle
jarsigner -keystore release.keystore \
  -storepass $KEYSTORE_PASSWORD \
  -keypass $KEY_PASSWORD \
  app.aab release

# Verify
jarsigner -verify -verbose -certs app.aab
```

### utm-dev Integration

```go
// pkg/signer/android.go
package signer

import (
    "os/exec"
    "fmt"
)

type AndroidSigner struct {
    keystorePath     string
    keystorePassword string
    keyAlias         string
    keyPassword      string
}

func (s *AndroidSigner) SignAPK(apkPath, outputDir string) (string, error) {
    // Align first
    alignedAPK := filepath.Join(outputDir, "app-aligned.apk")
    if err := s.alignAPK(apkPath, alignedAPK); err != nil {
        return "", err
    }

    // Sign
    signedAPK := filepath.Join(outputDir, "app-release.apk")
    if err := s.signAPK(alignedAPK, signedAPK); err != nil {
        return "", err
    }

    return signedAPK, nil
}

func (s *AndroidSigner) alignAPK(inputAPK, outputAPK string) error {
    cmd := exec.Command("zipalign", "-p", "-v", "4", inputAPK, outputAPK)
    return cmd.Run()
}

func (s *AndroidSigner) signAPK(inputAPK, outputAPK string) error {
    cmd := exec.Command("apksigner", "sign",
        "--ks", s.keystorePath,
        "--ks-pass", "pass:"+s.keystorePassword,
        "--key-pass", "pass:"+s.keyPassword,
        "--out", outputAPK,
        inputAPK,
    )
    return cmd.Run()
}
```

### Play Store Upload Key

```go
// pkg/signer/playstore.go
package signer

// For Play App Signing, upload key is used instead of app signing key
type PlayStoreUploader struct {
    uploadKeyPath string
    serviceAccount string
}

func (p *PlayStoreUploader) UploadToInternalTrack(aabPath string) error {
    // Use Google Play Developer API
    // Upload AAB to internal testing track
}
```

## Windows Code Signing

### Requirements

| Requirement | Description |
|-------------|-------------|
| **Code Signing Certificate** | From DigiCert, Sectigo, etc. (~$100-500/year) |
| **Hardware Token** | Some CAs require USB token |
| **Timestamp Server** | For long-term validity |

### Signtool Workflow

```bash
# Sign with signtool (Windows SDK)
signtool sign /f certificate.pfx /p password \
  /tr http://timestamp.digicert.com /td sha256 /fd sha256 \
  MyApp.exe

# Verify signature
signtool verify /v /pa MyApp.exe
```

### Authenticode Requirements

For Windows SmartScreen to not show warnings:

1. **Sign with EV certificate** (immediate reputation)
2. **Build reputation over time** (standard certificate)
3. **Use timestamp server** (signature survives cert expiry)

### utm-dev Integration

```go
// pkg/signer/windows.go
//go:build windows
package signer

import (
    "os/exec"
    "fmt"
)

type WindowsSigner struct {
    pfxPath         string
    pfxPassword     string
    timestampServer string
}

func (s *WindowsSigner) Sign(exePath string) error {
    cmd := exec.Command("signtool", "sign",
        "/f", s.pfxPath,
        "/p", s.pfxPassword,
        "/tr", s.timestampServer,
        "/td", "sha256",
        "/fd", "sha256",
        exePath,
    )
    return cmd.Run()
}

func (s *WindowsSigner) Verify(exePath string) error {
    cmd := exec.Command("signtool", "verify", "/v", "/pa", exePath)
    return cmd.Run()
}
```

## Linux Signing (Optional)

### GPG Signing for Packages

```bash
# Sign .deb package
dpkg-sig -k YOUR_KEY_ID --sign builder app.deb

# Sign .rpm package
rpm --addsign app.rpm
```

### AppImage Signing

```bash
# Sign AppImage
/appimage-tool.sh -n -s ./signature.txt app.AppImage
```

## Secure Credential Storage

### macOS Keychain

```go
// pkg/secrets/keychain.go
package secrets

import (
    "github.com/keybase/go-keychain"
)

type KeychainStore struct {
    serviceName string
}

func (k *KeychainStore) Set(key, value string) error {
    item := keychain.NewItem()
    item.SetSecClass(keychain.SecClassGenericPassword)
    item.SetAccount(key)
    item.SetService(k.serviceName)
    item.SetData([]byte(value))
    item.SetAccessible(keychain.AccessibleWhenUnlocked)
    return keychain.AddItem(item)
}

func (k *KeychainStore) Get(key string) (string, error) {
    item := keychain.NewItem()
    item.SetSecClass(keychain.SecClassGenericPassword)
    item.SetAccount(key)
    item.SetService(k.serviceName)
    item.SetMatchSearchList(keychain.NewItem())
    item.SetReturnData(true)

    result, err := keychain.QueryItem(item)
    if err != nil {
        return "", err
    }
    return string(result[0].Data), nil
}
```

### Cross-Platform Secrets

```go
// pkg/secrets/secrets.go
package secrets

type SecretsStore interface {
    Set(key, value string) error
    Get(key string) (string, error)
    Delete(key string) error
}

func NewSecretsStore() (SecretsStore, error) {
    switch runtime.GOOS {
    case "darwin":
        return &KeychainStore{serviceName: "utm-dev"}, nil
    case "windows":
        return &WindowsCredentialStore{serviceName: "utm-dev"}, nil
    case "linux":
        return &LibsecretStore{serviceName: "utm-dev"}, nil
    default:
        return &FileStore{}, nil // Fallback (less secure)
    }
}
```

## CI/CD Signing Integration

### GitHub Actions

```yaml
# .github/workflows/release.yml
name: Release

on:
  release:
    types: [published]

jobs:
  build-macos:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4

      - name: Install signing certificate
        uses: apple-actions/import-codesign-certs@v2
        with:
          p12-file-base64: ${{ secrets.DEV_ID_CERT_P12 }}
          p12-password: ${{ secrets.DEV_ID_CERT_PASSWORD }}

      - name: Build and sign
        run: |
          utm-dev build macos ./cmd/myapp
          utm-dev sign macos ./myapp.app \
            --developer-id "${{ secrets.DEVELOPER_ID }}" \
            --notarize \
            --apple-id "${{ secrets.APPLE_ID }}" \
            --team-id "${{ secrets.TEAM_ID }}"

      - name: Upload to release
        uses: softprops/action-gh-release@v1
        with:
          files: myapp.dmg

  build-android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Decode keystore
        run: echo ${{ secrets.KEYSTORE_BASE64 }} | base64 --decode > release.keystore

      - name: Build and sign
        run: |
          utm-dev build android ./cmd/myapp \
            --sign \
            --keystore release.keystore \
            --keystore-password ${{ secrets.KEYSTORE_PASSWORD }} \
            --key-alias ${{ secrets.KEY_ALIAS }} \
            --key-password ${{ secrets.KEY_PASSWORD }}
```

## Summary

Production signing essentials:

1. **macOS** - Developer ID, codesign, notarization, staple
2. **iOS** - Provisioning profiles, embedded profiles, entitlements
3. **Android** - Keystore, apksigner, zipalign, Play signing
4. **Windows** - Authenticode, signtool, timestamp server
5. **Secrets** - Keychain (macOS), Credential Manager (Win), libsecret (Linux)
6. **CI/CD** - GitHub Actions with secure credential handling

---

*Related: `deployment-cicd-exploration.md`, `security-exploration.md`*
