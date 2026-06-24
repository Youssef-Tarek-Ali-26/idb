## ADDED Requirements

### Requirement: Ordered Subscription Event Delivery
The system SHALL deliver mutation events to active subscriptions in commit-sequence order per tenant and query subscription.

#### Scenario: Subscriber reconnects after disconnect
- **WHEN** a subscriber reconnects with a valid resume token
- **THEN** the system MUST resume event delivery from the next commit sequence without reordering events
