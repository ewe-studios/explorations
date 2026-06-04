---
title: Frame Protocol — RFC6455 Frame Format
---

# Frame Protocol — RFC6455 Frame Format

The WebSocket frame format defined in RFC6455.

## Frame Structure

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-------+-+-------------+-------------------------------+
|F|R|R|R| opcode|M| Payload len |    Extended payload length    |
|I|S|S|S|  (4)  |A|     (7)     |             (16/64)           |
|N|V|V|V|       |S|             |   (if payload len==126/127)   |
| |1|2|3|       |K|             |                               |
+-+-+-+-+-------+-+-------------+ - - - - - - - - - - - - - - - +
|     Extended payload length continued, if payload len == 127  |
+ - - - - - - - - - - - - - - - +-------------------------------+
|                               |Masking-key, if MASK set to 1  |
+-------------------------------+-------------------------------+
| Masking-key (continued)       |          Payload Data         |
+-------------------------------- - - - - - - - - - - - - - - - +
:                     Payload Data continued ...                :
+ - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - +
|                     Payload Data (continued)                  |
+---------------------------------------------------------------+
```

## Opcodes

| Opcode | Name | Direction | Purpose |
|--------|------|-----------|---------|
| 0x0 | Continuation | Both | Continuation of fragmented message |
| 0x1 | Text | Both | UTF-8 text message |
| 0x2 | Binary | Both | Binary data message |
| 0x8 | Close | Both | Connection close |
| 0x9 | Ping | Both | Ping request |
| 0xA | Pong | Both | Pong response |

Source: `tungstenite-rs/src/protocol/frame/coding.rs:1` — Opcode definitions.

## Masking

```
Client-to-server frames MUST be masked (MASK bit = 1)
Server-to-client frames MUST NOT be masked (MASK bit = 0)
```

The masking key is a 4-byte value XORed with the payload:

```
transformed-octet-i = original-octet-i XOR masking-key-octet-(i MOD 4)
```

**Aha:** The client-to-server masking asymmetry prevents cache poisoning attacks. A malicious client can't craft WebSocket frames that look like HTTP responses to intermediaries (caches, proxies) because client frames must be masked with a random key. Server frames are unmasked because servers are trusted entities.

Source: `tungstenite-rs/src/protocol/frame/mod.rs:1` — Frame masking implementation.

## Payload Length Encoding

| Value | Meaning |
|-------|---------|
| 0-125 | Payload length is the value |
| 126 | Next 2 bytes are the length (16-bit) |
| 127 | Next 8 bytes are the length (64-bit) |

Source: `tungstenite-rs/src/protocol/frame/mod.rs:1` — Payload length encoding.

## Close Codes

| Code | Name | Purpose |
|------|------|---------|
| 1000 | Normal Closure | Clean close |
| 1001 | Going Away | Server shutting down |
| 1002 | Protocol Error | Protocol violation |
| 1003 | Unsupported Data | Received unsupported data type |
| 1005 | Reserved | No close code received |
| 1006 | Reserved | Abnormal close |
| 1008 | Policy Violation | Message violated policy |
| 1009 | Too Big | Message too large |
| 1010 | Extension | Expected extension not negotiated |
| 1011 | Internal Error | Server encountered unexpected condition |
| 1012 | Restart | Service restarting |
| 1013 | Again | Try again later |
| 1015 | Reserved | TLS handshake failure |

Source: `tungstenite-rs/src/protocol/frame/close.rs:1` — Close code definitions.

## Related Documents

- [tungstenite-rs](../markdown/02-tungstenite-rs.md) — Core implementation
- [Data Flow](../markdown/09-data-flow.md) — Message flow
