## ADDED Requirements

### Requirement: Partitioned Ordered Event Log
The ordered event tier SHALL provide append-only partitioned logs with per-partition ordering and replay by sequence.

#### Scenario: Append and replay from offset
- **WHEN** events are appended with a partition key
- **THEN** events MUST receive monotonically increasing per-partition sequences and be replayable from any sequence offset

### Requirement: Consumer Group Offset Semantics
The ordered tier MUST support consumer-group offsets so multiple workers can process streams with resumable progress tracking.

#### Scenario: Worker restarts from committed offset
- **WHEN** a consumer group commits offsets and a worker restarts
- **THEN** polling MUST resume from the next uncommitted sequence for each assigned partition

### Requirement: Retention and Optional Log Compaction
The ordered tier MUST support retention policies and optional latest-by-key compaction for long-running streams.

#### Scenario: Retention policy trims old records
- **WHEN** partition events exceed retention limits
- **THEN** old records MUST be pruned according to configured retention constraints

#### Scenario: Latest-by-key compaction runs
- **WHEN** compaction is configured for a topic/entity stream
- **THEN** only the latest record per key SHOULD remain, while preserving replay order for retained entries

### Requirement: Queue and Pub/Sub Pattern Absorption
The ordered tier + subscriptions MUST support queue-style processing and pub/sub replay semantics without requiring external broker systems.

#### Scenario: Queue-style claiming and dead-letter routing
- **WHEN** worker attempts exceed policy for a task event
- **THEN** platform MUST support routing to a dead-letter stream/entity for later inspection and recovery
