## ADDED Requirements

### Requirement: Watch Query Update Batch API
The CPU backend SHALL expose a watch polling API that includes current query row state for each returned mutation event.

#### Scenario: Updated record no longer matches query
- **WHEN** a tracked record mutation is polled and the current record does not satisfy query constraints
- **THEN** the update entry MUST include `current = null`

#### Scenario: Updated record still matches query
- **WHEN** a tracked record mutation is polled and the current record satisfies query constraints
- **THEN** the update entry MUST include hydrated current row data
