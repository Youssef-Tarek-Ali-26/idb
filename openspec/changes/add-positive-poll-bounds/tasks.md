## 1. Gateway Validation
- [x] 1.1 Enforce positive non-zero bounds for watch poll and durable poll payloads.
- [x] 1.2 Add normalization tests that reject zero poll bounds.

## 2. Backend Guardrails
- [x] 2.1 Reject zero watch poll bounds in CPU backend API.
- [x] 2.2 Reject zero durable poll bounds in CPU backend API.
- [x] 2.3 Add backend tests for zero-bound rejection.

## 3. Validation
- [x] 3.1 Run `cargo test --workspace` and `openspec validate --changes --strict --no-interactive`.
