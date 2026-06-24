## ADDED Requirements

### Requirement: Durable Consumer Group Must Be Non-Empty
Gateway normalization SHALL reject durable stream commands with empty consumer group identifiers.

#### Scenario: Durable poll rejects empty consumer group
- **WHEN** a durable mutation poll payload provides an empty consumer group
- **THEN** normalization MUST reject the payload as invalid

#### Scenario: Durable commit rejects empty consumer group
- **WHEN** a durable mutation commit payload provides an empty consumer group
- **THEN** normalization MUST reject the payload as invalid
