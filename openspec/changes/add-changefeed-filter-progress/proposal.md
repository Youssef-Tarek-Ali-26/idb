# Change: Ensure Filtered Changefeed Poll Progress

## Why
Filtered subscriptions can stall when many non-matching mutation events appear before matching events because the cursor does not advance unless a matching event is delivered.

## What Changes
- Advance subscription cursor to the last scanned commit sequence even when no matching event is delivered.
- Reject invalid poll bounds (`max_events = 0`) at the changefeed engine layer.
- Add regression tests for sparse filtered subscriptions to guarantee eventual progress.

## Impact
- Affected specs: `live-changefeed`
- Affected code: `idb-storage`
