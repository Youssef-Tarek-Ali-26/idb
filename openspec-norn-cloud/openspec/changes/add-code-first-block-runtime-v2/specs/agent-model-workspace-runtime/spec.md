## ADDED Requirements

### Requirement: Agent Behavior Extends the Block Model
The platform SHALL model agents as stateful block or flow compositions rather than as a separate orchestration abstraction.

#### Scenario: Run a long-lived agent workflow
- **WHEN** a developer defines an agent with memory, timers, tools, and retries
- **THEN** the platform SHALL represent it using the same block, flow, registry, observability, and deployment contracts used by ordinary workloads

### Requirement: Model Execution Is a Runtime Capability
The platform SHALL expose model execution as a runtime capability that blocks can invoke or host.

#### Scenario: Run remote-provider inference
- **WHEN** a block uses a hosted external model provider
- **THEN** the platform SHALL bind that call through capabilities, secret references, and observability metadata without changing the block contract shape

#### Scenario: Run accelerator-backed inference
- **WHEN** a block requires local or attached accelerator resources
- **THEN** the scheduler SHALL be able to place the block on eligible runtime targets using capability and hardware intent metadata

### Requirement: Workspace and Sandbox Execution Uses Runtime Targets
The platform SHALL support workspace or sandbox execution as a runtime target for code execution and tool-using tasks.

#### Scenario: Execute an ephemeral tool environment
- **WHEN** a flow step requires an isolated workspace for code execution or artifact generation
- **THEN** the scheduler SHALL be able to place that step on a workspace or sandbox target while preserving the block contract, observability, and secret injection model
