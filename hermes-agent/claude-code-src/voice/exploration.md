# Voice Module — Deep-Dive Exploration

**Module:** `voice/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/voice/`  
**Files:** 1 TypeScript file  
**Created:** 2026-04-07

---

## 1. Module Overview

The `voice/` module implements **voice mode enablement logic** — feature gating and authentication checks for Claude Code's voice interaction feature. Voice mode enables real-time voice conversations with Claude via the `/voice` command, using claude.ai's voice_stream endpoint.

### Core Responsibilities

1. **Feature Gating** — GrowthBook kill-switch:
   - `VOICE_MODE` feature flag check
   - Emergency kill-switch via `tengu_amber_quartz_disabled`
   - Default "not killed" for fresh installs

2. **Authentication Check** — OAuth token validation:
   - Anthropic OAuth required (API keys not supported)
   - Memoized token retrieval from keychain
   - First call spawns security framework on macOS (~20-50ms)

3. **Runtime Enablement** — Combined check:
   - Both auth AND feature gate must pass
   - Used by `/voice` command registration
   - Config UI visibility control

### Key Design Patterns

- **Dual-Gate Pattern**: Feature flag + auth check
- **Memoized Auth**: Keychain access cached, cleared on refresh
- **Positive Gate Logic**: Positive ternary for proper DCE

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `voiceModeEnabled.ts` | ~55 | Voice mode enablement checks |

**Total:** ~55 lines

---

## 3. Key Exports

```typescript
// GrowthBook kill-switch check
export function isVoiceGrowthBookEnabled(): boolean

// Auth-only check
export function hasVoiceAuth(): boolean

// Full runtime check (auth + feature gate)
export function isVoiceModeEnabled(): boolean
```

---

## 4. Line-by-Line Analysis

### 4.1 GrowthBook Kill-Switch (`voiceModeEnabled.ts` lines 16-23)

```typescript
/**
 * Kill-switch check for voice mode. Returns true unless the
 * `tengu_amber_quartz_disabled` GrowthBook flag is flipped on (emergency
 * off). Default `false` means a missing/stale disk cache reads as "not
 * killed" — so fresh installs get voice working immediately without
 * waiting for GrowthBook init. Use this for deciding whether voice mode
 * should be *visible* (e.g., command registration, config UI).
 */
export function isVoiceGrowthBookEnabled(): boolean {
  // Positive ternary pattern — see docs/feature-gating.md.
  // Negative pattern (if (!feature(...)) return) does not eliminate
  // inline string literals from external builds.
  return feature('VOICE_MODE')
    ? !getFeatureValue_CACHED_MAY_BE_STALE('tengu_amber_quartz_disabled', false)
    : false
}
```

**Positive Ternary Pattern**: "Positive ternary pattern — see docs/feature-gating.md. Negative pattern (if (!feature(...)) return) does not eliminate inline string literals from external builds."

**Default Safe**: Missing/stale cache = "not killed" = voice works immediately.

**Usage**: For visibility decisions (command registration, config UI).

### 4.2 Auth Check (`voiceModeEnabled.ts` lines 32-44)

```typescript
/**
 * Auth-only check for voice mode. Returns true when the user has a valid
 * Anthropic OAuth token. Backed by the memoized getClaudeAIOAuthTokens —
 * first call spawns `security` on macOS (~20-50ms), subsequent calls are
 * cache hits. The memoize clears on token refresh (~once/hour), so one
 * cold spawn per refresh is expected. Cheap enough for usage-time checks.
 */
export function hasVoiceAuth(): boolean {
  // Voice mode requires Anthropic OAuth — it uses the voice_stream
  // endpoint on claude.ai which is not available with API keys,
  // Bedrock, Vertex, or Foundry.
  if (!isAnthropicAuthEnabled()) {
    return false
  }
  
  // isAnthropicAuthEnabled only checks the auth *provider*, not whether
  // a token exists. Without this check, the voice UI renders but
  // connectVoiceStream fails silently when the user isn't logged in.
  const tokens = getClaudeAIOAuthTokens()
  return Boolean(tokens?.accessToken)
}
```

**OAuth Required**: "Voice mode requires Anthropic OAuth — it uses the voice_stream endpoint on claude.ai which is not available with API keys, Bedrock, Vertex, or Foundry."

**Provider vs Token**: `isAnthropicAuthEnabled()` checks provider, not token existence.

**Performance**: First call ~20-50ms on macOS (keychain access), then cached.

### 4.3 Full Enablement Check (`voiceModeEnabled.ts` lines 52-54)

