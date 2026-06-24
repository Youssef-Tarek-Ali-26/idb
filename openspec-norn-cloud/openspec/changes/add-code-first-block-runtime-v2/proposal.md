# Change: Code-First Block Runtime V2

## Why
NornCloud already has a platform foundation direction around compute, state, identity, and mesh, but it does not yet define the developer-facing application model that turns those primitives into a simpler day-to-day system. The current gap is the missing code-first block abstraction that can unify polyglot code, per-block observability, visual architecture, and portable deployment under one repo and one execution contract.

## What Changes
- Add a code-first `Block` and `Flow` model as the canonical application contract for NornCloud.
- Define a hybrid runtime architecture with BEAM as the orchestration plane, Rust as the execution kernel/tooling substrate, WASM as the default portable compute target, and native/container targets for compatibility and privileged workloads.
- Add a block registry, per-block observability, execution replay, and an auto-generated visual graph sourced from code.
- Add a portable deployment contract so single-node, cluster, and serverless-style operation are scheduling modes over the same model instead of separate products.
- Add a runtime secret and capability binding model where repos declare references and policy while providers resolve secret values at execution time.
- Add a state binding model where iDB is the preferred substrate but the block contract remains portable across fallback backends.
- Add first-class agent, model, and workspace runtime extensions built on top of the same block and flow contract.

## Impact
- Affected specs: `code-first-block-model`, `block-runtime-execution`, `block-graph-observability`, `portable-deployment-contract`, `secret-capability-binding`, `agent-model-workspace-runtime`
- Related existing changes: `add-norn-cloud-platform-foundation`, `add-tiered-compute-fabric`
- Affected code (future): manifest/schema toolchain, orchestration runtime, execution kernel, scheduler, observability pipeline, edge route ingestion, secret provider bindings, state adapter layer
