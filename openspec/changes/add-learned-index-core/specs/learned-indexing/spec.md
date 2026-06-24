## ADDED Requirements

### Requirement: Learned Position Model With Bounded Fallback
The system SHALL provide a learned index model that predicts key position while preserving correctness via deterministic bounded fallback search.

#### Scenario: Prediction window is produced for lookup
- **WHEN** a lookup key is evaluated by the learned model
- **THEN** the model MUST return a bounded search window that includes worst-case prediction error

#### Scenario: Exact lookup is resolved through bounded fallback
- **WHEN** a key lookup is executed through learned index APIs
- **THEN** the system MUST return exact match/no-match results equivalent to full search semantics
