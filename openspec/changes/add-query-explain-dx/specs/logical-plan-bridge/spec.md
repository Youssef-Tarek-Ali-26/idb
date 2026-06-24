## ADDED Requirements

### Requirement: Query Explain Output
The planner bridge SHALL provide deterministic explain output for query text.

#### Scenario: Supported query explain includes request projection
- **WHEN** explain is requested for a query that can be projected to executable request
- **THEN** the explain output MUST include logical plan summary and an executable request projection section

#### Scenario: Unsupported query explain includes reason
- **WHEN** explain is requested for a query that cannot be projected to executable request
- **THEN** the explain output MUST include logical plan summary and a deterministic unsupported reason
