## ADDED Requirements

### Requirement: Pluggable Authorization Runtime for Execution
The execution backend SHALL support pluggable authorization decisions through caller context and policy interfaces without coupling core execution semantics to native auth services.

#### Scenario: External policy provider denies query
- **WHEN** a query is executed with caller context and the configured authorization provider returns deny
- **THEN** the backend MUST reject execution with an authorization error

#### Scenario: Tenant scope mismatch is rejected
- **WHEN** caller context tenant scope does not include the request tenant
- **THEN** the backend MUST reject execution before candidate generation

#### Scenario: Auth modules are optional by default
- **WHEN** no external authorization provider is configured
- **THEN** execution and mutation flows MUST remain operational for machine/internal workloads
