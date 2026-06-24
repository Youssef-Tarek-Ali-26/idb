## ADDED Requirements

### Requirement: Versioned Kernel I/O Contract
The system SHALL define versioned kernel input/output envelopes for each supported accelerated query operation.

#### Scenario: Host dispatches accelerated query
- **WHEN** a query stage is routed to the Cerebras backend
- **THEN** the host runtime MUST validate the envelope version and reject unsupported versions before dispatch
