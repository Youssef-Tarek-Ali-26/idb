## 1. Gateway Validation
- [x] 1.1 Enforce non-empty durable stream consumer group during transport normalization.
- [x] 1.2 Add normalization tests that reject empty consumer group payloads.

## 2. Backend Guardrails
- [x] 2.1 Reject empty consumer group names in CPU durable stream poll API.
- [x] 2.2 Reject empty consumer group names in CPU durable stream commit API.
- [x] 2.3 Add backend tests for empty consumer group rejection.

## 3. Validation
- [x] 3.1 Run `cargo test --workspace` and `openspec validate --changes --strict --no-interactive`.
