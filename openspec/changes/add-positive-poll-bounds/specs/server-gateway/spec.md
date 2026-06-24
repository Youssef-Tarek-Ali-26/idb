## ADDED Requirements

### Requirement: Poll Bounds Must Be Positive
Gateway normalization SHALL reject non-positive event bounds for poll-style commands.

#### Scenario: Watch poll rejects zero bound
- **WHEN** a watch poll payload specifies `max_events = 0`
- **THEN** normalization MUST reject the payload as invalid

#### Scenario: Durable poll rejects zero bound
- **WHEN** a durable mutation poll payload specifies `max_events_per_partition = 0`
- **THEN** normalization MUST reject the payload as invalid
