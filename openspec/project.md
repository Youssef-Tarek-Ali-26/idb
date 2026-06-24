# Project Context

## Purpose
iDB (NornDB) is an experimental, from-scratch data system that treats all data as one unified storage surface in an N-dimensional space.

The long-term thesis is:
- data placement is geometric,
- retrieval is hybrid (structured + semantic + graph-aware),
- execution can scale from CPU to specialized hardware (Cerebras).

This repository is currently design/spec heavy and is used to converge on architecture before large implementation investments.

## Tech Stack
- Core engine language: Rust
- Hardware host/runtime orchestration: Python
- Wafer kernels (future and experimental): Cerebras CSL
- Storage/interchange formats: Arrow and Parquet
- Specification workflow: OpenSpec

## Project Conventions

### Code Style
- Prefer explicit, boring, auditable code over clever code.
- Favor predictable APIs and deterministic behavior.
- Keep public interfaces small and stable.
- Use ASCII in source/docs by default unless required.

### Architecture Patterns
- Unified storage model (no separate "database vs files" product boundary).
- Logical model first, physical backend second.
- CPU-first reference path for correctness.
- Backend abstraction boundary so Cerebras acceleration can be added without rewriting logical semantics.
- Hybrid retrieval pipeline:
  1. candidate generation,
  2. filtering,
  3. ranking/reranking,
  4. hydration.

### Testing Strategy
- Property tests for coordinate transforms and key-space mapping.
- Determinism tests for ranking and tie-break behavior.
- Golden tests for parser/planner/executor boundaries.
- Differential tests to ensure equivalent results across backends (CPU vs accelerated).
- Performance benchmarks are informative only until correctness baselines are locked.

### Git Workflow
- Use focused branches (prefix `codex/` when created by assistant tooling).
- Keep changes small and reviewable.
- Do not mix architecture/spec changes with unrelated refactors.
- Avoid rewriting history for shared branches.

## Domain Context
- iDB targets LLM-era application workloads that mix:
  - structured attributes,
  - high-dimensional embeddings,
  - relationship traversals,
  - real-time product/app constraints.
- iDB is intentionally explored as an independent R&D track before coupling into product repos.
- Query language examples in docs are exploratory and not locked.

## Important Constraints
- Current stage is architecture and specification, not production implementation.
- DX is a first-class goal, not an afterthought.
- Rust is the required engine language for the core runtime.
- Live updates/changefeed semantics are important but can be deferred from storage-layer v0.
- Cerebras should be an acceleration layer, not a blocker for v0 correctness.

## External Dependencies
- Cerebras SDK documentation and runtime APIs
- Apache Arrow format and ecosystem
- Apache Parquet format specification
- ANN and indexing literature (HNSW, FAISS/PQ, learned indexes, hybrid retrieval)
- Optional future reference pattern: RethinkDB-style changefeed semantics (conceptual inspiration only)
