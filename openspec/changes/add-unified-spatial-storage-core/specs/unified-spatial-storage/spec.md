## ADDED Requirements

### Requirement: Canonical Unified Record Envelope
The system SHALL store each logical record using a canonical envelope that includes stable logical identity, tenant scope, schema version metadata, and references to structured/vector/blob/edge payload segments.

#### Scenario: Record is ingested
- **WHEN** a client submits a valid entity record for ingestion
- **THEN** the system MUST persist a canonical envelope with `record_id`, `tenant_id`, `entity_type`, `schema_version`, and payload references

#### Scenario: Record is re-hydrated after query
- **WHEN** a query returns candidate record ids
- **THEN** the system MUST resolve those ids through the canonical envelope before returning hydrated entities

### Requirement: Dimension Registry for Arbitrary Vector Space
The system SHALL define a versioned dimension registry that declares all indexed dimensions, including source field mapping, normalization policy, and missing-value handling.

#### Scenario: New indexed dimension is introduced
- **WHEN** a schema introduces a new structured or projected embedding dimension
- **THEN** the dimension registry MUST add a new definition and assign it to a new `dimension_version`

#### Scenario: Mapping metadata is requested for debugging
- **WHEN** a developer requests mapping diagnostics
- **THEN** the system MUST expose dimension definitions and normalization policies for the active `dimension_version`

### Requirement: Deterministic Coordinate Mapping Pipeline
The system SHALL map records into coordinate vectors using a deterministic pipeline for the same input payload, schema version, and dimension version.

#### Scenario: Same record is mapped twice without version change
- **WHEN** the same canonical envelope is remapped under unchanged mapping configuration
- **THEN** the resulting coordinate vector MUST be byte-equivalent

#### Scenario: Mapping logic evolves
- **WHEN** mapping parameters are changed intentionally
- **THEN** the system MUST produce a new `dimension_version` and preserve ability to identify records produced by prior versions

### Requirement: Bounded Space Partitioning
The system SHALL partition the key/coordinate space into bounded logical pages (or tiles) with explicit metadata for range bounds, record count, and version markers.

#### Scenario: Page exceeds configured capacity
- **WHEN** inserts cause a page to exceed configured bounds
- **THEN** the system MUST split the page and update routing metadata atomically with visibility guarantees

#### Scenario: Sparse neighboring pages are compacted
- **WHEN** maintenance identifies merge-eligible pages
- **THEN** the system MUST merge pages without losing logical record identity or tenant isolation

### Requirement: Durable Mutation Protocol
The system SHALL apply writes using a durability-first protocol that records mutation intent before making changes visible in queryable state.

#### Scenario: Insert mutation is committed
- **WHEN** a record insert is acknowledged as committed
- **THEN** the system MUST have persisted mutation intent in WAL-equivalent durable storage before visibility marker publication

#### Scenario: Crash occurs during mutation
- **WHEN** the engine restarts after a crash
- **THEN** the system MUST replay durable mutation intents idempotently and restore last committed visibility state

### Requirement: Hybrid Retrieval Stages
The system SHALL execute retrieval in explicit stages: candidate generation, structured filtering, vector/score evaluation, deterministic ranking, and hydration.

#### Scenario: Hybrid query is executed
- **WHEN** a query contains both structured predicates and vector similarity terms
- **THEN** the engine MUST execute stage-ordered retrieval and return a deterministic top-k set for identical inputs

#### Scenario: Query includes only structured predicates
- **WHEN** no vector term is provided
- **THEN** the engine MUST bypass vector scoring stage and still preserve deterministic ranking semantics

### Requirement: Full-Fidelity Hydration Layer
The system SHALL preserve full-fidelity payloads (including variable-size fields) in a linked storage layer and use it for result hydration.

#### Scenario: Candidate result requires long text/blob references
- **WHEN** result projection includes fields not present in compact hot records
- **THEN** the engine MUST hydrate those fields from the full-fidelity layer before response serialization

#### Scenario: Historical replay is requested
- **WHEN** an audit or replay process requests historical payload snapshots
- **THEN** the system MUST resolve data through stored full-fidelity records for the requested version/time window

### Requirement: Tenant Isolation in Storage and Retrieval
The system SHALL enforce tenant boundaries during write routing, page/index operations, and query execution.

#### Scenario: Query omits tenant predicate explicitly
- **WHEN** a query is executed under authenticated tenant context
- **THEN** tenant scoping MUST still be applied before candidate generation and filtering

#### Scenario: Compaction touches mixed records
- **WHEN** maintenance jobs process adjacent storage pages
- **THEN** tenant isolation MUST be preserved and cross-tenant co-mingling MUST NOT occur in logical visibility

### Requirement: Deterministic Tie-Break and Score Policy
The system SHALL define deterministic tie-break rules and score policy metadata so repeated queries under same state produce stable ordering.

#### Scenario: Two records receive identical hybrid score
- **WHEN** ranking scores are equal within configured precision
- **THEN** ordering MUST follow documented tie-break keys (for example stable id order)

#### Scenario: Score policy changes
- **WHEN** ranking weights or normalization behavior change
- **THEN** the system MUST record policy version metadata for explain/debug output

### Requirement: Mutation Event Hooks for Future Live Updates
The system SHALL emit canonical mutation events for inserts, updates, and deletes to an internal event interface, even when full changefeeds are not yet implemented.

#### Scenario: Record update is committed
- **WHEN** a committed mutation modifies a logical record
- **THEN** the system MUST emit a mutation event containing record identity, tenant scope, mutation type, and commit marker

#### Scenario: v0 without subscriber engine
- **WHEN** no live-query subscription engine is configured
- **THEN** mutation event emission MUST remain enabled for downstream replay/integration testing

### Requirement: v0 Scope Excludes Full Live Query Reconciliation
The v0 unified storage core SHALL NOT require full subscriber-state reconciliation or diff-push semantics as part of acceptance for this change.

#### Scenario: Storage core milestone review
- **WHEN** v0 acceptance criteria are evaluated
- **THEN** absence of full live-query reconciliation MUST NOT block acceptance if mutation event hooks and core storage guarantees are satisfied
