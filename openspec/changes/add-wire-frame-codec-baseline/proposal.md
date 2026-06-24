## Why
Protocol version negotiation exists, but we still need an executable binary frame format for request/response exchange over TCP-style transports. A stable frame codec is required before transport listeners can exchange canonical commands safely.

## What Changes
- Add wire frame header/payload types in `idb-wire`.
- Implement binary frame encode/decode with magic/version/opcode/request-id/length fields.
- Add strict validation for magic mismatch, truncation, and payload length inconsistencies.

## Impact
- Establishes the binary framing baseline for future high-performance TCP server paths.
- Prevents silent framing corruption by enforcing explicit decode errors.
