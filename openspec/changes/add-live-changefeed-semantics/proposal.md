# Change: Add Live Changefeed Semantics

## Why
The storage core now emits mutation events, but there is no canonical spec for subscription state, ordering, replay, and diff delivery.

## What Changes
- Add `live-changefeed` capability spec for subscription semantics.
- Define ordering, delivery contract, resume tokens, and consistency windows.
- Define required failure and reconnection behavior.

## Impact
- Affected specs: `live-changefeed` (new)
- Affected code: future reactive engine and websocket delivery layer
