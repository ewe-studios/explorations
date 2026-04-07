# Buddy Module — Deep-Dive Exploration

**Module:** `buddy/`  
**Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/buddy/`  
**Files:** 6 TypeScript/TSX files  
**Created:** 2026-04-07

---

## 1. Module Overview

The `buddy/` module implements Claude Code's **companion pet system** — a gamified UI feature where users can hatch and interact with a small ASCII animal companion that sits beside the input box and occasionally comments in speech bubbles. This is a lighthearted engagement feature with randomized rarity, species, and personality generation.

### Core Responsibilities

1. **Companion Generation** — Deterministic randomized companion creation:
   - Species selection from 18 species (duck, goose, blob, cat, dragon, etc.)
   - Rarity rolling (common → legendary with weighted probabilities)
   - Stat generation (DEBUGGING, PATIENCE, CHAOS, WISDOM, SNARK)
   - Visual traits (eyes, hats)

2. **Sprite Rendering** — ASCII art animation frames:
   - 3 animation frames per species for idle fidget animation
   - Hat overlay rendering
   - Eye character substitution

3. **System Prompt Integration** — Companion intro attachment:
   - Injects companion introduction into system prompt
   - Deduplicates across messages
   - Feature-gated via `BUDDY` flag

4. **Teaser Notifications** — Launch promotion:
   - Rainbow `/buddy` teaser notification on startup
   - Time-gated (April 1-7, 2026 teaser window)
   - Only shown if no companion hatched yet

### Key Design Patterns

- **Deterministic RNG**: Companion bones derived from hash(userId) — same user always gets same companion
- **Bones/Soul Separation**: Bones (visual traits) regenerated from hash; Soul (name/personality) persisted in config
- **Mulberry32 PRNG**: Tiny seeded random number generator for reproducible rolls
- **Feature Gating**: All buddy features gated behind `feature('BUDDY')` checks

---

## 2. File Inventory

| File | Lines | Description |
|------|-------|-------------|
| `CompanionSprite.tsx` | ~37 | System prompt attachment injector |
| `companion.ts` | ~134 | Companion rolling/generation logic |
| `prompt.ts` | ~36 | System prompt integration helpers |
| `sprites.ts` | ~515+ | ASCII sprite frames and rendering |
| `types.ts` | ~149 | Type definitions for companions |
| `useBuddyNotification.tsx` | ~98 | Rainbow teaser notification hook |

**Total:** ~969 lines across 6 files

---

## 3. Key Exports

### Types (`types.ts`)

```typescript
// Rarity tiers with drop rates
export const RARITIES = [
  'common', 'uncommon', 'rare', 'epic', 'legendary',
] as const
export type Rarity = (typeof RARITIES)[number]

// 18 species available
export const SPECIES = [
  duck, goose, blob, cat, dragon, octopus, owl, penguin,
  turtle, snail, ghost, axolotl, capybara, cactus, robot,
  rabbit, mushroom, chonk,
] as const
export type Species = (typeof SPECIES)[number]

// Eye styles
export const EYES = ['·', '✦', '×', '◉', '@', '°'] as const
export type Eye = (typeof EYES)[number]

// Hat accessories
export const HATS = [
  'none', 'crown', 'tophat', 'propeller', 'halo',
  'wizard', 'beanie', 'tinyduck',
] as const
export type Hat = (typeof HATS)[number]

// Personality stats
export const STAT_NAMES = [
  'DEBUGGING', 'PATIENCE', 'CHAOS', 'WISDOM', 'SNARK',
] as const
export type StatName = (typeof STAT_NAMES)[number]

// Deterministic traits — derived from hash(userId)
export type CompanionBones = {
  rarity: Rarity
  species: Species
  eye: Eye
  hat: Hat
  shiny: boolean  // 1% chance
  stats: Record<StatName, number>
}

// Model-generated soul — stored in config after first hatch
export type CompanionSoul = {
  name: string
  personality: string
}

// Full companion type (bones + soul + metadata)
export type Companion = CompanionBones & CompanionSoul & {
  hatchedAt: number
}

