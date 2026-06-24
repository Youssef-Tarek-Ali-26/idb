# RFC: N-Dimensional to PE Grid Spatial Mapping Strategy

**Status:** Draft
**Scope:** Research
**Created:** 2026-04-05

## Problem

The current architecture maps N-dimensional data (24+ dims: structured + semantic + topology) to a 1D key via bit interleaving (Z-order), which then maps to PE positions on the Cerebras WSE 2D mesh (750x994).

This N→1→2 path has two lossy steps:
1. N-dim → 1D key: Hilbert/Z-order curves lose locality significantly above ~8 dimensions
2. 1D key → 2D grid: recovers some structure but the damage from step 1 is done

The result: points close in 24D space may land on distant PEs, defeating the purpose of spatial placement for query broadcast optimization.

## Current Implementation

`InterleavedKeyMapper` in `idb-core/src/keyspace.rs` does Z-order bit interleaving (not true Hilbert). For N dimensions at B bits each, interleaves bit B-1 of all dims, then B-2, etc., producing a u128 key.

`DimensionProjectionPolicy` only supports `FirstN` (take first N dimensions), not Johnson-Lindenstrauss random projection.

## Proposed Solutions (ranked by feasibility)

### Option A: JL Projection + Hilbert 8D (near-term, prototype)

```
24D data → JL random projection → 8D → Hilbert curve → PE assignment
```

- Replace `FirstN` with proper Johnson-Lindenstrauss random projection matrix
- Replace Z-order interleaving with true Hilbert encoding (Skilling's algorithm)
- At 8D, Hilbert locality preservation is still reasonable
- JL lemma guarantees pairwise distances preserved within (1±ε)

**Effort:** ~500 LOC (projection matrix + Hilbert encoder)
**Risk:** 8D is at the edge of where Hilbert starts degrading

### Option B: Self-Organizing Map (long-term, production)

```
24D data → trained SOM (750x994 grid) → direct PE assignment
```

Skip the 1D intermediary entirely. Train a Self-Organizing Map where the grid dimensions match the PE mesh. Each PE cell learns to represent a Voronoi region of the N-dimensional space.

- SOM is literally designed for this: map high-D data onto a 2D grid preserving topology
- The PE grid IS the spatial index -- data placed where it's most similar
- Queries map to the same grid coordinates, broadcast to nearby PEs holding nearby data
- SOM training is massively parallel -- could run ON the Cerebras hardware itself (each PE computes distance to input, winner + neighborhood update)
- New records: BMU (Best Matching Unit) lookup is O(grid_size) but trivially parallelizable on PEs

**Effort:** ~2-3k LOC (SOM training, BMU lookup, incremental updates, model persistence)
**Risk:** Training is offline/batch. Cold start needs pre-training. Data distribution shifts need periodic retraining.

### Option C: Hierarchical Decomposition

Don't treat all dimensions equally. Decompose by block type:

```
Structured dims (5-8D) → coarse PE region assignment (Hilbert works fine here)
Semantic dims (8-16D)  → fine-grained PE within region (clustered, not spatially ordered)
Topology dims (2-4D)   → not spatially encoded (graph traversal is pointer-following)
```

- Each level is low-dimensional where Hilbert/Z-order works well
- Structured dimensions have natural orderings
- Semantic dimensions only need coarse candidate generation, not perfect ordering
- Graph topology shouldn't be spatially encoded at all

**Effort:** ~1k LOC (hierarchical key construction, region assignment, cluster-within-region)
**Risk:** Non-uniform PE utilization. Some regions will be hot, others empty. Needs rebalancing.

### Option D: Product Quantization-style Subspace Partitioning

Split N dimensions into M subspaces (e.g., 4 groups of 6 dimensions). Run low-D Hilbert in each independently. PE assignment uses composite key.

**Effort:** ~800 LOC
**Risk:** Records may need replication across subspace perspectives. Storage overhead.

## Recommendation

**Phase 1 (now):** Option A -- JL + Hilbert 8D. Smallest code change, biggest improvement over current Z-order 24D. Gets us to a working system where locality actually means something.

**Phase 2 (pre-Cerebras):** Option B -- SOM. The grid IS the hardware. This is the architecturally correct solution for wafer-scale. The SOM can be trained on data distribution, and the mapping self-optimizes.

Option C (hierarchical) is a good fallback if SOM training proves too expensive or unstable.

## Code Changes Required

### Phase 1 (JL + Hilbert)

```
idb-core/src/dimensions.rs:
  - Add DimensionProjectionPolicy::JohnsonLindenstrauss { target_dims: 8, seed: u64 }
  - Implement random projection matrix generation (Achlioptas sparse projection)
  - Apply projection in normalize_and_project()

idb-core/src/keyspace.rs:
  - Replace InterleavedKeyMapper with HilbertKeyMapper
  - Implement Skilling's Hilbert encode/decode for 8D
  - Same u128 output key

Tests:
  - Property test: projected distances preserve ordering within ε
  - Hilbert locality test: nearby points in 8D → nearby keys
  - Round-trip: encode → decode → verify coordinates
```

### Phase 2 (SOM)

```
New crate: idb-som/
  - SOM grid structure (750x994 weight vectors, each 24D)
  - Training: batch SOM with learning rate decay + neighborhood shrinking
  - BMU lookup: find nearest grid cell for a record
  - Incremental update: adjust weights for new records
  - Persistence: save/load trained model

idb-core/src/keyspace.rs:
  - Add SOMMapper implementing same trait as HilbertKeyMapper
  - PE assignment = (bmu_row, bmu_col) directly

idb-storage/src/spatial.rs:
  - Support both key types (Hilbert u128 and SOM (u16, u16))
  - Index rebuilds when SOM is retrained

Cerebras integration:
  - SOM training kernel (each PE computes distance, collective neighborhood update)
  - Tile assignment follows SOM mapping directly
  - Query broadcast: map query to BMU, wavelet to neighborhood PEs
```

## Open Questions

1. **SOM stability under data drift:** How often does the map need retraining as data distribution changes? Can incremental SOM updates (online learning) keep up, or does it need periodic full retraining?

2. **Multi-tenant SOM:** One SOM per tenant, or a shared SOM across tenants? Per-tenant is more accurate but 750x994 grid per tenant is wasteful if tenants are small.

3. **Embedding latency vs. Cerebras latency:** If `meaning("elegant")` requires a 5ms embedding model call, the 33μs Cerebras execution is dominated by embedding time. Should embedding inference run on-wafer too? (This is a separate research problem.)

4. **48KB constraint with SOM:** SOM doesn't change the PE memory constraint. Records still need to fit in 48KB SRAM. The SOM just ensures the RIGHT records land on nearby PEs.

5. **Hybrid queries:** A query like `Product where price < 100 and meaning("elegant")` has both structured and semantic components. The SOM should be trained on the FUSED space, not just embeddings. This means the SOM weights are in the same N-space as the fused similarity kernel.

## References

- Kohonen, T. (1982). Self-organized formation of topologically correct feature maps.
- Skilling, J. (2004). Programming the Hilbert curve.
- Kraska, T. et al. (2018). The Case for Learned Index Structures.
- Johnson, W.B. & Lindenstrauss, J. (1984). Extensions of Lipschitz mappings into a Hilbert space.
- Achlioptas, D. (2003). Database-friendly random projections.
