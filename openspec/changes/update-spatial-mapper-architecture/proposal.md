# Change: Update Spatial Mapper Architecture

## Why
Current repo materials overstate confidence in a single Hilbert-style placement path for high-dimensional fused storage. Recent synthesis across internal docs and external research points to a better architecture boundary: the fused N-space contract should remain the logical truth, while physical placement should be modeled as a pluggable spatial mapper.

## What Changes
- Add a first-class spatial mapper abstraction to the unified spatial storage spec.
- Clarify that fused similarity semantics and physical placement semantics are distinct but connected concerns.
- Establish deterministic spatial mapping as the v0 baseline path.
- Establish learned spatial mapping, including SOM-like approaches, as experimental and benchmark-gated.
- Capture the repo synthesis in a canonical research document.

## Impact
- Affected specs: `unified-spatial-storage`
- Affected code: future `idb-core`, `idb-storage`, candidate generation, Cerebras routing path
- Affected docs: research guidance and architecture positioning
