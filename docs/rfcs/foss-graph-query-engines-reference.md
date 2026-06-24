# FOSS Graph Query Engines Reference

This file captures practical engines and design patterns we can learn from while building iDB query/runtime internals.

## Engines To Study

- JanusGraph
  - Docs: https://docs.janusgraph.org/
  - Focus: global indexes, vertex-centric indexes, and index-first query discipline (`force-index`) to avoid accidental scans.

- Kuzu
  - Docs: https://docs.kuzudb.com/
  - Focus: vectorized graph execution, compact adjacency layout, join-order behavior, and profiling/tuning workflow.

- Neo4j (Cypher engine concepts)
  - Docs: https://neo4j.com/docs/cypher-manual/current/planning-and-tuning/
  - Focus: mature query planner/runtime patterns, explain/profile ergonomics, and index-aware hints.

- ArangoDB (AQL graph traversal)
  - Docs: https://docs.arangodb.com/3.12/aql/graphs/traversals/
  - Focus: traversal pruning, optimizer rules, and practical filter pushdown in graph walks.

- NebulaGraph
  - Docs: https://docs.nebula-graph.io/
  - Focus: explain/profile operator view, native index behavior, and query tuning workflow.

- Apache AGE
  - Docs: https://age.apache.org/
  - Focus: graph query support integrated into a relational planning/execution stack.

## Anti N+1 / High-Performance Query Patterns

- Set-at-a-time execution
  - Compile nested operations into one physical plan instead of per-node follow-up queries.

- Index-anchored starts
  - Start traversals from the most selective predicates and indexes, then expand from a reduced frontier.

- Early pruning and filter pushdown
  - Apply filter predicates during traversal expansion to reduce fanout.

- Batched adjacency expansion
  - Expand neighbors in batches over adjacency-oriented structures, not one random lookup per result row.

- Deterministic explain/profile loop
  - Expose plan operators and cardinalities so bad plans are visible and tunable.

- Stable tie-break and ordering semantics
  - Keep deterministic order under ties for reproducibility and testing.

## Translation To iDB (Near-Term)

- Keep parser/planner/executor boundaries explicit.
- Maintain CPU-first deterministic reference path.
- Prefer index-first candidate generation and bounded fanout.
- Treat `EXPLAIN` as a first-class DX feature, not optional tooling.
