# Memcpy and Host Interface

> Source: Cerebras SDK 1.4.0 — Host Runtime and Tensor Streaming

## Overview

The memcpy infrastructure bridges the host CPU and the WSE, providing data transfer
and kernel invocation capabilities.

## Setup

### Layout File

```csl
const memcpy = @import_module("<memcpy/get_params>", .{
    .width = width,
    .height = height
});

layout {
    @set_rectangle(width, height);

    // Get per-PE memcpy parameters
    const memcpy_params_0 = memcpy.get_params(0);  // Column 0
    const memcpy_params_1 = memcpy.get_params(1);  // Column 1
    // ...

    @set_tile_code(x, y, "pe_program.csl", .{
        .memcpy_params = memcpy_params_x,
        // ... other params
    });
}
```

### PE Program

```csl
param memcpy_params: comptime_struct;
const sys_mod = @import_module("<memcpy/memcpy>", memcpy_params);
```

## Data Transfer Modes

### Copy Mode

Host explicitly copies data to/from device memory. PE code exports symbol pointers.

**PE code:**
```csl
export var data: [100]f32;
var data_ptr: [*]f32 = &data;

fn f_compute() void {
    // Process data
    // ...
    sys_mod.unblock_cmd_stream();  // Signal completion
}

comptime {
    @export_symbol(data_ptr, "data");
    @export_symbol(f_compute);
}
```

**Host code (Python):**
```python
from cerebras.sdk.runtime.sdkruntimepybind import (
    SdkRuntime, MemcpyDataType, MemcpyOrder
)

runner = SdkRuntime(args.name, cmaddr=args.cmaddr)
data_symbol = runner.get_id('data')

runner.load()
runner.run()

# Host → Device (copy mode)
runner.memcpy_h2d(
    data_symbol,          # dest symbol on device
    input_array,          # source numpy array
    0, 0,                 # (x, y) start PE
    width, height,        # PE rectangle size
    elements_per_pe,      # number of elements per PE
    streaming=False,      # copy mode
    data_type=MemcpyDataType.MEMCPY_32BIT,
    order=MemcpyOrder.ROW_MAJOR,
    nonblock=False
)

# Launch kernel function
runner.launch('f_compute', nonblock=False)

# Device → Host (copy mode)
runner.memcpy_d2h(
    output_array,         # dest numpy array
    data_symbol,          # source symbol on device
    0, 0,                 # (x, y) start PE
    width, height,        # PE rectangle size
    elements_per_pe,      # number of elements per PE
    streaming=False,
    data_type=MemcpyDataType.MEMCPY_32BIT,
    order=MemcpyOrder.ROW_MAJOR,
    nonblock=False
)

runner.stop()
```

### Streaming Mode

Data arrives as wavelets, triggering data tasks. No explicit kernel launch needed.

**PE code:**
```csl
// Queue IDs
const h2d_iq: input_queue = @get_input_queue(2);
const d2h_oq: output_queue = @get_output_queue(3);

// Data task triggered by incoming H2D wavelets
const recv_task_id: data_task_id =
    if (@is_arch("wse2")) @get_data_task_id(sys_mod.MEMCPYH2D_1)
    else if (@is_arch("wse3")) @get_data_task_id(h2d_iq);

task recv_data(wavelet_data: i16) void {
    // Process incoming data
    global = wavelet_data;
    // Send result back
    @mov16(out_dsd, global);
}

const out_dsd = @get_dsd(fabout_dsd, .{
    .extent = 1,
    .fabric_color = sys_mod.MEMCPYD2H_1,
    .output_queue = d2h_oq
});

comptime {
    @bind_data_task(recv_data, recv_task_id);

    if (@is_arch("wse3")) {
        @initialize_queue(h2d_iq, .{ .color = sys_mod.MEMCPYH2D_1 });
        @initialize_queue(d2h_oq, .{ .color = sys_mod.MEMCPYD2H_1 });
    }
}
```

