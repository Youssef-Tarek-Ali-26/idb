## ADDED Requirements

### Requirement: Secondary Indexing for Non-Spatial Access
The system SHALL support secondary indexes for non-spatial access paths such as categorical, range, and bitmap-oriented predicates.

#### Scenario: Query uses non-spatial selective predicate
- **WHEN** a predicate is better served by a secondary index than primary spatial routing
- **THEN** planner/executor MUST be able to use secondary index candidates without breaking result semantics
