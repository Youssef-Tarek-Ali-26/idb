## ADDED Requirements

### Requirement: Text Traversal Execution on CPU
The CPU text-query backend SHALL execute traversal-source plans over record edge references.

#### Scenario: Outbound traversal query is executed
- **WHEN** the source is `A -> B` with optional final predicates and ranking transforms
- **THEN** the backend MUST walk outbound edges from A candidates to B candidates and return ranked hydrated B records

#### Scenario: Inbound traversal query is executed
- **WHEN** the source is `A <- B` with optional final predicates and ranking transforms
- **THEN** the backend MUST walk inbound edges to resolve B candidates and return ranked hydrated B records
