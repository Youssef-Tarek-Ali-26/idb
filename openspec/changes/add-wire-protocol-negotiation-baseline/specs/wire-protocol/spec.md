## ADDED Requirements

### Requirement: Deterministic Protocol Version Negotiation
The wire protocol subsystem SHALL perform deterministic client/server version negotiation before request execution.

#### Scenario: Overlapping client/server version ranges
- **WHEN** both peers provide valid protocol ranges with a non-empty overlap
- **THEN** negotiation MUST succeed and choose the highest common protocol version

#### Scenario: No overlapping version range
- **WHEN** client and server version ranges do not overlap
- **THEN** session establishment MUST fail with an explicit compatibility error

#### Scenario: Invalid version range is rejected
- **WHEN** either peer provides a range where min version is greater than max version
- **THEN** negotiation MUST fail before compatibility selection