// What actually persists — bones regenerated on read
export type StoredCompanion = CompanionSoul & { hatchedAt: number }

// Rarity drop weights (total = 100)
export const RARITY_WEIGHTS = {
  common: 60,
  uncommon: 25,
  rare: 10,
  epic: 4,
  legendary: 1,
} as const

// Rarity display stars
export const RARITY_STARS = {
  common: '★',
  uncommon: '★★',
  rare: '★★★',
  epic: '★★★★',
  legendary: '★★★★★',
} as const

// Rarity color mappings (for theme)
export const RARITY_COLORS = {
  common: 'inactive',
  uncommon: 'success',
  rare: 'permission',
  epic: 'autoAccept',
  legendary: 'warning',
} as const
```

### Core Functions (`companion.ts`)

```typescript
// Deterministic companion roll from userId
export function roll(userId: string): Roll {
  const key = userId + SALT
  if (rollCache?.key === key) return rollCache.value
  const value = rollFrom(mulberry32(hashString(key)))
  rollCache = { key, value }
  return value
}

// Roll with custom seed (for preview/testing)
export function rollWithSeed(seed: string): Roll

// Get current user's companion (regenerates bones from userId)
export function getCompanion(): Companion | undefined

// Get companion user ID from config
export function companionUserId(): string
```

### Sprite Rendering (`sprites.ts`)

```typescript
// Render ASCII sprite for a companion (3 frames for animation)
export function renderSprite(bones: CompanionBones, frame = 0): string[]

// Get number of animation frames for a species
export function spriteFrameCount(species: Species): number

// Render face emoji-style representation
export function renderFace(bones: CompanionBones): string
```

### System Prompt Integration (`prompt.ts`)

```typescript
// Generate companion intro text for system prompt
export function companionIntroText(name: string, species: string): string

// Get attachment for companion intro (deduplicated)
export function getCompanionIntroAttachment(
  messages: Message[] | undefined,
): Attachment[]
```

### Notification Hook (`useBuddyNotification.tsx`)

```typescript
// Check if in teaser window (April 1-7, 2026)
export function isBuddyTeaserWindow(): boolean

// Check if buddy is live (April 2026+)
export function isBuddyLive(): boolean

// Hook to show rainbow /buddy teaser notification
export function useBuddyNotification(): void

