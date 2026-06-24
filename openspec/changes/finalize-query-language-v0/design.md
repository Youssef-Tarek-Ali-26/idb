## Context

v0 query syntax has been exploratory. Parser and SDK work require a deterministic grammar and semantic contract.

## Goals

- Provide one canonical grammar for parser implementation.
- Define precedence and stage semantics for hybrid retrieval operators.
- Define graph traversal semantics that can be mapped to planner primitives.
- Provide machine-readable fixtures for SDK/client conformance.

## Non-Goals

- Full language completeness for advanced macros/metaprogramming.
- Query optimizer behavior guarantees.

## Decisions

1. Expression model: `source -> filters -> pipes`.
2. Traversal operators (`->`, `<-`) bind tighter than `where` filters.
3. `where` predicate conjunction defaults to `AND` for newline-separated clauses.
4. `meaning("...")` is modeled as vector intent predicate, not a standalone query class.
5. `watch` is a query mode wrapper over a canonical query body.

## Tradeoffs

- Simple grammar now enables parser progress; language ergonomics can evolve via additive syntax.
- v0 keeps explicitness over syntactic sugar to reduce parser ambiguity.
