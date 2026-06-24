## 1. Spec and Theory
- [x] 1.1 Add N-space decomposition and fused similarity requirements to unified-spatial-storage spec delta.
- [x] 1.2 Add theory document with formulas, invariants, and v0 implementation assumptions.

## 2. Core Reference Primitives
- [x] 2.1 Add N-space point decomposition types in `idb-core`.
- [x] 2.2 Add fused similarity kernel implementation with deterministic behavior and bounds.
- [x] 2.3 Add tests for symmetry, boundedness, and self-similarity.

## 3. Validation
- [x] 3.1 Run `cargo test -p idb-core --lib` and `openspec validate add-n-space-theory-foundation --type change --strict --no-interactive`.
