# Project Context

## Purpose
NornCloud is a cloud runtime platform built around three primitives: WASM for compute isolation, iDB for unified state, and capability-based identity for access control. The goal is to replace fragmented cloud stacks (containers, orchestrators, service meshes, brokers, and many separate data systems) with one coherent, programmable platform.

## Tech Stack
- Rust for core platform runtime and control plane components
- WASM components (WASI Preview 2) for user functions
- iDB (NornDB) as the shared state substrate
- QUIC + HTTP/2 + HTTP/3 networking surfaces
- OpenSpec for spec-driven architecture and implementation planning

## Project Conventions

### Code Style
- Rust-first implementation with deterministic, explicit data models
- Clear separation between transport adapters, normalization layers, and execution runtime
- Configuration and policy as typed entities, not implicit global state
- Prefer append-only logs and deterministic replay for evented workflows

### Architecture Patterns
- Five-layer architecture: Edge, Compute, State, Identity, Mesh
- Functions communicate via shared state + typed events, not direct service-to-service RPC coupling
- Ordered event tier is 1D log-structured (partition + sequence), separate from multidimensional spatial indexing tiers
- All ingress transports normalize to one canonical execution contract before dispatch

### Testing Strategy
- Unit tests for protocol codecs, normalization, and lifecycle state machines
- Deterministic replay tests for ordered event flows and consumer offset progression
- Cross-transport parity tests (HTTP/WebSocket/TCP) for equivalent request semantics
- Spec validation required for every architecture or capability change

### Git Workflow
- Short-lived feature branches scoped to a single OpenSpec change when possible
- Proposal and tasks are committed with implementation changes to keep intent and execution aligned
- Validate with strict OpenSpec checks and workspace tests before merging

## Domain Context
- NornCloud targets self-hosted and cloud-hosted deployments that need low operational complexity and strong multi-tenant isolation.
- iDB stores application state, logs, metrics, events, secrets, and platform metadata.
- WASM function lifecycle supports deploy, cold start, warm execution, autoscale, and scale-to-zero behavior.
- Reactive subscriptions and ordered event streams power pub/sub, queue-like processing, and stream-style pipelines.

## Important Constraints
- Keep identity and authorization pluggable and decoupled from core storage/query semantics.
- Preserve deterministic behavior for replayable event processing.
- Prioritize data-local execution placement when possible.
- Support machine-only workloads without requiring full user/session auth subsystems.

## External Dependencies
- Let's Encrypt/ACME for certificate automation
- Wasmtime-compatible WASM toolchains (Rust, TypeScript, Python, Go)
- QUIC-capable networking stack for mesh and transport surfaces
- Optional external policy providers for authz decisions
