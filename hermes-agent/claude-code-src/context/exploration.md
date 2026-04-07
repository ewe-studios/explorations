# Context Module - Deep Dive Exploration

**Source Directory:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.ClaudOpen/claude-code-main/src/context/`

**Files:** 9 files total

---

## Table of Contents

1. [File Inventory](#file-inventory)
2. [Module Overview](#module-overview)
3. [Key Exports and Type Signatures](#key-exports-and-type-signatures)
4. [Line-by-Line Analysis](#line-by-line-analysis)
5. [Integration Points](#integration-points)

---

## File Inventory

| File | Lines | Key Exports | Description |
|------|-------|-------------|-------------|
| `QueuedMessageContext.tsx` | ~63 | `QueuedMessageContext`, `QueuedMessageProvider`, `useQueuedMessage` | Context for queued message layout state (isQueued, isFirst, paddingWidth) |
| `fpsMetrics.tsx` | ~30 | `FpsMetricsContext`, `FpsMetricsProvider`, `useFpsMetrics` | Context for FPS metrics getter function distribution |
| `mailbox.tsx` | ~38 | `MailboxContext`, `MailboxProvider`, `useMailbox` | Context providing Mailbox instance for async message passing |
| `modalContext.tsx` | ~58 | `ModalContext`, `useIsInsideModal`, `useModalOrTerminalSize`, `useModalScrollRef` | Context for modal layout state (rows, columns, scrollRef) |
| `notifications.tsx` | ~240 | `useNotifications`, `getNext`, `Notification`, `Priority` | Full notification system with priority queue, folding, timeouts |
| `overlayContext.tsx` | ~151 | `overlayContext`, `useRegisterOverlay`, `useIsOverlayActive`, `useIsModalOverlayActive` | Overlay tracking for Escape key coordination |
| `promptOverlayContext.tsx` | ~125 | `PromptOverlayProvider`, `usePromptOverlay`, `usePromptOverlayDialog`, `useSetPromptOverlay`, `useSetPromptOverlayDialog` | Portal for floating content above prompt (slash-command suggestions, dialogs) |
| `stats.tsx` | ~220 | `StatsContext`, `StatsProvider`, `useStats`, `useCounter`, `useGauge`, `useTimer`, `useSet` | Metrics/stats collection with histograms, reservoir sampling, percentiles |
| `voice.tsx` | ~88 | `VoiceContext`, `VoiceProvider`, `useVoiceState`, `useSetVoiceState`, `useGetVoiceState` | Voice state management (recording, processing, transcript, audio levels) |

**Total Lines:** ~1,013 lines

---

## Module Overview

### Purpose and Responsibilities

The `context/` module provides React Context providers that establish the global state infrastructure for the Claude Code TUI application. These contexts form the backbone of the application's state management, enabling:

1. **Cross-component communication** without prop drilling
2. **Global state sharing** for UI state, notifications, metrics, and voice
3. **Layout coordination** between fullscreen layouts, modals, and overlays
4. **Performance monitoring** through FPS tracking and metrics collection
5. **Async messaging** through the Mailbox pattern

### Architectural Patterns

The module follows several key patterns:

1. **Context + Provider + Hook Trinity**: Each context exports:
   - A `React.Context` object
   - A `Provider` component that supplies the value
   - One or more hooks for consuming the context

2. **Stable Setter Pattern**: Contexts like `voice.tsx` and `notifications.tsx` separate data from setters, allowing components to call setters without causing re-renders

3. **Reservoir Sampling**: The `stats.tsx` uses Algorithm R for memory-efficient histogram storage

4. **Priority Queue with Folding**: Notifications support priority-based ordering and folding (merging) duplicate keys

---

## Key Exports and Type Signatures

### QueuedMessageContext.tsx

```typescript
type QueuedMessageContextValue = {
  isQueued: boolean;
  isFirst: boolean;
  paddingWidth: number;  // Width reduction for container padding
};

const QueuedMessageContext: React.Context<QueuedMessageContextValue | undefined>;

function QueuedMessageProvider({
  isFirst,
  useBriefLayout?,
  children
}: Props): React.ReactNode;

function useQueuedMessage(): QueuedMessageContextValue | undefined;
```

### ModalContext.tsx

```typescript
type ModalCtx = {
  rows: number;
  columns: number;
  scrollRef: RefObject<ScrollBoxHandle | null> | null;
};

const ModalContext: React.Context<ModalCtx | null>;

