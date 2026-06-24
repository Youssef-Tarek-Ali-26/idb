# Change: Add Positive Poll Bounds Validation

## Why
Watch and durable mutation stream poll APIs accept optional max-event bounds, but zero values should be rejected to avoid invalid polling semantics and accidental no-op loops.

## What Changes
- Require watch poll `max_events` to be strictly positive at gateway normalization.
- Require durable stream poll `max_events_per_partition` to be strictly positive at gateway normalization.
- Add backend guardrails so direct CPU backend API calls also reject zero poll bounds.
- Add tests for normalization and backend validation behavior.

## Impact
- Affected specs: `server-gateway`
- Affected code: `idb-gateway`, `idb-executor-cpu`
