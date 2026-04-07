# Claude Code Components Module — Deep-Dive Exploration

**Module:** `components/`  
**Parent Project:** [index.md](../index.md)  
**Created:** 2026-04-07  
**Files:** 389 TypeScript/TSX files

---

## 1. Module Overview

The `components/` module is the **React-based UI layer** for Claude Code's terminal interface. It implements a comprehensive design system built on top of Ink (React for terminal), providing all interactive components, dialogs, messages, and layout infrastructure for the CLI application.

### Core Responsibilities

1. **Design System** — Reusable UI primitives:
   - `ThemedBox`, `ThemedText`: Theme-aware layout and typography
   - `ThemeProvider`: Context-based theming with auto-detection
   - `Dialog`, `Pane`: Modal containers with consistent styling
   - `Tabs`: Tabbed content navigation
   - `ProgressBar`, `Spinner`: Loading and progress indicators

2. **Layout Infrastructure**:
   - `App`: Top-level context provider wrapper
   - `FullscreenLayout`: Sticky-scroll layout for REPL (message pill, sticky headers)
   - `VirtualMessageList`: Virtualized scrolling for large message histories

3. **Message Components** — Chat transcript rendering:
   - `AssistantTextMessage`, `AssistantThinkingMessage`, `AssistantToolUseMessage`
   - `UserTextMessage`, `UserPromptMessage`, `UserCommandMessage`
   - `SystemTextMessage`, `PlanApprovalMessage`, `RateLimitMessage`
   - `AttachmentMessage`: Image/file attachment display

4. **Dialog System** — Modal interactions:
   - Permission prompts (file, bash, MCP, skill approvals)
   - Settings dialogs (theme, model, config)
   - Onboarding flows, error dialogs, confirmation prompts

5. **Interactive Inputs**:
   - `PromptInput`: Main chat input with slash commands, history, suggestions
   - `CustomSelect/Select`: Dropdown selection with keyboard navigation
   - `BaseTextInput`, `TextInput`: Text input primitives
   - `ModelPicker`, `ThemePicker`: Specialized selectors

### Key Design Patterns

- **React Compiler**: All components use `_c()` memoization cache for performance
- **Theme-aware rendering**: All colors resolve through `useTheme()` context
- **Ink integration**: Terminal rendering via `<Box>`, `<Text>`, `<Box>` primitives
- **Context composition**: Nested providers for modal, stats, fps, prompt overlay
- **Virtual scrolling**: `VirtualMessageList` for efficient large transcript rendering
- **Sticky scroll**: `FullscreenLayout` maintains "N new messages" pill while scrolling

---

## 2. File Inventory

### Root-Level Components (100+ files)

| File | Lines | Description |
|------|-------|-------------|
| `App.tsx` | ~55 | Top-level provider wrapper (FPS, stats, AppState) |
| `FullscreenLayout.tsx` | ~636 | Sticky-scroll layout with message pill, modal pane |
| `VirtualMessageList.tsx` | ~1081 | Virtualized message list rendering |
| `Messages.tsx` | ~833 | Message list container with divider logic |
| `Message.tsx` | ~626 | Individual message renderer |
| `MessageSelector.tsx` | ~830 | Message selection/copy functionality |
| `LogSelector.tsx` | ~1574 | Log viewing/selection UI |
| `PromptInput/PromptInput.tsx` | ~2338 | Main chat input component |
| `Spinner.tsx` | ~561 | Loading spinner with verb display |
| `Feedback.tsx` | ~591 | Feedback collection UI |
| `Stats.tsx` | ~1227 | Token/cost statistics display |
| `TaskListV2.tsx` | ~470 | Task/todo list component |
| `CoordinatorAgentStatus.tsx` | ~360 | Agent/swarm status display |
| `StatusLine.tsx` | ~465 | Bottom status bar |
| `ModelPicker.tsx` | ~447 | Model selection dropdown |
| `ThemePicker.tsx` | ~335 | Theme selection UI |
| `Markdown.tsx` | ~281 | Markdown rendering |
| `MarkdownTable.tsx` | ~475 | Table rendering in markdown |

### Design System (`design-system/`)

| File | Lines | Description |
|------|-------|-------------|
| `ThemeProvider.tsx` | ~188 | Theme context provider with auto-detection |
| `ThemedBox.tsx` | ~195 | Theme-aware Box component |
| `ThemedText.tsx` | ~138 | Theme-aware Text component |
| `Tabs.tsx` | ~414 | Tabbed content navigation |
| `Dialog.tsx` | ~141 | Dialog container with keybindings |
| `Pane.tsx` | ~69 | Pane/border container |
| `Byline.tsx` | ~68 | Footer action hints |
| `Divider.tsx` | ~110 | Horizontal divider |
| `ListItem.tsx` | ~195 | Selectable list item |
| `KeyboardShortcutHint.tsx` | ~68 | Keyboard shortcut display |
| `ProgressBar.tsx` | ~71 | Progress bar indicator |
| `LoadingState.tsx` | ~69 | Loading state display |
| `StatusIcon.tsx` | ~75 | Status icon component |
| `Ratchet.tsx` | ~71 | Ratchet/spinner component |
| `color.ts` | ~30 | Theme color resolver |

### Messages (`messages/`)

| File | Lines | Description |
|------|-------|-------------|
| `AssistantToolUseMessage.tsx` | ~452 | Tool call display with progress |
| `AssistantTextMessage.tsx` | ~304 | Assistant text response |
| `AssistantThinkingMessage.tsx` | ~80 | Thinking indicator |
| `UserTextMessage.tsx` | ~290 | User text message |
| `UserPromptMessage.tsx` | ~151 | User prompt display |
| `UserCommandMessage.tsx` | ~92 | User command display |
| `AttachmentMessage.tsx` | ~714 | Image/file attachments |
| `SystemTextMessage.tsx` | ~826 | System messages |
| `PlanApprovalMessage.tsx` | ~253 | Plan approval request |
| `RateLimitMessage.tsx` | ~171 | Rate limit notification |
| `HighlightedThinkingText.tsx` | ~149 | Thinking text highlighting |
| `GroupedToolUseContent.tsx` | ~82 | Grouped tool use display |

### Permissions (`permissions/`)

| File | Lines | Description |
|------|-------|-------------|
| `PermissionPrompt.tsx` | ~373 | Base permission prompt |
| `PermissionRequest.tsx` | ~335 | Permission request container |
| `PermissionDialog.tsx` | ~73 | Permission dialog wrapper |
| `PermissionExplanation.tsx` | ~237 | Permission explanation text |
| `PermissionRuleList.tsx` | ~1178 | Permission rules display |
| `FallbackPermissionRequest.tsx` | ~799 | Fallback permission handler |
| `SandboxPermissionRequest.tsx` | ~149 | Sandbox permission request |
| `shellPermissionHelpers.tsx` | ~225 | Shell permission utilities |
| `PermissionDecisionDebugInfo.tsx` | ~525 | Debug info for permissions |
| `BashPermissionRequest/BashPermissionRequest.tsx` | ~481 | Bash command approval |
| `FilePermissionDialog/` | ~4 files | File access dialogs |
| `AskUserQuestionPermissionRequest/` | ~3 files | Question approval |

### UI Components (`ui/`)

| File | Lines | Description |
|------|-------|-------------|
| `TreeSelect.tsx` | ~389 | Tree selection component |
| `OrderedList.tsx` | ~72 | Ordered list wrapper |
| `OrderedListItem.tsx` | ~34 | Ordered list item |

### Custom Select (`CustomSelect/`)

| File | Lines | Description |
|------|-------|-------------|
| `select.tsx` | ~689 | Base select component |
| `SelectMulti.tsx` | ~414 | Multi-select variant |
| `use-select-state.ts` | ~200 | Select state management |
| `use-select-navigation.ts` | ~653 | Keyboard navigation |
| `use-select-input.ts` | ~150 | Input mode handling |
| `use-multi-select-state.ts` | ~428 | Multi-select state |
| `select-option.tsx` | ~487 | Option renderer |
| `select-input-option.tsx` | ~487 | Input option renderer |
| `option-map.ts` | ~100 | Option mapping utilities |

### LogoV2 (`LogoV2/`)

| File | Lines | Description |
|------|-------|-------------|
| `LogoV2.tsx` | ~542 | Main logo component |
| `WelcomeV2.tsx` | ~432 | Welcome screen |
| `Clawd.tsx` | ~185 | Mascot animation |
| `AnimatedClawd.tsx` | ~140 | Animated mascot |
| `CondensedLogo.tsx` | ~193 | Condensed logo variant |
| `ChannelsNotice.tsx` | ~295 | Channels announcement |
| `Feed.tsx` | ~138 | Feed display |
| `FeedColumn.tsx` | ~53 | Feed column layout |
| `GuestPassesUpsell.tsx` | ~91 | Guest passes upsell |
| `OverageCreditUpsell.tsx` | ~183 | Credit upsell |

### Tasks (`tasks/`)

| File | Lines | Description |
|------|-------|-------------|
| `BackgroundTasksDialog.tsx` | ~651 | Background tasks list |
| `BackgroundTaskStatus.tsx` | ~428 | Task status display |
| `RemoteSessionDetailDialog.tsx` | ~903 | Remote session details |
| `ShellDetailDialog.tsx` | ~391 | Shell session details |
| `AsyncAgentDetailDialog.tsx` | ~298 | Agent detail view |
| `ShellProgress.tsx` | ~70 | Shell progress indicator |

### Agents (`agents/`)

| File | Lines | Description |
|------|-------|-------------|
| `AgentsMenu.tsx` | ~799 | Agents dropdown menu |
| `AgentsList.tsx` | ~439 | Agents list display |
| `ToolSelector.tsx` | ~561 | Tool selection for agents |
| `AgentEditor.tsx` | ~264 | Agent configuration editor |
| `AgentDetail.tsx` | ~235 | Agent detail view |
| `ColorPicker.tsx` | ~142 | Color picker for agents |
| `generateAgent.ts` | ~101 | Agent generation logic |
| `validateAgent.ts` | ~31 | Agent validation |

### MCP (`mcp/`)

| File | Lines | Shares |
|------|-------|--------|
| `MCPListPanel.tsx` | ~503 | MCP server list |
| `MCPSettings.tsx` | ~397 | MCP configuration |
| `MCPServerDialog.tsx` | ~200 | MCP server dialog |
| `MCPServerMultiselectDialog.tsx` | ~160 | Multi-server selection |
| `ElicitationDialog.tsx` | ~1168 | MCP elicitation flow |
| `MCPRemoteServerMenu.tsx` | ~648 | Remote server menu |

### PromptInput (`PromptInput/`)

