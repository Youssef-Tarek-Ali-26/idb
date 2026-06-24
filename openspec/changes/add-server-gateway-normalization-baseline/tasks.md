## 1. Gateway Crate
- [x] 1.1 Add `idb-gateway` crate and workspace wiring.
- [x] 1.2 Define normalized request/response command model shared by transports.

## 2. Transport Normalization
- [x] 2.1 Implement HTTP path normalization into canonical commands.
- [x] 2.2 Implement WebSocket event normalization into canonical commands.
- [x] 2.3 Implement TCP opcode normalization and payload decoding.
- [x] 2.4 Add TCP session negotiation hook using wire protocol versioning.

## 3. Runtime Dispatch
- [x] 3.1 Implement CPU gateway runtime command dispatch for query/explain/watch/mutate operations.
- [x] 3.2 Preserve caller-context propagation into auth-aware CPU backend APIs.

## 4. Validation
- [x] 4.1 Add transport normalization tests for parity and failure cases.
- [x] 4.2 Add gateway runtime tests for query parity, watch lifecycle, and auth denial behavior.
- [x] 4.3 Run `cargo test -p idb-gateway` and `openspec validate --changes --strict --no-interactive`.
