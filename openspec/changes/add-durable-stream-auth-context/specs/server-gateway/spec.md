## ADDED Requirements

### Requirement: Durable Stream Commands Preserve Caller Context
Gateway normalization SHALL parse optional caller context for durable mutation stream poll/commit commands across HTTP, WebSocket, and TCP transports.

#### Scenario: Durable stream payload omits caller
- **WHEN** a durable stream poll or commit payload is normalized without a caller field
- **THEN** the canonical command MUST use an anonymous caller context

#### Scenario: Durable stream payload includes caller
- **WHEN** a durable stream poll or commit payload is normalized with caller context
- **THEN** the canonical command MUST preserve that caller context for runtime dispatch

### Requirement: Durable Stream Dispatch Enforces Auth and Tenant Scope
Gateway runtime SHALL dispatch durable mutation stream commands through auth-aware backend APIs.

#### Scenario: Durable stream command denied by auth policy
- **WHEN** a durable stream poll or commit command is executed with caller context and auth denies watch access
- **THEN** runtime MUST return authorization error and MUST NOT invoke mutation stream poll/commit state changes

#### Scenario: Durable stream tenant scope mismatch
- **WHEN** caller tenant scope does not include the durable stream command tenant
- **THEN** runtime MUST reject durable stream poll/commit dispatch before durable stream data access
