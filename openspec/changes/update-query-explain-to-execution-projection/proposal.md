# Change: Update Explain to Execution Projection

## Why
Traversal text queries are executable on CPU, but planner explain output still uses strict projection and may report traversal as unsupported.

## What Changes
- Update `explain_query_text` to use execution-oriented request projection.
- Keep deterministic unsupported reasons for genuinely unsupported plans (for example `watch`).
- Add traversal explain test coverage.

## Impact
- Affected specs: `logical-plan-bridge`
- Affected code: `idb-planner`
