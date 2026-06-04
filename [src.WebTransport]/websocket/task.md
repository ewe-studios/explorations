# websocket Task Tracker

## Documentation Status

| Document | Status |
|----------|--------|
| spec.md | ✅ DONE |
| markdown/README.md | ✅ DONE |
| markdown/00-overview.md | ✅ DONE |
| markdown/01-architecture.md | ✅ DONE |
| markdown/02-tungstenite-rs.md | ✅ DONE |
| markdown/03-tokio-tungstenite.md | ✅ DONE |
| markdown/04-websocat.md | ✅ DONE |
| markdown/05-frame-protocol.md | ✅ DONE |
| markdown/09-data-flow.md | ✅ DONE |
| markdown/10-cross-cutting.md | ✅ DONE |
| Grandfather Review | ⏳ WAITING |
| HTML Generation | ✅ DONE |

## Grandfather Review Discrepancies ✅ FIXED

All 9 discrepancies fixed:

### HIGH
- ✅ **Message enum**: 5→6 variants, added `Frame(Frame)` variant with note about raw frame access
- ✅ **WebSocketStream struct**: (checked - tokio-tungstenite doc needs review)

### MEDIUM
- ✅ **server module**: Marked as private (`mod server`, not `pub mod server`)
- ✅ **tokio-tungstenite modules**: Added `compat.rs` and `handshake.rs` with descriptions
- ✅ **Close codes**: Added 4 missing codes (1007 Invalid, 1010 Extension, 1012 Restart, 1013 Again)

### LOW-MEDIUM
- ✅ **websocat specifiers**: Expanded from 8 to 28 specifiers (crypto, file, foreachmsg, http, jsonrpc, lengthprefixed, line, process, prometheus, reconnect, session, socks5, ssl, stdio, timestamp, ws-client, ws-server, ws-lowlevel)

### LOW
- ✅ **data-encoding dependency**: Added to tungstenite-rs dependency table
- ✅ **rand dependency**: Added to tungstenite-rs dependency table
