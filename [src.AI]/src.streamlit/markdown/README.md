# Streamlit — Turn Python Scripts into Interactive Web Apps

Rerun-based reactivity model, Protobuf bridge to React frontend, 83 `.proto` files defining the Python-to-JS contract.

## Documents

| # | Document | Category | What You'll Learn |
|---|----------|----------|-------------------|
| 00 | [Overview](00-overview.md) | Foundation | Architecture overview, rerun model, DeltaGenerator, protobuf bridge, SessionState |
| 01 | [Rerun Execution Model](01-rerun-execution-model.md) | Core | ScriptRunner, ScriptRequests queue, ScriptRunContext, execution loop, yield points |
| 02 | [DeltaGenerator System](02-deltagenerator-system.md) | Core | DeltaGenerator, 40+ mixins, RunningCursor/LockedCursor, context stack, blocks |
| 03 | [Protobuf Bridge](03-protobuf-bridge.md) | Core | 83 .proto files, ForwardMsg, BackMsg, Delta, WidgetStates, message cache |
| 04 | [SessionState Management](04-session-state.md) | Core | SessionState, WStates, KeyIdMapper, lifecycle callbacks, SafeSessionState, QueryParams |
| 05 | [Widget Protocol](05-widget-protocol.md) | Deep Dive | Widget registration, serde, element ID generation, forms, value sync flow |
| 06 | [Caching Mechanisms](06-caching.md) | Deep Dive | @st.cache_data, @st.cache_resource, hashing, message replay, storage layers |
| 07 | [Fragment Partial Reruns](07-fragments.md) | Deep Dive | @st.fragment, FragmentStorage, cursor snapshots, auto-rerun, preemption |
| 08 | [Frontend React Architecture](08-frontend-architecture.md) | Cross-Cutting | AppNode tree, WebSocket FSM, ForwardMessageCache, WidgetStateManager, rendering |
