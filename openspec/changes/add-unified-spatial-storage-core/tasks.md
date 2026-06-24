## 1. OpenSpec Foundation
- [x] 1.1 Finalize and approve `proposal.md` scope and non-goals.
- [x] 1.2 Finalize `design.md` decisions on mapping, partitioning, and backend boundaries.
- [x] 1.3 Resolve open questions that block implementation sequencing.

## 2. Rust Core Scaffolding (CPU-first)
- [x] 2.1 Create core crate/module boundaries for envelope, mapping, storage, and query contracts.
- [x] 2.2 Define canonical `RecordEnvelope` and typed field model.
- [x] 2.3 Implement `DimensionRegistry` models and validation rules.
- [x] 2.4 Implement deterministic normalization/projection interfaces with version tags.

## 3. Space Partition and Durability
- [x] 3.1 Implement logical page/tile metadata structures.
- [x] 3.2 Implement key mapping adapter (initial Hilbert-compatible adapter).
- [x] 3.3 Implement WAL append + replay + visibility marker.
- [x] 3.4 Implement split/merge policies with deterministic thresholds.

## 4. Query Pipeline v0
- [x] 4.1 Implement candidate generation contract (range + ANN hook).
- [x] 4.2 Implement structured filter stage.
- [x] 4.3 Implement hybrid scoring and deterministic top-k tie-breaks.
- [x] 4.4 Implement hydration from full-fidelity storage layer.

## 5. Backend Contract
- [x] 5.1 Define backend trait with capability negotiation.
- [x] 5.2 Implement full CPU backend as correctness oracle.
- [x] 5.3 Implement fallback behavior for partially-supported accelerated backends.

## 6. Correctness and Test Harness
- [x] 6.1 Add property tests for mapping stability and boundary conditions.
- [x] 6.2 Add replay/recovery tests for WAL crash scenarios.
- [x] 6.3 Add differential tests: logical query equivalence across backends.
- [x] 6.4 Add deterministic ranking tests for tied scores.

## 7. Performance and Observability Baseline
- [x] 7.1 Define benchmark corpus and workload classes.
- [x] 7.2 Add baseline metrics for ingest latency, query latency, and storage amplification.
- [x] 7.3 Add query-stage tracing and explainability output for debug mode.

## 8. Deferred Work Registration
- [x] 8.1 Create a follow-up OpenSpec change for live updates/changefeed semantics.
- [x] 8.2 Create a follow-up OpenSpec change for full query-language finalization.
- [x] 8.3 Create a follow-up OpenSpec change for Cerebras-specific kernel contracts.
