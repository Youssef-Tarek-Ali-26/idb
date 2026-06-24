## ADDED Requirements

### Requirement: Durable Mutation Stream API
The CPU backend SHALL expose a durable mutation stream API backed by ordered per-tenant logs.

#### Scenario: Polling returns durable mutation records
- **WHEN** a consumer group polls the durable mutation stream for a tenant
- **THEN** the backend MUST return mutation records with stable partition and sequence metadata suitable for offset commits

#### Scenario: Committed offsets advance consumer replay position
- **WHEN** a consumer commits offsets for partitions in the durable mutation stream
- **THEN** subsequent polling for the same consumer group MUST continue after the committed partition sequence positions

#### Scenario: Durable stream survives backend reopen
- **WHEN** the backend process is restarted on the same data directory
- **THEN** durable mutation records and committed consumer offsets MUST remain available
