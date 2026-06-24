# Change: Add TopK Ordering Bridge Support

## Why
The planner bridge currently rejects `top(k, field desc|asc)` clauses, preventing ordered top-k queries from executing through the text-query path.

## What Changes
- Extend core request contract with optional result ordering metadata.
- Compile `top(k, field dir)` from logical plan into request-level ordering.
- Enforce deterministic field-based ordering in CPU ranking.
- Add planner and CPU integration tests for ordered top-k behavior.

## Impact
- Affected specs: `logical-plan-bridge`, `execution-backend-contract`
- Affected code: `idb-core`, `idb-planner`, `idb-executor-cpu`