**Host code:**
```python
# Streaming H2D
runner.memcpy_h2d(
    data_symbol, input_data,
    0, 0, width, height, elements_per_pe,
    streaming=True,  # streaming mode
    data_type=MemcpyDataType.MEMCPY_16BIT,
    nonblock=False
)

# Streaming D2H
runner.memcpy_d2h(
    output_data, result_symbol,
    0, 0, width, height, elements_per_pe,
    streaming=True,
    data_type=MemcpyDataType.MEMCPY_16BIT,
    nonblock=False
)
```

## System Colors

The memcpy infrastructure provides up to 4 input and 4 output streaming colors:

| Color | Direction | Usage |
|-------|-----------|-------|
| `sys_mod.MEMCPYH2D_1` | Host → Device | Streaming input 1 |
| `sys_mod.MEMCPYH2D_2` | Host → Device | Streaming input 2 |
| `sys_mod.MEMCPYH2D_3` | Host → Device | Streaming input 3 |
| `sys_mod.MEMCPYH2D_4` | Host → Device | Streaming input 4 |
| `sys_mod.MEMCPYD2H_1` | Device → Host | Streaming output 1 |
| `sys_mod.MEMCPYD2H_2` | Device → Host | Streaming output 2 |
| `sys_mod.MEMCPYD2H_3` | Device → Host | Streaming output 3 |
| `sys_mod.MEMCPYD2H_4` | Device → Host | Streaming output 4 |

**Important**: Do NOT define routing for these colors — the memcpy module handles it.

## Reserved Resources

The memcpy infrastructure reserves:
- **Colors**: 21, 22, 23
- **Local task IDs**: 27, 28, 30
- **Control task IDs**: 33, 34, 35, 36, 37
- **Microthread 0** (WSE-3)
- **Input queue 0, output queue 0, input queue 1** (WSE-3)

## Data Types

| Type | Size | Notes |
|------|------|-------|
| `MEMCPY_16BIT` | 16-bit | Zero-extended to 32-bit on input; strip upper 16 bits on output |
| `MEMCPY_32BIT` | 32-bit | Native wavelet size |

## Tensor Layout

Host tensor is a 1D array of length `width * height * elements_per_pe`.

- **ROW_MAJOR**: `elements_per_pe` is fastest-varying dimension
- **COL_MAJOR**: `width` is fastest-varying (better bandwidth)

## SdkLayout API (Beta)

Newer alternative to layout.csl, using Python for layout:

```python
from cerebras.sdk.runtime.sdkruntimepybind import (
    SdkRuntime, SdkTarget, SdkLayout, SimfabConfig, get_platform
)

config = SimfabConfig(dump_core=True)
target = SdkTarget.WSE3
platform = get_platform(cmaddr, config, target)
layout = SdkLayout(platform)

# Create a code region
code = layout.create_code_region('./pe_program.csl', 'my_kernel', width, height)
code.set_param_all('param_name', value)

# Compile
artifacts = layout.compile(out_prefix='out')

# Run
runtime = SdkRuntime(artifacts, platform, memcpy_required=False)
runtime.load()
runtime.run()
runtime.stop()

# Read results
result = runtime.read_symbol(pe_x, pe_y, 'symbol_name', dtype='uint16')
```

## Compilation Flags

```bash
cslc --arch=wse3 layout.csl \
    --fabric-dims=DX,DY \          # Total fabric dimensions
    --fabric-offsets=OX,OY \       # Kernel offset within fabric
    --memcpy \                     # Enable memcpy infrastructure
    --channels=K \                 # I/O channels (1-16)
    --params=PARAM1:VAL1,PARAM2:VAL2 \  # Compile-time parameters
    --width-west-buf=0 \           # Optional prefetch buffers
    --width-east-buf=0 \
    -o output_dir
```

**Constraints:**
- `fabric_dims_x >= 7 + kernel_width`
- `fabric_dims_y >= 2 + kernel_height`
- `fabric_offset_x >= 4`
- `fabric_offset_y >= 1`

## Command Stream

After any PE computation completes, you MUST call:

```csl
sys_mod.unblock_cmd_stream();
```

This tells the memcpy infrastructure that the kernel is done and the next
memcpy or launch command can proceed.