| File | Lines | Description |
|------|-------|-------------|
| `PromptInput.tsx` | ~2338 | Main input component |
| `PromptInputFooterLeftSide.tsx` | ~516 | Footer left controls |
| `PromptInputFooter.tsx` | ~331 | Footer container |
| `PromptInputFooterSuggestions.tsx` | ~341 | Context suggestions |
| `PromptInputHelpMenu.tsx` | ~328 | Help menu display |
| `PromptInputQueuedCommands.tsx` | ~195 | Queued commands display |
| `PromptInputModeIndicator.tsx` | ~111 | Mode indicator |
| `ShimmeredInput.tsx` | ~166 | Input with shimmer |
| `VoiceIndicator.tsx` | ~108 | Voice mode indicator |
| `Notifications.tsx` | ~481 | Input notifications |
| `HistorySearchInput.tsx` | ~50 | History search |

### Settings (`Settings/`)

| File | Lines | Description |
|------|-------|-------------|
| `Config.tsx` | ~1821 | Main settings panel |

### HelpV2 (`HelpV2/`)

| File | Lines | Description |
|------|-------|-------------|
| `HelpV2.tsx` | ~200 | Help dialog |
| `Commands.tsx` | ~100 | Commands reference |
| `General.tsx` | ~22 | General help |

### HighlightedCode (`HighlightedCode/`)

| File | Lines | Description |
|------|-------|-------------|
| `HighlightedCode.tsx` | ~175 | Code syntax highlighting |
| `Fallback.tsx` | ~486 | Fallback renderer |

### FeedbackSurvey (`FeedbackSurvey/`)

| File | Lines | Description |
|------|-------|-------------|
| `FeedbackSurvey.tsx` | ~200 | Survey container |
| `FeedbackSurveyView.tsx` | ~150 | Survey view |
| `useFeedbackSurvey.tsx` | ~100 | Survey state hook |
| `TranscriptSharePrompt.tsx` | ~80 | Transcript sharing |

### TrustDialog (`TrustDialog/`)

| File | Lines | Description |
|------|-------|-------------|
| `TrustDialog.tsx` | ~200 | Trust/permission dialog |

### Spinner (`Spinner/`)

| File | Lines | Description |
|------|-------|-------------|
| `SpinnerAnimationRow.tsx` | ~100 | Animation frame |
| `TeammateSpinnerTree.tsx` | ~150 | Teammate tree view |
| `index.ts` | ~10 | Exports/constants |

### StructuredDiff (`StructuredDiff/`)

| File | Lines | Description |
|------|-------|-------------|
| `StructuredDiff.tsx` | ~250 | Diff display |
| `colorDiff.ts` | ~37 | Diff coloring |
| `Fallback.tsx` | ~486 | Fallback renderer |

### Hooks (`hooks/`)

| File | Lines | Description |
|------|-------|-------------|
| `HooksConfigMenu.tsx` | ~577 | Hooks configuration |
| `PromptDialog.tsx` | ~74 | Prompt dialog |
| `SelectEventMode.tsx` | ~135 | Event mode selector |
| `SelectHookMode.tsx` | ~129 | Hook mode selector |
| `SelectMatcherMode.tsx` | ~148 | Matcher mode selector |
| `ViewHookMode.tsx` | ~179 | Hook view mode |

### Memory (`memory/`)

| File | Lines | Description |
|------|-------|-------------|
| `MemoryFileSelector.tsx` | ~437 | Memory file selection |
| `MemoryUpdateNotification.tsx` | ~44 | Memory update notice |

### Passes (`Passes/`)

| File | Lines | Description |
|------|-------|-------------|
| (various) | — | Pass/credit UI components |

### Sandbox (`sandbox/`)

| File | Lines | Description |
|------|-------|-------------|
| `SandboxConfigTab.tsx` | ~44 | Sandbox config |
| `SandboxDoctorSection.tsx` | ~45 | Sandbox diagnostics |

### Skills (`skills/`)

| File | Lines | Description |
|------|-------|-------------|
| (various) | — | Skill-related UI |

### Teams (`teams/`)

| File | Lines | Description |
|------|-------|-------------|
| `TeamsDialog.tsx` | ~714 | Team management dialog |

### Wizard (`wizard/`)

| File | Lines | Description |
|------|-------|-------------|
| `useWizard.ts` | ~13 | Wizard state hook |
| `WizardNavigationFooter.tsx` | ~23 | Wizard footer |
| `index.ts` | ~9 | Wizard exports |

### Diff (`diff/`)

| File | Lines | Description |
|------|-------|-------------|
| (various) | — | Diff rendering utilities |

### Grove (`grove/`)

| File | Lines | Description |
|------|-------|-------------|
| `Grove.tsx` | ~462 | Grove integration UI |

### LspRecommendation (`LspRecommendation/`)

| File | Lines | Description |
|------|-------|-------------|
| `LspRecommendationMenu.tsx` | ~50 | LSP suggestions |

### ManagedSettingsSecurityDialog (`ManagedSettingsSecurityDialog/`)

| File | Lines | Description |
|------|-------|-------------|
| `ManagedSettingsSecurityDialog.tsx` | ~100 | Security settings |

### DesktopUpsell (`DesktopUpsell/`)

| File | Lines | Description |
|------|-------|-------------|
| `DesktopUpsellStartup.tsx` | ~50 | Desktop upsell |

### ClaudeCodeHint (`ClaudeCodeHint/`)

| File | Lines | Description |
|------|-------|-------------|
| `PluginHintMenu.tsx` | ~50 | Plugin hints |

---

## 3. Key Exports

### Design System Primitives

```typescript
// design-system/ThemeProvider.tsx
type ThemeContextValue = {
  themeSetting: ThemeSetting;  // 'light' | 'dark' | 'auto'
  setThemeSetting: (setting: ThemeSetting) => void;
  setPreviewTheme: (setting: ThemeSetting) => void;
  savePreview: () => void;
  cancelPreview: () => void;
  currentTheme: ThemeName;  // Resolved: 'light' | 'dark'
};

export function ThemeProvider({ children, initialState, onThemeSave }: Props)
export function useTheme(): [ThemeName, (setting: ThemeSetting) => void]
export function useThemeSetting(): ThemeSetting
export function usePreviewTheme(): { setPreviewTheme, savePreview, cancelPreview }
```

```typescript
// design-system/ThemedBox.tsx
type Props = BaseStyles & {
  borderColor?: keyof Theme | Color;
  borderTopColor?: keyof Theme | Color;
  borderBottomColor?: keyof Theme | Color;
  borderLeftColor?: keyof Theme | Color;
  borderRightColor?: keyof Theme | Color;
  backgroundColor?: keyof Theme | Color;
  ref?: Ref<DOMElement>;
  tabIndex?: number;
  autoFocus?: boolean;
  onClick?: (event: ClickEvent) => void;
  onFocus?: (event: FocusEvent) => void;
  onBlur?: (event: FocusEvent) => void;
  onKeyDown?: (event: KeyboardEvent) => void;
}

export default function ThemedBox(props: Props): ReactNode
```

```typescript
// design-system/ThemedText.tsx
type Props = {
  color?: keyof Theme | Color;
  backgroundColor?: keyof Theme;
  dimColor?: boolean;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
  strikethrough?: boolean;
  inverse?: boolean;
  wrap?: Styles['textWrap'];
  children?: ReactNode;
};

export const TextHoverColorContext: Context<keyof Theme | undefined>
export default function ThemedText(props: Props): ReactNode
```

```typescript
// design-system/Tabs.tsx
type TabsProps = {
  children: Array<React.ReactElement<TabProps>>;
  title?: string;
  color?: keyof Theme;
  defaultTab?: string;
  hidden?: boolean;
  useFullWidth?: boolean;
  selectedTab?: string;  // Controlled mode
  onTabChange?: (tabId: string) => void;
  banner?: React.ReactNode;
  disableNavigation?: boolean;
  initialHeaderFocused?: boolean;
  contentHeight?: number;
  navFromContent?: boolean;
};

export function Tabs(props: TabsProps): ReactNode
```

```typescript
// design-system/Dialog.tsx
type DialogProps = {
  title: React.ReactNode;
  subtitle?: React.ReactNode;
  children: React.ReactNode;
  onCancel: () => void;
  color?: keyof Theme;
  hideInputGuide?: boolean;
  hideBorder?: boolean;
  inputGuide?: (exitState: ExitState) => React.ReactNode;
  isCancelActive?: boolean;
};

export function Dialog(props: DialogProps): ReactNode
```

### Layout Components

```typescript
// App.tsx
type Props = {
  getFpsMetrics: () => FpsMetrics | undefined;
  stats?: StatsStore;
  initialState: AppState;
  children: React.ReactNode;
};

export function App(props: Props): ReactNode
```

```typescript
// FullscreenLayout.tsx
type Props = {
  scrollable: ReactNode;  // Content that scrolls (messages)
  bottom: ReactNode;      // Pinned bottom content (prompt, spinner)
  overlay?: ReactNode;    // Overlay content (permission requests)
  bottomFloat?: ReactNode; // Floating bottom-right content
  modal?: ReactNode;      // Slash-command dialog
  modalScrollRef?: RefObject<ScrollBoxHandle>;
  scrollRef?: RefObject<ScrollBoxHandle>;
  dividerYRef?: RefObject<number>;  // Unseen divider Y position
  hidePill?: boolean;
  hideSticky?: boolean;
  newMessageCount?: number;
  onPillClick?: () => void;
};

export const ScrollChromeContext: Context<{ setStickyPrompt: (p: StickyPrompt) => void }>

export function useUnseenDivider(messageCount: number): {
  dividerIndex: number | null;
  dividerYRef: RefObject<number | null>;
  onScrollAway: (handle: ScrollBoxHandle) => void;
  onRepin: () => void;
  jumpToNew: (handle: ScrollBoxHandle) => void;
  shiftDivider: (indexDelta: number, heightDelta: number) => void;
}

export function countUnseenAssistantTurns(messages: Message[], dividerIndex: number): number
export function computeUnseenDivider(messages: Message[], dividerIndex: number): UnseenDivider | undefined
export function FullscreenLayout(props: Props): ReactNode
```

### Select Components

```typescript
// CustomSelect/select.tsx
type OptionWithDescription<T = string> = BaseOption<T> & {
  type?: 'text';
} | (BaseOption<T> & {
  type: 'input';
  onChange: (value: string) => void;
  placeholder?: string;
  initialValue?: string;
  allowEmptySubmitToCancel?: boolean;
  showLabelWithValue?: boolean;
  labelValueSeparator?: string;
  resetCursorOnUpdate?: boolean;
});

type SelectProps<T> = {
  isDisabled?: boolean;
  disableSelection?: boolean;
  hideIndexes?: boolean;
  visibleOptionCount?: number;
  highlightText?: string;
  options: OptionWithDescription<T>[];
  defaultValue?: T;
  onCancel?: () => void;
  onChange?: (value: T) => void;
  onFocus?: (value: T) => void;
  defaultFocusValue?: T;
  layout?: 'compact' | 'expanded' | 'compact-vertical';
  inlineDescriptions?: boolean;
  onUpFromFirstItem?: () => void;
  onDownFromLastItem?: () => void;
  onInputModeToggle?: (value: T) => void;
  onOpenEditor?: (currentValue: string, setValue: (value: string) => void) => void;
  onImagePaste?: (base64Image: string, mediaType?: string, filename?: string, dimensions?: ImageDimensions, sourcePath?: string) => void;
  pastedContents?: Record<number, PastedContent>;
  onRemoveImage?: (id: number) => void;
};

export function Select<T>(props: SelectProps<T>): ReactNode
```