function useIsInsideModal(): boolean;
function useModalOrTerminalSize(fallback: { rows: number; columns: number }): { rows: number; columns: number };
function useModalScrollRef(): RefObject<ScrollBoxHandle | null> | null;
```

### notifications.tsx

```typescript
type Priority = 'low' | 'medium' | 'high' | 'immediate';

type BaseNotification = {
  key: string;
  invalidates?: string[];  // Keys of notifications this invalidates
  priority: Priority;
  timeoutMs?: number;
  fold?: (accumulator: Notification, incoming: Notification) => Notification;
};

type TextNotification = BaseNotification & {
  text: string;
  color?: keyof Theme;
};

type JSXNotification = BaseNotification & {
  jsx: React.ReactNode;
};

type Notification = TextNotification | JSXNotification;

type AddNotificationFn = (content: Notification) => void;
type RemoveNotificationFn = (key: string) => void;

function useNotifications(): {
  addNotification: AddNotificationFn;
  removeNotification: RemoveNotificationFn;
};

function getNext(queue: Notification[]): Notification | undefined;
```

### overlayContext.tsx

```typescript
const NON_MODAL_OVERLAYS = new Set(['autocomplete']);

function useRegisterOverlay(id: string, enabled?: boolean): void;
function useIsOverlayActive(): boolean;
function useIsModalOverlayActive(): boolean;
```

### promptOverlayContext.tsx

```typescript
type PromptOverlayData = {
  suggestions: SuggestionItem[];
  selectedSuggestion: number;
  maxColumnWidth?: number;
};

type Setter<T> = (d: T | null) => void;

const DataContext: React.Context<PromptOverlayData | null>;
const SetContext: React.Context<Setter<PromptOverlayData> | null>;
const DialogContext: React.Context<ReactNode>;
const SetDialogContext: React.Context<Setter<ReactNode> | null>;

function PromptOverlayProvider({ children }: Props): ReactNode;
function usePromptOverlay(): PromptOverlayData | null;
function usePromptOverlayDialog(): ReactNode;
function useSetPromptOverlay(data: PromptOverlayData | null): void;
function useSetPromptOverlayDialog(node: ReactNode): void;
```

### stats.tsx

```typescript
type StatsStore = {
  increment(name: string, value?: number): void;
  set(name: string, value: number): void;
  observe(name: string, value: number): void;  // Histogram observation
  add(name: string, value: string): void;      // Set addition
  getAll(): Record<string, number>;
};

function createStatsStore(): StatsStore;

const StatsContext: React.Context<StatsStore | null>;

function useStats(): StatsStore;
function useCounter(name: string): (value?: number) => void;
function useGauge(name: string): (value: number) => void;
function useTimer(name: string): (value: number) => void;
function useSet(name: string): (value: string) => void;
```

### voice.tsx

```typescript
type VoiceState = {
  voiceState: 'idle' | 'recording' | 'processing';
  voiceError: string | null;
  voiceInterimTranscript: string;
  voiceAudioLevels: number[];
  voiceWarmingUp: boolean;
};

type VoiceStore = Store<VoiceState>;

const VoiceContext: React.Context<VoiceStore | null>;

function VoiceProvider({ children }: Props): React.ReactNode;
function useVoiceState<T>(selector: (state: VoiceState) => T): T;
function useSetVoiceState(): (updater: (prev: VoiceState) => VoiceState) => void;
function useGetVoiceState(): () => VoiceState;
```

### mailbox.tsx

```typescript
const MailboxContext: React.Context<Mailbox | undefined>;

function MailboxProvider({ children }: Props): React.ReactNode;
function useMailbox(): Mailbox;  // Throws if used outside provider
```

### fpsMetrics.tsx

```typescript
type FpsMetricsGetter = () => FpsMetrics | undefined;

const FpsMetricsContext: React.Context<FpsMetricsGetter | undefined>;

function FpsMetricsProvider({
  getFpsMetrics,
  children
}: Props): React.ReactNode;

