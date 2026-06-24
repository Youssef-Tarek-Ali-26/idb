## ADDED Requirements

### Requirement: Durable Stream Adapter Semantics Across HTTP/WS/TCP
Server gateway adapter APIs SHALL preserve equivalent durable mutation stream poll/commit behavior across HTTP, WebSocket, and TCP transports.

#### Scenario: Durable poll returns equivalent mutation records across adapters
- **WHEN** equivalent durable mutation poll requests are executed through HTTP, WebSocket, and TCP adapters
- **THEN** each adapter MUST return durable mutation records with equivalent commit semantics

#### Scenario: Durable commit advances replay position across adapters
- **WHEN** durable mutation offsets are committed through any adapter
- **THEN** subsequent durable poll requests through that adapter MUST resume after committed offsets
