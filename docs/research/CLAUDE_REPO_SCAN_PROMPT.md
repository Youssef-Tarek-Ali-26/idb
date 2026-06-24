# iDB Upstream Repo Scan Prompt (for Claude)

Use this prompt exactly (or adapt lightly) when scanning upstream DB repos for iDB design patterns.

---

You are helping design iDB, a Rust-first database with:
- unified storage model (structured + semantic + graph-like traversal),
- live updates (watch/changefeed-like behavior),
- durable mutation stream replay with consumer offsets,
- eventual Cerebras acceleration (CPU reference path first).

## Hard Constraints
- Do NOT copy code verbatim.
- Extract patterns, edge cases, invariants, and architecture decisions only.
- Flag licenses and any legal risk if pattern transfer could be too close.
- Keep suggestions implementation-oriented for Rust.

## Primary Goal
Produce a practical implementation blueprint for:
1. Live updates / changefeeds / subscriptions
2. Graph traversal execution at scale
3. Planner/runtime edge-case handling

## Repos To Scan (priority order)
1. `rethinkdb/rethinkdb` (deprecated in 2016, C++, likely best historical reference for live changefeeds)
2. `kuzudb/kuzu` (graph engine/runtime/operator ideas)
3. `arangodb/arangodb` (AQL traversal pruning/filter pushdown patterns)
4. `apache/age` (graph semantics in relational stack)
5. `JanusGraph/janusgraph` (index-first traversal discipline)
6. `nebula-graph/nebula` (planner/runtime profiling and traversal patterns)

Optional extra references:
- `postgres/postgres` (logical decoding / replication edge cases)
- `MaterializeInc/materialize` or similar streaming DB for incremental semantics

## Deliverables (required)

### 1) Pattern Catalog (table)
For each repo, list:
- Pattern name
- Problem it solves
- Core invariant(s)
- Failure modes / edge cases
- How to translate into iDB (Rust)
- Priority: `now` / `next` / `later`

### 2) Edge Case Matrix
At minimum include:
- subscriber reconnect + resume token semantics
- exactly-once vs at-least-once delivery tradeoffs
- offset commit races
- backpressure + slow consumer handling
- out-of-order or duplicate event protection
- tombstones/deletes in live views
- traversal fanout explosions
- cycle handling and depth limits
- deterministic ordering / tie-break rules
- multi-tenant isolation leaks

### 3) iDB Implementation Plan
A concrete phased plan:
- Phase A: must-have for stable v0
- Phase B: performance hardening
- Phase C: advanced features

For each phase:
- API/contract changes
- internal data structures
- runtime invariants
- test plan (unit + property + integration)
- observability metrics

### 4) Anti-Patterns To Avoid
Call out design mistakes observed in upstream systems (or known pitfalls), especially around:
- silent full scans
- unbounded subscriptions
- non-deterministic traversal results
- weak replay guarantees

### 5) "Ship First" Recommendation
Give a short answer:
- what we should build first (2-4 items),
- what can wait,
- what is likely overengineering for now.

## iDB Context Notes
- iDB is different on the N-dimensional unified storage thesis, but NOT completely different in live + traversal mechanics.
- We should borrow proven patterns for live/stream/traversal infra.
- The N-dimensional placement + hardware acceleration layer likely needs more original work + papers.

## Output Format
Return sections in this exact order:
1. Executive Summary
2. Pattern Catalog
3. Edge Case Matrix
4. Phased iDB Plan
5. Anti-Patterns
6. Ship-First Recommendation

Keep it concise but concrete. Prefer actionable engineering detail over theory.
