## Context

The project direction is to build a unified storage engine where data is represented as points in an arbitrary-dimensional space, then queried through a hybrid retrieval model (structured constraints + vector proximity + optional graph adjacency).

Current docs explain the vision, but v0 lacks a strict system contract for:
- what is stored on write,
- how coordinates are derived and versioned,
- how durable storage and retrieval interact,
- how CPU and future accelerated backends remain semantically aligned.

This design defines that contract without freezing final query syntax.

## Goals / Non-Goals

### Goals
- Define a buildable v0 storage core that can ingest and query mixed structured/vector data.
- Keep a single logical data model independent from physical backend.
- Make CPU backend the correctness oracle.
- Create clean acceleration seams for Cerebras later.
- Keep developer experience as a first-order requirement.

### Non-Goals
- Final iQL syntax design.
- Full real-time changefeed semantics (deferred).
- Distributed multi-node consensus.
- Complete cost-based optimizer.

## Decisions

### 1) Canonical Record Envelope
Use a canonical `RecordEnvelope` per entity instance:
- `record_id` (stable logical id)
- `tenant_id`
- `entity_type`
- `schema_version`
- `dimension_version`
- `structured_fields` (typed)
- `embedding_fields` (original vectors)
- `blob_refs` (optional)
- `edge_refs` (optional)
- `event_time`, `ingest_time`

Rationale:
- Keeps one source of truth while allowing multiple physical projections.
- Supports deterministic re-projection when dimension logic evolves.

### 2) Dimension Registry + Versioned Mapping
Introduce a `DimensionRegistry` that describes each indexed dimension:
- origin (`structured`, `embedding_projection`, `derived`)
- type (numeric, categorical ordinal, temporal, projected float)
- normalization policy
- missing-value policy
- weight contribution policy

Mapping pipeline per write:
1. Validate envelope against schema.
2. Compute/attach embeddings as needed.
3. Project embeddings into indexing dimensions.
4. Normalize all indexing dimensions into bounded coordinate space.
5. Emit coordinate vector + `dimension_version`.

Rationale:
- Arbitrary-dimensional support needs explicit metadata, not hardcoded assumptions.
- Versioning avoids hard-breaking reindex events.

### 3) Space Partitioning and Locality Strategy
Use logical `SpacePages` (or tiles) as bounded coordinate ranges.
- Primary mapping function is configurable (`Hilbert` default in docs, pluggable for alternatives).
- Page metadata tracks min/max key, count, hotness, and version.
- Split/merge policy maintains bounded page size and write amplification control.

Rationale:
- Prevents full-space scans in common retrieval flows.
- Keeps physical layout stable enough for CPU correctness and future hardware export.

### 4) Two-Layer Physical Storage
Store data in two linked layers:
- Hot operational layer: compact fixed/semifixed records + page metadata + mutable indexes.
- Cold/full-fidelity layer: Arrow/Parquet with full payload fidelity and historical snapshots.

Rationale:
- Fast retrieval path for scoring/ranking.
- Full fidelity retained for hydration, audit, and downstream analytics.

### 5) Hybrid Retrieval Contract
Define retrieval as explicit stages:
1. Candidate generation from page ranges and ANN/operator hints.
2. Structured predicate filtering.
3. Distance/score computation (hybrid weighting policy).
4. Deterministic top-k selection (stable tie-break rules).
5. Hydration from full-fidelity layer.

Rationale:
- Backends can optimize internals but must preserve stage semantics.

### 6) Backend Interface Contract
Define backend trait with capabilities:
- `ingest_batch`
- `delete_or_tombstone`
- `query_candidates`
- `score_and_rank`
- `hydrate`
- `capabilities()`

CPU backend MUST implement full contract first.
Cerebras backend MAY implement subset with fallback to CPU for unsupported operations.

Rationale:
- Enables hardware acceleration without coupling core semantics to one backend.

