# Research Gaps: Unified Spatial Storage v0

This file records unresolved technical decisions that need targeted experiments before committing to irreversible implementation details.

## Gap 1: ANN Baseline for Hybrid Workloads

### Why it matters
The engine must support structured filters and vector similarity together. ANN choices differ significantly under filtered retrieval and update-heavy ingest.

### Candidate options
- HNSW (strong recall/latency, mutable graph complexity)
- IVF-PQ style indexes (compression + speed, tuning complexity)
- DiskANN/Vamana family (high scale, dynamic update costs)
- Exact scan with pruning for early correctness baseline

### Decision criteria
- Recall@k under filter selectivity buckets
- p50/p95 latency under mixed read/write load
- Index build/update cost
- Memory amplification and operational simplicity

### Sources
- HNSW: [https://arxiv.org/abs/1603.09320](https://arxiv.org/abs/1603.09320)
- FAISS overview: [https://arxiv.org/abs/2401.08281](https://arxiv.org/abs/2401.08281)
- DiskANN project: [https://www.microsoft.com/en-us/research/project/project-akupara-approximate-nearest-neighbor-search-for-large-scale-semantic-search/](https://www.microsoft.com/en-us/research/project/project-akupara-approximate-nearest-neighbor-search-for-large-scale-semantic-search/)
- Filtered ANN (ACORN): [https://arxiv.org/abs/2403.04871](https://arxiv.org/abs/2403.04871)

## Gap 2: Coordinate Mapping Strategy Stability

### Why it matters
Arbitrary-dimensional mapping is core to the thesis. If coordinate derivation changes too often, reindexing cost and operational complexity explode.

### Candidate options
- Fixed mapping registry with strict version upgrades
- Per-entity mapping profiles with local versioning
- Adaptive mapping (dynamic weighting/projection) with periodic rebuilds

### Decision criteria
- Determinism across releases
- Reprojection cost and migration safety
- Query behavior stability across dimension versions

## Gap 3: Locality Function Selection

### Why it matters
Space-filling curve choice influences locality, range decomposition, and split/merge behavior.

### Candidate options
- Hilbert-first strategy across all indexes
- Strategy pluggable per index (`hilbert`, `morton`, custom)
- No global key map, page partition by learned clustering only

### Decision criteria
- Candidate generation efficiency
- Ease of incremental writes
- Simplicity of implementation and explainability

## Gap 4: Write Path and Compaction Economics

### Why it matters
A unified storage system must preserve durability while avoiding excessive write amplification from page maintenance and index updates.

### Candidate options
- WAL + mutable pages + background compaction
- LSM-like staged runs for all indexed structures
- Append-only immutable pages with periodic rebuild windows

### Decision criteria
- Crash recovery correctness
- Sustained ingest throughput
- Storage and compaction amplification

### Sources
- Bigtable/LSM-inspired architecture paper: [https://research.google/pubs/pub27898/](https://research.google/pubs/pub27898/)

## Gap 5: Explainability Contract for DX

### Why it matters
If hybrid retrieval behavior is opaque, DX degrades quickly even if performance is good.

### Candidate options
- Minimal plan/debug view (stages + counts + final score)
- Full per-stage explain traces (dev mode)
- Natural-language explanation layer on top

### Decision criteria
- Developer comprehension time
- Runtime overhead in normal mode
- Stability of semantics as internals evolve

## Gap 6: Deferred Live Update Model

### Why it matters
Live query updates are part of product vision, but storage core should not block on full reactive engine design.

### Candidate options
- Emit canonical mutation events only in v0
- Add subscription index in same milestone
- Use external stream processor for first iteration

### Decision criteria
- v0 delivery speed
- Forward compatibility with rich changefeeds
- Consistency guarantees for subscribers

### Reference for future inspiration
- RethinkDB changefeeds docs: [https://rethinkdb.com/docs/changefeeds/javascript/](https://rethinkdb.com/docs/changefeeds/javascript/)

## Suggested Experiment Sequence

1. Build CPU-only baseline with exact scan + structured filters + deterministic ranking.
2. Plug one ANN baseline (HNSW) behind same candidate interface.
3. Add filtered ANN benchmark scenarios and measure recall/latency tradeoff.
4. Compare one alternative locality function using same dataset and query corpus.
5. Lock v0 defaults only after benchmark and correctness thresholds are met.
