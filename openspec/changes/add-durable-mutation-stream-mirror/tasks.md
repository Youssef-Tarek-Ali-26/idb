## 1. Runtime Wiring
- [x] 1.1 Add `idb-ordered-log` to the workspace/runtime dependency graph.
- [x] 1.2 Initialize durable ordered log state in CPU backend data directory.

## 2. Mutation Mirroring
- [x] 2.1 Mirror ingest/update/delete mutation events into per-tenant durable mutation topics.
- [x] 2.2 Add CPU APIs to poll durable mutation records and commit consumer-group offsets.

## 3. Validation
- [x] 3.1 Add CPU tests for durable stream replay and committed offset progression.
- [x] 3.2 Add CPU test for durable stream persistence across backend reopen.
- [x] 3.3 Run `cargo check --workspace` and `openspec validate --changes --strict --no-interactive`.
