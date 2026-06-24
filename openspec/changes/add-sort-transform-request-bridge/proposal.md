# Change: Add Sort Transform Request Bridge

## Why
The planner bridge still rejects `sort(...)` transforms even though CPU execution supports ordered ranking via `QueryRequest.order_by`.

## What Changes
- Extend planner bridge to compile `sort(field asc|desc)` into `QueryRequest.order_by`.
- Preserve deterministic conflict behavior when multiple ordering transforms disagree.
- Add planner and CPU tests for `sort + take` text query execution.

## Impact
- Affected specs: `logical-plan-bridge`
- Affected code: `idb-planner`, `idb-executor-cpu`
