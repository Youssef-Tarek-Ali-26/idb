# N-Space Theory Foundation (iDB)

This note formalizes iDB's N-space thesis as an implementable contract.

## 1) Decomposition

Represent each record as a decomposed point:

- `x_s`: structured block (normalized numeric/categorical/temporal projections)
- `x_m`: semantic block (embedding or embedding projections)
- `x_t`: topology block (graph/relationship-derived signals)

Unified point:

`x = (x_s, x_m, x_t) in R^(d_s + d_m + d_t)`

Each block has its own dimensionality and update semantics, but all blocks share:

- one versioned contract (`nspace_version`)
- one fusion policy
- one deterministic scoring boundary

## 2) Similarity Kernel

Define block-level similarity functions:

- `S_s(x_s, y_s)` for structured similarity
- `S_m(x_m, y_m)` for semantic similarity
- `S_t(x_t, y_t)` for topology similarity

Each `S_*` is bounded to `[0, 1]`.

Fused similarity:

`S(x, y) = (w_s * S_s + w_m * S_m + w_t * S_t) / (w_s + w_m + w_t)`

where `w_s, w_m, w_t >= 0` and total weight is strictly positive.

Fused distance:

`D(x, y) = 1 - S(x, y)`

## 3) v0 Kernel Choices

For practical v0 behavior:

- Structured: relative L1 similarity
- Semantic: cosine similarity mapped from `[-1, 1]` to `[0, 1]`
- Topology: weighted Jaccard over non-negative vectors

Why this combination:

- bounded outputs and deterministic behavior
- interpretable weight tuning
- low implementation risk in Rust CPU reference path

## 4) Invariants

For a fixed `nspace_version`:

1. Determinism:
`S(x, y)` must be deterministic for identical inputs.
2. Boundedness:
`0 <= S(x, y) <= 1`.
3. Max self-similarity:
`S(x, x) = 1` (under the configured kernel).
4. Symmetry:
`S(x, y) = S(y, x)`.
5. Version locality:
scores are only comparable inside the same version contract.

## 5) Query Interpretation

Treat query intent as constraints over the decomposed point:

- structured predicates constrain `x_s`
- semantic predicates score/rank via `x_m`
- traversal/topology constraints constrain `x_t` and frontier expansion

Hybrid retrieval remains stage-based:

1. candidate generation
2. filtering
3. fused ranking
4. hydration

## 6) Cerebras Mapping (Theory)

The decomposition naturally maps to accelerator execution lanes:

- lane/group A: structured block eval
- lane/group B: semantic dot/cosine
- lane/group C: topology overlap/graph signal
- reducer: weighted fusion and top-k

This keeps hardware optional for correctness while preserving a clean acceleration seam.

## 7) What To Keep Original vs Borrow

Borrow from existing DBs:

- live update distribution/backpressure patterns
- traversal frontier and uniqueness control patterns

Keep original in iDB:

- unified N-space decomposition and contract versioning
- fused score policy as first-class runtime primitive
- hardware-conscious decomposition boundaries

## 8) v0 Non-Goals

- proving a strict metric-space theorem for every kernel combination
- distributed/global order guarantees across many nodes
- automatic weight learning in-engine

