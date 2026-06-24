# Change: NornCloud Platform Foundation Spec

## Why
NornCloud is a long-term platform effort and needs an independent OpenSpec track from iDB so cloud-layer decisions, sequencing, and ownership do not get mixed with database-internal specs.

## What Changes
- Establish a separate NornCloud architecture foundation change that defines normative requirements for:
  - Edge gateway and transport normalization
  - WASM compute runtime and scheduler behavior
  - Unified state layer usage model
  - Capability-driven identity model
  - Ordered event tier (Kafka/Rabbit/SQS/NATS pattern absorption)
  - Function-triggered event processing
  - Mesh/runtime behavior across nodes
  - Platform observability baseline
- Define concrete scenarios for transport parity, function lifecycle, partitioned replay, queue semantics, and stream-style processing.
- Add an implementation task map to drive phased buildout.

## Impact
- Affected specs: edge-gateway, wasm-compute-runtime, unified-state-layer, identity-capability-model, ordered-event-tier, function-event-processing, mesh-runtime, platform-observability
- Affected code (future): edge service, scheduler/runtime, state adapters, auth/capability engine, ordered log subsystem, trigger executor, mesh control plane
- Governance: NornCloud planning can evolve independently from iDB internal roadmap while still referencing iDB as the state substrate
