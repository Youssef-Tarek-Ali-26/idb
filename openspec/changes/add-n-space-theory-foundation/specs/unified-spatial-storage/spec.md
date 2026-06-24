## ADDED Requirements

### Requirement: N-Space Decomposition Contract
The system SHALL model logical record coordinates as a decomposition of structured, semantic, and topology signal blocks under a versioned N-space contract.

#### Scenario: N-space decomposition metadata is inspected
- **WHEN** developers inspect active N-space metadata
- **THEN** the system MUST expose block definitions, dimensionality, and fusion weight policy

#### Scenario: N-space contract version changes
- **WHEN** decomposition or fusion policy changes
- **THEN** the system MUST assign a new version identifier and preserve deterministic behavior within each version

### Requirement: Fused Similarity Kernel
The system SHALL provide a fused similarity kernel over structured, semantic, and topology blocks with deterministic, bounded output.

#### Scenario: Similarity is evaluated for two points
- **WHEN** the fused similarity kernel is run for points in the same N-space contract version
- **THEN** the output MUST be deterministic and bounded within a documented numeric range

#### Scenario: Similarity is evaluated on identical points
- **WHEN** all corresponding block vectors are equal
- **THEN** fused similarity MUST produce maximal similarity for that kernel configuration
