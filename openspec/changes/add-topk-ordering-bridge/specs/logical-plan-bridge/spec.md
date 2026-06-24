## ADDED Requirements

### Requirement: Ordered TopK Compilation
The bridge SHALL compile `top(k, field dir)` clauses into executable request ordering metadata for the CPU-supported v0 subset.

#### Scenario: Ordered top-k query is compiled
- **WHEN** a logical plan contains `TopK` with `order_by`
- **THEN** the bridge MUST emit a `QueryRequest` with an order-by field and direction matching the logical plan
