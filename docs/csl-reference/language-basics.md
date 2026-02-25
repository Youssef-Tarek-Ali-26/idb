# CSL Language Basics

> Source: Cerebras SDK 1.4.0 — CSL Language Guide

CSL (Cerebras Software Language) is a C-like language built around a dataflow
programming model, designed for programming the WSE.

## Type System

### Numeric Types

| Type | Size | Description |
|------|------|-------------|
| `bool` | 1 bit | Boolean |
| `i8` | 8-bit | Signed integer |
| `u8` | 8-bit | Unsigned integer |
| `i16` | 16-bit | Signed integer |
| `u16` | 16-bit | Unsigned integer |
| `i32` | 32-bit | Signed integer |
| `u32` | 32-bit | Unsigned integer |
| `f16` | 16-bit | IEEE 754 half-precision float |
| `f32` | 32-bit | IEEE 754 single-precision float |
| `f64` | 64-bit | IEEE 754 double-precision float (WSE-3 only in HW) |
| `cb16` | 16-bit | Cerebras cbfloat16 (6-bit exponent, 9-bit explicit mantissa) |
| `bf16` | 16-bit | bfloat16 |

### Special Types

| Type | Description |
|------|-------------|
| `void` | No value |
| `type` | The type of types (comptime only) |
| `color` | Wavelet color identifier |
| `comptime_struct` | Compile-time struct (for params) |
| `data_task_id` | Data task identifier |
| `local_task_id` | Local task identifier |
| `control_task_id` | Control task identifier |
| `input_queue` | Input queue (WSE-3) |
| `output_queue` | Output queue (WSE-3) |

### Array Types

Arrays are declared as `[size]T`. Multidimensional: `[d1, d2, d3]T`.

```csl
var data: [100]f32;                    // 1D array of 100 floats
var matrix: [10, 10]f32;               // 2D 10x10 matrix
var tensor: [4, 8, 16]i16;             // 3D tensor

// Array literals
var vals = [3]i16 { 1, 2, 3 };

// Zero-initialized
var zeros = @zeros([1024]f32);
```

**Important**: Element type must NOT be another array type (no `[N][M]T`).
Use multidimensional syntax `[N, M]T` instead.

### Struct Types

Three kinds of structs in CSL:

```csl
// 1. Anonymous struct (comptime literal)
const config = .{ .width = 10, .height = 20 };

// 2. Named struct
const Point = struct {
    x: f32,
    y: f32,
};

// 3. comptime_struct (for passing compile-time parameters)
param memcpy_params: comptime_struct;
```

### Pointer Types

```csl
var x: i32 = 42;
var ptr: *i32 = &x;          // Pointer to single element
var arr_ptr: [*]i32 = &arr;  // Pointer to unknown number of elements

// Dereferencing
var val = ptr.*;

// Array element access through pointer
(arr_ptr.*)[0] = 10;

// Const pointers
var cptr: *const i32 = &x;   // Cannot modify through this pointer
```

### Enumeration Types

```csl
const Direction = enum(u8) {
    North = 0,
    South = 1,
    East = 2,
    West = 3,
};

var d: Direction = Direction.North;
```

## Variables and Constants

```csl
// Runtime variables
var count: u32 = 0;
var buffer: [256]f32 = undefined;  // Uninitialized

// Compile-time constants
const MAX_SIZE: u16 = 1024;
const PI: f32 = 3.14159;

// Parameters (set by layout or host)
param width: u16;
param memcpy_params: comptime_struct;

// Export for host access
export var result: [1]i32;
```

## Functions

```csl
// Regular function
fn add(a: i32, b: i32) i32 {
    return a + b;
}

// Void function
fn process(data: *[100]f32) void {
    // ...
}

// Inline function
inline fn fast_mul(x: f32, y: f32) f32 {
    return x * y;
}
```

## Control Flow

```csl
// If/else
if (x > 0) {
    // ...
} else if (x == 0) {
    // ...
} else {
    // ...
}

// While loop
while (i < n) : (i += 1) {
    // ...
}

// For loop
for (var i: u16 = 0; i < count; i += 1) {
    // ...
}

// Switch
switch (op) {
    0 => result = a + b,
    1 => result = a - b,
    2 => result = a * b,
    else => result = 0,
}
```

## Type Casting

```csl
// @as — explicit cast
var x: i32 = @as(i32, some_u16);
var f: f32 = @as(f32, some_i32);

// @bitcast — reinterpret bits
var bits: u32 = @bitcast(u32, some_f32);
```

## Modules

```csl
// Importing a module
const math = @import_module("<math>");
const my_lib = @import_module("./my_lib.csl");

// Importing with parameters
const sys_mod = @import_module("<memcpy/memcpy>", memcpy_params);

// Using imported symbols
var result = math.sqrt(x);
```

## Symbol Export

```csl
// Export a variable for host access
export var result: [1]f32;
var result_ptr: [*]f32 = &result;

comptime {
    @export_symbol(result_ptr, "result");
    @export_symbol(f_run);           // Export function
    @export_name("result", [*]f32, false);  // Named export
}
```

## Important Differences from C

1. **No heap allocation** — All memory must be statically allocated
2. **No recursion** — Stack depth is extremely limited
3. **48 KB total memory** — All code + data must fit in PE SRAM
4. **No standard library** — Use CSL builtins and Cerebras libraries
5. **Comptime** — Heavy use of compile-time computation for configuration
6. **Tasks, not threads** — Execution model is task-based, not thread-based
7. **Array syntax** — `[N]T` not `T[N]`
8. **No dynamic dispatch** — Everything is resolved at compile time
