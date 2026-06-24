## ADDED Requirements

### Requirement: Unified Blob and Structured Storage Boundary
The system SHALL support large-object/blob references as part of unified storage metadata and retrieval contracts.

#### Scenario: Record references external or large blob payload
- **WHEN** blob-linked records are ingested or queried
- **THEN** record envelopes MUST preserve durable blob linkage without requiring separate application-level metadata systems
