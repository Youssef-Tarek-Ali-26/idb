## ADDED Requirements

### Requirement: Block Is the Canonical Application Boundary
The platform SHALL define a `Block` as the canonical application boundary for execution, policy, and observability.

#### Scenario: Define a block in the repo
- **WHEN** a developer defines a block in the repo
- **THEN** the block contract records its input schema, output schema, runtime kind, capability requirements, secret references, state binding intent, and execution policy

#### Scenario: Choose block granularity
- **WHEN** a developer models a function, worker, service, or opaque external adapter as a block
- **THEN** the platform SHALL preserve the same contract shape regardless of the chosen granularity

### Requirement: Flow Composition Uses Blocks Instead of Manually Managed Service APIs
The platform SHALL allow developers to compose blocks into `Flow` definitions without requiring direct service-to-service API wiring as the primary application model.

#### Scenario: Compose a flow from multiple blocks
- **WHEN** a developer wires blocks into a flow in code
- **THEN** the flow definition SHALL become the authoritative composition artifact for execution, routing metadata, and generated graph structure

### Requirement: Block Boundaries Carry the Validation Cost
The platform SHALL apply validation, capability enforcement, and observability at declared block boundaries rather than every internal function call.

#### Scenario: Keep a hot internal algorithm inside one block
- **WHEN** a developer keeps multiple internal function calls inside one block implementation
- **THEN** the platform SHALL treat those internals as opaque implementation details and only enforce boundary semantics at block entry and exit
