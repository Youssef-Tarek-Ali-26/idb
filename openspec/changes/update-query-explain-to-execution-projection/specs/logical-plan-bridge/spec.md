## ADDED Requirements

### Requirement: Explain Mirrors Execution Projection
Explain output SHALL reflect execution-oriented request projection status.

#### Scenario: Traversal explain reflects executable projection
- **WHEN** explain is requested for a traversal query in the CPU-supported subset
- **THEN** explain MUST include a supported request projection

#### Scenario: Non-executable mode remains unsupported in explain
- **WHEN** explain is requested for a watch query
- **THEN** explain MUST include a deterministic unsupported reason
