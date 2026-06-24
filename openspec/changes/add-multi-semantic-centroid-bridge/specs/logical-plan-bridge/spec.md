## ADDED Requirements

### Requirement: Multi-Semantic Predicate Compilation
The bridge SHALL compile multiple semantic predicates (`meaning(...)`) into a single executable `QueryRequest.vector_query` using deterministic centroid composition.

#### Scenario: Multiple semantic predicates are compiled
- **WHEN** a logical plan contains more than one semantic predicate
- **THEN** the bridge MUST emit one `vector_query` whose vector is the normalized centroid of the predicate embeddings
- **AND** the bridge MUST keep deterministic output for the same predicate sequence and bridge options

#### Scenario: Thresholds across semantic predicates are compiled deterministically
- **WHEN** multiple semantic predicates include threshold values
- **THEN** the bridge MUST compile `min_vector_score` as the strictest threshold value among those predicates
