# Change: Add Durable Mutation Stream Mirror

## Why
In-memory changefeed polling is useful for live sessions but does not provide a durable consumer-group replay surface for mutation events.

## What Changes
- Mirror CPU ingest/delete mutation events into a per-tenant ordered topic using `idb-ordered-log`.
- Expose CPU APIs to poll and commit durable mutation stream offsets.
- Add CPU tests for durable stream replay, offset commit, and state persistence across backend reopen.

## Impact
- Affected specs: `execution-backend-contract`
- Affected code: `idb-executor-cpu`, workspace wiring for `idb-ordered-log`
