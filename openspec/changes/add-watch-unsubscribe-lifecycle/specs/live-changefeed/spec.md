## ADDED Requirements

### Requirement: Subscription Unsubscribe
The changefeed engine SHALL support explicit subscription removal.

#### Scenario: Active subscription is unsubscribed
- **WHEN** unsubscribe is called with an existing subscription id
- **THEN** the subscription MUST be removed and no longer pollable
