## Why
Transport adapters exist, but we need a deterministic harness to replay mixed transport sequences for integration and regression testing. A script runner enables reproducible protocol-flow validation without standing up real listeners.

## What Changes
- Add script step/event types in `idb-server` for HTTP/WebSocket/TCP sequences.
- Implement `run_script` to execute ordered steps against adapter methods and maintain session-key mapping.
- Add explicit errors for missing script session keys.

## Impact
- Makes multi-transport behavior easier to regression test and debug.
- Provides a reusable harness for future end-to-end fixture suites.
