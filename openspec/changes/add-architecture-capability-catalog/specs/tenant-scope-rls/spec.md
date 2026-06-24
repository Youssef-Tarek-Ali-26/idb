## ADDED Requirements

### Requirement: Planner-Level Tenant Scope and RLS Injection
The system SHALL apply tenant scoping and row-level security at planner/runtime boundaries rather than ad-hoc application checks.

#### Scenario: Query executes under tenant-scoped caller context
- **WHEN** a query is compiled/executed with tenant context
- **THEN** plan and result set MUST enforce tenant boundary semantics deterministically
