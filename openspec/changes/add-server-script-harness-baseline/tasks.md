## 1. Script Model
- [x] 1.1 Add server script step/event types for HTTP/WebSocket/TCP flows.
- [x] 1.2 Add script-level unknown session-key error handling.

## 2. Script Runtime
- [x] 2.1 Implement ordered step execution against existing adapter methods.
- [x] 2.2 Maintain TCP session-key to session-id mapping during script execution.

## 3. Validation
- [x] 3.1 Add tests for mixed-transport script execution.
- [x] 3.2 Add tests for unknown script session-key rejection.
- [x] 3.3 Run `cargo test -p idb-server` and `openspec validate --changes --strict --no-interactive`.
