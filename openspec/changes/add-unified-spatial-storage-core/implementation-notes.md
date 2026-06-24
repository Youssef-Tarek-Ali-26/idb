# Implementation Notes (2026-02-25)

## Completed in this pass

- Created Rust workspace and crate boundaries:
  - `crates/idb-core`
  - `crates/idb-storage`
  - `crates/idb-executor-cpu`
  - `crates/idb-executor-cerebras-stub`
- Implemented canonical record model (`RecordEnvelope`, typed fields, tenant/entity identity).
- Implemented dimension registry and deterministic coordinate mapping pipeline.
- Implemented keyspace adapter (`InterleavedKeyMapper`) for coordinate-to-key encoding.
- Implemented page metadata and split/merge threshold checks.
- Implemented JSONL WAL + visibility marker + replayable durable state.
- Implemented hot/cold separation in durable state:
  - hot compact records for candidate/filter/score stages
  - cold full envelopes for hydration
- Implemented mutation event emission hooks.
- Implemented backend contract + capability model.
- Implemented fallback backend wrapper (`FallbackBackend`) for partial backend support.
- Added Cerebras stub backend and validated CPU fallback routing through wrapper tests.
- Implemented CPU backend with staged query flow:
  - candidate generation
  - predicate filtering
  - hybrid scoring
  - deterministic top-k tie-break
  - hydration
- Implemented query-stage trace output (`QueryTrace`, `StageTrace`) from CPU execution.
- Implemented baseline engine metrics (ingest/query latency, WAL bytes/entries, record counts, storage amplification).
- Implemented benchmark corpus/workload scaffolding for v0 synthetic workloads.
- Resolved v0 blocking design questions in `open-questions-resolution.md`.
- Registered follow-up OpenSpec changes:
  - `add-live-changefeed-semantics`
  - `finalize-query-language-v0`
  - `add-cerebras-kernel-contracts`
- Added spatial candidate pruning support:
  - optional spatial indexer in durable state
  - per-record `space_key` computation at ingest/replay
  - candidate key-range hint filtering in CPU backend
- Implemented first live-changefeed runtime slice:
  - in-memory subscription registry
  - resume token support
  - ordered event polling per tenant
  - optional record-level dependency filters
- Implemented first Cerebras contract slice in stub backend:
  - versioned kernel input/output envelopes
  - host-side dispatch validation
  - CPU-oracle conformance comparison helper
- Finalized query-language-v0 artifact pack:
  - EBNF grammar
  - semantics and precedence notes
  - conformance case corpus
  - canonical SDK AST fixtures
  - compatibility/deprecation policy
- Implemented `idb-parser` crate:
  - lexer + recursive-descent parser for v0 grammar subset
  - AST helpers for root/mode/predicate/semantic/hop conformance checks
  - canonical fixture serializer
  - tests that read OpenSpec conformance YAML and fixture JSON artifacts
- Added tests for:
  - mapping determinism
  - property-based mapping and keyspace stability
  - key mapping determinism
  - WAL replay recovery
  - deterministic ranking tie-breaks
  - CPU vs independent reference differential query equivalence
  - key-range candidate pruning behavior
  - spatial indexer hot-record key population
  - changefeed reconnect + resume token continuity

## Current known gaps

- CPU vs accelerated backend differential tests are still not implemented (only CPU vs reference for now).
- live-changefeed dependency tracking and multi-tenant isolation tests are not implemented yet.

## Next high-leverage steps

1. Add multi-tenant isolation tests and query-level dependency tracking for changefeeds.
2. Add CPU-vs-accelerated differential test scaffolding using cerebras stub contracts.
3. Add richer explain/debug output payloads (per-stage counters, predicate stats).
4. Expand benchmark harness from corpus spec to runnable workload driver.
