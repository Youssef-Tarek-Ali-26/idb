## Why
The architecture requirements include a versioned wire protocol, but there is no shared crate implementing protocol version negotiation yet. We need a baseline negotiation contract to ensure clients and servers enforce compatibility before executing requests.

## What Changes
- Add an `idb-wire` crate with protocol version types and range validation.
- Implement client/server version negotiation that selects highest common compatible version.
- Add a handshake helper for `SessionHello` -> `SessionWelcome` flow.

## Impact
- Provides a reusable compatibility layer for future HTTP/WebSocket/TCP gateways.
- Prevents undefined behavior from mismatched client/server protocol versions.
