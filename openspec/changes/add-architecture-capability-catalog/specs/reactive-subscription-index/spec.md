## ADDED Requirements

### Requirement: Reactive Subscription Dependency Index
The system SHALL maintain subscription dependency indexing for efficient change-to-subscription fanout.

#### Scenario: Mutation event maps to affected subscriptions
- **WHEN** a record mutation is committed
- **THEN** reactive runtime MUST resolve affected subscriptions using dependency/index structures instead of scanning all subscriptions
