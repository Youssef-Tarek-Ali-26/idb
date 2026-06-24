## ADDED Requirements

### Requirement: Sort Transform Request Projection
The bridge SHALL compile a `sort(field asc|desc)` transform into executable `QueryRequest.order_by` for the CPU-supported v0 subset.

#### Scenario: Sort + take query is projected
- **WHEN** a logical plan contains `sort(field dir)` and optional `take/top` transforms
- **THEN** the bridge MUST emit `QueryRequest.order_by` matching the sort transform
- **AND** the bridge MUST preserve deterministic `top_k` selection behavior

#### Scenario: Conflicting ordering transforms are rejected
- **WHEN** a logical plan contains multiple ordering transforms that disagree on field or direction
- **THEN** the bridge MUST fail with a deterministic planner error describing the ordering conflict