```typescript
/**
 * Full runtime check: auth + GrowthBook kill-switch. Callers: `/voice`
 * (voice.ts, voice/index.ts), ConfigTool, VoiceModeNotice — command-time
 * paths where a fresh keychain read is acceptable. For React render
 * paths use useVoiceEnabled() instead (memoizes the auth half).
 */
export function isVoiceModeEnabled(): boolean {
  return hasVoiceAuth() && isVoiceGrowthBookEnabled()
}
```

**Combined Check**: Both auth AND feature gate must pass.

**Usage**: Command-time paths (`/voice`, ConfigTool, VoiceModeNotice).

**React Hook Alternative**: For React render paths, use `useVoiceEnabled()` which memoizes auth check.

---

## 5. Integration Points

### 5.1 With `utils/auth.js`

| Component | Integration |
|-----------|-------------|
| `hasVoiceAuth()` | Uses `isAnthropicAuthEnabled()`, `getClaudeAIOAuthTokens()` |

### 5.2 With `services/analytics/growthbook.js`

| Component | Integration |
|-----------|-------------|
| `isVoiceGrowthBookEnabled()` | Uses `feature()`, `getFeatureValue_CACHED_MAY_BE_STALE()` |

### 5.3 With `hooks/useVoiceIntegration.js`

| Component | Integration |
|-----------|-------------|
| Voice hooks | Conditional import based on `feature('VOICE_MODE')` |

### 5.4 With `hooks/useVoiceEnabled.js`

| Component | Integration |
|-----------|-------------|
| React hook | Memoizes auth check for render paths |

---

## 6. Data Flow

### 6.1 Voice Mode Check Flow

```
/voice command invoked
    │
    ▼
isVoiceModeEnabled()
    │
    ├──► hasVoiceAuth()
    │    ├──► isAnthropicAuthEnabled() → provider check
    │    └──► getClaudeAIOAuthTokens() → token check
    │
    └──► isVoiceGrowthBookEnabled()
         ├──► feature('VOICE_MODE')
         └──► !getFeatureValue_CACHED_MAY_BE_STALE('tengu_amber_quartz_disabled', false)
    │
    ▼
Both true? → Voice mode enabled
```

### 6.2 Command Registration Flow

```
Startup
    │
    ▼
registerCommands()
    │
    ├──► isVoiceGrowthBookEnabled()?
    │    └──► Yes → Register /voice command
    │    └──► No → Skip registration
    │
    ▼
Command available in UI
```

---

## 7. Key Patterns

### 7.1 Positive Ternary for DCE

```typescript
// Correct (eliminates string literals from external builds)
return feature('VOICE_MODE')
  ? !getFeatureValue_CACHED_MAY_BE_STALE('tengu_amber_quartz_disabled', false)
  : false

// Incorrect (does NOT eliminate string literals)
if (!feature('VOICE_MODE')) return false
return !getFeatureValue_CACHED_MAY_BE_STALE(...)
```

**Why**: Positive ternary pattern ensures proper dead code elimination in external builds.

### 7.2 Dual-Gate Pattern

```
Feature Gate (GrowthBook)
    │
    ▼
    AND → Voice enabled
    ▲
Auth Check (OAuth token)
```

**Purpose**: Emergency kill-switch + user authorization.

### 7.3 Memoized Keychain Access

```typescript
// getClaudeAIOAuthTokens() is memoized
// First call: ~20-50ms (macOS security framework)
// Subsequent: cache hit (<1ms)
// Token refresh: cache clears, one cold spawn
```

**Performance**: Acceptable for command-time checks, memoized for render paths.

---

## 8. Environment Variables

| Variable | Purpose | Values |
|----------|---------|--------|
| `VOICE_MODE` | Feature gate (via GrowthBook) | `true`/`false` |
| `tengu_amber_quartz_disabled` | Emergency kill-switch | `true`/`false` |

---

## 9. Feature Gates

| Feature | Purpose |
|---------|---------|
| `VOICE_MODE` | Master gate for voice mode |
| `tengu_amber_quartz_disabled` | Emergency kill-switch (inverts enablement) |

---

## 10. Summary

The `voice/` module provides **voice mode enablement logic**:

1. **Feature Gating** — GrowthBook kill-switch with safe defaults
2. **Authentication** — Anthropic OAuth token validation
3. **Combined Check** — Both gates must pass for voice mode

**Key Design Decisions**:
- **Positive ternary** for proper dead code elimination
- **Dual-gate pattern** for emergency off + auth
- **Memoized keychain** access for performance
- **OAuth required** (API keys not supported)

**Usage**:
- `isVoiceGrowthBookEnabled()` — Visibility decisions
- `hasVoiceAuth()` — Auth-only checks
- `isVoiceModeEnabled()` — Full runtime check

---

**Last Updated:** 2026-04-07  
**Status:** Complete — 1 of 1 files analyzed
