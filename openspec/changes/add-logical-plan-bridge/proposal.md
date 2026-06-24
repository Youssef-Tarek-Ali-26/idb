# Change: Add Logical Plan Bridge

## Why
The parser now produces ASTs, but there is no canonical translation into executable query requests for the CPU backend. A planner bridge is needed to turn query text into typed plan/request structures.

## What Changes
- Add `logical-plan-bridge` capability spec.
- Add `idb-planner` crate with AST-to-logical-plan translation.
- Add translator from logical plan to `idb_core::QueryRequest` for executable subset.
- Add integration helper in CPU backend to execute query text through parser + planner.

## Impact
- Affected specs: `logical-plan-bridge` (new)
- Affected code: `idb-parser`, new `idb-planner`, `idb-executor-cpu`
