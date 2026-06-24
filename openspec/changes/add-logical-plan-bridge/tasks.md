## 1. Planning Model
- [x] 1.1 Define logical plan structs for source/filter/traversal/transform stages.
- [x] 1.2 Define translation rules from parser AST to logical plan.

## 2. Execution Bridge
- [x] 2.1 Translate logical plan subset to executable `QueryRequest`.
- [x] 2.2 Add CPU backend helper to execute query text via parser/planner bridge.

## 3. Validation
- [x] 3.1 Add planner conformance tests against OpenSpec cases.
- [x] 3.2 Add integration tests for text-query execution path.
