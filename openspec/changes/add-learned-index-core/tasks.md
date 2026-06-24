## 1. Crate and Model
- [x] 1.1 Add `idb-index` crate and workspace wiring.
- [x] 1.2 Implement a linear learned position model with max-error tracking over sorted keys.

## 2. Correctness-Safe APIs
- [x] 2.1 Implement bounded prediction window API for fallback search.
- [x] 2.2 Implement exact lookup helper using predicted window + bounded binary search fallback.

## 3. Validation
- [x] 3.1 Add unit tests for model training, window bounds, and exact lookup behavior.
- [x] 3.2 Run `cargo test --workspace` and `openspec validate --changes --strict --no-interactive`.
