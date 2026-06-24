## ADDED Requirements

### Requirement: Filtered Poll Cursor Progress
The system SHALL advance subscription cursor progress even when a poll scan yields zero matching events after dependency filtering.

#### Scenario: Sparse dependency filter eventually reaches matching event
- **WHEN** non-matching mutation events precede matching events for a filtered subscription
- **THEN** repeated polls MUST eventually deliver the matching event without requiring subscription reset

### Requirement: Positive Poll Bounds
The system SHALL reject non-positive changefeed poll bounds.

#### Scenario: Poll rejects zero max events
- **WHEN** a subscription poll is requested with `max_events = 0`
- **THEN** the poll MUST fail with a validation error
