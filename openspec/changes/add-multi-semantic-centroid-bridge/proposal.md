# Change: Add Multi-Semantic Centroid Bridge

## Why
The planner bridge currently rejects queries with multiple `meaning(...)` predicates, which blocks expressing multi-intent semantic retrieval in text-query execution.

## What Changes
- Extend planner bridge to compile multiple semantic predicates into one executable vector query.
- Use deterministic centroid composition (normalized mean of semantic query embeddings).
- Support threshold compilation by applying the strictest configured threshold across semantic predicates.
- Add planner and CPU tests for multi-semantic execution behavior.

## Impact
- Affected specs: `logical-plan-bridge`
- Affected code: `idb-planner`, `idb-executor-cpu`
