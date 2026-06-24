## Context

iDB currently has three overlapping spatial narratives:

- older architecture docs that assume Hilbert-like mapping as the default placement path,
- newer N-space theory work that defines fused similarity more carefully,
- recent spatial mapping research showing that high-dimensional placement quality degrades and that learned mapping may be more appropriate on Cerebras-class hardware.

The repo needs one coherent architectural boundary.

## Goals

- Preserve fused N-space as the logical storage model.
- Separate similarity semantics from placement semantics.
- Make mapper choice explicit and swappable.
- Keep CPU-first correctness as the baseline.
- Allow learned mapper research without making it a core dependency.

## Non-Goals

- Select a final production learned mapper in this change.
- Commit the repo to SOM as the only learned mapper.
- Implement the mapper interface in this proposal.

## Decisions

### Decision: Introduce a first-class spatial mapper layer

The architecture should treat physical placement as a mapper layer that consumes a versioned fused N-space point and emits physical placement metadata.

Why:

- It keeps logical retrieval semantics stable while placement evolves.
- It lets deterministic and learned approaches coexist behind one contract.
- It avoids prematurely locking the engine to Hilbert-style placement.

### Decision: Deterministic mapper is the baseline

The first accepted mapper for v0 should be deterministic, explainable, and rebuildable with predictable behavior on CPU.

Why:

- It gives the project a measurable baseline.
- It avoids making a learned mapper a prerequisite for correctness.
- It reduces debugging and migration complexity in early phases.

### Decision: Learned mapper is benchmark-gated

Learned placement, including SOM-style mapping, should remain experimental until it demonstrates meaningful gains under explicit benchmarks.

Why:

- Literature suggests multi-dimensional learned indexing is fragile under updates.
- Cerebras-native mapping is compelling, but only if it survives rebuild, drift, and tenant-partition pressure.
- A benchmark gate prevents architectural drift into unvalidated complexity.

## Mapper Contract Shape

The contract should be able to express:

- input:
  - fused N-space point,
  - contract version,
  - mapper version/config
- output:
  - placement token or region id,
  - candidate-generation seed(s),
  - neighborhood/routing metadata,
  - explain/debug metadata

It should also define whether a mapper is:

- deterministic or learned,
- exact or approximate for candidate seeding,
- rebuild-required on config changes,
- compatible with CPU-only validation,
- compatible with Cerebras routing acceleration.

## Risks / Trade-offs

- A mapper abstraction adds one more concept to the system model.
  - Mitigation: keep the interface narrow and focused on placement/routing only.

- Deterministic baseline work may feel less exciting than jumping straight to SOM.
  - Mitigation: keep SOM as an explicit research track, not an abandoned idea.

- Learned mappers may require tenant-aware partitioning or retraining logic that complicates the storage engine.
  - Mitigation: do not promote learned mapping without benchmarked operational stories.

## Migration Plan

1. Canonicalize repo research posture in docs.
2. Add OpenSpec requirements for the mapper layer.
3. Introduce mapper metadata types in `idb-core`.
4. Adapt current keyspace mapping to implement the deterministic mapper contract.
5. Add learned mapper experiments behind the same contract.

## Open Questions

- Should the deterministic baseline be Hilbert-like, hierarchical, or another low-dimensional mapping?
- What exact debug/explain surface should a mapper expose?
- Should mapper selection be per-entity, per-index, per-tenant, or global?