function useFpsMetrics(): FpsMetricsGetter | undefined;
```

---

## Line-by-Line Analysis

### notifications.tsx - Priority Queue Processing

**Lines 46-77: processQueue callback**

```typescript
const processQueue = useCallback(() => {
  setAppState(prev => {
    const next = getNext(prev.notifications.queue);
    if (prev.notifications.current !== null || !next) {
      return prev;
    }
    currentTimeoutId = setTimeout((setAppState, nextKey, processQueue) => {
      currentTimeoutId = null;
      setAppState(prev => {
        if (prev.notifications.current?.key !== nextKey) {
          return prev;
        }
        return {
          ...prev,
          notifications: {
            queue: prev.notifications.queue,
            current: null
          }
        };
      });
      processQueue();
    }, next.timeoutMs ?? DEFAULT_TIMEOUT_MS, setAppState, next.key, processQueue);
    return {
      ...prev,
      notifications: {
        queue: prev.notifications.queue.filter(_ => _ !== next),
        current: next
      }
    };
  });
}, [setAppState]);
```

**Explanation:**
- `processQueue` is the core engine that moves notifications from queue to display
- It first checks if there's already a current notification or empty queue (lines 49-51)
- If not, it schedules a timeout to clear the notification after `timeoutMs` (default 8000ms)
- The timeout callback verifies the notification hasn't changed before clearing (line 56)
- Finally, it moves the highest priority notification from queue to current (lines 69-75)
- Priority is determined by `getNext()` which finds the minimum priority value (0=immediate, 3=low)

**Lines 78-192: addNotification with folding**

```typescript
const addNotification = useCallback<AddNotificationFn>((notif: Notification) => {
  // Handle immediate priority notifications
  if (notif.priority === 'immediate') {
    // Clear any existing timeout since we're showing a new immediate notification
    if (currentTimeoutId) {
      clearTimeout(currentTimeoutId);
      currentTimeoutId = null;
    }
    // ... timeout setup ...
    
    // Show the immediate notification right away
    setAppState(prev => ({
      ...prev,
      notifications: {
        current: notif,
        queue: [...(prev.notifications.current ? [prev.notifications.current] : []), ...prev.notifications.queue]
          .filter(_ => _.priority !== 'immediate' && !notif.invalidates?.includes(_.key))
      }
    }));
    return;
  }
  
  // Handle non-immediate notifications with folding
  setAppState(prev => {
    if (notif.fold) {
      // Fold into current notification if keys match
      if (prev.notifications.current?.key === notif.key) {
        const folded = notif.fold(prev.notifications.current, notif);
        // Reset timeout for the folded notification
        // ...
      }
      // Fold into queued notification if keys match
      const queueIdx = prev.notifications.queue.findIndex(_ => _.key === notif.key);
      if (queueIdx !== -1) {
        const folded = notif.fold(prev.notifications.queue[queueIdx]!, notif);
        const newQueue = [...prev.notifications.queue];
        newQueue[queueIdx] = folded;
        return { ...prev, notifications: { current: prev.notifications.current, queue: newQueue } };
      }
    }
    // ... duplicate prevention and invalidation logic ...
  });
  processQueue();
}, [setAppState, processQueue]);
```

**Explanation:**
- Immediate notifications bypass the queue and display instantly (line 80)
- They also clear any existing timeout and re-queue the current notification (lines 82-85)
- The fold mechanism (lines 122-169) allows notifications with the same key to merge
- This is like `Array.reduce()` - the fold function receives accumulator and incoming, returns merged
- Invalidates (line 176) allow a notification to remove others from the queue

### stats.tsx - Reservoir Sampling

**Lines 11-19: Percentile calculation**

```typescript
function percentile(sorted: number[], p: number): number {
  const index = (p / 100) * (sorted.length - 1);
  const lower = Math.floor(index);
  const upper = Math.ceil(index);
  if (lower === upper) {
    return sorted[lower]!;
  }
  return sorted[lower]! + (sorted[upper]! - sorted[lower]!) * (index - lower);
}
```

**Explanation:**
- Linear interpolation between two nearest values for accurate percentile
- When index lands exactly on an element, returns that element directly
- Otherwise, interpolates between lower and upper based on fractional part

**Lines 28-98: Stats store with reservoir sampling**

```typescript
export function createStatsStore(): StatsStore {
  const metrics = new Map<string, number>();
  const histograms = new Map<string, Histogram>();
  const sets = new Map<string, Set<string>>();
  
  return {
    observe(name: string, value: number) {
      let h = histograms.get(name);
      if (!h) {
        h = { reservoir: [], count: 0, sum: 0, min: value, max: value };
        histograms.set(name, h);
      }
      h.count++;
      h.sum += value;
      if (value < h.min) h.min = value;
      if (value > h.max) h.max = value;
      
      // Reservoir sampling (Algorithm R)
      if (h.reservoir.length < RESERVOIR_SIZE) {
        h.reservoir.push(value);
      } else {
        const j = Math.floor(Math.random() * h.count);
        if (j < RESERVOIR_SIZE) {
          h.reservoir[j] = value;
        }
      }
    },
    getAll() {
      // ... metrics ...
      for (const [name, h] of histograms) {
        result[`${name}_count`] = h.count;
        result[`${name}_min`] = h.min;
        result[`${name}_max`] = h.max;
        result[`${name}_avg`] = h.sum / h.count;
        const sorted = [...h.reservoir].sort((a, b) => a - b);
        result[`${name}_p50`] = percentile(sorted, 50);
        result[`${name}_p95`] = percentile(sorted, 95);
        result[`${name}_p99`] = percentile(sorted, 99);
      }
      // ... sets ...
      return result;
    }
  };
}
```

**Explanation:**
- Reservoir sampling (lines 59-67) maintains a fixed-size sample from an unbounded stream
- Algorithm R: for each new value at count `n`, replace a random reservoir element with probability `RESERVOIR_SIZE/n`
- This gives uniform probability sampling without storing all values
- The reservoir is then sorted to compute accurate p50, p95, p99 percentiles
- Memory usage is bounded to `RESERVOIR_SIZE` (1024) per histogram

### overlayContext.tsx - Escape Key Coordination

**Lines 38-103: useRegisterOverlay**

```typescript
export function useRegisterOverlay(id: string, enabled = true): void {
  const store = useContext(AppStoreContext);
  const setAppState = store?.setState;
  
  useEffect(() => {
    if (!enabled || !setAppState) return;
    setAppState(prev => {
      if (prev.activeOverlays.has(id)) return prev;
      const next = new Set(prev.activeOverlays);
      next.add(id);
      return { ...prev, activeOverlays: next };
    });
    return () => {
      setAppState(prev => {
        if (!prev.activeOverlays.has(id)) return prev;
        const next = new Set(prev.activeOverlays);
        next.delete(id);
        return { ...prev, activeOverlays: next };
      });
    };
  }, [id, enabled, setAppState]);
  
  useLayoutEffect(() => {
    if (!enabled) return;
    return () => instances.get(process.stdout)?.invalidatePrevFrame();
  }, [enabled]);
}
```

**Explanation:**
- Solves Escape key handling when overlays are open (Select with onCancel)
- On mount: adds overlay ID to `activeOverlays` Set in AppState
- On unmount: removes the ID, allowing Escape to cancel requests again
- The `useLayoutEffect` cleanup (lines 88-103) forces a full-damage diff instead of blit
- This prevents "ghost" artifacts when tall overlays shrink the layout

**Lines 122-149: useIsModalOverlayActive**

```typescript
export function useIsModalOverlayActive(): boolean {
  return useAppState(s => {
    for (const id of s.activeOverlays) {
      if (!NON_MODAL_OVERLAYS.has(id)) return true;
    }
    return false;
  });
}
```

**Explanation:**
- Distinguishes modal overlays (capture all input) from non-modal (autocomplete)
- Non-modal overlays like autocomplete don't disable TextInput focus
- Used for `focus: !isSearchingHistory && !isModalOverlayActive`

### promptOverlayContext.tsx - Dual Channel Design

**Lines 34-60: PromptOverlayProvider**

```typescript
export function PromptOverlayProvider({ children }: Props): ReactNode {
  const [data, setData] = useState<PromptOverlayData | null>(null);
  const [dialog, setDialog] = useState<ReactNode>(null);
  
  return (
    <SetContext.Provider value={setData}>
      <SetDialogContext.Provider value={setDialog}>
        <DataContext.Provider value={data}>
          <DialogContext.Provider value={dialog}>
            {children}
          </DialogContext.Provider>
        </DataContext.Provider>
      </SetDialogContext.Provider>
    </SetContext.Provider>
  );
}
```

**Explanation:**
- Split into data/setter context pairs so writers never re-render on their own writes
- `useSetPromptOverlay` writes to `SetContext`, doesn't subscribe to `DataContext`
- Two channels: suggestions (from PromptInputFooter) and dialog (from PromptInput)
- FullscreenLayout reads both and renders outside the clipped prompt slot

**Lines 72-95: useSetPromptOverlay**

```typescript
export function useSetPromptOverlay(data: PromptOverlayData | null): void {
  const set = useContext(SetContext);
  useEffect(() => {
    if (!set) return;
    set(data);
    return () => set(null);  // Clear on unmount
  }, [set, data]);
}
```

**Explanation:**
- No-op outside provider (non-fullscreen renders inline)
- Automatically clears on unmount via cleanup callback
- Structured data for slash-command suggestions

### voice.tsx - Selector-based Subscription

**Lines 55-69: useVoiceState**

```typescript
export function useVoiceState<T>(selector: (state: VoiceState) => T): T {
  const store = useVoiceStore();
  const get = () => selector(store.getState());
  return useSyncExternalStore(store.subscribe, get, get);
}
```

**Explanation:**
- Uses `useSyncExternalStore` for efficient subscription to state slices
- Only re-renders when the selected value changes (compared via Object.is)
- The `get` function is memoized to avoid unnecessary re-subscriptions

**Lines 76-87: Stable setters**

```typescript
export function useSetVoiceState(): (updater: (prev: VoiceState) => VoiceState) => void {
  return useVoiceStore().setState;
}

