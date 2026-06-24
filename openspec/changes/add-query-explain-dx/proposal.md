# Change: Add Query Explain DX

## Why
The current text-query bridge executes or fails, but there is no developer-facing introspection artifact showing how text maps to logical plan and executable request.

## What Changes
- Add a planner-level `explain_query_text` API.
- Return deterministic explain output with parsed plan summary and request projection status.
- Include unsupported-reason details when request projection cannot be built.
- Add unit tests for supported and unsupported explain flows.

## Impact
- Affected specs: `logical-plan-bridge`
- Affected code: `idb-planner`
