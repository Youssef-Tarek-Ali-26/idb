# Change: Add CPU Explain API

## Why
Planner explain exists, but runtime callers currently have to invoke planner directly instead of using the backend surface they already use for text execution.

## What Changes
- Add `CpuBackend::explain_query_text` and options variant.
- Return planner explain output through core result/error contracts.
- Add CPU tests for explain behavior.

## Impact
- Affected specs: `execution-backend-contract`
- Affected code: `idb-executor-cpu`
