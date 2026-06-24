# Change: Add Watch Query Update Batches

## Why
Current watch polling returns only raw mutation events. Runtime clients still need custom logic to map those events back to current query row state.

## What Changes
- Add CPU API that polls watch events and resolves current row state for each changed dependency.
- Track active watch query request metadata per subscription.
- Return deterministic update batches with per-event current match state.

## Impact
- Affected specs: `execution-backend-contract`
- Affected code: `idb-executor-cpu`
