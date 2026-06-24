# Change: Finalize Query Language v0

## Why
Current query examples are exploratory. We need a stable v0 language contract for parser and SDK generation.

## What Changes
- Add `query-language-v0` capability spec.
- Define grammar, operator precedence, and canonical semantics.
- Define compatibility and deprecation policy for syntax evolution.

## Impact
- Affected specs: `query-language-v0` (new)
- Affected code: parser, analyzer, SDK query builders, docs
