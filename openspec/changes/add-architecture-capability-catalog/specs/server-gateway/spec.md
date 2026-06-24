## ADDED Requirements

### Requirement: Multi-Transport Server Gateway
The system SHALL provide gateway surfaces for TCP/WebSocket/HTTP request handling over common core semantics.

#### Scenario: Query arrives over different transports
- **WHEN** equivalent query requests arrive over supported transports
- **THEN** gateway layers MUST normalize them to the same execution contracts
