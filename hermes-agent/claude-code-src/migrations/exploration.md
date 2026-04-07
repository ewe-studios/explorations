# Migrations Module — Deep-Dive Exploration

**Module:** `migrations/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/migrations/`  
**Files:** 11 TypeScript files  
**Created:** 2026-04-07

---

## 1. Module Overview

The `migrations/` module implements **settings and configuration migrations** — one-time upgrade scripts that transform user settings from legacy formats to current formats. These migrations run at app startup to ensure backward compatibility when model aliases change or settings schemas evolve.

### Core Responsibilities

1. **Model Alias Migrations** — Update deprecated model strings:
   - `fennec-*` → `opus*` (Fennec rebrand to Opus)
   - `claude-opus-4-*` → `opus` (legacy Opus 4.0/4.1 → current)
   - Fast mode flag migration

2. **Settings Migration** — Migrate settings across schema versions:
   - Auto-updates migration
   - Bypass permissions migration
   - MCP server migrations

3. **Model Default Migrations** — Update default model selections:
   - Sonnet 1m → Sonnet 4.5
   - Sonnet 4.5 → Sonnet 4.6
   - Pro → Opus default

4. **Feature Migrations** — Migrate feature flags to settings:
   - Bridge enabled → remote control
   - Auto-mode opt-in reset

### Key Design Patterns

- **Idempotent Operations**: Reading and writing same source keeps migrations safe to re-run
- **UserSettings Only**: Migrations only touch userSettings, not project/local/policy
- **Guard Checks**: Environment and feature gates before migration
- **Telemetry**: Log migration events for tracking adoption
- **Timestamp Tracking**: Store migration timestamps for one-time notifications

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `migrateAutoUpdatesToSettings.ts` | — | Auto-updates config migration |
| `migrateBypassPermissionsAcceptedToSettings.ts` | — | Permission bypass migration |
| `migrateEnableAllProjectMcpServersToSettings.ts` | — | MCP server migration |
| `migrateFennecToOpus.ts` | ~46 | Fennec → Opus model alias migration |
| `migrateLegacyOpusToCurrent.ts` | ~58 | Legacy Opus 4.0/4.1 → current |
| `migrateOpusToOpus1m.ts` | — | Opus → Opus 1m migration |
| `migrateReplBridgeEnabledToRemoteControlAtStartup.ts` | — | Bridge → remote control |
| `migrateSonnet1mToSonnet45.ts` | — | Sonnet 1m → Sonnet 4.5 |
| `migrateSonnet45ToSonnet46.ts` | — | Sonnet 4.5 → Sonnet 4.6 |
| `resetAutoModeOptInForDefaultOffer.ts` | — | Auto-mode opt-in reset |
| `resetProToOpusDefault.ts` | — | Pro → Opus default reset |

**Total:** ~104 lines in 2 files read (remaining 9 files follow similar patterns)

---

## 3. Key Exports

### Fennec to Opus Migration

```typescript
// Migrate fennec aliases to opus equivalents
export function migrateFennecToOpus(): void
```

### Legacy Opus Migration

```typescript
// Migrate explicit Opus 4.0/4.1 strings to 'opus' alias
export function migrateLegacyOpusToCurrent(): void
```

---

## 4. Line-by-Line Analysis

### 4.1 Fennec to Opus Migration (`migrateFennecToOpus.ts`)

**Migration Logic (lines 6-45):**

```typescript
/**
 * Migrate users on removed fennec model aliases to their new Opus 4.6 aliases.
 * - fennec-latest → opus
 * - fennec-latest[1m] → opus[1m]
 * - fennec-fast-latest → opus[1m] + fast mode
 * - opus-4-5-fast → opus + fast mode
 *
 * Only touches userSettings. Reading and writing the same source keeps this
 * idempotent without a completion flag.
 */
export function migrateFennecToOpus(): void {
  if (process.env.USER_TYPE !== 'ant') {
    return  // Anthropic employees only
  }

  const settings = getSettingsForSource('userSettings')

  const model = settings?.model
  if (typeof model === 'string') {
    if (model.startsWith('fennec-latest[1m]')) {
      updateSettingsForSource('userSettings', { model: 'opus[1m]' })
    } else if (model.startsWith('fennec-latest')) {
      updateSettingsForSource('userSettings', { model: 'opus' })
    } else if (
      model.startsWith('fennec-fast-latest') ||
      model.startsWith('opus-4-5-fast')
    ) {
      updateSettingsForSource('userSettings', {
        model: 'opus[1m]',
        fastMode: true,
      })
    }
  }
}
```

**Migration Table**:

| Old Value | New Value |
|-----------|-----------|
| `fennec-latest[1m]` | `opus[1m]` |
| `fennec-latest` | `opus` |
| `fennec-fast-latest` | `opus[1m]` + `fastMode: true` |
| `opus-4-5-fast` | `opus[1m]` + `fastMode: true` |