### 7) Durability and Mutation Rules
Write protocol for v0:
1. Append mutation intent to WAL.
2. Apply to page/index state.
3. Commit visibility marker.
4. Asynchronously compact/materialize cold layer.

Required guarantees:
- crash-safe replay to last committed visibility marker,
- idempotent mutation application,
- deterministic conflict policy for same-record updates.

Rationale:
- correctness-first foundation with operationally realistic recovery behavior.

### 8) Explicit Deferral of Live Updates
Live updates/reactive queries are not part of this core change.
Only requirement now: mutation events must be emitted to an internal event bus interface so future changefeed work has stable hooks.

Rationale:
- keeps scope tight while preserving forward path.

## Alternatives Considered

### A) Vector-DB-only architecture
- Pros: fast path to semantic search.
- Cons: weak fit for mixed structured + graph + durable operational workflows.

### B) Relational-first with vector extension only
- Pros: mature durability/query ecosystem.
- Cons: central thesis (single geometric storage model) becomes secondary.

### C) Hardware-first design (Cerebras required from day one)
- Pros: maximal novelty/perf focus.
- Cons: blocks iteration speed and raises execution risk before correctness baseline.

Chosen direction: CPU-first correctness with backend abstraction, Cerebras acceleration later.

## Risks / Trade-offs

- Risk: Arbitrary-dimensional mapping can become unstable across versions.
  - Mitigation: explicit `dimension_version`, dual-read migration windows, reproducible mapping tests.

- Risk: Hybrid score semantics may be hard to explain to developers.
  - Mitigation: explicit score policy objects, explainability output in debug mode.

- Risk: Write amplification from page split/reindex events.
  - Mitigation: bounded page sizes, async compaction thresholds, staged indexes.

- Risk: Overfitting design to planned hardware.
  - Mitigation: strict backend contract and CPU oracle tests.

## Migration Plan

1. Land OpenSpec and approve scope.
2. Implement minimal Rust crates for envelope, registry, mapping, page manager, WAL.
3. Ship CPU-only vertical slice with ingest/query/hydration.
4. Add benchmark and differential correctness suite.
5. Add optional backend capability adapter for Cerebras proof-of-concept.

## Open Questions

1. Which ANN baseline should be v0 default for mixed workloads: HNSW, IVF-PQ, or exact scan with pruning?
2. Should page locality mapping stay Hilbert-first or be selected per entity/index profile?
3. How should graph adjacency influence candidate generation without exploding latency?
4. What is the minimal explainability contract for hybrid ranking?
5. Which embedding drift policy should trigger reprojection/reindex?

## Research Notes (Primary Sources)

- HNSW graph indexing paper: [https://arxiv.org/abs/1603.09320](https://arxiv.org/abs/1603.09320)
- FAISS library/paper index (ANN and IVF/PQ ecosystem): [https://arxiv.org/abs/2401.08281](https://arxiv.org/abs/2401.08281)
- DiskANN and dynamic graph-based ANN line: [https://www.microsoft.com/en-us/research/project/project-akupara-approximate-nearest-neighbor-search-for-large-scale-semantic-search/](https://www.microsoft.com/en-us/research/project/project-akupara-approximate-nearest-neighbor-search-for-large-scale-semantic-search/)
- ACORN (filtered ANN) paper entry: [https://arxiv.org/abs/2403.04871](https://arxiv.org/abs/2403.04871)
- Learned index framing (RMI): [https://arxiv.org/abs/1712.01208](https://arxiv.org/abs/1712.01208)
- Apache Arrow format/spec docs: [https://arrow.apache.org/docs/format/Columnar.html](https://arrow.apache.org/docs/format/Columnar.html)
- DataFusion project docs (Rust query engine context): [https://datafusion.apache.org/](https://datafusion.apache.org/)
- RethinkDB changefeed docs (future inspiration only): [https://rethinkdb.com/docs/changefeeds/javascript/](https://rethinkdb.com/docs/changefeeds/javascript/)
