# CSL Reference Documentation

This directory contains local reference docs for the Cerebras Software Language (CSL),
compiled from the official Cerebras SDK 1.4.0 documentation.

In iDB, search and execution are designed to run at wafer scale. These references
exist so kernel work can move quickly without depending on external docs every time.

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

- SDK Documentation: [sdk.cerebras.net](https://sdk.cerebras.net/)
- SDK Examples: [github.com/Cerebras/sdk-examples](https://github.com/Cerebras/sdk-examples)
- CSL Language Guide: [sdk.cerebras.net/csl/language_index](https://sdk.cerebras.net/csl/language_index)
