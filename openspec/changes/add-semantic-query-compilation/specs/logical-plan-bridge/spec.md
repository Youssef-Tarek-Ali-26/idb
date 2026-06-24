## ADDED Requirements

### Requirement: Semantic Predicate Compilation to Executable Requests
The system SHALL compile one unthresholded semantic predicate (`meaning(...)`) from logical plan filters into `QueryRequest.vector_query` for the CPU-supported v0 subset.

#### Scenario: Semantic text query is compiled
- **WHEN** a logical plan contains exactly one `meaning("...")` filter and no semantic threshold
- **THEN** the bridge MUST emit an executable `QueryRequest` with a populated `vector_query`
- **AND** the emitted vector query field and dimensions MUST be derived from bridge configuration

#### Scenario: Unsupported semantic forms are rejected deterministically
- **WHEN** a logical plan contains more than one semantic predicate or a semantic predicate with `threshold`
- **THEN** the bridge MUST fail with a deterministic planner error describing the unsupported semantic form
