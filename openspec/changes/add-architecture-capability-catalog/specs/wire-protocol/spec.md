## ADDED Requirements

### Requirement: Versioned Binary Wire Protocol
The system SHALL expose a versioned wire protocol for client/server query and mutation operations.

#### Scenario: Client and server negotiate protocol version
- **WHEN** a session is established
- **THEN** both sides MUST enforce protocol version compatibility rules before request execution
