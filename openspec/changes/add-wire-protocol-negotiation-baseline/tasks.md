## 1. Crate
- [x] 1.1 Add `idb-wire` crate and workspace wiring.
- [x] 1.2 Add protocol version and session handshake types.

## 2. Negotiation
- [x] 2.1 Implement range validation and protocol negotiation.
- [x] 2.2 Return explicit compatibility errors for invalid/non-overlapping ranges.

## 3. Validation
- [x] 3.1 Add tests for successful negotiation and failure cases.
- [x] 3.2 Run `cargo test -p idb-wire` and `openspec validate --changes --strict --no-interactive`.
