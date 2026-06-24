## 1. Specification
- [ ] 1.1 Add mapper abstraction requirements to the unified spatial storage spec.
- [ ] 1.2 Define deterministic mapper baseline requirements and acceptance criteria.
- [ ] 1.3 Define learned mapper experimental requirements and benchmark gate.

## 2. Design
- [ ] 2.1 Define the mapper contract between fused N-space points and physical placement/routing.
- [ ] 2.2 Define how mapper metadata is versioned and exposed for explain/debug workflows.
- [ ] 2.3 Define migration and rebuild behavior when mapper configuration changes.

## 3. Implementation Follow-Up
- [ ] 3.1 Add mapper traits and metadata types in `idb-core`.
- [ ] 3.2 Refactor current keyspace code to sit behind the mapper contract.
- [ ] 3.3 Add at least one deterministic mapper implementation for CPU-first validation.
- [ ] 3.4 Add benchmark plan and baseline comparisons before learned mapper promotion.

## 4. Validation
- [ ] 4.1 Validate the OpenSpec change with `openspec validate update-spatial-mapper-architecture --strict --no-interactive`.
