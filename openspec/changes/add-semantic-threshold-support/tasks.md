## 1. Request Contract
- [x] 1.1 Extend `QueryRequest` with optional minimum semantic score field.
- [x] 1.2 Keep existing callers backward-compatible by setting the field to `None` where unset.

## 2. Planner + CPU Execution
- [x] 2.1 Compile `meaning(..., threshold=...)` into the request minimum semantic score field.
- [x] 2.2 Enforce minimum semantic score filtering in CPU ranking for vector queries.

## 3. Validation
- [x] 3.1 Add/adjust planner tests for semantic threshold success and invalid threshold bounds.
- [x] 3.2 Add/adjust CPU integration tests for semantic threshold filtering behavior.
- [x] 3.3 Run `cargo test --workspace` and `openspec validate --changes --strict --no-interactive`.
