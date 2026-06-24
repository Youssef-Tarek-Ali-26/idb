## Context
The v2 direction for NornCloud is not only a cloud/runtime foundation problem. It is also a developer experience and architecture legibility problem. The desired outcome is one repo, one application model, fewer manually maintained APIs and deployment descriptors, and a polyglot execution story that remains observable and portable.

Two constraints shape this design:
- NornCloud already has a stated direction around WASM, iDB, capability-based identity, deterministic replay, and transport normalization.
- iDB is strategically important but not yet proven enough to be the only acceptable state backend for every workload.

## Goals / Non-Goals
- Goals:
  - Define a code-first application contract that unifies blocks, flows, deployment intent, secrets, capabilities, and observability.
  - Support mostly-single-language projects without imposing permanent polyglot overhead.
  - Make per-block architecture visible without requiring visual-first authoring.
  - Preserve portability across bare metal, cluster, and serverless-style operation.
  - Keep iDB as the preferred state substrate without making platform viability depend on it.
- Non-Goals:
  - Do not replace the existing foundation change or tiered compute model.
  - Do not require a custom language in the first phase.
  - Do not require every internal function to become a block.
  - Do not eliminate all edge infrastructure or secret providers; instead, absorb them behind one platform contract.

## Decisions
- Decision: Blocks are the canonical observable boundary.
  - A block defines schemas, runtime kind, secret references, capabilities, state binding intent, and execution policy.
  - A block may represent a small function, a long-lived worker, a service boundary, or an opaque external integration.
  - Observability and cross-runtime validation happen at declared block boundaries, not every internal function call.
- Decision: Flows are code-first compositions of blocks.
  - Flow definitions live in the repo and are the primary composition model.
  - Generated graph views and future visual editing both derive from the same registry-backed model.
  - The visual graph is an interface to the model, not an alternative source of truth.
- Decision: Use a hybrid runtime split.
  - BEAM owns orchestration concerns: timers, retries, supervision, watches, subscriptions, workflow progression, and failure containment across long-lived coordination.
  - Rust owns execution kernel concerns: schema tooling, transport normalization, state adapters, runtime bridges, placement inputs, and performance-sensitive data paths.
  - WASM is the default portable compute target for sandboxed user code.
  - Native processes and containers remain supported for trusted or compatibility workloads.
- Decision: Make deployment modes policy, not product boundaries.
  - Single-node, clustered, and serverless-style modes all execute the same block contract.
  - The scheduler chooses placement and lifecycle behavior from deployment intent, capabilities, locality, and isolation policy.
- Decision: Keep state abstracted behind a block contract.
  - iDB is the preferred substrate for shared state, ordered events, and metadata.
  - Fallback adapters for SQLite, DuckDB, Postgres, or similar backends are permitted where iDB is not ready or not suitable.
  - Application contracts bind to state capabilities and entities, not directly to backend-specific APIs.
- Decision: Secrets are declared in code and resolved at runtime.
  - Repos declare secret references and capability requirements.
  - Secret values are resolved from linked providers at execution time and never become part of source truth or deploy artifacts.
  - The runtime must enforce redaction and least-privilege access per block.
- Decision: Same-language traffic should be optimized.
  - Same-language block composition may bypass slow cross-runtime serialization when policy, safety, and observability guarantees still hold.
  - Cross-language boundaries must remain schema-validated and observable.
- Decision: Agent, model, and workspace features are extensions of blocks, not parallel abstractions.
  - Agents are long-lived stateful block/flow compositions with tool access, memory bindings, timers, and supervision.
  - Models are runtime capabilities that blocks can invoke or host with placement policies for accelerator or provider-backed execution.
  - Workspaces and sandboxes are runtime targets used for code execution, ephemeral environments, and tool-driven tasks.
  - These features reuse the same registry, observability, secret, capability, and deployment contracts as ordinary blocks.

## Architecture Shape
The architecture is layered:

1. Edge layer
   - Accepts HTTP, WebSocket, and other ingress forms.
   - Normalizes transport requests to one canonical execution contract.
2. Orchestration plane
   - Runs on BEAM and owns workflow progression, timers, retries, watches, scheduling intents, and failure supervision.
3. Execution kernel
   - Runs in Rust and owns block loading, validation, runtime bridges, placement hints, data-path optimization, and portable execution target handling.
4. Runtime targets
   - WASM for default portable compute.
   - Native process for trusted high-performance or language-specific workers.
   - Container target for compatibility packaging and legacy/runtime-heavy dependencies.
   - Workspace/sandbox target for ephemeral code execution and agent tool environments.
5. State and identity plane
   - iDB preferred for state, ordered events, metadata, and replay records.
   - Fallback adapters permitted for selected workloads.
   - Capability and secret bindings enforced at runtime.

## Trade-Offs
- BEAM improves orchestration semantics but adds a mixed-language platform boundary.
  - Mitigation: keep the seam explicit and narrow, with Rust owning the kernel contract.
- Same-language fast paths reduce overhead but can create semantic divergence from cross-language boundaries.
  - Mitigation: require identical tracing, policy, and validation checkpoints even when serialization is bypassed.
- Portable state bindings reduce lock-in but may weaken backend-specific optimization opportunities.
  - Mitigation: keep the portable contract small and allow capability-based opt-ins for backend-specific features.
- Visual editing increases usability but risks drift from code if treated as a separate model.
  - Mitigation: require the block registry to remain the only source of truth.

## Migration Plan
1. Define and validate the block/flow schema and registry model.
2. Implement code-first block registration and generated graph output without visual editing.
3. Add per-block traces, metrics, and replay checkpoints.
4. Add BEAM orchestration around the existing compute/state direction.
5. Add runtime target adapters and same-language fast-path optimization.
6. Add portable deploy intents and substrate adapters.
7. Evaluate whether a custom orchestration DSL is still necessary after the code-first model stabilizes.

## Open Questions
- Should BEAM be mandatory for every deployment profile, or optional for some single-node developer modes?
- How much same-language bypass is acceptable before debugging parity degrades?
- Which subset of state operations must be guaranteed portable across iDB and fallback backends?
- Should visual editing be limited to flow wiring at first, leaving block internals code-only?
