# Change: Add Traversal Text Query Execution

## Why
Traversal query text (`A -> B`, `A <- B`) is parsed and planned but still not executable through the CPU text-query path.

## What Changes
- Add execution-oriented planner bridge that can compile request semantics for traversal sources.
- Extend CPU `run_query_text` to execute traversal sources using record edge references.
- Support deterministic multi-hop traversal over outbound/inbound directions with final-step filtering/ranking.
- Add planner and CPU tests for traversal execution behavior.

## Impact
- Affected specs: `logical-plan-bridge`, `execution-backend-contract`
- Affected code: `idb-planner`, `idb-executor-cpu`
