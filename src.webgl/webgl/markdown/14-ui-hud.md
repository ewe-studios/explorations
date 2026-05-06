# Tiny Skies -- UI and HUD

The user interface is built entirely with inline CSS (no external stylesheet), rendered as DOM overlays on top of the Three.js canvas. The UI system includes a lobby screen, heads-up display, control hints, debug menu, and various context-specific overlays.

Source: `tinyskies/client/src/ui/HUD.ts` — main HUD (~1890 lines)
Source: `tinyskies/client/src/ui/Lobby.ts` — lobby screen (~1344 lines)
Source: `tinyskies/client/src/ui/CircularProgressRing.ts` — SVG progress ring
Source: `tinyskies/client/src/ui/ControlHints.ts` — context-sensitive hints
Source: `tinyskies/client/src/ui/TransitionOverlay.ts` — phase transitions
Source: `tinyskies/client/src/ui/LevelUpCards.ts` — upgrade card selection

## HUD

```typescript
// HUD.ts — ~1890 lines of inline CSS DOM manipulation
class HUD {
  private container: HTMLElement;
  private worldNameEl: HTMLElement;
  private playerCountEl: HTMLElement;
  private xpBarEl: HTMLElement;
  private levelEl: HTMLElement;
  private brazierIcons: HTMLElement[];
  private toastContainer: HTMLElement;

  // Elements created once, updated each frame
  constructor(app: HTMLElement) {
    this.container = document.createElement("div");
    this.container.className = "hud-container";
    // ... create all sub-elements with inline styles
    app.appendChild(this.container);
  }

  update(worldName: string, playerCount: number, xp: number, level: number, ...): void {
    this.worldNameEl.textContent = worldName;
    this.playerCountEl.textContent = `${playerCount} player${playerCount !== 1 ? "s" : ""}`;
    this.xpBarEl.style.width = `${(xp / xpToNext) * 100}%`;
    this.levelEl.textContent = `Level ${level}`;

    // Braziers: 5 flame icons with clip-path fill
    for (let i = 0; i < 5; i++) {
      this.brazierIcons[i].className = brazierLit[i] ? "brazier lit" : "brazier unlit";
    }
  }
}
```

### HUD Components

| Element | Position | Content |
|---------|----------|---------|
| World name | Top-left | World display name |
| Player count | Top-left | Number of players in room |
| Quest trackers | Left side | Package delivery progress |
| XP bar | Top-center | Current XP / XP to next level |
| Level display | Top-center | Current level number |
| Brazier tracker | Top-right | 5 flame icons (clip-path fill) |
| Race timer | Top-center | Countdown timer during races |
| Flag warning | Center | "FLAG CARRIER" warning text |
| Toasts | Center | Temporary notifications |
| Fullscreen button | Top-right | Toggle fullscreen API |
| Mute button | Top-right | Toggle audio |

### Paintball Splatter Overlay

```typescript
// CSS mask-based splatter overlay
function showSplatterOverlay(color: number, rotation: number): void {
  this.splatterEl.style.cssText = `
    position: fixed;
    inset: 0;
    background: #${color.toString(16).padStart(6, "0")};
    -webkit-mask-image: radial-gradient(circle, transparent 30%, black 70%);
    mask-image: radial-gradient(circle, transparent 30%, black 70%);
    transform: rotate(${rotation}deg);
    opacity: 0.3;
    pointer-events: none;
    animation: splatterFade 0.5s ease-out forwards;
  `;
}
```

The splatter overlay uses a CSS mask to create a radial vignette effect, rotated randomly, that fades out over 0.5 seconds. This gives visual feedback when the player is hit by a paintball.

### Race Win Confetti

```typescript
// 72 CSS-animated confetti pieces
function showRaceWinConfetti(): void {
  for (let i = 0; i < 72; i++) {
    const piece = document.createElement("div");
    piece.style.cssText = `
      position: fixed;
      width: ${random(5, 10)}px;
      height: ${random(5, 10)}px;
      background: ${randomColor()};
      left: ${random(0, 100)}vw;
      top: -20px;
      animation: confettiFall ${random(2, 4)}s linear forwards;
      animation-delay: ${random(0, 0.5)}s;
    `;
    document.body.appendChild(piece);
    // Auto-remove after animation
    setTimeout(() => piece.remove(), 4500);
  }
}
```

## Lobby

```typescript
// Lobby.ts — ~1344 lines
class Lobby {
  // Per-letter animated title ("Tiny Skies")
  // Whimsical name generator (for player names)
  // Vehicle selection (plane/carpet/boat)
  // GO! button with fullscreen request
  // Freeplay mode toggle
  // Save feed (recent world activity)
  // Unlock celebration modals with 3D preview canvases
  // VibeJam portal link
  // All inline CSS with responsive breakpoints
}
```

### Lobby Title Animation

