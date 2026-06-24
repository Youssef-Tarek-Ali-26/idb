# Change: Add Durable Stream Gateway Commands

## Why
CPU backend now exposes durable mutation stream polling and offset commit APIs, but gateway/server transports do not expose these commands.

## What Changes
- Add canonical gateway commands for durable mutation stream poll and commit.
- Add HTTP/WebSocket/TCP normalization mappings for durable stream commands.
- Dispatch durable stream commands in gateway runtime to CPU backend APIs.
- Add transport/runtime/server tests for durable stream command flow.

## Impact
- Affected specs: `server-gateway`
- Affected code: `idb-gateway`, `idb-server`
