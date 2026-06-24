## ADDED Requirements

### Requirement: Stateful TCP Session Handling in Server Gateway
Server gateway adapters SHALL maintain TCP session state for negotiated protocol sessions before frame dispatch.

#### Scenario: TCP frame arrives for unknown session
- **WHEN** a TCP frame is received for a missing session id
- **THEN** the server adapter MUST reject the frame with an unknown-session error

#### Scenario: TCP session is closed
- **WHEN** a session is closed and subsequent frames arrive for that session
- **THEN** the server adapter MUST reject those frames as unknown-session requests

### Requirement: Adapter Methods Route Through Canonical Gateway Runtime
Server gateway adapter APIs SHALL route HTTP/WebSocket/TCP transport inputs through canonical gateway normalization and runtime dispatch.

#### Scenario: Equivalent query via HTTP/WS/TCP adapters
- **WHEN** equivalent query payloads are submitted through server adapter APIs
- **THEN** responses MUST preserve equivalent result semantics across transports
