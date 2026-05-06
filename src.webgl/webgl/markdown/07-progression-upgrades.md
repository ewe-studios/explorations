# Tiny Skies -- Progression and Upgrade Systems

The progression system awards XP for various activities (diamond collection, quest completion, combat, exploration). Accumulated XP leads to level-ups, which trigger upgrade card draws that permanently enhance vehicle performance, paintball capabilities, and economic multipliers.

Source: `tinyskies/client/src/game/UpgradeManager.ts` — upgrade card system
Source: `tinyskies/client/src/game/ProgressionManager.ts` — XP/levels/localStorage
Source: `tinyskies/client/src/ui/LevelUpCards.ts` — upgrade card UI
Source: `tinyskies/client/src/ui/VehicleUnlockPreview.ts` — 3D vehicle previews

## XP Sources and Values

| Activity | XP | Notes |
|----------|-----|-------|
| Diamond collection | 10 | Base, multiplied by upgrade cards |
| Package delivery | 50 | Per delivery, multiplied by delivery XP cards |
| Landmark selfie | 25 | Carpet only |
| Gremlin takedown | 15 | Per gremlin killed |
| Gremlin King takedown | Special | Triggers eternal flame reward |
| Race completion | Variable | Based on time |
| Firefly cluster | 20 | Per cluster |
| Lantern cluster | 20 | Per cluster |
| Rainbow collect | 20 | Per rainbow |
| Bird flock formation | 15 | Per flock |
| Volcano eruption | 20 | Per volcano |
| Jellyfish capture | 25 | Per jellyfish |
| Fish catch | 20 | Per fish |

### XP Combo System

Diamond collections can chain into combos:

```typescript
const DIAMOND_COMBO_WINDOW_MS = 900;    // 900ms window
const DIAMOND_COMBO_MAX_STEPS = 5;       // Max 5 combo steps
const DIAMOND_COMBO_XP_PER_STEP = 0.05;  // 5% bonus per step

function awardDiamondXP(): number {
  const now = Date.now();
  if (now - this.lastDiamondMs < DIAMOND_COMBO_WINDOW_MS) {
    this.comboStep = Math.min(this.comboStep + 1, DIAMOND_COMBO_MAX_STEPS);
  } else {
    this.comboStep = 1;
  }
  this.lastDiamondMs = now;

  const baseXP = 10;
  const comboBonus = 1 + (this.comboStep - 1) * DIAMOND_COMBO_XP_PER_STEP;
  // Step 5 = +25% XP total
  return baseXP * comboBonus * this.upgradeMultipliers.diamondXP;
}
```

The combo window is tuned so that a player flying through a diamond cluster at normal speed can realistically chain 3-5 diamonds.

## Progression Manager

```typescript
// ProgressionManager.ts
interface SavedPlayerWorldState {
  level: number;
  xp: number;
  xpToNext: number;
  upgrades: UpgradeState;
  vehicleUnlocks: { carpet: boolean; boat: boolean };
  gremlinTakedowns: number;
  worldEvents: string[];
}

class ProgressionManager {
  static save(state: SavedPlayerWorldState): void {
    localStorage.setItem("tinyskies_progress", JSON.stringify(state));
  }

  static load(): SavedPlayerWorldState | null {
    const raw = localStorage.getItem("tinyskies_progress");
    return raw ? JSON.parse(raw) : null;
  }

  static clearAll(): void {
    localStorage.removeItem("tinyskies_progress");
  }

  static awardXP(amount: number, source: XpSource): number {
    const state = this.load() ?? this.initialState();
    state.xp += amount * this.getMultiplier(source);

    while (state.xp >= state.xpToNext) {
      state.xp -= state.xpToNext;
      state.level++;
      state.xpToNext = this.calculateXPForNextLevel(state.level);
      this.triggerLevelUp(state);
    }

    this.save(state);
    return state.level;
  }
}
```

XP is stored in `localStorage` and persists across sessions. The `?clearSave` URL parameter in development mode clears this storage for testing.

## Level Curve

```typescript
function calculateXPForNextLevel(level: number): number {
  // Exponential curve: each level requires more XP
  return Math.floor(100 * Math.pow(1.15, level - 1));
}
```

| Level | XP to Next | Cumulative XP |
|-------|-----------|---------------|
| 1 | 100 | 0 |
| 2 | 115 | 100 |
| 3 | 132 | 215 |
| 4 | 152 | 347 |
| 5 | 175 | 500 |
| 6 | 201 | 674 |
| 7 | 231 | 875 |
| 8 | 266 | 1106 |

## Upgrade Manager

