## ADDED Requirements

### Requirement: Learned Indexing Layer
The system SHALL support a learned indexing layer for key-position prediction with deterministic bounded fallback search.

#### Scenario: Learned index prediction is used for lookup
- **WHEN** a lookup/range seed uses learned index prediction
- **THEN** execution MUST apply bounded fallback search to preserve correctness under model error
