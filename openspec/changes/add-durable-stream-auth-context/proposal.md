# Change: Add Durable Stream Caller/Auth Context

## Why
Durable mutation stream poll/commit gateway commands currently bypass caller context and therefore skip auth/tenant-scope checks used by the rest of gateway command dispatch.

## What Changes
- Add caller context to canonical durable stream poll/commit gateway commands.
- Update HTTP/WebSocket/TCP durable stream payload parsing to accept optional caller and default to anonymous caller.
- Dispatch durable stream commands through auth-aware CPU backend APIs.
- Add tests for transport normalization and auth/tenant-scope enforcement on durable stream commands.

## Impact
- Affected specs: `server-gateway`
- Affected code: `idb-gateway`, `idb-executor-cpu`
