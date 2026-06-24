## ADDED Requirements

### Requirement: Hardware-Aware Planner Routing
The system SHALL route executable plans across CPU, GPU, and Cerebras backends through a stable planner contract.

#### Scenario: Backend is selected for a query class
- **WHEN** a logical query plan is compiled for execution
- **THEN** the planner MUST choose a backend based on query shape, capabilities, and deterministic routing rules
