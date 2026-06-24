## 1. Request Contract
- [x] 1.1 Add optional order-by metadata to `QueryRequest`.
- [x] 1.2 Keep existing callers backward-compatible when order-by is unset.

## 2. Planner + Executor
- [x] 2.1 Compile `top(k, field dir)` into request order-by metadata.
- [x] 2.2 Apply deterministic field ordering in CPU ranking with stable tie-breaks.

## 3. Validation
- [x] 3.1 Add planner tests for ordered top-k compilation.
- [x] 3.2 Add CPU tests for asc/desc top-k ordering semantics.
- [x] 3.3 Run `cargo test --workspace` and `openspec validate --changes --strict --no-interactive`.
