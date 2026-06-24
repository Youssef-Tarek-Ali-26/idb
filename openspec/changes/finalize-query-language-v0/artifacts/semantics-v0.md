# Semantics v0

## Precedence (high to low)

1. Entity reference and traversal (`A(...) -> B` / `<-`)
2. `where` predicates within current source/traversal segment
3. Pipe transforms (`| sort(...) | top(...)`)
4. Query mode wrapper (`watch`)

## Hybrid Retrieval Semantics

- Structured predicates constrain candidate set first.
- `meaning(...)` contributes semantic scoring intent for ranking stage.
- If both structured and semantic predicates exist, the query is hybrid and must preserve deterministic tie-break behavior.

## Graph Traversal Semantics

- `A -> B` means outbound edge traversal from A-typed records to B-typed records.
- `A <- B` means inbound edge traversal.
- `where` after traversal applies to the final relation output unless explicit step-local `where` is used in a traversal step.

## watch Semantics

- `watch Q` is semantically equivalent to:
  1. Execute snapshot query `Q`
  2. Register subscription with dependency tracking for `Q`
  3. Stream ordered diffs using commit sequence tokens
