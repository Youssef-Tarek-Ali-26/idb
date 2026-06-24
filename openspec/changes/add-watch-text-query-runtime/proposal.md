# Change: Add Watch Text Query Runtime

## Why
Parser and planner understand `watch` mode, but CPU runtime only supports one-shot text query execution. There is no backend API to start and poll a watch query session.

## What Changes
- Add CPU watch text-query APIs for session start and polling.
- On watch start: execute snapshot and register dependency-tracked subscription.
- Reuse existing changefeed engine for ordered mutation delivery.
- Add tests for watch session behavior and deterministic error handling.

## Impact
- Affected specs: `execution-backend-contract`
- Affected code: `idb-executor-cpu`
