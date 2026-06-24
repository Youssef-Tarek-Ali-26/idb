## ADDED Requirements

### Requirement: Spatial Mapper Abstraction
The system SHALL model physical placement and candidate-seeding behavior through a first-class spatial mapper abstraction that is separate from the fused N-space similarity contract.

#### Scenario: Logical similarity contract is inspected
- **WHEN** developers inspect active unified storage metadata
- **THEN** the system MUST be able to distinguish the fused N-space contract from the spatial mapper configuration used for placement

#### Scenario: Placement strategy is changed without changing similarity semantics
- **WHEN** the physical mapper configuration changes while the fused similarity contract remains stable
- **THEN** the system MUST treat the mapper change as a placement/versioning event rather than a semantic similarity change

### Requirement: Deterministic Mapper Baseline
The system SHALL support at least one deterministic spatial mapper suitable for CPU-first correctness validation and rebuildable placement behavior.

#### Scenario: Deterministic mapper is used for baseline validation
- **WHEN** a fused N-space point is mapped under the same mapper version and configuration
- **THEN** the placement output and candidate-seeding metadata MUST be deterministic

#### Scenario: Deterministic mapper configuration changes
- **WHEN** a mapper configuration change alters placement behavior
- **THEN** the system MUST identify the new mapper version and preserve the ability to explain which mapper version produced stored placement metadata

### Requirement: Learned Mapper Promotion Is Benchmark-Gated
The system SHALL treat learned spatial mappers as experimental until they satisfy explicit benchmark and operational acceptance criteria relative to the deterministic baseline.

#### Scenario: Learned mapper is proposed for broader use
- **WHEN** a learned mapper is evaluated for promotion beyond experimental status
- **THEN** the project MUST compare it against the deterministic baseline using documented workload, update, and rebuild metrics

#### Scenario: Learned mapper experiences data drift
- **WHEN** data distribution changes invalidate learned placement quality
- **THEN** the system MUST provide an explicit retraining or rebuild path rather than silently preserving stale placement

### Requirement: Mapper Metadata for Explain and Debug
The system SHALL expose mapper identity, version, and placement metadata for explain/debug workflows.

#### Scenario: Query explain output references placement
- **WHEN** explain or debug metadata is requested for a query or record
- **THEN** the system MUST be able to report which mapper produced the relevant placement and candidate-seeding behavior

#### Scenario: Operator investigates placement behavior
- **WHEN** a developer inspects a mapped record
- **THEN** the system MUST expose enough metadata to distinguish logical coordinates from physical placement output
