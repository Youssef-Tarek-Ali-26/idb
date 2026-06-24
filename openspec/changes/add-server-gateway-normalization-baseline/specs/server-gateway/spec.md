## ADDED Requirements

### Requirement: Transport-Normalized Gateway Commands
Gateway layers SHALL normalize HTTP, WebSocket, and TCP transport envelopes into a shared command contract before execution.

#### Scenario: Equivalent query requests across transports
- **WHEN** semantically equivalent query requests arrive via HTTP/WebSocket/TCP
- **THEN** normalization MUST produce equivalent canonical command payloads

#### Scenario: Unsupported transport routes or opcodes
- **WHEN** a transport envelope contains an unknown route/event/opcode
- **THEN** normalization MUST reject the request with a transport-specific unsupported error

### Requirement: Gateway Runtime Dispatch to Execution Backend
The gateway runtime SHALL dispatch canonical commands into backend execution APIs while preserving caller context.

#### Scenario: Query command is dispatched with caller context
- **WHEN** a canonical query command is executed
- **THEN** backend dispatch MUST invoke auth-aware query APIs with the provided caller context

#### Scenario: Watch lifecycle commands are dispatched
- **WHEN** canonical watch start/poll/stop commands are executed
- **THEN** runtime MUST return watch session/update/stop responses consistent with backend semantics
