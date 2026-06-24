## 1. Core Contracts
- [x] 1.1 Add caller-context, action, request, decision, and authorizer contracts in `idb-core`.
- [x] 1.2 Add an auth runtime wrapper with default allow-all behavior.

## 2. CPU Integration
- [x] 2.1 Add context-aware query/explain/watch APIs that enforce authorization decisions.
- [x] 2.2 Add context-aware mutation APIs and keep trait-based paths functional via internal caller context.

## 3. Validation
- [x] 3.1 Add tests for policy denial and tenant-scope denial behavior.
- [x] 3.2 Run `cargo test -p idb-core -p idb-executor-cpu` and `openspec validate --changes --strict --no-interactive`.
