## 1. Foundation and Governance
- [ ] 1.1 Finalize layer boundaries (Edge, Compute, State, Identity, Mesh) and ownership.
- [ ] 1.2 Define compatibility policy between NornCloud specs and iDB specs.
- [ ] 1.3 Establish migration/versioning rules for platform capabilities.

## 2. Edge Layer
- [ ] 2.1 Implement HTTP/2 + HTTP/3 ingress with TLS termination.
- [ ] 2.2 Add route declaration ingestion from function manifests.
- [ ] 2.3 Add rate-limiting policy hooks scoped by tenant and function.
- [ ] 2.4 Add certificate lifecycle automation backed by state entities.

## 3. Compute Layer
- [ ] 3.1 Implement WASM function deployment, loading, and warm cache lifecycle.
- [ ] 3.2 Implement scheduler placement heuristics with data-local preference.
- [ ] 3.3 Implement autoscale + scale-to-zero lifecycle controls.
- [ ] 3.4 Add event-trigger registration and dispatch pipeline.

## 4. State + Ordered Event Tier
- [ ] 4.1 Implement partitioned ordered event log with append/replay semantics.
- [ ] 4.2 Implement consumer-group offset and rebalance primitives.
- [ ] 4.3 Implement retention and optional key-based compaction policies.
- [ ] 4.4 Implement queue-style claiming + retry + dead-letter patterns.

## 5. Identity and Policy
- [ ] 5.1 Implement capability declarations and runtime enforcement hooks.
- [ ] 5.2 Implement tenant scoping and row-level policy enforcement boundaries.
- [ ] 5.3 Add pluggable external authn/authz provider integration points.

## 6. Mesh + Replication
- [ ] 6.1 Implement node membership and health model over QUIC mesh.
- [ ] 6.2 Implement state replication coordination and failover behavior.
- [ ] 6.3 Integrate scheduler with mesh-awareness and data placement.

## 7. Observability and Reliability
- [ ] 7.1 Define platform event taxonomy and structured logs.
- [ ] 7.2 Add metrics and traces for ingress, execution, and state tiers.
- [ ] 7.3 Add deterministic replay/regression harnesses for mixed transport + event flows.

## 8. Validation
- [ ] 8.1 Validate this change with `openspec validate add-norn-cloud-platform-foundation --strict --no-interactive`.
- [ ] 8.2 Add phased implementation changes referencing this foundation.
