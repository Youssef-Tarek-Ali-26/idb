## ADDED Requirements

### Requirement: Traversal Execution Projection
The planner SHALL provide an execution projection path that compiles traversal-source logical plans into executable request semantics.

#### Scenario: Traversal query is projected for execution
- **WHEN** a logical plan source is traversal and transforms are within the supported subset
- **THEN** the execution projection API MUST return executable request semantics for filters/semantic scoring/top-k/ordering
- **AND** the source traversal description MUST remain available for backend traversal execution
