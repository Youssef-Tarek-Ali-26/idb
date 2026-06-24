## 1. Tier Model
- [ ] 1.1 Define Tier 0 eligibility contract (pure, deterministic, no side effects).
- [ ] 1.2 Define Tier 1 WASM contract and Tier 2 native capability contract.
- [ ] 1.3 Define fallback/escalation behavior across tiers.

## 2. Spatial Transform Runtime (Tier 0)
- [ ] 2.1 Define transform artifact model and versioning lifecycle.
- [ ] 2.2 Define static validation and admission controls for Tier 0 transforms.
- [ ] 2.3 Define deterministic execution and replay guarantees.

## 3. Tier Routing Policy
- [ ] 3.1 Define scheduler routing inputs (capabilities, data locality, hardware intent, policy).
- [ ] 3.2 Define required vs preferred capability semantics.
- [ ] 3.3 Define observability and explain output for tier-routing decisions.

## 4. Shared WASM Executable Pages
- [ ] 4.1 Define binary identity and compatibility keys for page sharing.
- [ ] 4.2 Define isolation guarantees for shared executable memory and per-tenant data memory.
- [ ] 4.3 Define eviction and cache-pressure behavior.

## 5. Validation
- [ ] 5.1 Validate this change with `openspec validate add-tiered-compute-fabric --strict --no-interactive`.
- [ ] 5.2 Integrate with follow-up implementation changes in compute/runtime/scheduler tracks.
