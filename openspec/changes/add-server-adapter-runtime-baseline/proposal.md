## Why
The gateway crate now normalizes transport envelopes, but we still need server-facing adapter surfaces that can be embedded by real network listeners. A lightweight server adapter layer makes transport handling reusable and keeps networking concerns separate from command normalization/runtime dispatch.

## What Changes
- Add `idb-server` crate with stateful TCP session registry and adapter methods for HTTP/WebSocket/TCP.
- Expose typed envelope handlers and JSON helper methods for transport integration boundaries.
- Route all adapter calls through `idb-gateway` canonical normalization/runtime execution flow.

## Impact
- Creates a direct integration seam for future Axum/WebSocket/TCP listeners.
- Ensures transport-specific session handling remains consistent with canonical gateway semantics.