### Message Components

```typescript
// messages/AssistantToolUseMessage.tsx
type Props = {
  param: ToolUseBlockParam;
  addMargin: boolean;
  tools: Tools;
  commands: Command[];
  verbose: boolean;
  inProgressToolUseIDs: Set<string>;
  progressMessagesForMessage: ProgressMessage[];
  shouldAnimate: boolean;
  shouldShowDot: boolean;
  inProgressToolCallCount?: number;
  lookups: ReturnType<typeof buildMessageLookups>;
  isTranscriptMode?: boolean;
};

export function AssistantToolUseMessage(props: Props): ReactNode
```

### Prompt Input

```typescript
// PromptInput/PromptInput.tsx
type PromptInputProps = {
  // Control props
  value: string;
  onChange: (value: string) => void;
  onSubmit: (value: string) => void;
  onCancel?: () => void;
  
  // State props
  isComposing?: boolean;
  suggestions?: Suggestion[];
  history?: string[];
  queuedCommands?: QueuedCommand[];
  
  // Callbacks
  onSlashCommand?: (command: string) => void;
  onHistoryNavigate?: (direction: 'prev' | 'next') => void;
  onAttachmentAdd?: (attachment: Attachment) => void;
  onModeChange?: (mode: InputMode) => void;
  
  // Display props
  placeholder?: string;
  mode?: InputMode;
  showSuggestions?: boolean;
  showHistory?: boolean;
};

export function PromptInput(props: PromptInputProps): ReactNode
```

---

## 4. Line-by-Line Analysis

### 4.1 ThemeProvider — Theme Context with Auto-Detection

```typescript
// design-system/ThemeProvider.tsx (lines 1-170)
type ThemeContextValue = {
  themeSetting: ThemeSetting;  // User preference: 'light' | 'dark' | 'auto'
  setThemeSetting: (setting: ThemeSetting) => void;
  setPreviewTheme: (setting: ThemeSetting) => void;  // For theme picker preview
  savePreview: () => void;
  cancelPreview: () => void;
  currentTheme: ThemeName;  // Resolved theme for rendering
};

const ThemeContext = createContext<ThemeContextValue>({
  themeSetting: DEFAULT_THEME,
  setThemeSetting: () => {},
  setPreviewTheme: () => {},
  savePreview: () => {},
  cancelPreview: () => {},
  currentTheme: DEFAULT_THEME
});

export function ThemeProvider({
  children,
  initialState,
  onThemeSave = defaultSaveTheme
}: Props) {
  const [themeSetting, setThemeSetting] = useState(initialState ?? defaultInitialTheme);
  const [previewTheme, setPreviewTheme] = useState<ThemeSetting | null>(null);
  
  // Terminal theme for 'auto' mode (seeds from $COLORFGBG)
  const [systemTheme, setSystemTheme] = useState<SystemTheme>(() =>
    (initialState ?? themeSetting) === 'auto' ? getSystemThemeName() : 'dark'
  );
  
  // Preview wins while picker is open
  const activeSetting = previewTheme ?? themeSetting;
  
  // Watch for terminal theme changes when 'auto' is active
  useEffect(() => {
    if (feature('AUTO_THEME')) {
      if (activeSetting !== 'auto' || !internal_querier) return;
      let cleanup: (() => void) | undefined;
      let cancelled = false;
      void import('../../utils/systemThemeWatcher.js').then(({ watchSystemTheme }) => {
        if (cancelled) return;
        cleanup = watchSystemTheme(internal_querier, setSystemTheme);
      });
      return () => {
        cancelled = true;
        cleanup?.();
      };
    }
  }, [activeSetting, internal_querier]);
  
  const currentTheme: ThemeName = activeSetting === 'auto' ? systemTheme : activeSetting;
  
  const value = useMemo<ThemeContextValue>(() => ({
    themeSetting,
    setThemeSetting: (newSetting) => {
      setThemeSetting(newSetting);
      setPreviewTheme(null);
      if (newSetting === 'auto') {
        setSystemTheme(getSystemThemeName());  // Seed from cache
      }
      onThemeSave?.(newSetting);
    },
    setPreviewTheme: (newSetting) => {
      setPreviewTheme(newSetting);
      if (newSetting === 'auto') {
        setSystemTheme(getSystemThemeName());
      }
    },
    savePreview: () => {
      if (previewTheme !== null) {
        setThemeSetting(previewTheme);
        setPreviewTheme(null);
        onThemeSave?.(previewTheme);
      }
    },
    cancelPreview: () => {
      if (previewTheme !== null) {
        setPreviewTheme(null);
      }
    },
    currentTheme,
  }), [themeSetting, previewTheme, currentTheme, onThemeSave]);
  
  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme(): [ThemeName, (setting: ThemeSetting) => void] {
  const { currentTheme, setThemeSetting } = useContext(ThemeContext);
  return [currentTheme, setThemeSetting];
}
```

**Key Patterns**:
- **Preview mechanism**: `previewTheme` allows live preview without committing
- **Auto theme**: Watches OSC 11 escape sequences for terminal theme changes
- **Feature flags**: `feature('AUTO_THEME')` for dead-code elimination
- **Stable setters**: useMemo ensures callbacks don't change on re-render

---

### 4.2 FullscreenLayout — Sticky Scroll with Message Pill

```typescript
// FullscreenLayout.tsx (lines 86-190)
export function useUnseenDivider(messageCount: number): {
  dividerIndex: number | null;
  dividerYRef: RefObject<number | null>;
  onScrollAway: (handle: ScrollBoxHandle) => void;
  onRepin: () => void;
  jumpToNew: (handle: ScrollBoxHandle | null) => void;
  shiftDivider: (indexDelta: number, heightDelta: number) => void;
} {
  const [dividerIndex, setDividerIndex] = useState<number | null>(null);
  
  // Ref holds current count for onScrollAway to snapshot
  const countRef = useRef(messageCount);
  countRef.current = messageCount;
  
  // scrollHeight snapshot at first scroll-away — the divider's Y position
  const dividerYRef = useRef<number | null>(null);
  
  const onRepin = useCallback(() => {
    setDividerIndex(null);  // Cleared on scroll-to-bottom
  }, []);
  
  const onScrollAway = useCallback((handle: ScrollBoxHandle) => {
    const max = Math.max(0, handle.getScrollHeight() - handle.getViewportHeight());
    if (handle.getScrollTop() + handle.getPendingDelta() >= max) return;
    
    // Snapshot only on FIRST scroll-away
    if (dividerYRef.current === null) {
      dividerYRef.current = handle.getScrollHeight();
      setDividerIndex(countRef.current);
    }
  }, []);
  
  const jumpToNew = useCallback((handle: ScrollBoxHandle | null) => {
    if (!handle) return;
    handle.scrollToBottom();  // Sets stickyScroll=true
  }, []);
  
  // Sync dividerYRef with dividerIndex state
  useEffect(() => {
    if (dividerIndex === null) {
      dividerYRef.current = null;
    } else if (messageCount < dividerIndex) {
      dividerYRef.current = null;
      setDividerIndex(null);  // Count dropped (e.g., /clear)
    }
  }, [messageCount, dividerIndex]);
  
  const shiftDivider = useCallback((indexDelta: number, heightDelta: number) => {
    setDividerIndex(idx => idx === null ? null : idx + indexDelta);
    if (dividerYRef.current !== null) {
      dividerYRef.current += heightDelta;
    }
  }, []);
  
  return { dividerIndex, dividerYRef, onScrollAway, onRepin, jumpToNew, shiftDivider };
}

export function countUnseenAssistantTurns(
  messages: readonly Message[],
  dividerIndex: number
): number {
  let count = 0;
  let prevWasAssistant = false;
  
  for (let i = dividerIndex; i < messages.length; i++) {
    const m = messages[i]!;
    if (m.type === 'progress') continue;
    
    // Skip tool-use-only entries (not "new messages" to users)
    if (m.type === 'assistant' && !assistantHasVisibleText(m)) continue;
    
    const isAssistant = m.type === 'assistant';
    if (isAssistant && !prevWasAssistant) count++;
    prevWasAssistant = isAssistant;
  }
  return count;
}

export function computeUnseenDivider(
  messages: readonly Message[],
  dividerIndex: number | null
): UnseenDivider | undefined {
  if (dividerIndex === null) return undefined;
  
  // Skip progress and null-rendering attachments for anchor
  let anchorIdx = dividerIndex;
  while (anchorIdx < messages.length && 
    (messages[anchorIdx]?.type === 'progress' || isNullRenderingAttachment(messages[anchorIdx]!))) {
    anchorIdx++;
  }
  
  const uuid = messages[anchorIdx]?.uuid;
  if (!uuid) return undefined;
  
  const count = countUnseenAssistantTurns(messages, dividerIndex);
  return {
    firstUnseenUuid: uuid,
    count: Math.max(1, count)  // Floor at 1 once ANY message arrives
  };
}
```

**Why This Matters**:
- **Ref-based state**: `dividerYRef` changes don't trigger re-renders (scroll events are frequent)
- **Snapshot on first scroll**: Captures the Y position where new messages will appear
- **Turn counting**: Counts assistant "turns" (not raw messages) for user-friendly pill text
- **Shift on prepend**: When infinite-scroll prepends messages, divider shifts accordingly

---

### 4.3 Select — Keyboard-Driven Dropdown

