# Query Language Compatibility Policy

## v0 Contract

- Queries valid under v0 grammar MUST continue to parse under v0.x.
- AST structural changes require a version bump for SDK fixture schema.

## Deprecation Workflow

1. Mark syntax as deprecated in docs and parser warnings.
2. Keep deprecated syntax for at least one minor version.
3. Provide automated rewrite hints in parser diagnostics.
4. Remove only in next major language version.

## Governance

- Grammar changes require:
  - OpenSpec change proposal,
  - updated conformance cases,
  - updated SDK AST fixtures.
