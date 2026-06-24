<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

## Diagram Maintenance

When implementation changes touch runtime or architecture behavior, keep both visual guides in sync in the same change:
- `docs/book/DB_DIAGRAMS.md`
- `docs/book/DB_DIAGRAMS_ASCII.md`

Minimum required sync checks:
- read/query flow still matches parser/planner/executor stages
- write/mutation flow still matches storage + durable stream behavior
- watch flow and durable stream flow remain accurate
- transport parity diagram still matches HTTP/WS/TCP normalization behavior
