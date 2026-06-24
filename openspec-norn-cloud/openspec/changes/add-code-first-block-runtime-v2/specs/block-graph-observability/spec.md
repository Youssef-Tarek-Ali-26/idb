## ADDED Requirements

### Requirement: Block Registry Is the Source of Truth
The platform SHALL maintain a block registry that is the source of truth for runtime metadata, graph generation, and toolchain introspection.

#### Scenario: Register blocks and flows
- **WHEN** the repo is analyzed for block and flow definitions
- **THEN** the registry SHALL expose the block contracts, flow edges, runtime metadata, and version identifiers needed by runtime and tooling

### Requirement: Per-Block Observability Is Built In
The platform SHALL emit traces, metrics, and structured events at block boundaries by default.

#### Scenario: Inspect a failed block execution
- **WHEN** a block fails during a flow execution
- **THEN** operators SHALL be able to inspect the block-level input context, output or error state, timing, retries, and linked flow position

### Requirement: Graph Views Derive from Code
The platform SHALL generate a visual graph from block and flow definitions without making the visual layer the primary source of truth.

#### Scenario: Generate the graph from repo definitions
- **WHEN** the tooling renders a graph for a project
- **THEN** the graph SHALL be derived from the registry-backed block and flow model defined in code

### Requirement: Future Visual Editing Must Preserve Model Integrity
The platform SHALL only support visual editing when edits round-trip through the same underlying block and flow model.

#### Scenario: Edit flow wiring visually
- **WHEN** a user edits flow wiring in a visual tool
- **THEN** the resulting changes SHALL update the same registry-backed model used by code-first tooling rather than creating a separate visual-only artifact

### Requirement: Replay Uses Block Checkpoints
The platform SHALL support replay and debugging using execution checkpoints recorded at block boundaries.

#### Scenario: Replay from a mid-flow block
- **WHEN** an operator requests replay from a previously recorded block boundary
- **THEN** the platform SHALL use the recorded checkpoint and flow metadata to restart execution from that boundary
