## ADDED Requirements

### Requirement: Stable v0 Query Grammar
The system SHALL publish a stable v0 query grammar with deterministic parsing for equivalent query text.

#### Scenario: Query is parsed by different SDK clients
- **WHEN** the same query text is submitted through different SDK implementations
- **THEN** the parser MUST produce equivalent canonical AST output
