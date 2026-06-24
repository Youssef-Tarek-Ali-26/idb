## ADDED Requirements

### Requirement: Explicit Compute Tier Routing
The scheduler SHALL route execution across Tier 0 (spatial transform), Tier 1 (WASM runtime), and Tier 2 (native capability) based on declared requirements and platform policy.

#### Scenario: Request qualifies for Tier 0
- **WHEN** an operation matches an eligible Tier 0 transform and policy allows direct execution
- **THEN** scheduler MUST route execution to Tier 0 as preferred path

#### Scenario: Tier escalation is required
- **WHEN** Tier 0 cannot satisfy required logic or capability constraints
- **THEN** scheduler MUST escalate to Tier 1 or Tier 2 according to declared fallback policy

### Requirement: Required vs Preferred Capability Semantics
Execution intents MUST distinguish required capabilities from preferred capabilities, and routing MUST enforce required capabilities strictly.

#### Scenario: Required capability unavailable
- **WHEN** a request declares a required capability that is unavailable on candidate nodes
- **THEN** scheduler MUST fail placement rather than silently downgrade behavior

#### Scenario: Preferred capability unavailable
- **WHEN** a request declares a preferred capability that is unavailable
- **THEN** scheduler MAY route to a valid lower tier if fallback policy permits

### Requirement: Tier Routing Explainability
The platform SHALL provide explainable routing decisions that include selected tier, rejected candidates, and policy reasons.

#### Scenario: Operator requests routing explain
- **WHEN** a request is executed with explain enabled
- **THEN** the response MUST include tier decision metadata sufficient to audit placement outcomes
