## ADDED Requirements

### Requirement: Unified Edge Ingress
The platform SHALL provide a unified edge gateway that terminates TLS, accepts HTTP/2 and HTTP/3 traffic, and dispatches requests to function routes.

#### Scenario: Request is routed from edge to function
- **WHEN** an external request matches a function-declared route
- **THEN** edge MUST resolve tenant context and dispatch to the corresponding function execution path

### Requirement: Route Model Is Function-Declared
Routes MUST be derived from function metadata stored in platform state instead of separate ingress-only configuration artifacts.

#### Scenario: Function deployment updates route table
- **WHEN** a function with route declarations is deployed
- **THEN** edge routing state MUST update from platform state without manual ingress yaml edits

### Requirement: Transport-Normalized Dispatch Boundary
Edge transport adapters MUST normalize HTTP and WebSocket request envelopes into a canonical command model before runtime dispatch.

#### Scenario: Equivalent requests across transports
- **WHEN** equivalent logical operations are submitted over HTTP and WebSocket
- **THEN** normalized commands MUST preserve equivalent execution semantics