```typescript
// Each letter of "TINY SKIES" animates independently
function animateTitle(): void {
  for (const letter of this.titleLetters) {
    letter.style.cssText = `
      display: inline-block;
      animation: titleBounce 2s ease-in-out infinite;
      animation-delay: ${letter.index * 0.1}s;
    `;
  }
}
```

### Whimsical Name Generator

```typescript
function generateWhimsicalName(): string {
  const adjectives = ["Brave", "Swift", "Gentle", "Bold", "Curious"];
  const nouns = ["Falcon", "Badger", "Otter", "Fox", "Owl"];
  return `${pickRandom(adjectives)} ${pickRandom(nouns)}`;
}
```

### Vehicle Selection

The lobby shows three vehicle cards:
- **Plane**: Always unlocked (Level 1)
- **Carpet**: Unlocked at Level 2 (shows lock icon if not unlocked)
- **Boat**: Unlocked at Level 4 (shows lock icon if not unlocked)

Clicking "GO!" sends the player into the game with their selected vehicle and requests fullscreen via the Fullscreen API.

### Unlock Celebration

When a new vehicle is unlocked, a modal appears with:
- 3D canvas preview of the vehicle (Three.js scene rendered to a small canvas)
- Confetti animation
- "New Vehicle Unlocked!" text

## CircularProgressRing

```typescript
// CircularProgressRing.ts — SVG-based circular progress
class CircularProgressRing {
  private svg: SVGSVGElement;
  private circle: SVGCircleElement;
  private text: SVGTextElement;

  // SVG circle with stroke-dasharray for progress
  // circumference = 2 * PI * radius
  // dashoffset = circumference * (1 - progress)

  setProgress(value: number, label: string): void {
    const circumference = 2 * Math.PI * this.radius;
    this.circle.style.strokeDashoffset = String(circumference * (1 - value));
    this.text.textContent = label;
  }
}
```

Used for race countdown, flag capture progress, and XP progress display.

## Control Hints

```typescript
// ControlHints.ts
interface ControlHintRow {
  keys: string[];  // ["W", "A", "S", "D"]
  label: string;   // "Fly with WASD"
}

class ControlHints {
  // Shows context-sensitive key bindings
  // Desktop: keyboard keys with keycap styling
  // Mobile: button icons matching touch layout

  update(rows: ControlHintRow[]): void {
    this.container.innerHTML = rows.map(row => `
      <div class="hint-row">
        ${row.keys.map(k => `<kbd class="key">${k}</kbd>`).join(" ")}
        <span class="hint-label">${row.label}</span>
      </div>
    `).join("");
  }
}
```

## Debug Menu

```typescript
// DebugMenu.ts
class DebugMenu {
  // Toggled with backtick (`) key
  // Shows:
  // - FPS counter
  // - Current vehicle state (position, heading, speed)
  // - Day/night cycle phase
  // - Moon progress
  // - Player HP
  // - Upgrade multipliers
  // - Network latency
  // - Debug buttons: force flag spawn, clear save, teleport
}
```

## Transition Overlay

```typescript
// TransitionOverlay.ts
class TransitionOverlay {
  // Full-screen overlay for phase transitions
  // Fade-in/fade-out with text
  // Used for: moon impact, campsite transition, rewind

  fadeIn(text: string, duration: number): void {
    this.textEl.textContent = text;
    this.container.style.opacity = "1";
    this.container.style.transition = `opacity ${duration}s ease-in`;
  }

  fadeOut(duration: number): void {
    this.container.style.opacity = "0";
    this.container.style.transition = `opacity ${duration}s ease-out`;
  }
}
```

## Vehicle Tutorial Hints

Each vehicle has a tutorial sequence shown at game start:

```typescript
const VEHICLE_TUTORIAL_STEPS = {
  plane: [
    { keys: ["W", "A", "S", "D"], label: "Fly with WASD" },
    { keys: ["↑"], label: "Climb with Up Arrow" },
    { keys: ["Space"], label: "Shoot with Space" },
    { keys: [], label: "That's it. Enjoy flying!" },
  ],
  carpet: [
    { keys: ["W", "A", "S", "D"], label: "Fly with WASD" },
    { keys: ["Space"], label: "Open Portal 1 with Space" },
    { keys: ["Space"], label: "Open Portal 2 with Space" },
    { keys: [], label: "Fly through a portal" },
    { keys: [], label: "That's it. Enjoy flying!" },
  ],
  boat: [
    { keys: ["W", "A", "S", "D"], label: "Move with WASD" },
    { keys: [], label: "Find a fish pool and catch a fish" },
    { keys: [], label: "That's it. Enjoy boating!" },
  ],
};
```

The tutorial auto-advances with a 2-second delay (0.3s for carpet portal steps), and auto-dismisses after 30 seconds if not completed.

See [Flight Controls](03-flight-controls.md) for touch/mobile controls.
See [Progression](07-progression-upgrades.md) for level-up card UI.