```typescript
// CustomSelect/select.tsx (partial analysis)
type OptionWithDescription<T> = {
  label: ReactNode;
  value: T;
  description?: string;
  disabled?: boolean;
  type?: 'input';  // Input option (type to edit value)
  onChange?: (value: string) => void;
  placeholder?: string;
  initialValue?: string;
  allowEmptySubmitToCancel?: boolean;  // Submit empty = cancel
  showLabelWithValue?: boolean;
  labelValueSeparator?: string;
  resetCursorOnUpdate?: boolean;  // Auto-reset cursor on value change
};

export function Select<T>(props: SelectProps<T>): ReactNode {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [headerFocused, setHeaderFocused] = useState(true);
  const [optInCount, setOptInCount] = useState(0);
  
  // Context for child components to opt-in to keyboard handling
  const registerOptIn = () => {
    setOptInCount(c => c + 1);
    return () => setOptInCount(c => c - 1);
  };
  
  const handleTabChange = (offset: number) => {
    const newIndex = (selectedIndex + options.length + offset) % options.length;
    if (isControlled && onTabChange) {
      onTabChange(options[newIndex].value);
    } else {
      setSelectedIndex(newIndex);
    }
    setHeaderFocused(true);  // Return focus to header
  };
  
  // Keyboard navigation (up/down)
  useKeybindings({
    "select:next": () => handleTabChange(1),
    "select:previous": () => handleTabChange(-1),
  }, { context: "Select", isActive: !hidden && !disableNavigation && headerFocused });
  
  // Down arrow from header → blur header, focus content
  const handleKeyDown = (e: KeyboardEvent) => {
    if (!headerFocused || !optedIn || hidden) return;
    if (e.key === "down") {
      e.preventDefault();
      setHeaderFocused(false);
    }
  };
  
  // Allow Tab/Left/Right to switch tabs from focused content
  useKeybindings({
    "select:next": () => {
      handleTabChange(1);
      setHeaderFocused(true);
    },
    "select:previous": () => {
      handleTabChange(-1);
      setHeaderFocused(true);
    },
  }, { context: "Select", isActive: navFromContent && !headerFocused && optedIn });
  
  return (
    <TabsContext.Provider value={{ selectedTab, width, headerFocused, focusHeader, blurHeader, registerOptIn }}>
      <Box flexDirection="column" onKeyDown={handleKeyDown}>
        {/* Header row with tabs */}
        <Box>
          {title && <Text color={color}>{title}</Text>}
          {tabs.map((tab, i) => (
            <Box key={tab.id} onClick={() => handleTabChange(i - selectedIndex)}>
              <Text bold={i === selectedIndex}>{tab.label}</Text>
            </Box>
          ))}
        </Box>
        
        {/* Content area with fixed height */}
        <ScrollBox height={contentHeight}>
          {children[selectedTabIndex]}
        </ScrollBox>
      </Box>
    </TabsContext.Provider>
  );
}
```

**Key Patterns**:
- **Opt-in keyboard handling**: Child components call `registerOptIn()` to claim arrow keys
- **Focus management**: Header focused by default, blur on down arrow
- **Controlled/uncontrolled**: Supports both `selectedTab` prop and internal state
- **Fixed content height**: Prevents layout shift when switching tabs

---

### 4.4 Dialog — Modal with Keybindings

```typescript
// design-system/Dialog.tsx (lines 30-137)
export function Dialog({
  title,
  subtitle,
  children,
  onCancel,
  color = 'permission',
  hideInputGuide,
  hideBorder,
  inputGuide,
  isCancelActive = true,
}: DialogProps): ReactNode {
  const exitState = useExitOnCtrlCDWithKeybindings(undefined, undefined, isCancelActive);
  
  // ESC / 'n' to cancel (configurable via isCancelActive)
  useKeybinding("confirm:no", onCancel, {
    context: "Confirmation",
    isActive: isCancelActive,
  });
  
  const defaultInputGuide = exitState.pending ? (
    <Text>Press {exitState.keyName} again to exit</Text>
  ) : (
    <Byline>
      <KeyboardShortcutHint shortcut="Enter" action="confirm" />
      <ConfigurableShortcutHint action="confirm:no" context="Confirmation" fallback="Esc" description="cancel" />
    </Byline>
  );
  
  const content = (
    <>
      <Box flexDirection="column" gap={1}>
        <Box flexDirection="column">
          <Text bold color={color}>{title}</Text>
          {subtitle && <Text dimColor>{subtitle}</Text>}
        </Box>
        {children}
      </Box>
      {!hideInputGuide && (
        <Box marginTop={1}>
          <Text dimColor italic>
            {inputGuide ? inputGuide(exitState) : defaultInputGuide}
          </Text>
        </Box>
      )}
    </>
  );
  
  if (hideBorder) {
    return content;
  }
  
  return <Pane color={color}>{content}</Pane>;
}
```

**Key Patterns**:
- **Ctrl+C/D exit pattern**: `useExitOnCtrlCDWithKeybindings` handles double-press confirmation
- **Configurable keybindings**: `isCancelActive` disables when child input needs those keys
- **Custom input guide**: Allows callers to override footer hints
- **Optional border**: `hideBorder` for inline dialogs

---

### 4.5 AssistantToolUseMessage — Tool Call Rendering

```typescript
// messages/AssistantToolUseMessage.tsx (lines 35-200)
export function AssistantToolUseMessage({
  param,
  addMargin,
  tools,
  commands,
  verbose,
  inProgressToolUseIDs,
  progressMessagesForMessage,
  shouldAnimate,
  shouldShowDot,
  inProgressToolCallCount,
  lookups,
  isTranscriptMode,
}: Props): ReactNode {
  // Parse tool input against schema
  const parsed = useMemo(() => {
    if (!tools) return null;
    const tool = findToolByName(tools, param.name);
    if (!tool) return null;
    const input = tool.inputSchema.safeParse(param.input);
    const data = input.success ? input.data : undefined;
    return {
      tool,
      input,
      userFacingToolName: tool.userFacingName(data),
      userFacingToolNameBackgroundColor: tool.userFacingNameBackgroundColor?.(data),
      isTransparentWrapper: tool.isTransparentWrapper?.() ?? false,
    };
  }, [param.input, param.name, tools]);
  
  if (!parsed) {
    logError(new Error(tools ? `Tool ${param.name} not found` : `Tools array is undefined`));
    return null;
  }
  
  const { tool: tool_0, input: input_0, userFacingToolName, isTransparentWrapper } = parsed;
  
  // Check if tool use is resolved (result received)
  const isResolved = lookups.resolvedToolUseIDs.has(param.id);
  
  // Check if tool use is queued (not started)
  const isQueued = !inProgressToolUseIDs.has(param.id) && !isResolved;
  
  // Transparent wrappers (e.g., agents) don't render while queued/resolved
  if (isTransparentWrapper) {
    if (isQueued || isResolved) return null;
    return (
      <Box backgroundColor={bg}>
        {renderToolUseProgressMessage(tool_0, tools, lookups, param.id, progressMessagesForMessage, {
          verbose,
          inProgressToolCallCount,
          isTranscriptMode,
        }, terminalSize)}
      </Box>
    );
  }
  
  if (userFacingToolName === "") return null;
  
  const renderedToolUseMessage = input_0.success 
    ? renderToolUseMessage(tool_0, input_0.data, { theme, verbose, commands })
    : null;
  
  if (renderedToolUseMessage === null) return null;
  
  // Render status indicator (dot, spinner, or error)
  const statusIndicator = shouldShowDot && (
    isQueued ? (
      <Box minWidth={2}>
        <Text dimColor={isQueued}>{BLACK_CIRCLE}</Text>
      </Box>
    ) : (
      <ToolUseLoader 
        shouldAnimate={shouldAnimate}
        isUnresolved={!isResolved}
        isError={lookups.erroredToolUseIDs.has(param.id)}
      />
    )
  );
  
  return (
    <Box flexDirection="column" gap={addMargin ? 1 : 0}>
      <Box>
        {statusIndicator}
        <Text bold backgroundColor={userFacingToolNameBackgroundColor}>
          {userFacingToolName}
        </Text>
        {renderedToolUseMessage}
      </Box>
    </Box>
  );
}
```

**Key Patterns**:
- **Schema validation**: Parses tool input against Zod schema before rendering
- **Transparent wrappers**: Some tools (agents) hide their wrapper UI
- **Status tracking**: Queued → In Progress → Resolved/Errored
- **User-facing names**: Tools provide custom display names with optional background colors

---

## 5. Component Categories

### 5.1 Layout Components

| Component | Purpose | Key Props |
|-----------|---------|-----------|
| `App` | Top-level provider wrapper | `getFpsMetrics`, `stats`, `initialState` |
| `FullscreenLayout` | Sticky-scroll REPL layout | `scrollable`, `bottom`, `modal`, `dividerYRef` |
| `VirtualMessageList` | Virtualized message rendering | `messages`, `dividerIndex`, `height` |
| `TagTabs` | Tab-based navigation | `tabs`, `selectedTab`, `onTabChange` |

### 5.2 Dialog Components

| Component | Purpose | Key Props |
|-----------|---------|-----------|
| `Dialog` | Generic dialog container | `title`, `onCancel`, `isCancelActive` |
| `Pane` | Bordered pane container | `color`, `children` |
| `BridgeDialog` | Bridge/remote session setup | `onComplete`, `defaultTab` |
| `GlobalSearchDialog` | Search across sessions | `onSelect`, `onCancel` |
| `ExportDialog` | Session export | `onExport`, `format` |
| `Settings dialogs` | Various settings panels | Varies by dialog |

### 5.3 Interactive Components

| Component | Purpose | Key Props |
|-----------|---------|-----------|
| `Select` | Dropdown selection | `options`, `onChange`, `layout` |
| `PromptInput` | Main chat input | `value`, `onChange`, `onSubmit` |
| `BaseTextInput` | Basic text input | `value`, `onChange`, `placeholder` |
| `ModelPicker` | Model selection | `selectedModel`, `onSelect` |
| `ThemePicker` | Theme selection | `selectedTheme`, `onSelect` |
| `Tabs` | Tabbed content | `children`, `defaultTab`, `contentHeight` |

### 5.4 Display Components

| Component | Purpose | Key Props |
|-----------|---------|-----------|
| `Message` | Individual message | `message`, `tools`, `commands` |
| `Messages` | Message list container | `messages`, `dividerIndex` |
| `Spinner` | Loading indicator | `mode`, `overrideMessage`, `verbose` |
| `ProgressBar` | Progress bar | `progress`, `label` |
| `StatusLine` | Bottom status bar | `tokens`, `cost`, `model` |
| `EffortIndicator` | Effort level display | `effort` |

### 5.5 Logo/Branding Components

| Component | Purpose | Key Props |
|-----------|---------|-----------|
| `LogoV2` | Main logo | `animate`, `condensed` |
| `WelcomeV2` | Welcome screen | `onDismiss` |
| `Clawd` | Mascot display | `variant` |
| `AnimatedClawd` | Animated mascot | `playing` |
| `Feed` | Activity feed | `items` |

---

## 6. State Management

### 6.1 AppState Integration

```typescript
// state/AppState.tsx (summary)
type AppState = {
  // Session state
  messages: Message[];
  tasks: Record<string, Task>;
  viewingAgentTaskId?: string;
  expandedView?: 'tasks' | 'teammates';
  
  // UI state
  isBriefOnly: boolean;
  selectedMessageIndex?: number;
  permissionMode: 'always_ask' | 'auto' | 'plan';
  
  // Settings
  effortValue: Effort;
  model: string;
  theme: ThemeSetting;
  
  // Derived selectors
  viewingAgentTaskId: (s: AppState) => string | undefined;
  expandedView: (s: AppState) => 'tasks' | 'teammates' | undefined;
};

export function AppStateProvider({ initialState, onChangeAppState, children })
export function useAppState<T>(selector: (s: AppState) => T): T
```

**Components that consume AppState**:
- `Spinner`: Reads `tasks`, `viewingAgentTaskId`, `expandedView`, `effortValue`
- `Message`: Reads `messages`, `selectedMessageIndex`
- `FullscreenLayout`: Reads `stickyPrompt` via context
- `ModelPicker`, `ThemePicker`: Read/write settings

### 6.2 Context Providers