```typescript
// UpgradeManager.ts
interface UpgradeState {
  // Plane performance
  planeSpeedMult: number;      // Base 1.0, max ~1.3
  planeBoostMult: number;
  planeBankMult: number;
  planeGremlinHpMaxMult: number;
  // Carpet performance
  carpetSpeedMult: number;
  carpetHoverMult: number;
  carpetPortalCooldownMult: number;
  // Boat performance
  boatSpeedMult: number;
  boatFishingMult: number;
  // Paintball
  paintballSpeedMult: number;
  paintballRangeMult: number;
  paintballDoubleTap: boolean;
  // Hearts (HP)
  hearts: number;              // Max HP
  // Economy
  diamondXPMult: number;
  deliveryXPMult: number;
  comboTuningMult: number;
}
```

### Upgrade Card Pools

```typescript
// Upgrade pools: SHARED (2 cards), PLANE (8 cards), CARPET (3 active), BOAT (6 active)
const POOLS = {
  SHARED: [
    "heartUpgrade",    // +2 max HP
    "economyTuning",   // Better diamond/delivery XP
  ],
  PLANE: [
    "planeSpeed",      // +15% cruise speed
    "planeBoost",      // +10% boost multiplier
    "planeBank",       // +20% bank rate
    "planeHP",         // +30% max HP
    "paintballSpeed",  // +20% projectile speed
    "paintballRange",  // +25% projectile range
    "doubleTap",       // Unlock burst fire
    "comboTuning",     // Extend diamond combo window
  ],
  CARPET: [
    "carpetSpeed",     // +15% cruise speed
    "carpetHover",     // +30% hover altitude
    "portalCooldown",  // -20% portal cooldown
  ],
  BOAT: [
    "boatSpeed",       // +20% cruise speed
    "boatFishing",     // +50% fishing XP
    // ... more
  ],
  LEGACY_ECONOMY: [
    // 6 removed economy cards — apply() still runs for save compatibility
  ],
};
```

### Hybrid Card Draw

When a level-up occurs, the player draws 3 upgrade cards:

```typescript
function drawUpgrades(vehicle: Vehicle, hearts: number): UpgradeCard[] {
  // At least 1 from vehicle-specific pool
  const vehiclePool = POOLS[vehicle];
  const vehicleCard = shuffle(vehiclePool)[0];

  // Up to 2 from shared pool
  const sharedPool = POOLS.SHARED.filter(c => c !== vehicleCard);
  const sharedCards = shuffle(sharedPool).slice(0, 2);

  // Fisher-Yates shuffle, no duplicates
  const allCards = [vehicleCard, ...sharedCards];

  // Top up from leftover if needed
  while (allCards.length < 3) {
    const pool = vehicle === "plane" ? POOLS.PLANE : POOLS.CARPET;
    const remaining = pool.filter(c => !allCards.includes(c));
    allCards.push(shuffle(remaining)[0]);
  }

  return allCards;
}
```

The draw ensures **variety**: at least one card is always relevant to the player's current vehicle, with the remaining drawn from shared economy/HP cards.

### Legacy Card Support

```typescript
// Legacy cards are removed from active pools but still apply()
// when loading a saved state that has them
class LegacyUpgradeCard {
  apply(state: UpgradeState): void {
    // Run for save compatibility even if card removed
    // This ensures players who earned old cards don't lose progress
  }
}
```

## Upgrade Card UI

```typescript
// LevelUpCards.ts
class LevelUpCards {
  // Shows 3 cards with descriptions
  // Player clicks one to apply
  // Remaining cards discarded

  // Each card shows:
  // - Icon
  // - Name
  // - Description ("+15% plane speed")
  // - Before/after visualization
}
```

The UI presents 3 face-down cards that flip over one at a time with an animation. The player selects one card, which is applied immediately and saved to `localStorage`.

## Vehicle Unlocks

| Vehicle | Unlock Level | Preview |
|---------|-------------|---------|
| Plane | Level 1 (default) | N/A |
| Carpet | Level 2 | 3D canvas preview of carpet |
| Boat | Level 4 | 3D canvas preview of boat |

Unlock celebrations show a modal with a 3D preview canvas rotating the new vehicle, along with confetti and sound effects.

## Save Feed

Periodically (every 60 seconds), the game posts a heartbeat to the server's `/api/save-feed` endpoint with current world stats. The feed is debounced to prevent duplicate requests within 10 seconds.

```typescript
// Save feed posting
function postSaveFeed(): void {
  const now = Date.now();
  if (now - this.lastSaveFeedMs < SAVE_FEED_MIN_INTERVAL_MS) return;
  this.lastSaveFeedMs = now;

  fetch("/api/save-feed", {
    method: "POST",
    body: JSON.stringify({
      worldSlug: this.worldConfig.slug,
      vehicle: this.localPlayer.vehicleType,
      level: this.progression.level,
      xp: this.progression.xp,
    }),
  });
}
```

See [Quest Systems](06-quest-systems.md) for XP sources and rewards.
See [Database Schema](13-database-schema.md) for save feed storage.
