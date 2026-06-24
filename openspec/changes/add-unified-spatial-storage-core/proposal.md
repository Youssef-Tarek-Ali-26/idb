# Change: Add Unified Spatial Storage Core Spec (v0)

## Why
The project has a strong vision but no canonical OpenSpec definition for the first buildable core. We need a concrete, reviewable spec for the unified arbitrary vector-space storage layer so implementation can start without locking query syntax or hardware-specific decisions too early.

## What Changes
- Add a new capability spec `unified-spatial-storage` that defines:
  - canonical storage envelope and identity model,
  - dimension registry and coordinate transform pipeline,
  - partitioning and tile/page lifecycle,
  - durability and consistency rules for ingest/mutation,
  - hybrid retrieval contract (structured + vector + graph adjacency hooks),
  - deterministic ranking and result hydration behavior,
  - tenant and access isolation requirements.
- Add a new capability spec `execution-backend-contract` that defines:
  - backend trait/interface contract,
  - CPU reference backend requirements,
  - cross-backend consistency expectations,
  - optional Cerebras backend integration boundary.
- Add a detailed design document for architecture decisions, tradeoffs, and phased implementation.
- Add a research gap register grounded in primary sources to identify unresolved choices before coding.
- Add a detailed task plan oriented around shipping a correctness-first v0 slice.

## Scope Notes
- Query language syntax is intentionally out of scope for finalization in this change.
- Live changefeeds/reactive updates are explicitly deferred to a later change; only storage/event hooks are specified.

## Impact
- Affected specs:
  - `unified-spatial-storage` (new)
  - `execution-backend-contract` (new)
- Affected code (future implementation phase):
  - Rust core crates for storage, mapping, planner, and CPU executor
  - Python/Cerebras integration boundary code
  - ingestion and query interfaces
- Repo/process impact:
  - Establishes OpenSpec as the source of truth for iDB v0 architecture decisions
