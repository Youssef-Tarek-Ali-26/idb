## 1. Spec
- [x] 1.1 Add execution-backend-contract requirements for learned key-range candidate acceleration.

## 2. Implementation
- [x] 2.1 Maintain per-tenant learned key-position index over spatial keys in durable state.
- [x] 2.2 Use learned-index-backed key-range candidates in CPU `query_candidates` path.
- [x] 2.3 Keep index state correct across replay, upserts, updates, and deletes.

## 3. Validation
- [x] 3.1 Add storage tests for update/delete correctness under key-range candidate lookup.
- [x] 3.2 Run `cargo test --workspace` and `openspec validate --changes --strict --no-interactive`.
