## ADDED Requirements

### Requirement: Vector Score Threshold Enforcement
Execution backends SHALL enforce the request-level minimum semantic score constraint when vector scoring is active.

#### Scenario: Candidate falls below minimum semantic score
- **WHEN** a request provides a minimum semantic score and a candidate's vector score is lower
- **THEN** the candidate MUST be excluded from ranked results
