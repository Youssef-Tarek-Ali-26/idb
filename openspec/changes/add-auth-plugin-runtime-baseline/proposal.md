## Why
The architecture catalog defines pluggable authn/authz at a high level, but the CPU runtime did not yet expose concrete caller-context authorization hooks in execution APIs. We need a baseline implementation that keeps auth optional while enabling external policy engines to gate query/mutation/watch behavior.

## What Changes
- Add caller-context and authorizer contracts to `idb-core` as a pluggable auth runtime layer.
- Wire CPU execution paths to optional authorization checks for query/explain/watch/mutate actions.
- Keep default runtime behavior machine-friendly by using an allow-all provider unless configured otherwise.

## Impact
- Core storage/query execution remains usable without native auth services.
- Deployments can attach external auth providers without changing core query semantics.
- Establishes a stable extension seam for future gateway/session auth integration.
