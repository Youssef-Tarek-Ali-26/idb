## Why
The learned indexing core model exists, but candidate generation behavior for key-range hints in the CPU executor was not explicitly captured as a concrete OpenSpec execution contract. We need a spec-backed requirement that range candidate generation can use learned prediction while preserving correctness and mutation consistency.

## What Changes
- Add execution-backend-contract requirements for learned key-range candidate acceleration.
- Specify correctness-preserving behavior for range hints and non-spatial records.
- Specify mutation consistency expectations for upsert/delete/replay.

## Impact
- Documents and stabilizes learned-index-driven candidate generation semantics in the CPU path.
- Reduces ambiguity as we add GPU/Cerebras implementations of equivalent behavior.
