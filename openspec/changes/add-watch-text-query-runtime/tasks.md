## 1. CPU Watch API
- [x] 1.1 Add watch session start API for text queries with default/custom bridge options.
- [x] 1.2 Add polling API for active watch subscriptions.

## 2. Runtime Behavior
- [x] 2.1 Execute watch snapshot from query text and derive dependency record IDs.
- [x] 2.2 Register dependency-tracked subscription with resume token at start sequence.

## 3. Validation
- [x] 3.1 Add CPU tests for watch-mode requirement, snapshot/session start, and dependency-filtered polling.
- [x] 3.2 Run `cargo test -p idb-executor-cpu` and `openspec validate --changes --strict --no-interactive`.
