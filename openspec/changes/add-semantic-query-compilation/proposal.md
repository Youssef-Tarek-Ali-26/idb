# Change: Add Semantic Query Compilation

## Why
The planner bridge currently rejects `meaning(...)` predicates, which blocks end-to-end execution of semantic or hybrid text queries from query string to CPU backend.

## What Changes
- Extend planner bridge to compile a single `meaning(...)` predicate into `QueryRequest.vector_query`.
- Add deterministic text embedding generation for CPU reference path.
- Keep thresholded semantic predicates explicitly unsupported in v0 bridge until min-score contracts are added.
- Add CPU integration tests for semantic text query execution.

## Impact
- Affected specs: `logical-plan-bridge`
- Affected code: `idb-planner`, `idb-executor-cpu`, `idb-core` (error propagation only)
