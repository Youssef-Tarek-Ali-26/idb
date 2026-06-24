## ADDED Requirements

### Requirement: Binary Frame Codec For Wire Transport
The wire protocol subsystem SHALL expose a deterministic binary frame codec for transport operations.

#### Scenario: Frame roundtrip encode/decode
- **WHEN** a valid wire frame is encoded and then decoded
- **THEN** decoded frame fields and payload MUST match the original frame

#### Scenario: Frame decode rejects invalid magic
- **WHEN** incoming bytes contain an unexpected wire magic value
- **THEN** decoding MUST fail with an explicit invalid-magic error

#### Scenario: Frame decode rejects truncation and length mismatch
- **WHEN** incoming bytes are shorter than declared payload length or include unexpected trailing bytes
- **THEN** decoding MUST fail with explicit truncation or payload-length mismatch errors
