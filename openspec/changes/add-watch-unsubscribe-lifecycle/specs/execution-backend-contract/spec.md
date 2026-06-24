## ADDED Requirements

### Requirement: CPU Watch Stop API
The CPU backend SHALL expose a stop-watch API for active watch sessions.

#### Scenario: Stopped watch can no longer be polled
- **WHEN** a watch session is stopped
- **THEN** subsequent watch polling for that subscription MUST fail deterministically
