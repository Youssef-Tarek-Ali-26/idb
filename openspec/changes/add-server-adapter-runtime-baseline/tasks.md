## 1. Server Crate
- [x] 1.1 Add `idb-server` crate and workspace wiring.
- [x] 1.2 Define server config and TCP session id/session registry state.

## 2. Adapter APIs
- [x] 2.1 Implement HTTP envelope and JSON adapter methods.
- [x] 2.2 Implement WebSocket envelope and JSON adapter methods.
- [x] 2.3 Implement TCP session open/close and frame dispatch adapter methods.

## 3. Validation
- [x] 3.1 Add tests for HTTP/WS/TCP query consistency through server adapters.
- [x] 3.2 Add tests for unknown/closed TCP sessions and malformed JSON payloads.
- [x] 3.3 Run `cargo test -p idb-server` and `openspec validate --changes --strict --no-interactive`.
