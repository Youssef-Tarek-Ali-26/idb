## Why
The architecture catalog specifies HTTP/WebSocket/TCP gateway parity, but no executable gateway normalization layer exists yet. We need a shared gateway crate that maps transport-specific envelopes into one canonical execution contract.

## What Changes
- Add `idb-gateway` crate with transport envelope models and normalization functions.
- Normalize HTTP route payloads, WebSocket events, and TCP opcodes into a single `GatewayRequest` command model.
- Add CPU gateway runtime dispatch that executes normalized requests over existing CPU backend APIs.

## Impact
- Establishes a concrete path for multi-transport parity before full network server integration.
- Reduces duplication in future server adapters by centralizing command parsing and dispatch semantics.