```typescript
// Context hierarchy (top to bottom)
<App
  getFpsMetrics={...}
  stats={...}
  initialState={...}
>
  <FpsMetricsProvider>     // FPS tracking
    <StatsProvider>        // Token/cost stats
      <AppStateProvider>   // Global app state
        <ThemeProvider>    // Theme context
          <ModalContext>   // Modal dialog state
            <PromptOverlayContext>  // Overlay positioning
              {children}
```

### 6.3 Key Hooks

| Hook | Purpose | Returns |
|------|---------|---------|
| `useTheme()` | Theme context | `[ThemeName, setter]` |
| `useThemeSetting()` | Raw theme setting | `ThemeSetting` |
| `useAppState(selector)` | AppState selector | `T` (selected value) |
| `useTerminalSize()` | Terminal dimensions | `{ rows, columns }` |
| `useKeybindings(bindings, options)` | Register keybindings | `void` |
| `useModalScrollRef()` | Modal scroll ref | `RefObject<ScrollBoxHandle>` |
| `usePromptOverlay()` | Overlay context | `OverlayContextValue` |
| `useExitOnCtrlCDWithKeybindings()` | Ctrl+C/D exit | `ExitState` |

---

## 7. Key Patterns

### 7.1 React Compiler Integration

All components use React Compiler for automatic memoization:

```typescript
import { c as _c } from "react/compiler-runtime";

export function Component(props) {
  const $ = _c(10);  // Memo cache slot array
  
  if ($[0] !== props.value) {
    // Compute expensive value
    $[0] = props.value;
    $[1] = computedValue;
  }
  
  return <Box>{$[1]}</Box>;
}
```

**Benefits**:
- Automatic dependency tracking
- No manual `useMemo` calls needed
- Props changes trigger selective re-renders

### 7.2 Theme-Aware Styling

```typescript
// All colors resolve through theme
function ThemedBox({ borderColor, backgroundColor, children }) {
  const [themeName] = useTheme();
  const theme = getTheme(themeName);
  
  const resolvedBorderColor = resolveColor(borderColor, theme);
  const resolvedBackgroundColor = resolveColor(backgroundColor, theme);
  
  return (
    <Box borderColor={resolvedBorderColor} backgroundColor={resolvedBackgroundColor}>
      {children}
    </Box>
  );
}

function resolveColor(color: keyof Theme | Color | undefined, theme: Theme): Color {
  if (!color) return undefined;
  if (color.startsWith('rgb(') || color.startsWith('#')) return color as Color;
  return theme[color as keyof Theme] as Color;
}
```

### 7.3 Keyboard Navigation

```typescript
// Keybinding registration pattern
useKeybindings({
  "action:name": () => handleAction(),
}, {
  context: "ComponentName",
  isActive: !disabled && focused,
});

// Configurable shortcuts via keybindings.json
// {
//   "keybindings": {
//     "confirm:no": ["escape", "n"],
//     "select:next": ["down", "j"],
//     "select:previous": ["up", "k"],
//   }
// }
```

### 7.4 Accessibility Patterns

```typescript
// Focus management in Tabs
const [headerFocused, setHeaderFocused] = useState(initialHeaderFocused);

const focusHeader = () => setHeaderFocused(true);
const blurHeader = () => setHeaderFocused(false);

// Down arrow from header → focus content
const handleKeyDown = (e: KeyboardEvent) => {
  if (headerFocused && e.key === "down") {
    e.preventDefault();
    blurHeader();
  }
};

// Tab/Left/Right from content → switch tabs
useKeybindings({
  "tabs:next": () => {
    handleTabChange(1);
    focusHeader();  // Return focus to header
  },
}, { isActive: navFromContent && !headerFocused });
```

### 7.5 Virtual Scrolling

```typescript
// VirtualMessageList pattern
function VirtualMessageList({ messages, height }) {
  const [scrollTop, setScrollTop] = useState(0);
  const virtualItems = useMemo(() => {
    // Calculate which items are visible
    const startIndex = Math.floor(scrollTop / ITEM_HEIGHT);
    const visibleCount = Math.ceil(height / ITEM_HEIGHT);
    return messages.slice(startIndex, startIndex + visibleCount);
  }, [messages, scrollTop, height]);
  
  return (
    <ScrollBox scrollTop={scrollTop} onScroll={setScrollTop}>
      <Box height={messages.length * ITEM_HEIGHT}>
        {virtualItems.map(item => (
          <Box key={item.id} top={item.index * ITEM_HEIGHT}>
            <Message message={item} />
          </Box>
        ))}
      </Box>
    </ScrollBox>
  );
}
```

---

## 8. Integration Points

### 8.1 With `ink/` Module (Terminal Rendering)

| Component | Ink Primitive | Purpose |
|-----------|---------------|---------|
| `ThemedBox` | `<Box>` | Layout container |
| `ThemedText` | `<Text>` | Text rendering |
| `ScrollBox` | `<Box>` + scroll logic | Scrollable area |
| All components | `useTerminalSize()` | Responsive layouts |
| Interactive components | `useKeybindings()` | Keyboard handling |
| Input components | `useStdin()` | Input handling |

### 8.2 With `hooks/` Module

| Component | Hook | Purpose |
|-----------|------|---------|
| All components | `useTerminalSize()` | Terminal dimensions |
| Interactive components | `useKeybindings()` | Keyboard shortcuts |
| Input components | `useStdin()` | Stdin handling |
| Modal components | `useModalScrollRef()` | Modal scroll |
| PromptInput | `usePromptOverlay()` | Overlay positioning |
| ThemePicker | `useTheme()`, `useThemeSetting()` | Theme selection |

### 8.3 With `state/` Module

| Component | State Selector | Purpose |
|-----------|---------------|---------|
| `Spinner` | `s.tasks`, `s.viewingAgentTaskId` | Task status |
| `Message` | `s.messages` | Message list |
| `ModelPicker` | `s.model` | Current model |
| `ThemePicker` | `s.theme` | Current theme |
| `FullscreenLayout` | `s.stickyPrompt` | Sticky header |
| `StatusLine` | `s.tokenBudget`, `s.turnOutputTokens` | Token display |

### 8.4 With `context/` Module

| Component | Context | Purpose |
|-----------|---------|---------|
| All components | `ThemeContext` | Theme values |
| Modal components | `ModalContext` | Modal state |
| PromptInput | `PromptOverlayContext` | Overlay positioning |
| `App` children | `StatsContext` | Token/cost stats |
| `App` children | `FpsMetricsContext` | FPS tracking |

---

## 9. Error Handling

### 9.1 Component Error Boundaries

```typescript
// SentryErrorBoundary.ts
export function SentryErrorBoundary({ children }) {
  return (
    <ErrorBoundary fallbackRender={({ error }) => (
      <Box>
        <Text color="red">Error: {error.message}</Text>
      </Box>
    )}>
      {children}
    </ErrorBoundary>
  );
}
```

### 9.2 Tool Use Error Handling

```typescript
// AssistantToolUseMessage.tsx
const parsed = useMemo(() => {
  const tool = findToolByName(tools, param.name);
  if (!tool) {
    logError(new Error(`Tool ${param.name} not found`));
    return null;
  }
  const input = tool.inputSchema.safeParse(param.input);
  if (!input.success) {
    logError(new Error(`Invalid tool input: ${input.error.message}`));
    return null;
  }
  return { tool, input, ... };
}, [param.input, param.name, tools]);

if (!parsed) return null;  // Graceful degradation
```

### 9.3 Permission Prompt Fallback

```typescript
// PermissionPrompt.tsx
if (!permissionRequest) {
  return <FallbackPermissionRequest />;
}
```

---

## 10. Testing Considerations

### 10.1 Mocking Theme Context

```typescript
// Test setup
function renderWithTheme(component, theme = 'dark') {
  return render(
    <ThemeProvider initialState={theme}>
      {component}
    </ThemeProvider>
  );
}

// Test
test('ThemedBox resolves theme colors', () => {
  const { container } = renderWithTheme(<ThemedBox borderColor="permission" />);
  // Assert border color resolved correctly
});
```

### 10.2 Mocking Terminal Size

```typescript
// Mock useTerminalSize
jest.mock('../../hooks/useTerminalSize', () => ({
  useTerminalSize: () => ({ rows: 24, columns: 80 }),
}));
```

### 10.3 Testing Keyboard Navigation

```typescript
test('Select navigates with arrow keys', async () => {
  const { container } = render(
    <Select options={options} onChange={handleChange} />
  );
  
  // Simulate down arrow
  await userEvent.keyboard('{ArrowDown}');
  
  // Assert selection changed
  expect(handleChange).toHaveBeenCalledWith(options[1].value);
});
```

---

## 11. Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CLAUDE_CODE_NO_FLICKER` | Enable fullscreen mode | `0` (ants), `1` (external) |
| `CLAUDE_CODE_BRIEF` | Enable brief spinner mode | `false` |
| `COLORFGBG` | Terminal background color (for auto theme) | — |
| `CLAUDE_CODE_WORKER_EPOCH` | Worker epoch for CCR v2 | Required for CCR |

---

## 12. Summary

The `components/` module is a **comprehensive React UI library** for Claude Code's terminal interface, featuring:

1. **Design System**: Theme-aware primitives (`ThemedBox`, `ThemedText`), layout components (`Tabs`, `Dialog`), and visual elements (`ProgressBar`, `Spinner`)

2. **Message Rendering**: Full chat transcript support with assistant/user messages, tool calls, attachments, and system notifications

3. **Interactive Inputs**: `PromptInput` (main chat), `Select` (dropdowns), `TextInput`, and specialized pickers

4. **Dialog System**: Permission prompts, settings dialogs, onboarding flows with consistent keybinding patterns

5. **State Integration**: Deep integration with `AppState`, `context/`, and `hooks/` modules for reactive UI updates

6. **Terminal Optimization**: Virtual scrolling, sticky headers, keyboard-first navigation, Ink-based rendering

The module comprises **389 TypeScript/TSX files** organized into 50+ subdirectories, representing the complete UI surface for Claude Code's terminal-based conversational interface.

---

## 13. Deep Dive: Core Component Implementations

### 13.1 Markdown.tsx — Streaming Markdown with Token Caching

```typescript
// Markdown.tsx
import { marked, type Token } from 'marked';
import { hashContent } from '../utils/hash.js';

// LRU token cache for marked.lexer (500 max entries)
const TOKEN_CACHE_MAX = 500;
const tokenCache = new Map<string, Token[]>();

// Fast syntax detection regex - single pass for all markdown markers
const MD_SYNTAX_RE = /[#*`|[>\-_~]|\n\n|^\d+\. |\n\d+\. /;

function hasMarkdownSyntax(s: string): boolean {
  // Sample first 500 chars - markdown usually appears early
  return MD_SYNTAX_RE.test(s.length > 500 ? s.slice(0, 500) : s);
}

