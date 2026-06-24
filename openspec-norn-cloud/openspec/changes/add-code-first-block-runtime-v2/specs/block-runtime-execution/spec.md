## ADDED Requirements

### Requirement: Hybrid Runtime Responsibilities
The platform SHALL separate orchestration responsibilities from execution-kernel responsibilities.

#### Scenario: Run a long-lived workflow
- **WHEN** a workflow requires timers, retries, supervision, or watch-driven progression
- **THEN** the orchestration plane SHALL own that lifecycle independently from the language runtime executing the block body

#### Scenario: Execute a performance-sensitive block
- **WHEN** a block requires transport normalization, state adapter access, or performance-sensitive runtime handling
- **THEN** the execution kernel SHALL provide those services without requiring the orchestration plane to implement them directly

### Requirement: Multiple Runtime Targets Share One Block Contract
The platform SHALL support WASM, native-process, and container execution targets under one block contract.

#### Scenario: Run a portable default workload
- **WHEN** a block does not require privileged native access or heavyweight compatibility packaging
- **THEN** the scheduler SHALL be able to place it on the default WASM target

#### Scenario: Run a compatibility workload
- **WHEN** a block requires native libraries, language-specific worker processes, or compatibility packaging
- **THEN** the scheduler SHALL be able to place it on a native-process or container target without changing the block contract shape

### Requirement: Same-Language Fast Path Must Preserve Semantics
The platform SHALL allow same-language execution optimizations only when policy, validation, and observability semantics remain equivalent to the normal boundary model.

#### Scenario: Optimize adjacent TypeScript blocks
- **WHEN** two connected blocks execute in the same language runtime and qualify for a fast path
- **THEN** the runtime MAY bypass expensive cross-runtime serialization but MUST preserve tracing, policy enforcement, and schema checkpoint semantics

### Requirement: State Binding Remains Portable
The platform SHALL bind blocks to a portable state contract with iDB as the preferred backend and fallback adapters permitted.

#### Scenario: Prefer iDB but run with a fallback backend
- **WHEN** a deployment selects a fallback backend because iDB is unavailable or not yet suitable
- **THEN** the block contract SHALL remain valid without requiring application-level flow rewrites
