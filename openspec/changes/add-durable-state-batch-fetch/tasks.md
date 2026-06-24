## 1. Storage Batch Fetch
- [x] 1.1 Add `DurableState::get_many(tenant_id, ids)` returning positional optional records.
- [x] 1.2 Add storage tests for ordering, missing IDs, and tenant isolation.

## 2. CPU Backend Adoption
- [x] 2.1 Update hydration to use storage batch fetch while preserving score ordering.
- [x] 2.2 Update outbound traversal frontier expansion to batch fetch target vertices.
- [x] 2.3 Add backend regression test that hydrate output order remains deterministic.

## 3. Documentation + Validation
- [x] 3.1 Sync `docs/book/DB_DIAGRAMS.md` and `docs/book/DB_DIAGRAMS_ASCII.md` with batch hydration semantics.
- [x] 3.2 Run `cargo test -p idb-storage --lib`, `cargo test -p idb-executor-cpu --lib`, and `openspec validate add-durable-state-batch-fetch --type change --strict --no-interactive`.
