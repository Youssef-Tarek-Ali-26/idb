## 1. Planner Bridge
- [x] 1.1 Add execution-focused query request compilation API that supports traversal sources.
- [x] 1.2 Keep existing strict bridge behavior (entity-scan-only) unchanged for `query_text_to_request`.

## 2. CPU Traversal Execution
- [x] 2.1 Execute traversal sources in `run_query_text` by walking `edge_refs` over hot/cold state.
- [x] 2.2 Apply query filters, semantic scoring, ordering, and top-k on traversal result candidates.

## 3. Validation
- [x] 3.1 Add planner test for traversal execution projection.
- [x] 3.2 Add CPU integration tests for outbound/inbound traversal text queries.
- [x] 3.3 Run `cargo test --workspace` and `openspec validate --changes --strict --no-interactive`.
