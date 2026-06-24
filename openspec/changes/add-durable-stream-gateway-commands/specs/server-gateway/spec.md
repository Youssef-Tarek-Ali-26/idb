## ADDED Requirements

### Requirement: Durable Stream Transport Normalization
Gateway normalization SHALL support durable mutation stream poll and commit commands across HTTP, WebSocket, and TCP transports.

#### Scenario: Equivalent durable stream poll payloads across transports
- **WHEN** semantically equivalent durable stream poll payloads arrive via HTTP/WebSocket/TCP
- **THEN** normalization MUST produce equivalent canonical durable stream poll commands

#### Scenario: Equivalent durable stream commit payloads across transports
- **WHEN** semantically equivalent durable stream commit payloads arrive via HTTP/WebSocket/TCP
- **THEN** normalization MUST produce equivalent canonical durable stream commit commands

### Requirement: Durable Stream Runtime Dispatch
Gateway runtime SHALL dispatch canonical durable stream commands into CPU backend durable stream APIs.

#### Scenario: Durable stream poll command is dispatched
- **WHEN** a canonical durable stream poll command is executed
- **THEN** runtime MUST return durable mutation records from backend poll APIs

#### Scenario: Durable stream commit command is dispatched
- **WHEN** a canonical durable stream commit command is executed
- **THEN** runtime MUST invoke backend offset commit APIs and return commit acknowledgment metadata
