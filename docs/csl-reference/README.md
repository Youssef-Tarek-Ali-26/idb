# CSL Reference Documentation

This directory contains reference documentation for the Cerebras Software Language (CSL),
compiled from the official Cerebras SDK 1.4.0 documentation.

These docs serve as a local reference for iDB/NornDB kernel development, ensuring
we have reliable CSL documentation available during development.

## Contents

- [Architecture Overview](./architecture.md) — WSE hardware, PEs, routers, SRAM
- [Language Basics](./language-basics.md) — Types, syntax, variables, functions
- [Tasks and Execution](./tasks-and-execution.md) — Data/local/control tasks, activation, blocking
- [Colors and Routing](./colors-and-routing.md) — Wavelet routing, color configuration, fabric
- [DSDs (Data Structure Descriptors)](./dsds.md) — Memory, fabric, and FIFO DSDs
- [Memcpy and Host Interface](./memcpy-and-host.md) — Host↔device data transfer, SdkRuntime
- [Comptime and Layout](./comptime-and-layout.md) — Compile-time programming, PE mesh configuration
- [Libraries](./libraries.md) — Built-in libraries and builtins reference
- [Code Patterns](./code-patterns.md) — Common CSL patterns with annotated examples

## Official Sources

- SDK Documentation: https://sdk.cerebras.net/
- SDK Examples: https://github.com/Cerebras/sdk-examples
- CSL Language Guide: https://sdk.cerebras.net/csl/language_index
