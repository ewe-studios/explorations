---
source: /home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.streamlit/streamlit/
language: Python, TypeScript/React
version: 1.45.1
explored_at: 2026-07-04
license: Apache-2.0
---

# spec.md — Streamlit Documentation

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.AI/src.streamlit/streamlit/`
- **Language:** Python (489 .py files in lib/), TypeScript/React (frontend), 83 .proto files
- **Version:** 1.45.1
- **License:** Apache 2.0

## What Streamlit Is

Streamlit turns Python scripts into interactive web apps with no frontend experience required. The core mechanism: every `st.foo()` call generates a Protobuf delta message sent over WebSocket to a React frontend. Widget interactions trigger full-script re-execution with preserved `SessionState`. The bridge between Python and JavaScript is 83 `.proto` files.

## Documentation Goal

After reading these documents, a reader should understand:
1. How the rerun execution model works — full script re-execution with preserved state
2. How the DeltaGenerator system creates UI elements via 40+ mixin classes
3. How the 83 protobuf files define the complete Python-to-JavaScript contract
4. How SessionState persists widget values and user data across reruns
5. How the widget protocol synchronizes values between frontend and backend
6. How @st.cache_data and @st.cache_resource skip function re-execution with message replay
7. How @st.fragment enables partial reruns of specific UI sections
8. How the React frontend processes protobuf messages and renders the immutable AppNode tree

## Documentation Structure

```
src.streamlit/
├── spec.md, build.py, styles.css
├── markdown/
│   ├── README.md
│   ├── 00-overview.md            ← Architecture overview, rerun model, protobuf bridge summary
│   ├── 01-rerun-execution-model.md  ← ScriptRunner, ScriptRequests, ScriptRunContext, execution loop
│   ├── 02-deltagenerator-system.md  ← DeltaGenerator, mixins, cursors, context stack, blocks
│   ├── 03-protobuf-bridge.md        ← 83 .proto files, ForwardMsg, BackMsg, Delta, message cache
│   ├── 04-session-state.md          ← SessionState, WStates, KeyIdMapper, lifecycle, SafeSessionState
│   ├── 05-widget-protocol.md        ← Widget registration, serde, forms, value synchronization
│   ├── 06-caching.md                ← @st.cache_data, @st.cache_resource, hashing, message replay
│   ├── 07-fragments.md              ← @st.fragment, FragmentStorage, partial reruns, auto-rerun
│   └── 08-frontend-architecture.md  ← React app, AppNode tree, WebSocket FSM, WidgetStateManager
├── html/
│   ├── index.html
│   └── *.html
```

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | Read source codebase (lib/, proto/, frontend/) | DONE |
| 2 | Read existing docs (00-overview.md, README.md) | DONE |
| 3 | Write 01-rerun-execution-model.md | DONE |
| 4 | Write 02-deltagenerator-system.md | DONE |
| 5 | Write 03-protobuf-bridge.md | DONE |
| 6 | Write 04-session-state.md | DONE |
| 7 | Write 05-widget-protocol.md | DONE |
| 8 | Write 06-caching.md | DONE |
| 9 | Write 07-fragments.md | DONE |
| 10 | Write 08-frontend-architecture.md | DONE |
| 11 | Update spec.md | DONE |
| 12 | Update README.md | DONE |
| 13 | Run build.py | TODO |
| 14 | Grandfather review | TODO |

## Build System

```bash
cd /home/darkvoid/Boxxed/@dev/repo-expolorations/[src.AI]/
python3 build.py src.streamlit
```

## Quality Requirements

All Iron Rules from documentation_directive.md apply:
1. Detailed sections with actual source code snippets
2. Key facts delivered quickly
3. Clear articulation
4. Minimum 2 mermaid diagrams per document
5. Good visual assets (tables, diagrams)
6. Generated HTML with navigation
7. Cross-references between documents
8. Source path references (file paths and line numbers)
9. Aha moments in every document
10. Navigation: index + prev/next buttons

## Expected Outcome

A reader can understand Streamlit's architecture end-to-end: from a `st.foo()` call in Python, through protobuf serialization, WebSocket transport, React rendering, widget interaction, and back to SessionState and rerun execution. They can explain the rerun model, customize the UI with layouts, understand caching behavior, and build fragment-based apps.

## Resume Point

If interrupted, all 8 documents (01-08) are written. Remaining tasks: run `python3 build.py src.streamlit` from the `[src.AI]/` directory, then perform grandfather review.
