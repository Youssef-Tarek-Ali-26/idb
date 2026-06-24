# Change: Add Watch Unsubscribe Lifecycle

## Why
Watch subscriptions can be created and polled, but there is no explicit lifecycle API to stop a watch and release runtime state.

## What Changes
- Add unsubscribe support in changefeed engine.
- Add CPU `stop_watch` API that removes both changefeed subscription and active watch metadata.
- Add tests for stop semantics.

## Impact
- Affected specs: `execution-backend-contract`, `live-changefeed`
- Affected code: `idb-storage`, `idb-executor-cpu`
