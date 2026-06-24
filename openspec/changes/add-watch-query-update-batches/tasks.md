## 1. Active Watch State
- [x] 1.1 Track query request metadata for active watch subscriptions.
- [x] 1.2 Add query-update polling API that enriches mutation events with current row state.

## 2. Runtime Semantics
- [x] 2.1 For each event, return `current=None` when record is deleted or no longer matches query constraints.
- [x] 2.2 For each event, return hydrated current row when the record still matches constraints.

## 3. Validation
- [x] 3.1 Add CPU tests for update batch behavior (match -> drop -> match transitions).
- [x] 3.2 Run `cargo test -p idb-executor-cpu` and `openspec validate --changes --strict --no-interactive`.
