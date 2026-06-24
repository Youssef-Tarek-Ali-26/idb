# iDB (NornDB)

iDB is a unified storage engine built around one idea:
all data can live in a single N-dimensional space.

Instead of separating "database data" and "file data", iDB treats everything
as storage in one system. Queries become geometric + ML search over that space:
find what is close, related, similar, or connected.

## Core Idea

- Build one N-dimensional representation for all stored data.
- Use ML embeddings plus structured attributes to place data in that space.
- Run search directly in that space (semantic, structured, graph-style, hybrid).
- Execute on Cerebras hardware to get wafer-scale parallel query speed.

## Why Cerebras

The project is designed for extreme parallelism. Cerebras gives a very large
mesh of processing elements that can run these space-search operations at once.
That is the performance bet behind iDB.

## What This Repo Is

Right now, this repository is the architecture and research/design foundation:

- Product and system architecture
- Data pipeline walkthrough
- iQL language examples
- Cerebras kernel and CSL reference docs

## Current Status

- Stage: design and architecture
- Main goal: validate the unified storage + N-dimensional search model
- Next step: implement the first working core/runtime path

## Implementation Snapshot

An initial CPU-first Rust workspace now exists:

- `crates/idb-core`: canonical envelope types, dimension registry, key mapping, query/backend contracts
- `crates/idb-storage`: WAL, visibility markers, replayable durable state, mutation events
- `crates/idb-executor-cpu`: reference backend with candidate/filter/score/rank/hydrate flow, stage tracing, baseline metrics, and text query execute/explain/watch entry points (including query-aware watch update batches and stop/unsubscribe lifecycle)
- `crates/idb-executor-cerebras-stub`: acceleration placeholder backend for capability/fallback wiring
- `crates/idb-parser`: v0 query lexer/parser + canonical AST fixture/conformance tests
- `crates/idb-planner`: AST-to-logical-plan translator + logical-plan-to-`QueryRequest` bridge (including deterministic v0 `meaning(...)` semantic compilation, threshold support, ordered `top(k, field dir)` mapping, traversal execution projection, and explain output)

## Documentation Map

- [Architecture Plan](./ARCHITECTURE.md)
- [Visual Guide (Diagrams)](./docs/book/DB_DIAGRAMS.md)
- [Visual Guide (ASCII)](./docs/book/DB_DIAGRAMS_ASCII.md)
- [Data Pipeline](./docs/book/DATA_PIPELINE.md)
- [iQL Language](./docs/book/IQL_LANGUAGE.md)
- [Cerebras Kernel Docs](./cerebras/docs/)
- [CSL Reference](./docs/csl-reference/README.md)
- [FOSS Engine Notes](./docs/rfcs/foss-graph-query-engines-reference.md)
- [Upstream Research Workflow](./docs/research/README.md)
- [N-Space Theory Foundation](./docs/research/N_SPACE_THEORY.md)
- [N-Space Architecture Synthesis](./docs/research/N_SPACE_SYNTHESIS.md)

Diagram docs are maintained alongside runtime/architecture changes.