// Find /buddy command trigger positions in text
export function findBuddyTriggerPositions(
  text: string,
): Array<{ start: number; end: number }>
```

---

## 4. Line-by-Line Analysis

### 4.1 Companion Rolling (`companion.ts`)

**Mulberry32 PRNG (lines 15-25):**

```typescript
// Mulberry32 — tiny seeded PRNG, good enough for picking ducks
function mulberry32(seed: number): () => number {
  let a = seed >>> 0
  return function () {
    a |= 0
    a = (a + 0x6d2b79f5) | 0
    let t = Math.imul(a ^ (a >>> 15), 1 | a)
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}
```

**Why Mulberry32**: 32-bit state, excellent distribution, tiny code footprint. Perfect for generating companion traits deterministically from a seed.

**String Hashing (lines 27-37):**

```typescript
function hashString(s: string): number {
  if (typeof Bun !== 'undefined') {
    return Number(BigInt(Bun.hash(s)) & 0xffffffffn)
  }
  // Fallback: FNV-1a hash
  let h = 2166136261
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i)
    h = Math.imul(h, 16777619)
  }
  return h >>> 0
}
```

**Design Note**: Uses Bun's native hash when available (faster), falls back to FNV-1a for compatibility.

**Rarity Rolling (lines 43-51):**

```typescript
function rollRarity(rng: () => number): Rarity {
  const total = Object.values(RARITY_WEIGHTS).reduce((a, b) => a + b, 0)
  let roll = rng() * total
  for (const rarity of RARITIES) {
    roll -= RARITY_WEIGHTS[rarity]
    if (roll < 0) return rarity
  }
  return 'common'
}
```

**Stat Generation (lines 62-82):**

```typescript
// One peak stat, one dump stat, rest scattered. Rarity bumps the floor.
function rollStats(
  rng: () => number,
  rarity: Rarity,
): Record<StatName, number> {
  const floor = RARITY_FLOOR[rarity]
  const peak = pick(rng, STAT_NAMES)
  let dump = pick(rng, STAT_NAMES)
  while (dump === peak) dump = pick(rng, STAT_NAMES)

  const stats = {} as Record<StatName, number>
  for (const name of STAT_NAMES) {
    if (name === peak) {
      stats[name] = Math.min(100, floor + 50 + Math.floor(rng() * 30))
    } else if (name === dump) {
      stats[name] = Math.max(1, floor - 10 + Math.floor(rng() * 15))
    } else {
      stats[name] = floor + Math.floor(rng() * 40)
    }
  }
  return stats
}
```

**Key Insight**: Each companion has one dominant stat (peak), one weak stat (dump), creating personality variety.

**Caching (lines 106-113):**

```typescript
// Called from three hot paths (500ms sprite tick, per-keystroke PromptInput,
// per-turn observer) with the same userId → cache the deterministic result.
let rollCache: { key: string; value: Roll } | undefined
export function roll(userId: string): Roll {
  const key = userId + SALT
  if (rollCache?.key === key) return rollCache.value
  const value = rollFrom(mulberry32(hashString(key)))
  rollCache = { key, value }
  return value
}
```

**Why Cache**: Called 3x per session tick + per keystroke — caching avoids redundant hashing.

**Bones/Soul Separation (lines 124-133):**

```typescript
// Regenerate bones from userId, merge with stored soul. Bones never persist
// so species renames and SPECIES-array edits can't break stored companions,
// and editing config.companion can't fake a rarity.
export function getCompanion(): Companion | undefined {
  const stored = getGlobalConfig().companion
  if (!stored) return undefined
  const { bones } = roll(companionUserId())
  // bones last so stale bones fields in old-format configs get overridden
  return { ...stored, ...bones }
}
```

**Security Note**: Bones are NEVER stored — they're regenerated from userId hash. This prevents:
1. Users editing config to get legendary rarity
2. Breaking changes when SPECIES array is modified
3. Version skew between client versions

### 4.2 Sprite Rendering (`sprites.ts`)

**Species ASCII Art Structure:**

```typescript
// Each sprite is 5 lines tall, 12 wide (after {E}→1char substitution).
// Multiple frames per species for idle fidget animation.
// Line 0 is the hat slot — must be blank in frames 0-1; frame 2 may use it.
const BODIES: Record<Species, string[][]> = {
  [duck]: [
    ['            ', '    __      ', '  <({E} )___  ', '   (  ._>   ', '    `--´    '],
    ['            ', '    __      ', '  <({E} )___  ', '   (  ._>   ', '    `--´~   '],  // Tail wag
    ['            ', '    __      ', '  <({E} )___  ', '   (  .__>  ', '    `--´    '],  // Blink
  ],
  // ... 17 more species
}
```

**{E} Placeholder**: Eye character substituted at render time for customization.

**Sprite Rendering (lines 454-469):**

```typescript
export function renderSprite(bones: CompanionBones, frame = 0): string[] {
  const frames = BODIES[bones.species]
  const body = frames[frame % frames.length]!.map(line =>
    line.replaceAll('{E}', bones.eye),
  )
  const lines = [...body]
  // Only replace with hat if line 0 is empty (some fidget frames use it for smoke etc)
  if (bones.hat !== 'none' && !lines[0]!.trim()) {
    lines[0] = HAT_LINES[bones.hat]
  }
  // Drop blank hat slot when safe (all frames have blank line 0)
  if (!lines[0]!.trim() && frames.every(f => !f[0]!.trim())) lines.shift()
  return lines
}
```

**Hat Lines (lines 443-452):**

```typescript
const HAT_LINES: Record<Hat, string> = {
  none: '',
  crown: '   \\^^^/    ',
  tophat: '   [___]    ',
  propeller: '    -+-     ',
  halo: '   (   )    ',
  wizard: '    /^\\     ',
  beanie: '   (___)    ',
  tinyduck: '    ,>      ',
}
```

### 4.3 System Prompt Integration (`prompt.ts`)

**Companion Intro Text (lines 7-13):**

```typescript
export function companionIntroText(name: string, species: string): string {
  return `# Companion

A small ${species} named ${name} sits beside the user's input box and occasionally comments in a speech bubble. You're not ${name} — it's a separate watcher.

When the user addresses ${name} directly (by name), its bubble will answer. Your job in that moment is to stay out of the way: respond in ONE line or less, or just answer any part of the message meant for you. Don't explain that you're not ${name} — they know. Don't narrate what ${name} might say — the bubble handles that.`
}
```

**Purpose**: Instructs Claude to recognize the companion as a separate entity and defer to it when the user addresses it by name.

**Intro Attachment (lines 15-36):**

```typescript
export function getCompanionIntroAttachment(
  messages: Message[] | undefined,
): Attachment[] {
  if (!feature('BUDDY')) return []
  const companion = getCompanion()
  if (!companion || getGlobalConfig().companionMuted) return []

  // Skip if already announced for this companion.
  for (const msg of messages ?? []) {
    if (msg.type !== 'attachment') continue
    if (msg.attachment.type !== 'companion_intro') continue
    if (msg.attachment.name === companion.name) return []
  }

  return [
    {
      type: 'companion_intro',
      name: companion.name,
      species: companion.species,
    },
  ]
}
```

**Deduplication**: Scans existing messages to avoid re-announcing the same companion.

### 4.4 Teaser Notification (`useBuddyNotification.tsx`)

**Teaser Window Detection (lines 12-21):**

```typescript
// Local date, not UTC — 24h rolling wave across timezones. Sustained Twitter
// buzz instead of a single UTC-midnight spike, gentler on soul-gen load.
// Teaser window: April 1-7, 2026 only. Command stays live forever after.
export function isBuddyTeaserWindow(): boolean {
  if ("external" === 'ant') return true  // Always true for internal dev
  const d = new Date()
  return d.getFullYear() === 2026 && d.getMonth() === 3 && d.getDate() <= 7
}

