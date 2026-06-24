# Change: Tiered Compute Fabric Model

## Why
NornCloud needs a compute model that preserves WASM simplicity while avoiding the hard limits of WASM-only execution. Many workloads are pure state transforms and should run directly in iDB, while others need imperative logic or native acceleration.

## What Changes
- Add a Tier 0 spatial transform runtime capability where pure deterministic transforms execute directly in iDB.
- Add explicit compute tier routing policy (Tier 0 spatial, Tier 1 WASM, Tier 2 native capabilities) with fallback semantics.
- Add shared WASM executable page requirements to improve multi-tenant memory density.

## Impact
- Affected specs: spatial-transform-runtime-tier0, compute-tier-routing-policy, shared-wasm-executable-pages
- Affected code (future): planner, scheduler, iDB transform executor, WASM runtime memory manager, capability metadata registry
- Product impact: one deployment model with clearer latency/cost tiers and better hardware utilization
