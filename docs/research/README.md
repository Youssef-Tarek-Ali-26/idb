# Upstream Research Workflow

## What is here
- Prompt for Claude repo-scanning: `docs/research/CLAUDE_REPO_SCAN_PROMPT.md`
- N-space theory foundation: `docs/research/N_SPACE_THEORY.md`
- N-space architecture synthesis: `docs/research/N_SPACE_SYNTHESIS.md`
- Clone helper script: `scripts/research/clone_upstream_repos.sh`
- Cloned repos location: `upstream/`

## Quick start
1. Ensure repos are cloned:
   - `./scripts/research/clone_upstream_repos.sh`
2. Give Claude the prompt file content.
3. Ask Claude to inspect `upstream/*` and return:
   - pattern catalog,
   - edge-case matrix,
   - phased implementation plan for iDB live + traversal.

## Notes
- Use patterns/invariants, not verbatim code.
- Prioritize `rethinkdb` for changefeed/live semantics.
- Prioritize `kuzu`, `arangodb`, `janusgraph`, `nebula`, `age` for traversal/planner/runtime ideas.
