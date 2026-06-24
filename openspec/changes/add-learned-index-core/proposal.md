# Change: Add Learned Index Core

## Why
Learned indexing is a core architectural direction but currently exists only as design intent, not executable Rust components.

## What Changes
- Add an `idb-index` crate implementing a deterministic learned position model.
- Provide bounded-fallback lookup and range-seed APIs to guarantee correctness under prediction error.
- Add unit tests for training quality, bounded windows, and exact lookup correctness.

## Impact
- Affected specs: `learned-indexing`
- Affected code: new `idb-index` crate, workspace wiring
