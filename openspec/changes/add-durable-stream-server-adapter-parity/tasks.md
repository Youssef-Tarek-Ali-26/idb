## 1. Adapter Parity Tests
- [x] 1.1 Add server tests that execute durable stream poll/commit flow across HTTP, WebSocket, and TCP adapters.
- [x] 1.2 Validate replay after commit returns no additional events for each transport flow.

## 2. Validation
- [x] 2.1 Run `cargo test -p idb-server --lib` and `openspec validate add-durable-stream-server-adapter-parity --type change --strict --no-interactive`.