export function isBuddyLive(): boolean {
  if ("external" === 'ant') return true
  const d = new Date()
  return d.getFullYear() > 2026 || d.getFullYear() === 2026 && d.getMonth() >= 3
}
```

**Timezone Handling**: Uses local date, not UTC — creates a 24-hour rolling wave of teaser visibility across timezones.

**Rainbow Text Component (lines 22-42):**

```typescript
function RainbowText({ text }: { text: string }): React.ReactElement {
  return (
    <>
      {[...text].map((ch, i) => (
        <Text key={i} color={getRainbowColor(i)}>
          {ch}
        </Text>
      ))}
    </>
  )
}
```

**Notification Hook (lines 43-78):**

```typescript
export function useBuddyNotification(): void {
  const { addNotification, removeNotification } = useNotifications()

  useEffect(() => {
    if (!feature('BUDDY')) return
    const config = getGlobalConfig()
    if (config.companion || !isBuddyTeaserWindow()) return
    
    addNotification({
      key: 'buddy-teaser',
      jsx: <RainbowText text="/buddy" />,
      priority: 'immediate',
      timeoutMs: 15_000,
    })
    return () => removeNotification('buddy-teaser')
  }, [addNotification, removeNotification])
}
```

**Trigger Position Finder (lines 79-97):**

```typescript
export function findBuddyTriggerPositions(text: string): Array<{
  start: number;
  end: number;
}> {
  if (!feature('BUDDY')) return []
  const triggers: Array<{ start: number; end: number }> = []
  const re = /\/buddy\b/g
  let m: RegExpExecArray | null
  while ((m = re.exec(text)) !== null) {
    triggers.push({
      start: m.index,
      end: m.index + m[0].length
    })
  }
  return triggers
}
```

---

## 5. Species Roster

| Species | Code Points | Description |
|---------|-------------|-------------|
| duck | 0x64,0x75,0x63,0x6b | Classic duck with bill |
| goose | 0x67,0x6f,0x6f,0x73,0x65 | Honk |
| blob | 0x62,0x6c,0x6f,0x62 | Amorphous creature |
| cat | 0x63,0x61,0x74 | Cat with ears |
| dragon | 0x64,0x72,0x61,0x67,0x6f,0x6e | Winged dragon |
| octopus | 0x6f,0x63,0x74,0x6f,0x70,0x75,0x73 | Eight tentacles |
| owl | 0x6f,0x77,0x6c | Wise owl with ear tufts |
| penguin | 0x70,0x65,0x6e,0x67,0x75,0x69,0x6e | Waddling penguin |
| turtle | 0x74,0x75,0x72,0x74,0x6c,0x65 | Shell-backed turtle |
| snail | 0x73,0x6e,0x61,0x69,0x6c | Shelled snail |
| ghost | 0x67,0x68,0x6f,0x73,0x74 | Spooky ghost |
| axolotl | 0x61,0x78,0x6f,0x6c,0x6f,0x74,0x6c | Axolotl with gills |
| capybara | 0x63,0x61,0x70,0x79,0x62,0x61,0x72,0x61 | Chill capybara |
| cactus | 0x63,0x61,0x63,0x74,0x75,0x73 | Prickly cactus |
| robot | 0x72,0x6f,0x62,0x6f,0x74 | Box robot |
| rabbit | 0x72,0x61,0x62,0x62,0x69,0x74 | Long-eared rabbit |
| mushroom | 0x6d,0x75,0x73,0x68,0x72,0x6f,0x6f,0x6d | Fungus friend |
| chonk | 0x63,0x68,0x6f,0x6e,0x6b | Chonky cat variant |

**Note**: Species names are encoded via `String.fromCharCode()` to avoid triggering string-match canary checks in build output.

---

## 6. Integration Points

### 6.1 With `context/notifications.js`

| Buddy Component | Integration |
|-----------------|-------------|
| `useBuddyNotification.tsx` | Uses `useNotifications()` hook to add/remove teaser |

### 6.2 With `utils/config.js`

| Buddy Component | Integration |
|-----------------|-------------|
| `companion.ts` | Reads/writes `config.companion` for soul persistence |
| `prompt.ts` | Checks `config.companionMuted` for mute state |
| `companion.ts` | Gets `config.oauthAccount.accountUuid` for userId |

### 6.3 With `utils/theming.js`

| Buddy Component | Integration |
|-----------------|-------------|
| `types.ts` | `RARITY_COLORS` maps to theme color keys |

### 6.4 With Feature Flags

| Feature | Gate | Purpose |
|---------|------|---------|
| `BUDDY` | `feature('BUDDY')` | Master gate for all buddy features |
| External dev | `"external" === 'ant'` | Bypass time gates for Anthropic devs |

---

## 7. Data Flow

### 7.1 Companion Hatch Flow

```
User runs /buddy command
         │
         ▼
  rollWithSeed(seed)
         │
         ▼
  mulberry32(hashString(seed))
         │
         ├──► rollRarity() ──► species, eye, hat, shiny, stats
         │
         ▼
  Model generates name + personality (soul)
         │
         ▼
  StoredCompanion persisted to config
         │
         ▼
  getCompanion() merges stored soul + regenerated bones
