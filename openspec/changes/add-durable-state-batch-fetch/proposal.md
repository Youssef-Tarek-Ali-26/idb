# Change: Add Durable State Batch Fetch

## Why
Traversal and hydration paths currently perform one record lookup per id, which increases overhead and blocks storage-layer batching optimizations.

## What Changes
- Add a `DurableState::get_many` API that returns positional `Option<RecordEnvelope>` results for a tenant and record id list.
- Use batch fetch in CPU hydration and outbound traversal expansion paths.
- Add regression tests for positional ordering, missing record handling, and hydrate ordering.
- Sync runtime diagrams to reflect batched hydration behavior.

## Impact
- Affected specs: `execution-backend-contract`
- Affected code: `idb-storage`, `idb-executor-cpu`, `docs/book/DB_DIAGRAMS.md`, `docs/book/DB_DIAGRAMS_ASCII.md`