function cachedLexer(content: string): Token[] {
  // Fast path: plain text with no markdown syntax
  if (!hasMarkdownSyntax(content)) {
    return [{
      type: 'paragraph',
      raw: content,
      text: content,
      tokens: [{ type: 'text', raw: content, text: content }]
    } as Token];
  }
  
  const key = hashContent(content);
  const hit = tokenCache.get(key);
  if (hit) {
    // Promote to MRU - prevents FIFO eviction during scrolling
    tokenCache.delete(key);
    tokenCache.set(key, hit);
    return hit;
  }
  
  const tokens = marked.lexer(content);
  if (tokenCache.size >= TOKEN_CACHE_MAX) {
    // LRU eviction: drop oldest
    const first = tokenCache.keys().next().value;
    if (first !== undefined) tokenCache.delete(first);
  }
  tokenCache.set(key, tokens);
  return tokens;
}

export function MarkdownBody({ children, dimColor, highlight }): ReactNode {
  const [theme] = useTheme();
  configureMarked();
  
  const tokens = cachedLexer(stripPromptXMLTags(children));
  const elements = [];
  let nonTableContent = "";
  
  const flushNonTableContent = () => {
    if (nonTableContent) {
      elements.push(
        <Ansi key={elements.length} dimColor={dimColor}>
          {nonTableContent.trim()}
        </Ansi>
      );
      nonTableContent = "";
    }
  };
  
  for (const token of tokens) {
    if (token.type === "table") {
      flushNonTableContent();
      elements.push(
        <MarkdownTable 
          key={elements.length} 
          token={token as Tokens.Table} 
          highlight={highlight} 
        />
      );
    } else {
      nonTableContent += formatToken(token, theme, 0, null, null, highlight);
    }
  }
  flushNonTableContent();
  
  return <Box flexDirection="column" gap={1}>{elements}</Box>;
}

// Streaming markdown - splits at last block boundary for incremental parsing
export function StreamingMarkdown({ children }: { children: string }): ReactNode {
  'use no memo';  // Opt out of React Compiler for ref-based boundary tracking
  configureMarked();
  
  // stablePrefixRef tracks the last complete block boundary
  // Only the final block is re-parsed on each stream delta
  const stablePrefixRef = useRef<string>("");
  
  const tokens = marked.lexer(children);
  // Find last complete block boundary...
  // (implementation continues with boundary tracking)
}
```

**Key Optimizations**:
- **3ms savings**: Skip `marked.lexer` for plain text (most short messages)
- **LRU cache**: 500 entries, MRU promotion prevents scroll-induced eviction
- **Content hashing**: Keys by hash, not content string (RSS regression fix #24180)
- **Streaming boundary**: Monotonic stable prefix enables incremental re-parse

---

### 13.2 BaseTextInput.tsx — Input with Cursor Parking and Paste Handling

```typescript
// BaseTextInput.tsx
import { useDeclaredCursor } from '../ink/hooks/use-declared-cursor.js';
import { usePasteHandler } from '../hooks/usePasteHandler.js';
import { Ansi, Box, Text, useInput } from '../ink.js';

type BaseTextInputComponentProps = BaseTextInputProps & {
  inputState: BaseInputState;
  children?: React.ReactNode;
  terminalFocus: boolean;
  highlights?: TextHighlight[];
  invert?: (text: string) => string;
  hidePlaceholderText?: boolean;
};

export function BaseTextInput({
  inputState,
  children,
  terminalFocus,
  invert,
  hidePlaceholderText,
  ...props
}: BaseTextInputComponentProps): ReactNode {
  const { onInput, renderedValue, cursorLine, cursorColumn } = inputState;
  
  // Park native terminal cursor at input caret position
  // Enables CKJ input (IME) and screen reader tracking
  const cursorRef = useDeclaredCursor({
    line: cursorLine,
    column: cursorColumn,
    active: Boolean(props.focus && props.showCursor && terminalFocus),
  });
  
  // Handle paste with Enter suppression during paste operation
  const { wrappedOnInput, isPasting } = usePasteHandler({
    onPaste: props.onPaste,
    onInput: (input, key) => {
      // Prevent Enter from triggering submit during paste
      if (isPasting && key.return) return;
      onInput(input, key);
    },
    onImagePaste: props.onImagePaste,
  });
  
  // Notify parent of paste state changes
  useEffect(() => {
    if (props.onIsPastingChange) {
      props.onIsPastingChange(isPasting);
    }
  }, [isPasting, props.onIsPastingChange]);
  
  // Placeholder rendering with invert support
  const { showPlaceholder, renderedPlaceholder } = renderPlaceholder({
    placeholder: props.placeholder,
    value: props.value,
    showCursor: props.showCursor,
    focus: props.focus,
    terminalFocus,
    invert,
    hidePlaceholderText,
  });
  
  useInput(wrappedOnInput, { isActive: props.focus });
  
  // Argument hint for slash commands (e.g., "/model [name]")
  const commandWithoutArgs = 
    (props.value && props.value.trim().indexOf(" ") === -1) ||
    (props.value && props.value.endsWith(" "));
  
  const showArgumentHint = Boolean(
    props.argumentHint && 
    props.value && 
    commandWithoutArgs && 
    props.value.startsWith("/")
  );
  
  // Filter highlights that contain cursor position
  const cursorFiltered = props.showCursor && props.highlights
    ? props.highlights.filter(h => 
        h.dimColor || 
        props.cursorOffset < h.start || 
        props.cursorOffset >= h.end
      )
    : props.highlights;
  
  // Adjust highlights for viewport windowing
  const { viewportCharOffset, viewportCharEnd } = inputState;
  const filteredHighlights = cursorFiltered && viewportCharOffset > 0
    ? cursorFiltered
        .filter(h => h.end > viewportCharOffset && h.start < viewportCharEnd)
        .map(h => ({
          ...h,
          start: Math.max(0, h.start - viewportCharOffset),
          end: h.end - viewportCharOffset,
        }))
    : cursorFiltered;
  
  const hasHighlights = filteredHighlights && filteredHighlights.length > 0;
  
  if (hasHighlights) {
    return (
      <Box ref={cursorRef}>
        <HighlightedInput 
          text={renderedValue} 
          highlights={filteredHighlights} 
        />
        {showArgumentHint && (
          <Text dimColor>
            {props.value?.endsWith(" ") ? "" : " "}
            {props.argumentHint}
          </Text>
        )}
        {children}
      </Box>
    );
  }
  
  return (
    <Box ref={cursorRef}>
      <Text wrap="truncate-end" dimColor={props.dimColor}>
        {showPlaceholder && props.placeholderElement
          ? props.placeholderElement
          : showPlaceholder && renderedPlaceholder
            ? <Ansi>{renderedPlaceholder}</Ansi>
            : <Ansi>{renderedValue}</Ansi>
        }
        {showArgumentHint && (
          <Text dimColor>
            {props.value?.endsWith(" ") ? "" : " "}
            {props.argumentHint}
          </Text>
        )}
        {children}
      </Text>
    </Box>
  );
}
```

**Key Patterns**:
- **Cursor parking**: Native IME and screen reader support via `useDeclaredCursor`
- **Paste suppression**: Blocks Enter during paste to prevent accidental submit
- **Viewport windowing**: Highlights adjusted for horizontal scroll offset
- **Argument hints**: Contextual help for slash commands

---

### 13.3 FullscreenLayout.tsx — Sticky Scroll with Message Pill

```typescript
// FullscreenLayout.tsx
const MODAL_TRANSCRIPT_PEEK = 2;  // Rows visible above modal divider

export const ScrollChromeContext = createContext<{
  setStickyPrompt: (p: StickyPrompt | null) => void;
}>({ setStickyPrompt: () => {} });

/**
 * Tracks "N new messages" divider position while user is scrolled up.
 * Snapshots messageCount AND scrollHeight on first scroll-away.
 */
export function useUnseenDivider(messageCount: number): {
  dividerIndex: number | null;       // Index in messages[] where divider renders
  dividerYRef: RefObject<number>;    // scrollHeight at snapshot (Y position)
  onScrollAway: (handle: ScrollBoxHandle) => void;
  onRepin: () => void;
  jumpToNew: (handle: ScrollBoxHandle) => void;
  shiftDivider: (indexDelta: number, heightDelta: number) => void;
} {
  const [dividerIndex, setDividerIndex] = useState<number | null>(null);
  
  // Ref holds current count for onScrollAway to snapshot
  // Written in render body (not useEffect) for fresh value on wheel events
  const countRef = useRef(messageCount);
  countRef.current = messageCount;
  
  // scrollHeight snapshot - divider's Y in content coordinates
  // null = pinned to bottom
  const dividerYRef = useRef<number | null>(null);
  
  const onRepin = useCallback(() => {
    // Don't clear dividerYRef here - wheel event racing in same stdin batch
    // would see null and re-snapshot, overriding setDividerIndex(null)
    setDividerIndex(null);
  }, []);
  
  const onScrollAway = useCallback((handle: ScrollBoxHandle) => {
    const max = Math.max(0, 
      handle.getScrollHeight() - handle.getViewportHeight()
    );
    
    // Account for pendingDelta (trackpad momentum scroll)
    if (handle.getScrollTop() + handle.getPendingDelta() >= max) return;
    
    // Snapshot only on FIRST scroll-away
    if (dividerYRef.current === null) {
      dividerYRef.current = handle.getScrollHeight();
      setDividerIndex(countRef.current);
    }
  }, []);
  
  const jumpToNew = useCallback((handle: ScrollBoxHandle) => {
    if (!handle) return;
    // scrollToBottom sets stickyScroll=true for useVirtualScroll pinning
    handle.scrollToBottom();
  }, []);
  
  // Sync dividerYRef with dividerIndex state
  useEffect(() => {
    if (dividerIndex === null) {
      dividerYRef.current = null;
    } else if (messageCount < dividerIndex) {
      // Count dropped (e.g., /clear) - divider would point at nothing
      dividerYRef.current = null;
      setDividerIndex(null);
    }
  }, [messageCount, dividerIndex]);
  
  const shiftDivider = useCallback((indexDelta: number, heightDelta: number) => {
    setDividerIndex(idx => idx === null ? null : idx + indexDelta);
    if (dividerYRef.current !== null) {
      dividerYRef.current += heightDelta;
    }
  }, []);
  
  return { dividerIndex, dividerYRef, onScrollAway, onRepin, jumpToNew, shiftDivider };
}

/**
 * Counts assistant "turns" (what users perceive as "a message from Claude").
 * Skips tool-use-only entries like progress messages.
 */
export function countUnseenAssistantTurns(
  messages: readonly Message[], 
  dividerIndex: number
): number {
  let count = 0;
  let prevWasAssistant = false;
  
  for (let i = dividerIndex; i < messages.length; i++) {
    const m = messages[i]!;
    if (m.type === 'progress') continue;
    
    // Skip tool-use-only entries
    if (m.type === 'assistant' && !assistantHasVisibleText(m)) continue;
    
    const isAssistant = m.type === 'assistant';
    if (isAssistant && !prevWasAssistant) count++;
    prevWasAssistant = isAssistant;
  }
  return count;
}
```

**Why Ref-based State**:
- **dividerYRef**: Changes don't trigger re-renders (scroll events are高频)
- **countRef**: Fresh value for wheel events between render and effect flush
- **Snapshot pattern**: Pill subscribes via `useSyncExternalStore` with boolean snapshot

---

### 13.4 Spinner.tsx — Animation with Thinking Status and Teammate Tree

```typescript
// Spinner.tsx
const SPINNER_FRAMES = [
  ...DEFAULT_CHARACTERS, 
  ...[...DEFAULT_CHARACTERS].reverse()
];

