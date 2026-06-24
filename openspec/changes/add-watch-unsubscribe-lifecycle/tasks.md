## 1. Changefeed Lifecycle
- [x] 1.1 Add unsubscribe operation to changefeed engine.

## 2. CPU Watch Lifecycle
- [x] 2.1 Add CPU stop-watch API that clears active watch metadata and underlying subscription.
- [x] 2.2 Ensure post-stop polling behaves deterministically.

## 3. Validation
- [x] 3.1 Add tests for unsubscribe/stop behavior.
- [x] 3.2 Run `cargo test --workspace` and `openspec validate --changes --strict --no-interactive`.
