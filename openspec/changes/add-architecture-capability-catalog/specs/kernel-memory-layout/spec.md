## ADDED Requirements

### Requirement: Deterministic Kernel Memory Layout Contracts
The system SHALL define deterministic memory layout contracts for accelerator kernels and host-side packing.

#### Scenario: Host packs tile payload for kernel execution
- **WHEN** records/index metadata are exported to kernel memory
- **THEN** payload layout MUST match versioned offsets/widths expected by kernels
