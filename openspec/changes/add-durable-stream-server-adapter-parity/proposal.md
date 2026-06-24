# Change: Add Durable Stream Server Adapter Parity

## Why
Durable stream adapter coverage currently validates HTTP flow only, while the server exposes WebSocket and TCP adapters that should preserve equivalent durable poll/commit semantics.

## What Changes
- Add server adapter test coverage for durable stream poll/commit via HTTP, WebSocket, and TCP.
- Validate equivalent durable stream replay and commit semantics across adapters.

## Impact
- Affected specs: `server-gateway`
- Affected code: `idb-server`
