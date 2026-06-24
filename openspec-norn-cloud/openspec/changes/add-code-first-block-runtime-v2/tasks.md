## 1. Application Model
- [ ] 1.1 Define the `Block` contract fields for schemas, runtime kind, capabilities, secret references, state bindings, and execution policy.
- [ ] 1.2 Define the `Flow` contract and block-to-block composition semantics.
- [ ] 1.3 Define granularity guidance so a block can represent a function, worker, service, or opaque external unit without changing the runtime contract.

## 2. Runtime Architecture
- [ ] 2.1 Define BEAM responsibilities for orchestration, timers, retries, supervision, watches, and long-lived workflow coordination.
- [ ] 2.2 Define Rust responsibilities for execution kernel, transport normalization, schema tooling, state adapters, and performance-sensitive paths.
- [ ] 2.3 Define WASM, native process, and container runtime targets plus placement and escalation rules.
- [ ] 2.4 Define same-language fast-path execution and cross-language validated boundary semantics.

## 3. State, Secrets, and Capabilities
- [ ] 3.1 Define the preferred iDB state binding contract and fallback adapter requirements.
- [ ] 3.2 Define runtime secret injection, provider link configuration, and redaction rules.
- [ ] 3.3 Define capability declarations, enforcement points, and privileged workload handling.

## 4. Observability and Visual Model
- [ ] 4.1 Define the block registry as the source of truth for graph generation and runtime metadata.
- [ ] 4.2 Define per-block traces, metrics, structured events, and replay checkpoints.
- [ ] 4.3 Define visual graph generation requirements and constraints for future bidirectional editing.

## 5. Agent, Model, and Workspace Extensions
- [ ] 5.1 Define agent blocks as long-lived stateful workflows with memory, tools, timers, and supervision.
- [ ] 5.2 Define model execution capabilities for local, remote-provider, and accelerator-backed inference.
- [ ] 5.3 Define workspace and sandbox runtime targets for code execution, ephemeral environments, and tool-using agent tasks.

## 6. Deployment Contract
- [ ] 6.1 Define portable deployment intent for single-node, cluster, and serverless-style execution.
- [ ] 6.2 Define ingress, routing, load-balancing, and edge integration requirements.
- [ ] 6.3 Define packaging rules for bare metal, VM, Kubernetes, and cloud substrate adapters.

## 7. Validation
- [ ] 7.1 Validate this change with `openspec validate add-code-first-block-runtime-v2 --strict --no-interactive`.
- [ ] 7.2 Use this change as the parent architecture reference for future implementation changes in scheduler, runtime, graph, and toolchain tracks.
