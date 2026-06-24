## ADDED Requirements

### Requirement: Unified Platform State Substrate
The platform SHALL use a unified state substrate (iDB) for application data and platform metadata including routes, policies, certificates, events, logs, and metrics references.

#### Scenario: Platform component reads configuration
- **WHEN** edge/compute/identity components need configuration or metadata
- **THEN** they MUST resolve canonical values from unified state entities rather than ad-hoc local config stores

### Requirement: Tiered Storage Semantics
The state layer MUST support tier-aware behavior so workloads can distinguish durability and access patterns across durable, ordered, and ephemeral-like needs.

#### Scenario: Event workload chooses ordered tier
- **WHEN** a workload requires append + replay semantics
- **THEN** writes MUST target ordered tier entities with partitioned sequence behavior

### Requirement: Tenant-Scoped State Isolation
State access MUST be tenant-scoped by default across platform and user workloads.

#### Scenario: Request resolves tenant context
- **WHEN** a tenant-scoped operation executes
- **THEN** reads and writes MUST be constrained to that tenant scope unless privileged system policy explicitly expands scope
