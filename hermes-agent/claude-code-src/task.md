# Claude Code Source Exploration — Task List

**Project:** claude-code-src  
**Index:** [index.md](./index.md)  
**Created:** 2026-04-07

---

## Pending Tasks

Each task below represents a deep-dive exploration of a module in the Claude Code source code. Explorations should be saved to `./hermes-agent/claude-code-src/[dir]/exploration.md`.

### Core Modules (High Priority)

- [x] **Explore `bridge/` module** — Bridge communication protocol, REPL transport, JWT utils, remote sessions, inbound/outbound messaging. 31 files.
  - Output: `./bridge/exploration.md`

- [x] **Explore `cli/` module** — CLI entrypoints, transports (SSE, WebSocket, Hybrid), NDJSON serialization, remote I/O, structured I/O. 19 files.
  - Output: `./cli/exploration.md`

- [x] **Explore `commands/` module** — All slash command implementations, command registry, subcommand handlers. 189 files.
  - Output: `./commands/exploration.md`

- [x] **Explore `components/` module** — React UI components, dialogs, layouts, interactive elements. 331 files.
  - Output: `./components/exploration.md`

- [x] **Explore `hooks/` module** — React hooks for state management, data fetching, subscriptions. 101 files.
  - Output: `./hooks/exploration.md`

- [x] **Explore `ink/` module** — Terminal UI rendering using Ink framework, TUI components. 96 files.
  - Output: `./ink/exploration.md`

- [x] **Explore `services/` module** — Background services, daemons, long-running processes. 130 files.
  - Output: `./services/exploration.md`

- [x] **Explore `tools/` module** — Tool implementations (bash, read_file, write_file, web tools, agents). 178 files.
  - Output: `./tools/exploration.md`

- [x] **Explore `utils/` module** — Utility functions, helpers, common operations. 542 files.
  - Output: `./utils/exploration.md`

### State and Configuration

- [x] **Explore `state/` module** — Application state management, Zustand/Redux patterns. 6 files.
  - Output: `./state/exploration.md`

- [x] **Explore `constants/` module** — Application constants, configuration defaults. 21 files.
  - Output: `./constants/exploration.md`

- [x] **Explore `context/` module** — React context providers, global state contexts. 9 files.
  - Output: `./context/exploration.md`

- [x] **Explore `types/` module** — TypeScript type definitions, interfaces, generics. 7 files.
  - Output: `./types/exploration.md`

### Session and Conversation

- [x] **Explore `assistant/` module** — Session history pagination, API fetching. 1 file.
  - Output: `./assistant/exploration.md`

- [x] **Explore `tasks/` module** — Task tracking, task lifecycle, task registry. 12 files.
  - Output: `./tasks/exploration.md`

### Specialized Modules

- [x] **Explore `buddy/` module** — Companion/buddy features, notifications, sprites. 6 files.
  - Output: `./buddy/exploration.md`

- [x] **Explore `coordinator/` module** — Coordinator mode, worker delegation, agent orchestration. 1 file.
  - Output: `./coordinator/exploration.md`

- [x] **Explore `keybindings/` module** — Keyboard shortcuts, vim-style bindings. 14 files.
  - Output: `./keybindings/exploration.md`

- [x] **Explore `memdir/` module** — Memory directory management, instruction files. 8 files.
  - Output: `./memdir/exploration.md`

- [x] **Explore `migrations/` module** — Data migrations, schema evolution. 11 files.
  - Output: `./migrations/exploration.md`

- [x] **Explore `plugins/` module** — Plugin system, plugin lifecycle. 2 files.
  - Output: `./plugins/exploration.md`

- [x] **Explore `query/` module** — Query engine interface, search functionality. 4 files.
  - Output: `./query/exploration.md`

- [x] **Explore `remote/` module** — Remote execution, SSH, tunneling. 4 files.
  - Output: `./remote/exploration.md`

- [x] **Explore `schemas/` module** — Hook Zod schemas, permission rule syntax. 1 file.
  - Output: `./schemas/exploration.md`

- [x] **Explore `screens/` module** — Screen components (Doctor, REPL, ResumeConversation). 3 files.
  - Output: `./screens/exploration.md`

- [x] **Explore `server/` module** — Direct connect server, session management. 3 files.
  - Output: `./server/exploration.md`

- [x] **Explore `skills/` module** — Skill definitions, skill loading. 20 files.
  - Output: `./skills/exploration.md`

- [x] **Explore `upstreamproxy/` module** — CCR upstream proxy relay, CONNECT-over-WebSocket tunnel, CA bundle injection, prctl security. 2 files.
  - Output: `./upstreamproxy/exploration.md`

