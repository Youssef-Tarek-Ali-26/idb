# Change: Add Architecture Capability Catalog

## Why
Core architecture intent currently lives mostly in `ARCHITECTURE.md`, which forces repeated cross-referencing while planning and execution continue via OpenSpec.

## What Changes
- Add a capability catalog in OpenSpec that mirrors major architecture modules and reserved future capabilities.
- Capture normative requirements and scenarios for each architecture area, even when implementation is deferred.
- Keep these capabilities as planning targets so future execution can continue directly from OpenSpec.

## Impact
- Affected specs:
  - `planner-routing`
  - `gpu-executor`
  - `cerebras-runtime`
  - `kernel-memory-layout`
  - `learned-indexing`
  - `secondary-indexing`
  - `blob-storage`
  - `wire-protocol`
  - `server-gateway`
  - `authn-authz-plugin-model`
  - `tenant-scope-rls`
  - `reactive-subscription-index`
  - `cluster-runtime`
  - `observability-and-benchmarks`
- Affected code: none (spec/catalog only)
