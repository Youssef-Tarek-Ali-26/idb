# N-Space Synthesis

This document is the current synthesis of the three main iDB spatial-design inputs:

- `ARCHITECTURE.md`
- `docs/book/DATA_PIPELINE.md`
- `openspec/specs/SPATIAL-MAPPING-RESEARCH.md`
- external research summary in `/Users/yousseftarek/Downloads/deep-research-report(7).md`

It is not a replacement for implementation specs. It is the current architectural position for how iDB should think about N-dimensional storage placement.

## What Still Holds

- The core iDB thesis is still good: all stored data can be modeled as one fused N-space.
- Structured, semantic, and topology signals should remain part of the same logical retrieval model.
- Cerebras is still interesting because its mesh makes locality and broadcast patterns matter in a way normal hardware does not.
- CPU-first correctness remains the right implementation path.

## What Changes

The old docs were too confident that a single Hilbert-like spatial mapping could be the main answer for placement.

That is no longer the working assumption.

The current synthesized view is:

- fused N-space is the logical model,
- physical placement is a separate mapper problem,
- mapper choice must be pluggable,
- deterministic mapping should be the baseline,
- learned mapping should be experimental until measured.

In other words: similarity semantics and storage placement are related, but they are not the same thing.

## What We Keep From The Older Architecture Docs

- "Every query is geometry" is still the right product-level framing.
- The system should still decompose records into:
  - structured block,
  - semantic block,
  - topology block.
- Hybrid retrieval should still operate over a fused score model.
- The hardware path should still be designed as a backend that benefits from placement quality.

## What We Reject From The Older Architecture Docs

- We should not assume high-dimensional Hilbert placement is good enough by default.
- We should not treat the old 24D-to-Hilbert story as solved.
- We should not lock the architecture to one curve or one learned-index trick too early.
- We should not confuse exact logical semantics with approximate physical placement.

## What We Keep From The Spatial Mapping RFC

- The criticism of `N-dim -> 1D -> 2D` loss is correct.
- Z-order is not good enough for the current thesis.
- A lower-dimensional deterministic mapper is a valid transitional path.
- SOM is the most interesting long-term hardware-native idea in the repo so far.

## What We Keep From The Deep Research Report

- Multi-dimensional learned indexes are real, but they are fragile under updates.
- Existing learned-index and ANN baselines are stronger than repo docs previously implied.
- Storage layout, rebuild cost, and update behavior matter more than abstract model elegance.
- If iDB cannot beat strong baselines on some workload slice, then the architecture remains research, not product.

## Current Working Architecture

### 1. Logical model

Every record is projected into a versioned N-space contract:

- structured coordinates,
- semantic coordinates,
- topology coordinates,
- fused similarity kernel.

This determines how closeness is defined.

### 2. Physical mapper

A mapper converts a point in the fused N-space into:

- physical placement,
- candidate-generation seeds,
- neighborhood/routing hints,
- optional rebuild metadata.

This determines how storage is organized and searched efficiently.

### 3. Mapper families

The current architecture should support at least two families:

- deterministic mapper:
  - simple,
  - rebuildable,
  - explainable,
  - CPU-first,
  - good for baseline correctness and benchmarking

- learned mapper:
  - adaptive,
  - hardware-aware,
  - more fragile under drift and updates,
  - only justified if benchmarks prove it

## Recommended Near-Term Position

For v0:

- keep the fused N-space contract,
- make mapper choice explicit,
- ship a deterministic spatial mapper first,
- keep JL/Hilbert-class ideas as transitional options rather than hard truth,
- defer SOM to an experimental track behind the same mapper interface.

For the research track:

- treat SOM as the most promising wafer-native mapper,
- but do not let it become a required dependency for baseline correctness.

## Why SOM Is Still Important

SOM is still the most exciting idea because it changes the role of the hardware:

- the mesh is not just where queries run,
- the mesh becomes the learned spatial index itself.

That is a real differentiator.

But it brings real costs:

- retraining,
- drift handling,
- tenant partitioning,
- cold start,
- rebuild orchestration,
- harder debugging and explainability.

So SOM should be treated as a serious research mapper, not as default truth.

## What The Repo Should Say Now

The repo should communicate these points clearly:

1. iDB believes in fused N-space as the logical storage model.
2. iDB does not yet claim one final physical mapping strategy.
3. Mapper architecture is pluggable by design.
4. Deterministic mapping is the baseline path.
5. Learned placement, including SOM, is the main research differentiator.

## Immediate Consequences

- OpenSpec should capture spatial mappers as a first-class abstraction.
- GitHub tracking should stop framing JL + Hilbert as the only next answer.
- Future implementation work should separate:
  - N-space contract work,
  - mapper work,
  - storage layout work,
  - backend/hardware work.

## Bottom Line

The big iDB idea survives.

What changes is the engineering stance:

- fused space stays,
- mapper becomes explicit,
- deterministic first,
- learned later,
- benchmark everything.