export function useGetVoiceState(): () => VoiceState {
  return useVoiceStore().getState;
}
```

**Explanation:**
- `useSetVoiceState` returns stable reference - never causes re-renders
- `store.setState` is synchronous - callers can read `getVoiceState()` immediately after
- `useGetVoiceState` for event handlers needing fresh state without subscription

---

## Integration Points

### How context/ Integrates with Other Modules

#### 1. With `state/` Module

```typescript
// notifications.tsx
import { useAppStateStore, useSetAppState } from 'src/state/AppState.js';

// overlayContext.tsx
import { AppStoreContext, useAppState } from '../state/AppState.js';
```

**Integration Pattern:**
- Contexts read from and write to the central `AppState` store
- `useNotifications` uses `useAppStateStore()` and `useSetAppState()`
- Notification queue and current notification stored in `AppState.notifications`
- Overlay tracking stored in `AppState.activeOverlays`

#### 2. With `ink/` Module

```typescript
// modalContext.tsx
import type { ScrollBoxHandle } from '../ink/components/ScrollBox.js';

// QueuedMessageContext.tsx
import { Box } from '../ink.js';
```

**Integration Pattern:**
- `ModalContext` provides `scrollRef` for `ScrollBoxHandle` reference
- Used by `FullscreenLayout` to reset scroll on tab switch
- `QueuedMessageProvider` wraps children in `Box` with conditional padding

#### 3. With `utils/` Module

```typescript
// mailbox.tsx
import { Mailbox } from '../utils/mailbox.js';

