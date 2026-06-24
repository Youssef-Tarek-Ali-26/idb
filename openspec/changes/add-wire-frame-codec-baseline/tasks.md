## 1. Codec Types
- [x] 1.1 Add wire frame header and frame payload types.
- [x] 1.2 Define frame-level constants for magic and header length.

## 2. Codec Implementation
- [x] 2.1 Implement frame encoding to bytes.
- [x] 2.2 Implement frame decoding from bytes with strict validation.
- [x] 2.3 Return explicit errors for invalid magic, truncation, and length mismatch.

## 3. Validation
- [x] 3.1 Add tests for roundtrip encoding and decode failure cases.
- [x] 3.2 Run `cargo test -p idb-wire` and `openspec validate --changes --strict --no-interactive`.
