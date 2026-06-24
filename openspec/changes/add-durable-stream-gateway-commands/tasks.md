## 1. Gateway Contract
- [x] 1.1 Add durable stream poll/commit command and response types to canonical gateway model.
- [x] 1.2 Add transport payload structs, routes/events/opcodes, and normalization for durable stream commands.

## 2. Runtime Dispatch
- [x] 2.1 Dispatch durable stream poll command to CPU backend durable stream poll API.
- [x] 2.2 Dispatch durable stream commit command to CPU backend durable stream commit API.

## 3. Validation
- [x] 3.1 Add gateway transport and runtime tests for durable stream commands.
- [x] 3.2 Add server adapter test coverage for durable stream command path.
- [x] 3.3 Run `cargo check --workspace` and `openspec validate --changes --strict --no-interactive`.
