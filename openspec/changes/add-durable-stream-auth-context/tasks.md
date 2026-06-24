## 1. Gateway Contract and Transport
- [x] 1.1 Add caller context to durable stream canonical command variants.
- [x] 1.2 Add optional caller field to durable stream transport payloads and normalization.

## 2. Auth-Aware Runtime Dispatch
- [x] 2.1 Add auth-aware durable stream poll/commit APIs in CPU backend.
- [x] 2.2 Route gateway durable stream dispatch through auth-aware CPU APIs.

## 3. Validation
- [x] 3.1 Add tests for durable stream caller normalization and auth/tenant enforcement.
- [x] 3.2 Run `cargo test --workspace` and `openspec validate --changes --strict --no-interactive`.
