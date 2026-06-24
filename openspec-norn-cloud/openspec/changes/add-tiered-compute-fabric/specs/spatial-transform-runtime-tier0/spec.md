## ADDED Requirements

### Requirement: Tier 0 Spatial Transform Runtime
The platform SHALL support a Tier 0 runtime where eligible transforms execute directly inside iDB without invoking a WASM runtime boundary.

#### Scenario: Pure transform executes in Tier 0
- **WHEN** a deployed transform is declared pure and passes Tier 0 validation
- **THEN** execution MUST run inside iDB transform runtime and return results without Tier 1 invocation

### Requirement: Tier 0 Purity and Determinism Contract
Tier 0 transforms MUST be side-effect free and deterministic with respect to declared inputs, snapshot boundaries, and transform version.

#### Scenario: Transform attempts side effect
- **WHEN** a transform declares or attempts network, filesystem, external capability, or mutation side effects
- **THEN** admission MUST reject Tier 0 execution for that transform

#### Scenario: Deterministic replay is requested
- **WHEN** an operator replays a Tier 0 transform with the same inputs and snapshot/version
- **THEN** output MUST be reproducible within declared numerical determinism bounds

### Requirement: Tier 0 Artifact Versioning
Tier 0 transform artifacts SHALL be versioned and routable so requests can target specific transform versions with rollback support.

#### Scenario: Transform version rollback
- **WHEN** a newer transform version is rolled back
- **THEN** routing MUST restore prior version behavior without requiring application-level endpoint rewrites
