## ADDED Requirements

### Requirement: Semantic Threshold Compilation
The bridge SHALL compile an optional semantic threshold from `meaning(...)` into executable request constraints for the CPU-supported v0 subset.

#### Scenario: Thresholded semantic query is compiled
- **WHEN** a logical plan includes one semantic predicate with `threshold`
- **THEN** the bridge MUST emit a `QueryRequest` containing a minimum semantic score constraint equal to the threshold value

#### Scenario: Invalid threshold bounds are rejected
- **WHEN** the threshold is outside supported cosine-score bounds
- **THEN** the bridge MUST fail with a deterministic planner error describing the invalid threshold
