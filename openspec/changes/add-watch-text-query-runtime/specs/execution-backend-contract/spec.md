## ADDED Requirements

### Requirement: CPU Watch Text Query Session API
The CPU backend SHALL expose watch text-query session start and poll APIs.

#### Scenario: Watch session start returns snapshot and subscription metadata
- **WHEN** a valid `watch` query is started
- **THEN** the backend MUST return snapshot results and subscription identifiers/resume token metadata

#### Scenario: Polling returns ordered dependency-filtered events
- **WHEN** events are polled for a watch session
- **THEN** the backend MUST return mutation events in commit-sequence order filtered by tracked dependencies

#### Scenario: Non-watch query text is rejected by watch start API
- **WHEN** watch start is invoked with non-watch query text
- **THEN** the backend MUST return a deterministic planning error
