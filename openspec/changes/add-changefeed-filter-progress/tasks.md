## 1. Changefeed Poll Progress
- [x] 1.1 Advance filtered subscription cursor on empty-match scans.
- [x] 1.2 Reject zero `max_events` poll requests.

## 2. Regression Coverage
- [x] 2.1 Add test for sparse dependency filter progression across multiple polls.
- [x] 2.2 Add test for zero `max_events` rejection.

## 3. Validation
- [x] 3.1 Run `cargo test -p idb-storage --lib` and `openspec validate add-changefeed-filter-progress --type change --strict --no-interactive`.
