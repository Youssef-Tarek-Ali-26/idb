# Change: Validate Durable Stream Consumer Group

## Why
Durable stream poll/commit relies on consumer groups for offset tracking. Empty group names can silently create ambiguous or unintended consumer state.

## What Changes
- Require durable stream `consumer_group` to be non-empty during gateway normalization.
- Reject empty consumer group names in CPU backend durable stream APIs for direct callers.
- Add normalization and backend tests for empty-group rejection.

## Impact
- Affected specs: `server-gateway`
- Affected code: `idb-gateway`, `idb-executor-cpu`
