## ADDED Requirements

### Requirement: CPU Text Query Explain API
The CPU backend SHALL expose explain output for text queries.

#### Scenario: Explain is requested via CPU backend
- **WHEN** caller requests explain for a text query
- **THEN** the backend MUST return deterministic explain output or a deterministic planning error
