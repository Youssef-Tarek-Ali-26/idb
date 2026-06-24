# Change: Add Semantic Threshold Support

## Why
`meaning(..., threshold=...)` is currently rejected by the planner bridge, so users cannot express minimum semantic similarity constraints in text queries.

## What Changes
- Extend planner bridge to map semantic threshold into executable request metadata.
- Extend core `QueryRequest` contract with optional minimum vector score.
- Enforce minimum vector score in CPU ranking path.
- Add planner + CPU tests for threshold behavior.

## Impact
- Affected specs: `logical-plan-bridge`, `execution-backend-contract`
- Affected code: `idb-core`, `idb-planner`, `idb-executor-cpu`, downstream tests
