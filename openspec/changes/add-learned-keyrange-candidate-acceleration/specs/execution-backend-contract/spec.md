## ADDED Requirements

### Requirement: Learned Key-Range Candidate Acceleration
The CPU execution backend SHALL support key-range candidate generation using learned position prediction with deterministic correctness-preserving filtering.

#### Scenario: Learned model seeds range candidate positions
- **WHEN** a query request includes non-empty key ranges in candidate generation hints
- **THEN** the backend MUST be able to seed candidate lookup using learned index position predictions
- **AND** final candidates MUST still be validated against exact key-range bounds

#### Scenario: Mutation consistency is preserved
- **WHEN** records are replayed, upserted, updated, or deleted
- **THEN** learned key-range candidate lookup MUST reflect the same committed record set as full scan semantics

#### Scenario: Records without spatial key remain eligible
- **WHEN** a query uses key-range hints and a tenant record has no spatial key
- **THEN** that record MUST remain eligible for predicate-based filtering under existing hint semantics
