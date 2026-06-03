---
title: Applications — moq-cli, moq-audio, moq-boy, moq-token
---

# Applications — moq-cli, moq-audio, moq-boy, moq-token

Application crates built on the MoQ protocol stack.

## moq-cli

CLI tool for publishing and subscribing to MoQ broadcasts.

Source: `moq/rs/moq-cli/src/` — CLI implementation.

## moq-audio

Opus audio codec for MoQ:
- Opus encoder/decoder
- Resampler
- Format conversion

Source: `moq/rs/moq-audio/src/` — Audio codec implementation.

## moq-boy

Game Boy emulator streaming application. Streams Game Boy video/audio over MoQ.

Source: `moq/rs/moq-boy/src/` — Game Boy streaming app.

## moq-token

JWT token generation and validation for relay authentication.

```rust
// moq/rs/moq-token/src/
pub struct Token { ... }
pub fn generate(...) -> String { ... }
pub fn validate(token: &str) -> Result<Payload> { ... }
```

Source: `moq/rs/moq-token/src/` — JWT token implementation.

### moq-token-cli

CLI for generating and validating JWT tokens.

Source: `moq/rs/moq-token-cli/src/` — Token CLI.

## moq-loc

LOC frame encoding/decoding for MoQ.

Source: `moq/rs/moq-loc/src/` — LOC implementation.

## moq-msf

MSF catalog types for MoQ media sessions.

Source: `moq/rs/moq-msf/src/` — MSF catalog types.

## Related Documents

- [moq-relay](../markdown/03-moq-relay.md) — Uses moq-token for auth
- [Data Flow](../markdown/09-data-flow.md) — Application data flow
