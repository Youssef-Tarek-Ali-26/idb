## 1. Planner Bridge
- [x] 1.1 Compile `sort(field dir)` transform into request `order_by` projection.
- [x] 1.2 Reject conflicting ordering transforms deterministically.
- [x] 1.3 Keep unsupported transform handling for `group` and `aggregate` unchanged.

## 2. Validation
- [x] 2.1 Add planner tests for sort projection and conflict handling.
- [x] 2.2 Add CPU integration test for text-query `sort + take` execution.
- [x] 2.3 Run `cargo check --workspace` and `openspec validate --changes --strict --no-interactive`.