- [x] **Explore `vim/` module** — Vim mode support, keybindings. 5 files.
  - Output: `./vim/exploration.md`

- [x] **Explore `voice/` module** — Voice mode feature flags, auth checks. 1 file.
  - Output: `./voice/exploration.md`

### Additional Modules

- [x] **Explore `entrypoints/` module** — Application entry points, CLI vs GUI. 8 files.
  - Output: `./entrypoints/exploration.md`

- [x] **Explore `moreright/` module** — Additional features. 1 file.
  - Output: `./moreright/exploration.md`

- [x] **Explore `native-ts/` module** — Native TypeScript bindings. 4 files.
  - Output: `./native-ts/exploration.md`

- [x] **Explore `outputStyles/` module** — Output style markdown loading. 1 file.
  - Output: `./outputStyles/exploration.md`

### Root-Level Files

- [x] **Explore `main.tsx`** — Main application entry point, React root. 803KB.
  - Output: `./root-files/main-exploration.md`

- [x] **Explore `commands.ts`** — Command registry, slash command definitions. 25KB.
  - Output: `./root-files/commands-exploration.md`

- [x] **Explore `tools.ts`** — Tool registry, tool specifications. 17KB.
  - Output: `./root-files/tools-exploration.md`

- [x] **Explore `Tool.ts`** — Tool base class, tool interface. 29KB.
  - Output: `./root-files/Tool-exploration.md`

- [x] **Explore `Task.ts`** — Task definition, task interface. 3KB.
  - Output: `./root-files/Task-exploration.md`

- [x] **Explore `QueryEngine.ts`** — Query engine implementation. 46KB.
  - Output: `./root-files/QueryEngine-exploration.md`

- [x] **Explore `query.ts`** — Query implementation, search logic. 68KB.
  - Output: `./root-files/query-exploration.md`

- [x] **Explore `cost-tracker.ts`** — Token cost tracking, pricing. 10KB.
  - Output: `./root-files/cost-tracker-exploration.md`

- [x] **Explore `dialogLaunchers.tsx`** — Dialog components, modal launchers. 22KB.
  - Output: `./root-files/dialogLaunchers-exploration.md`

- [x] **Explore `history.ts`** — History management, session persistence. 14KB.
  - Output: `./root-files/history-exploration.md`

- [x] **Explore `interactiveHelpers.tsx`** — Interactive helper functions. 57KB.
  - Output: `./root-files/interactiveHelpers-exploration.md`

- [x] **Explore `setup.ts`** — Application setup, initialization. 20KB.
  - Output: `./root-files/setup-exploration.md`

---

## Exploration Guidelines

Each exploration document should include:

1. **Module Overview** — Purpose and responsibilities
2. **File Inventory** — All files with line counts and descriptions
3. **Key Exports** — Main functions, classes, interfaces
4. **Line-by-Line Analysis** — Critical code sections with explanations
5. **Component Relationships** — How this module interacts with others
6. **Data Flow** — Input/output patterns, state changes
7. **Key Patterns** — Design patterns and architectural decisions
8. **Integration Points** — Dependencies and dependents

---

## Progress Summary — ALL COMPLETE

| Status | Count |
|--------|-------|
| Completed | 35 modules + 11 root files + 2 deep dives |
| In Progress | 0 |
| Pending | 0 |

### Completed Module Explorations

| Module | Lines | Module | Lines | Module | Lines |
|--------|-------|--------|-------|--------|-------|
| bridge/ | 3,182 | types/ | 1,233 | plugins/ | 389 |
| cli/ | 1,311 | assistant/ | 470 | query/ | 1,091 |
| commands/ | 3,392 | tasks/ | 1,528 | remote/ | 1,154 |
| components/ | 2,469 | buddy/ | 782 | schemas/ | 543 |
| hooks/ | 2,764 | coordinator/ | 560 | screens/ | 525 |
| ink/ | 1,484 | keybindings/ | 1,324 | server/ | 1,010 |
| services/ | 1,778 | memdir/ | 989 | skills/ | 742 |
| tools/ | 3,110 | migrations/ | 281 | upstreamproxy/ | 547 |
| utils/ | 3,121 | entrypoints/ | 599 | vim/ | 666 |
| state/ | 1,020 | moreright/ | 235 | voice/ | 298 |
| bootstrap/ | 1,365 | native-ts/ | 451 | **Root Files** | **101,670** |
| constants/ | 1,310 | outputStyles/ | 409 | **Deep Dives** | |
| context/ | 681 | | | teleport/ | 750+ |
| | | | | computer-use/ | 900+ |

**Total Documentation:** ~150,700+ lines with actual TypeScript code

---

**Last Updated:** 2026-04-07 (All 47 Tasks + 2 Deep Dives Complete: teleport, computer-use)
