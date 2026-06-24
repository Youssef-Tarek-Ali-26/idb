## ADDED Requirements

### Requirement: Positional Batch State Fetch
The system SHALL provide a storage batch fetch API that returns records in positional correspondence with requested IDs.

#### Scenario: Batch fetch preserves input order and missing IDs
- **WHEN** a caller requests multiple record IDs with a mix of existing and non-existing records
- **THEN** the storage API MUST return a vector with the same length and ordering as the request
- **AND** missing records MUST be represented as `None` at their input positions

#### Scenario: Batch fetch enforces tenant isolation
- **WHEN** a caller fetches record IDs for a tenant
- **THEN** records from other tenants MUST NOT be returned for those IDs

### Requirement: CPU Hydration Uses Batched Fetch
The CPU execution backend SHALL use storage batch fetch for hydration and preserve deterministic output ordering.

#### Scenario: Hydration returns rows in scored order
- **WHEN** scored records are hydrated in CPU execution
- **THEN** the hydrated results MUST remain in the same order as scored inputs
- **AND** missing records MUST still raise a storage error