// stats.tsx
import { saveCurrentProjectConfig } from '../utils/config.js';

// voice.tsx
import { createStore, type Store } from '../state/store.js';
```

**Integration Pattern:**
- `Mailbox` class from utils provides async message passing
- Stats flush to config on process exit via `saveCurrentProjectConfig`
- Voice uses generic `Store` pattern from state utils

#### 4. With `components/` Module

```typescript
// promptOverlayContext.tsx
import type { SuggestionItem } from '../components/PromptInput/PromptInputFooterSuggestions.js';
```

**Integration Pattern:**
- `PromptOverlayData.suggestions` typed with `SuggestionItem` from components
- `PromptInputFooter` writes suggestion data via `useSetPromptOverlay`
- `FullscreenLayout` reads and renders outside clipped slot

#### 5. With `services/` Module

```typescript
// fpsMetrics.tsx
import type { FpsMetrics } from '../utils/fpsTracker.js';
```

**Integration Pattern:**
- FPS metrics collected by `fpsTracker.js` service
- Context distributes getter function to consumers
- Allows any component to access current FPS metrics

---

## Provider Hierarchy

The contexts form a provider hierarchy in the application root:

```
<AppStateProvider>
  <StatsProvider>
    <MailboxProvider>
      <VoiceProvider>
        <FpsMetricsProvider>
          <ModalContext.Provider>  {/* FullscreenLayout */}
            <PromptOverlayProvider>
              <QueuedMessageContext.Provider>
                <NotificationProvider>  {/* Implicit via AppState */}
                  <OverlayContext>      {/* Implicit via AppState */}
                    {children}
```

**Key Observations:**

1. **AppState is the root**: Most contexts either wrap AppState or read from it
2. **Independent contexts**: Stats, Mailbox, Voice are self-contained
3. **Layout contexts are nested**: Modal, PromptOverlay, QueuedMessage follow layout structure
4. **No circular dependencies**: Clear separation between state and context layers

---

## Summary

The `context/` module is the central nervous system of Claude Code's TUI, providing:

- **9 distinct contexts** for different concerns (notifications, overlays, voice, stats, etc.)
- **Consistent patterns** across all contexts (Provider + Hook structure)
- **Advanced algorithms** (reservoir sampling, priority queue with folding)
- **Performance optimizations** (stable setters, selector-based subscriptions)
- **Escape key coordination** through overlay tracking
- **Layout management** through modal and prompt overlay contexts

The module demonstrates sophisticated React patterns while maintaining clarity and separation of concerns.
