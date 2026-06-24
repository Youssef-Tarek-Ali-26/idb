## ADDED Requirements

### Requirement: Parser AST to Logical Plan Translation
The system SHALL translate parser AST output into a typed logical plan with deterministic stage ordering.

#### Scenario: Hybrid query text is executed
- **WHEN** a valid query string is parsed to AST
- **THEN** the planner MUST produce a logical plan whose filter, semantic intent, and transform stages match the AST semantics

### Requirement: Executable QueryRequest Bridge
The system SHALL provide a bridge from logical plan to executable `QueryRequest` for the v0 CPU-supported query subset.

#### Scenario: Text query is run through CPU backend
- **WHEN** a query string uses v0-supported scan/filter/top semantics
- **THEN** the bridge MUST produce `QueryRequest` values that execute successfully on CPU backend
