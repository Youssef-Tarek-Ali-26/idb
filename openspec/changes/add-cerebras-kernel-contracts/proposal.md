# Change: Add Cerebras Kernel Contracts

## Why
Cerebras is planned as an acceleration layer, but kernel I/O contracts and host execution boundaries are not yet specified.

## What Changes
- Add `cerebras-kernel-contracts` capability spec.
- Define operation-level kernel input/output envelopes and error sentinels.
- Define host-runtime dispatch and fallback requirements.

## Impact
- Affected specs: `cerebras-kernel-contracts` (new)
- Affected code: Python host runtime, Rust backend adapter, CSL kernel interfaces