type Props = {
  mode: SpinnerMode;
  loadingStartTimeRef: RefObject<number>;
  totalPausedMsRef: RefObject<number>;
  pauseStartTimeRef: RefObject<number | null>;
  spinnerTip?: string;
  responseLengthRef: RefObject<number>;
  overrideColor?: keyof Theme | null;
  overrideMessage?: string | null;
  verbose: boolean;
  hasActiveTools?: boolean;
  leaderIsIdle?: boolean;  // Suppress stall-red when only teammates running
};

export function SpinnerWithVerb(props: Props): ReactNode {
  const isBriefOnly = useAppState(s => s.isBriefOnly);
  const viewingAgentTaskId = useAppState(s => s.viewingAgentTaskId);
  
  // Kairos brief spinner feature flags
  const briefEnvEnabled = feature('KAIROS') || feature('KAIROS_BRIEF')
    ? useMemo(() => isEnvTruthy(process.env.CLAUDE_CODE_BRIEF), [])
    : false;
  
  // Runtime gate for brief mode
  if ((feature('KAIROS') || feature('KAIROS_BRIEF')) &&
      (getKairosActive() || getUserMsgOptIn()) &&
      (briefEnvEnabled || getFeatureValue_CACHED_MAY_BE_STALE('tengu_kairos_brief', false)) &&
      isBriefOnly && 
      !viewingAgentTaskId) {
    return <BriefSpinner mode={props.mode} overrideMessage={props.overrideMessage} />;
  }
  
  return <SpinnerWithVerbInner {...props} />;
}

function SpinnerWithVerbInner(props: Props): ReactNode {
  const settings = useSettings();
  const reducedMotion = settings.prefersReducedMotion ?? false;
  
  const tasks = useAppState(s => s.tasks);
  const viewingAgentTaskId = useAppState(s => s.viewingAgentTaskId);
  const expandedView = useAppState(s => s.expandedView);
  const showExpandedTodos = expandedView === 'tasks';
  const showSpinnerTree = expandedView === 'teammates';
  
  // Foregrounded teammate when viewing teammate transcript
  const foregroundedTeammate = viewingAgentTaskId 
    ? getViewedTeammateTask({ viewingAgentTaskId, tasks })
    : undefined;
  
  const { columns } = useTerminalSize();
  const tasksV2 = useTasksV2();
  
  // Thinking status tracking with minimum 2s display
  const [thinkingStatus, setThinkingStatus] = useState<'thinking' | number | null>(null);
  const thinkingStartRef = useRef<number | null>(null);
  
  useEffect(() => {
    let showDurationTimer: NodeJS.Timeout | null = null;
    let clearStatusTimer: NodeJS.Timeout | null = null;
    
    if (props.mode === 'thinking') {
      if (thinkingStartRef.current === null) {
        thinkingStartRef.current = Date.now();
        setThinkingStatus('thinking');
      }
    } else if (thinkingStartRef.current !== null) {
      const duration = Date.now() - thinkingStartRef.current;
      const elapsed = Date.now() - thinkingStartRef.current;
      const remaining = Math.max(0, 2000 - elapsed);
      thinkingStartRef.current = null;
      
      // Show duration after remaining time (ensures 2s minimum)
      const showDuration = () => {
        setThinkingStatus(duration);
        clearStatusTimer = setTimeout(() => setThinkingStatus(null), 2000);
      };
      
      if (remaining > 0) {
        showDurationTimer = setTimeout(showDuration, remaining);
      } else {
        showDuration();
      }
    }
    
    return () => {
      showDurationTimer && clearTimeout(showDurationTimer);
      clearStatusTimer && clearTimeout(clearStatusTimer);
    };
  }, [props.mode]);
  
  // Task aggregation for display
  const activeTasks = Object.values(tasks).filter(t => 
    !isBackgroundTask(t) && 
    !isInProcessTeammateTask(t)
  );
  
  const teammateTasks = getAllInProcessTeammateTasks(tasks);
  const hasTeammateActivity = teammateTasks.length > 0;
  
  // Render logic branches based on state...
  if (showSpinnerTree && hasTeammateActivity) {
    return <TeammateSpinnerTree tasks={teammateTasks} />;
  }
  
  if (showExpandedTodos) {
    return <TaskListV2 tasks={tasksV2} columns={columns} />;
  }
  
  // Standard spinner with verb display
  return (
    <MessageResponse>
      <SpinnerAnimationRow 
        frames={SPINNER_FRAMES}
        mode={props.mode}
        overrideColor={props.overrideColor}
        reducedMotion={reducedMotion}
        thinkingStatus={thinkingStatus}
      />
    </MessageResponse>
  );
}
```

**Key Features**:
- **Brief mode**: Kairos feature flag for minimal spinner
- **Thinking status**: 2s minimum display prevents UI jank
- **Teammate tree**: Expanded view shows agent swarm activity
- **Stall detection**: `leaderIsIdle` suppresses false-positive stall red

---

### 13.5 BridgeDialog.tsx — Remote Session with QR Code

```typescript
// BridgeDialog.tsx
import { qrToString } from 'qrcode';
import { Dialog } from './design-system/Dialog.js';
import { saveGlobalConfig } from '../utils/config.js';

type Props = { onDone: () => void };

export function BridgeDialog({ onDone }: Props): ReactNode {
  useRegisterOverlay("bridge-dialog");
  
  const connected = useAppState(s => s.replBridgeConnected);
  const sessionActive = useAppState(s => s.replBridgeSessionActive);
  const reconnecting = useAppState(s => s.replBridgeReconnecting);
  const connectUrl = useAppState(s => s.replBridgeConnectUrl);
  const sessionUrl = useAppState(s => s.replBridgeSessionUrl);
  const error = useAppState(s => s.replBridgeError);
  const explicit = useAppState(s => s.replBridgeExplicit);
  
  const [showQR, setShowQR] = useState(false);
  const [qrText, setQrText] = useState("");
  const [branchName, setBranchName] = useState("");
  
  const repoName = basename(getOriginalCwd());
  
  // Fetch branch name for display
  useEffect(() => {
    getBranch().then(setBranchName).catch(() => {});
  }, []);
  
  // Generate QR code on demand
  useEffect(() => {
    if (!showQR || !connectUrl) {
      setQrText("");
      return;
    }
    qrToString(connectUrl, {
      type: "utf8",
      errorCorrectionLevel: "L",
      small: true,
    }).then(setQrText).catch(() => setQrText(""));
  }, [showQR, connectUrl]);
  
  // Keybindings: Enter to close, 'q' to toggle QR
  useKeybindings({
    "confirm:yes": onDone,
    "confirm:toggle": () => setShowQR(s => !s),
  }, { context: "Confirmation" });
  
  // 'd' key to disconnect (raw handler, not configurable)
  useInput(input => {
    if (input === "d") {
      if (explicit) {
        saveGlobalConfig(prev => ({ 
          ...prev, 
          remoteControlAtStartup: false 
        }));
      }
      setAppState(prev => ({ ...prev, replBridgeEnabled: false }));
      onDone();
    }
  });
  
  const status = getBridgeStatus({ 
    error, connected, sessionActive, reconnecting 
  });
  
  const displayUrl = sessionActive ? sessionUrl : connectUrl;
  
  return (
    <Dialog 
      title="Remote Control"
      subtitle={`Repo: ${repoName}${branchName ? ` (${branchName})` : ""}`}
      onCancel={onDone}
      color="permission"
    >
      <Box flexDirection="column" gap={1}>
        <Text>Status: {status.text}</Text>
        {displayUrl && (
          <Box flexDirection="column">
            <Text dimColor>Connect URL:</Text>
            <Text>{displayUrl}</Text>
            {showQR && qrText && (
              <Box marginTop={1}>
                <Ansi>{qrText}</Ansi>
              </Box>
            )}
          </Box>
        )}
        {error && (
          <Text color="error">Error: {error}</Text>
        )}
      </Box>
    </Dialog>
  );
}
```

**Key Patterns**:
- **QR on-demand**: Only generated when showQR toggled (saves ~50ms)
- **Raw 'd' handler**: Bypasses keybinding config for disconnect
- **Config persistence**: `saveGlobalConfig` for startup preference

---

### 13.6 MessageRow.tsx — Message Renderer with Collapsed Group Support

```typescript
// MessageRow.tsx
type Props = {
  message: RenderableMessage;
  isUserContinuation: boolean;
  hasContentAfter: boolean;  // For collapsed group spinner state
  tools: Tools;
  commands: Command[];
  verbose: boolean;
  inProgressToolUseIDs: Set<string>;
  streamingToolUseIDs: Set<string>;
  screen: Screen;
  canAnimate: boolean;
  onOpenRateLimitOptions?: () => void;
  lastThinkingBlockId: string | null;
  latestBashOutputUUID: string | null;
  columns: number;
  isLoading: boolean;
  lookups: ReturnType<typeof buildMessageLookups>;
};

/**
 * Scans forward from index+1 to check if real content follows.
 * Exported so Messages.tsx computes once per message (avoids 1-2MB array 
 * retention in React Compiler memo cache).
 */
export function hasContentAfterIndex(
  messages: RenderableMessage[], 
  index: number, 
  tools: Tools,
  streamingToolUseIDs: Set<string>
): boolean {
  for (let i = index + 1; i < messages.length; i++) {
    const msg = messages[i];
    
    if (msg?.type === 'assistant') {
      const content = msg.message.content[0];
      if (content?.type === 'thinking' || content?.type === 'redacted_thinking') {
        continue;
      }
      if (content?.type === 'tool_use') {
        if (getToolSearchOrReadInfo(content.name, content.input, tools).isCollapsible) {
          continue;
        }
        // Non-collapsible tool uses appear in syntheticStreamingToolUseMessages
        // before ID added to inProgressToolUseIDs
        if (streamingToolUseIDs.has(content.id)) continue;
      }
      return true;
    }
    
    if (msg?.type === 'system' || msg?.type === 'attachment') continue;
    
    // Tool results while collapsed group being built
    if (msg?.type === 'user') {
      const content = msg.message.content[0];
      if (content?.type === 'tool_result') continue;
    }
    
    // Collapsible grouped_tool_use before merge
    if (msg?.type === 'grouped_tool_use') {
      const firstInput = msg.messages[0]?.message.content[0]?.input;
      if (getToolSearchOrReadInfo(msg.toolName, firstInput, tools).isCollapsible) {
        continue;
      }
    }
    
    return true;
  }
  return false;
}

