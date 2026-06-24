## 1. Planner Bridge
- [x] 1.1 Extend `QueryRequestBridgeOptions` with semantic embedding config.
- [x] 1.2 Compile one unthresholded `meaning(...)` predicate to `QueryRequest.vector_query` using deterministic embeddings.
- [x] 1.3 Reject unsupported semantic forms (multiple semantic predicates, thresholded semantic predicates) with deterministic planner errors.

## 2. CPU Integration
- [x] 2.1 Ensure CPU `run_query_text` path supports semantic-only and hybrid semantic+structured queries.
- [x] 2.2 Add integration tests covering semantic query execution success and unsupported-threshold failure.

## 3. Validation
- [x] 3.1 Add planner unit tests for semantic compilation behavior.
- [x] 3.2 Run `cargo test --workspace` and `openspec validate --changes --strict --no-interactive`.