```

### 7.2 Sprite Animation Loop

```
500ms interval tick
         │
         ▼
  frame = (frame + 1) % spriteFrameCount(species)
         │
         ▼
  renderSprite(bones, frame)
         │
         ├──► Substitute {E} → eye char
         ├──► Add hat line if applicable
         └──► Drop blank hat line if safe
         │
         ▼
  Render to terminal UI
```

### 7.3 System Prompt Injection

```
Session start
         │
         ▼
  getCompanionIntroAttachment(messages)
         │
         ├──► Check feature('BUDDY')
         ├──► Check companion exists
         ├──► Check not muted
         └──► Deduplicate by scanning messages
         │
         ▼
  Attachment added to user context
         │
         ▼
  companionIntroText() injected as system instruction
```

---

## 8. Key Patterns

### 8.1 Deterministic Generation

```typescript
userId + SALT → hashString → mulberry32 → rollFrom → CompanionBones
```

Same userId always produces identical companion. Enables:
- Consistent experience across sessions
- No database needed — regeneration is cheap
- Cheat prevention — can't edit config for better rarity

### 8.2 Bones/Soul Separation

| Bones (Regenerated) | Soul (Persisted) |
|---------------------|------------------|
| Species | Name |
| Rarity | Personality |
| Eye style | — |
| Hat | — |
| Stats | — |
| Shiny flag | — |

**Why**: Bones can change (species renamed, new hats added) without breaking stored data. Soul is user-specific creative content.

### 8.3 Animation Frame Design

```
Frame 0: Neutral pose
Frame 1: Minor variation (tail wag, ear twitch)
Frame 2: Alternate expression (blink, mouth open)
```

Hat only appears on Frame 2 for some species (smoke, antenna effects use line 0).

---

## 9. Error Handling

### 9.1 Missing Companion

```typescript
export function getCompanion(): Companion | undefined {
  const stored = getGlobalConfig().companion
  if (!stored) return undefined  // Not hatched yet
  // ...
}
```

Callers handle `undefined` — no companion features shown.

### 9.2 Muted State

```typescript
if (!companion || getGlobalConfig().companionMuted) return []
```

User can mute companion via `/buddy mute` — suppresses intro attachment.

### 9.3 Feature Gate Fallback

```typescript
if (!feature('BUDDY')) return []
```

If feature disabled, all buddy functions become no-ops.

---

## 10. Testing Considerations

### 10.1 Deterministic Testing

```typescript
// Test: Same seed produces same companion
const roll1 = rollWithSeed('test-seed')
const roll2 = rollWithSeed('test-seed')
assert.deepStrictEqual(roll1, roll2)
```

### 10.2 Rarity Distribution

```typescript
// Test: Rarity distribution matches weights
const rolls = Array(10000).fill(null).map(() => rollRarity(mulberry32(Math.random())))
const counts = countBy(rolls)
assert.approximately(counts.common / 10000, 0.60, 0.05)  // 60% ± 5%
```

### 10.3 Sprite Rendering

```typescript
// Test: Sprite has correct dimensions
const sprite = renderSprite({ species: 'duck', eye: '·', hat: 'none' } as any)
assert.strictEqual(sprite.length, 5)  // 5 lines
assert.ok(sprite.every(line => line.length <= 12))
```

---

## 11. Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLAUDE_CODE_FEATURE_BUDDY` | Feature gate override | — |

---

## 12. Telemetry Events

| Event | Location | Fields |
|-------|----------|--------|
| `tengu_keybinding_fallback_used` | (not in buddy) | — |

*Note: Buddy-specific telemetry not found in scanned files.*

---

## 13. Summary

The `buddy/` module is a **companion pet system** that adds a lighthearted gamification layer to Claude Code:

1. **Deterministic Generation** — Same user always gets same companion from hash(userId)
2. **Rich Customization** — 18 species × 5 rarities × 6 eyes × 8 hats × 5 stats
3. **ASCII Animation** — 3-frame idle animation per species
4. **System Integration** — Companion intro injected into system prompt
5. **Teaser Campaign** — Rainbow notification during launch week (April 1-7, 2026)

The module demonstrates clever engineering:
- **Bones/Soul separation** prevents data corruption from version changes
- **Caching** avoids redundant hashing on hot paths
- **Character encoding** bypasses build-time canary checks
- **Local time gating** creates rolling 24-hour teaser window

---

**Last Updated:** 2026-04-07  
**Status:** Complete — all 6 files analyzed
