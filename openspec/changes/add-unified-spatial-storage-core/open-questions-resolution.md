# Open Question Resolution (v0 Defaults)

This document resolves the blocking design questions from `design.md` for v0 implementation sequencing.

## 1) ANN baseline choice
- **Decision (v0):** CPU reference path defaults to exact scan over candidate set.
- **Rationale:** correctness and deterministic behavior first; ANN plugged behind same interface later.

## 2) Locality mapping strategy
- **Decision (v0):** Interleaved/Hilbert-compatible key mapping is the default.
- **Rationale:** aligns existing architecture docs and keeps page partition strategy coherent.

## 3) Graph adjacency influence
- **Decision (v0):** graph adjacency is stored but does not alter candidate generation yet.
- **Rationale:** avoid latency explosion and isolate storage core scope.

## 4) Explainability contract
- **Decision (v0):** stage-level query trace with input/output counts and elapsed micros.
- **Rationale:** low-overhead, immediately actionable for DX/debug.

## 5) Embedding drift policy
- **Decision (v0):** drift does not auto-trigger remap; remap is manual via explicit dimension-version bump.
- **Rationale:** deterministic operations and explicit migration control.
