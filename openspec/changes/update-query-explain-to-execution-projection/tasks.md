## 1. Explain Projection
- [x] 1.1 Switch explain request projection to execution-oriented projection.
- [x] 1.2 Preserve deterministic unsupported reasons for non-executable modes.

## 2. Validation
- [x] 2.1 Add unit test asserting traversal explain returns supported projection.
- [x] 2.2 Run `cargo test -p idb-planner` and `openspec validate --changes --strict --no-interactive`.
