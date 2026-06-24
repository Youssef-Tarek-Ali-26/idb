## 1. Explain API
- [x] 1.1 Add explain output structs for mode/source/filters/transforms and request projection status.
- [x] 1.2 Implement `explain_query_text` by reusing parser + logical plan + request bridge.

## 2. Validation
- [x] 2.1 Add unit tests for supported explain output and unsupported projection reasons.
- [x] 2.2 Run `cargo test -p idb-planner` and `openspec validate --changes --strict --no-interactive`.
