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
- **Language:** Python (231 .py files), TypeScript/React (frontend)
- **Version:** 1.45.1
- **License:** Apache 2.0

## What Streamlit Is

Streamlit turns Python scripts into interactive web apps with no frontend experience required. The core mechanism: every `st.foo()` call generates a Protobuf delta message sent over WebSocket to a React frontend. Widget interactions trigger full-script re-execution with preserved `SessionState`. The bridge between Python and JavaScript is 81 `.proto` files.

## Documentation

```
src.streamlit/
├── spec.md, build.py, markdown/, html/
```