function MessageRowImpl({ message: msg, ...props }: Props): ReactNode {
  const isTranscriptMode = props.screen === "transcript";
  const isGrouped = msg.type === "grouped_tool_use";
  const isCollapsed = msg.type === "collapsed_read_search";
  
  // Active collapsed group = tools in progress OR loading with no content after
  const isActiveCollapsedGroup = 
    isCollapsed && 
    (hasAnyToolInProgress(msg, props.inProgressToolUseIDs) || 
     (props.isLoading && !props.hasContentAfter));
  
  // Normalize display message
  const displayMsg = isGrouped 
    ? msg.displayMessage 
    : isCollapsed 
      ? getDisplayMessageFromCollapsed(msg) 
      : msg;
  
  // Progress messages from lookup
  const progressMessages = isGrouped || isCollapsed 
    ? [] 
    : getProgressMessagesFromLookup(msg, props.lookups);
  
  // Render based on message type...
  if (msg.type === 'assistant') {
    return (
      <Message 
        message={displayMsg} 
        lookups={props.lookups}
        progressMessages={progressMessages}
        isActiveCollapsedGroup={isActiveCollapsedGroup}
        {...props}
      />
    );
  }
  
  // Other message types routed to specific renderers...
}
```

**Performance Optimization**:
- **Exported predicate**: `hasContentAfterIndex` computed in parent (Messages.tsx)
- **Avoids array retention**: React Compiler pins large arrays in memo cache
- **1-2MB savings**: Prevents accumulation of historical `renderableMessages` versions

---

## 14. Component Inventory by Category

### 14.1 Message Rendering (30+ files)

| Component | Lines | Purpose |
|-----------|-------|---------|
| `AssistantTextMessage.tsx` | 30,427 | Assistant text with error state handling |
| `AssistantToolUseMessage.tsx` | 45,285 | Tool call rendering with progress |
| `AttachmentMessage.tsx` | 71,430 | Image/file attachment display |
| `CollapsedReadSearchContent.tsx` | 78,078 | Collapsed read/search group rendering |
| `SystemTextMessage.tsx` | 79,395 | System messages (compact/expanded) |
| `UserPromptMessage.tsx` | 14,566 | User prompt with PromptXML tags |
| `UserTeammateMessage.tsx` | 24,126 | Teammate message display |
| `UserTextMessage.tsx` | 29,051 | User text message router |
| `PlanApprovalMessage.tsx` | 25,339 | Plan approval request |
| `RateLimitMessage.tsx` | 17,162 | Rate limit error with options |
| `HighlightedThinkingText.tsx` | 14,902 | Thinking block highlighting |
| `GroupedToolUseContent.tsx` | 8,289 | Grouped tool use display |

### 14.2 Input Components (15+ files)

| Component | Lines | Purpose |
|-----------|-------|---------|
| `PromptInput/PromptInput.tsx` | 2,338 | Main chat input component |
| `BaseTextInput.tsx` | 136 | Base text input with cursor parking |
| `TextInput.tsx` | ~400 | Text input with voice waveform |
| `CustomSelect/select.tsx` | 689 | Dropdown selection component |
| `SelectMulti.tsx` | 414 | Multi-select variant |
| `ShimmeredInput.tsx` | 166 | Input with shimmer effect |
| `HistorySearchInput.tsx` | 50 | History search |

### 14.3 Design System (20+ files)

| Component | Lines | Purpose |
|-----------|-------|---------|
| `ThemeProvider.tsx` | 188 | Theme context with auto-detection |
| `ThemedBox.tsx` | 195 | Theme-aware Box |
| `ThemedText.tsx` | 138 | Theme-aware Text |
| `Tabs.tsx` | 414 | Tabbed content navigation |
| `Dialog.tsx` | 141 | Dialog container |
| `Pane.tsx` | 69 | Pane/border container |
| `Byline.tsx` | 68 | Footer action hints |
| `Divider.tsx` | 110 | Horizontal divider |
| `ListItem.tsx` | 195 | Selectable list item |
| `ProgressBar.tsx` | 71 | Progress bar |
| `LoadingState.tsx` | 69 | Loading state |
| `StatusIcon.tsx` | 75 | Status icon |

### 14.4 Layout & Infrastructure (10+ files)

| Component | Lines | Purpose |
|-----------|-------|---------|
| `App.tsx` | 55 | Top-level provider wrapper |
| `FullscreenLayout.tsx` | 636 | Sticky-scroll layout |
| `VirtualMessageList.tsx` | 1,081 | Virtualized message list |
| `Messages.tsx` | 833 | Message list container |
| `Message.tsx` | 626 | Individual message renderer |
| `MessageRow.tsx` | ~400 | Message row with collapsed support |
| `MessageSelector.tsx` | 830 | Message selection/copy |
| `LogSelector.tsx` | 1,574 | Log viewing/selection |

### 14.5 Dialogs (50+ files)

| Component | Lines | Purpose |
|-----------|-------|---------|
| `BridgeDialog.tsx` | ~200 | Remote session dialog |
| `ExportDialog.tsx` | ~150 | Session export |
| `GlobalSearchDialog.tsx` | ~300 | Search across sessions |
| `HistorySearchDialog.tsx` | ~200 | History search |
| `Settings/Config.tsx` | 1,821 | Main settings panel |
| `TrustDialog.tsx` | ~200 | Trust dialog |
| `CostThresholdDialog.tsx` | ~150 | Cost threshold settings |
| `BypassPermissionsModeDialog.tsx` | ~100 | Bypass mode dialog |

### 14.6 Specialized UI (30+ files)

| Component | Lines | Purpose |
|-----------|-------|---------|
| `Spinner.tsx` | ~600 | Loading spinner with verb |
| `Feedback.tsx` | 591 | Feedback collection |
| `Stats.tsx` | 1,227 | Token/cost statistics |
| `TaskListV2.tsx` | 470 | Task/todo list |
| `CoordinatorAgentStatus.tsx` | 360 | Agent status display |
| `StatusLine.tsx` | 465 | Bottom status bar |
| `ModelPicker.tsx` | 447 | Model selection |
| `ThemePicker.tsx` | 335 | Theme selection |
| `Markdown.tsx` | 281 | Markdown rendering |
| `MarkdownTable.tsx` | 475 | Table rendering |

### 14.7 Logo & Branding (15+ files)

| Component | Lines | Purpose |
|-----------|-------|---------|
| `LogoV2.tsx` | 542 | Main logo |
| `WelcomeV2.tsx` | 432 | Welcome screen |
| `Clawd.tsx` | 185 | Mascot display |
| `AnimatedClawd.tsx` | 140 | Animated mascot |
| `Feed.tsx` | 138 | Activity feed |
| `FeedColumn.tsx` | 53 | Feed column |
| `GuestPassesUpsell.tsx` | 91 | Guest passes upsell |
| `OverageCreditUpsell.tsx` | 183 | Credit upsell |

### 14.8 Tasks & Agents (20+ files)

| Component | Lines | Purpose |
|-----------|-------|---------|
| `BackgroundTasksDialog.tsx` | 651 | Background tasks list |
| `BackgroundTaskStatus.tsx` | 428 | Task status display |
| `RemoteSessionDetailDialog.tsx` | 903 | Remote session details |
| `ShellDetailDialog.tsx` | 391 | Shell session details |
| `AsyncAgentDetailDialog.tsx` | 298 | Agent detail view |
| `AgentsMenu.tsx` | 799 | Agents dropdown |
| `AgentsList.tsx` | 439 | Agents list |
| `ToolSelector.tsx` | 561 | Tool selection |
| `AgentEditor.tsx` | 264 | Agent editor |

### 14.9 MCP (10+ files)

| Component | Lines | Purpose |
|-----------|-------|---------|
| `MCPListPanel.tsx` | 503 | MCP server list |
| `MCPSettings.tsx` | 397 | MCP configuration |
| `MCPServerDialog.tsx` | 200 | Server dialog |
| `ElicitationDialog.tsx` | 1,168 | Elicitation flow |
| `MCPRemoteServerMenu.tsx` | 648 | Remote server menu |

### 14.10 Permissions (20+ files)

| Component | Lines | Purpose |
|-----------|-------|---------|
| `PermissionPrompt.tsx` | 373 | Base permission prompt |
| `PermissionRequest.tsx` | 335 | Permission request container |
| `PermissionRuleList.tsx` | 1,178 | Permission rules |
| `FallbackPermissionRequest.tsx` | 799 | Fallback handler |
| `BashPermissionRequest/BashPermissionRequest.tsx` | 481 | Bash approval |

---

## 15. Performance Optimizations Summary

| Optimization | File | Impact |
|--------------|------|--------|
| **Token cache (LRU)** | `Markdown.tsx` | 3ms per message parse avoided |
| **WeakMap sticky prompt cache** | `VirtualMessageList.tsx` | O(1) lookup vs O(n) recompute |
| **Content hashing** | `Markdown.tsx` | Avoids retaining full content strings |
| **Plain text fast path** | `Markdown.tsx` | Skip marked.lexer for 60%+ messages |
| **Ref-based scroll state** | `FullscreenLayout.tsx` | No re-renders on scroll events |
| **Exported predicates** | `MessageRow.tsx` | Prevents 1-2MB array retention |
| **Incremental key array** | `VirtualMessageList.tsx` | Avoids O(n) churn on append |
| **Cursor parking** | `BaseTextInput.tsx` | Enables IME and screen readers |
| **Paste Enter suppression** | `BaseTextInput.tsx` | Prevents accidental submit |
| **Brief spinner mode** | `Spinner.tsx` | Reduced rendering in brief mode |

---

## 16. Related Modules

| Module | Relationship |
|--------|--------------|
| `ink/` | Terminal rendering primitives (Box, Text, useInput) |
| `hooks/` | Custom React hooks (useTerminalSize, useKeybindings) |
| `state/` | AppState for reactive UI updates |
| `context/` | Theme, Modal, PromptOverlay contexts |
| `design-system/` | Base UI primitives |
| `messages/` | Message-specific renderers |
| `permissions/` | Permission request dialogs |
| `PromptInput/` | Main chat input components |
| `CustomSelect/` | Dropdown selection components |
| `LogoV2/` | Branding and logo components |
| `tasks/` | Task-related UI components |
| `agents/` | Agent management UI |
| `mcp/` | MCP server management UI |
| `Settings/` | Settings dialogs |
| `HelpV2/` | Help documentation UI |
| `HighlightedCode/` | Syntax highlighting |
| `FeedbackSurvey/` | Survey collection UI |
| `TrustDialog/` | Trust configuration |
| `Spinner/` | Spinner animations |
| `StructuredDiff/` | Diff rendering |
| `hooks/` | Hook configuration UI |
| `memory/` | Memory file UI |
| `teams/` | Team management |
| `wizard/` | Wizard navigation |
| `grove/` | Grove integration |
| `sandbox/` | Sandbox configuration |
| `skills/` | Skill management |

---

**Last Updated:** 2026-04-07  
**Status:** Complete — 389 files inventoried, 16 sections with deep-dive code analysis

The module follows consistent patterns: React Compiler memoization, theme-aware styling, configurable keybindings, and graceful error handling.