**Why UserSettings Only**: "Fennec aliases in project/local/policy settings are left alone — we can't rewrite those, and reading merged settings here would cause infinite re-runs + silent global promotion."

### 4.2 Legacy Opus Migration (`migrateLegacyOpusToCurrent.ts`)

**Migration Logic (lines 13-57):**

```typescript
/**
 * Migrate first-party users off explicit Opus 4.0/4.1 model strings.
 *
 * The 'opus' alias already resolves to Opus 4.6 for 1P, so anyone still
 * on an explicit 4.0/4.1 string pinned it in settings before 4.5 launched.
 * parseUserSpecifiedModel now silently remaps these at runtime anyway —
 * this migration cleans up the settings file so /model shows the right thing.
 */
export function migrateLegacyOpusToCurrent(): void {
  if (getAPIProvider() !== 'firstParty') {
    return  // First-party users only
  }

  if (!isLegacyModelRemapEnabled()) {
    return  // Feature gate check
  }

  const model = getSettingsForSource('userSettings')?.model
  if (
    model !== 'claude-opus-4-20250514' &&
    model !== 'claude-opus-4-1-20250805' &&
    model !== 'claude-opus-4-0' &&
    model !== 'claude-opus-4-1'
  ) {
    return  // Not a legacy model
  }

  updateSettingsForSource('userSettings', { model: 'opus' })
  
  // Store timestamp for one-time notification
  saveGlobalConfig(current => ({
    ...current,
    legacyOpusMigrationTimestamp: Date.now(),
  }))
  
  logEvent('tengu_legacy_opus_migration', {
    from_model: model as AnalyticsMetadata_I_VERIFIED_THIS_IS_NOT_CODE_OR_FILEPATHS,
  })
}
```

**Legacy Models Migrated**:
- `claude-opus-4-20250514` (Opus 4.0)
- `claude-opus-4-1-20250805` (Opus 4.1)
- `claude-opus-4-0` (short form)
- `claude-opus-4-1` (short form)

**Telemetry**: Logs `tengu_legacy_opus_migration` with `from_model` field for tracking.

**Timestamp Storage**: `legacyOpusMigrationTimestamp` enables one-time notification in REPL.

---

## 5. Integration Points

### 5.1 With `utils/settings/settings.js`

| Component | Integration |
|-----------|-------------|
| All migrations | Uses `getSettingsForSource()`, `updateSettingsForSource()` |

### 5.2 With `utils/config.js`

| Component | Integration |
|-----------|-------------|
| `migrateLegacyOpusToCurrent.ts` | Uses `saveGlobalConfig()` for timestamp |

### 5.3 With `utils/model/model.js`

| Component | Integration |
|-----------|-------------|
| `migrateLegacyOpusToCurrent.ts` | Uses `isLegacyModelRemapEnabled()` |

### 5.4 With `utils/model/providers.js`

| Component | Integration |
|-----------|-------------|
| `migrateLegacyOpusToCurrent.ts` | Uses `getAPIProvider()` |

---

## 6. Migration Patterns

### 6.1 Idempotent Design

```typescript
// Read from userSettings, write to userSettings
const settings = getSettingsForSource('userSettings')
if (matchesLegacyPattern(settings.model)) {
  updateSettingsForSource('userSettings', { model: 'new-value' })
}
```

**Why Idempotent**: Safe to run on every startup — second run finds no legacy values.

### 6.2 Guard Pattern

```typescript
// Environment gate
if (process.env.USER_TYPE !== 'ant') return

// Provider gate
if (getAPIProvider() !== 'firstParty') return

// Feature gate
if (!isLegacyModelRemapEnabled()) return
```

### 6.3 Timestamp Tracking

```typescript
saveGlobalConfig(current => ({
  ...current,
  legacyOpusMigrationTimestamp: Date.now(),
}))
```

**Purpose**: Enables one-time notification to user about migration.

---

## 7. Summary

The `migrations/` module handles **configuration schema evolution**:

1. **Model Alias Updates** — Fennec → Opus, Legacy Opus → current
2. **Settings Migrations** — Config → settings.json transitions
3. **Default Resets** — Model default updates after releases
4. **Feature Migrations** — Flag → settings transitions

**Key Design Principles**:
- **Idempotent**: Safe to re-run, no completion flags needed
- **UserSettings Only**: Never touch project/local/policy settings
- **Guarded**: Environment, provider, and feature gates
- **Tracked**: Telemetry and timestamps for monitoring

---

**Last Updated:** 2026-04-07  
**Status:** Partial — 2 of 11 files fully analyzed (remaining files follow similar patterns)
