## ADDED Requirements

### Requirement: Scripted Multi-Transport Flow Harness
Server gateway SHALL provide a scripted flow harness that can execute ordered HTTP/WebSocket/TCP steps against canonical adapter behavior.

#### Scenario: Script mixes HTTP, WebSocket, and TCP steps
- **WHEN** a script includes mixed transport operations in sequence
- **THEN** execution MUST preserve step order and emit deterministic events for responses and TCP session lifecycle

#### Scenario: Script references unknown TCP session key
- **WHEN** a TCP frame/close step references a missing script session key
- **THEN** script execution MUST fail with an explicit unknown-session-key error
