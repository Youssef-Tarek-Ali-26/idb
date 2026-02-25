# CSL Code Patterns

Annotated examples of common CSL patterns, compiled from the Cerebras SDK examples
repository (https://github.com/Cerebras/sdk-examples).

## Pattern 1: Minimal Complete Program

The simplest complete CSL program — single PE, set a variable, read from host.

### layout.csl (SdkLayout approach)
```python
# run.py — using SdkLayout (beta API)
from cerebras.sdk.runtime.sdkruntimepybind import (
    SdkRuntime, SdkTarget, SdkLayout, SimfabConfig, get_platform
)

value = 550
config = SimfabConfig(dump_core=True)
target = SdkTarget.WSE3
platform = get_platform(cmaddr, config, target)
layout = SdkLayout(platform)

# Create 1x1 code region
code = layout.create_code_region('./gv.csl', 'gv', 1, 1)
code.set_param_all('value', value)

artifacts = layout.compile(out_prefix='out')
runtime = SdkRuntime(artifacts, platform, memcpy_required=False)
runtime.load()
runtime.run()
runtime.stop()

result = runtime.read_symbol(0, 0, 'gv', dtype='uint16')
assert result == [value]
```

### gv.csl
```csl
param value: i16;
export var gv: i16;

const main_id = @get_local_task_id(8);
task main() void {
    gv = value;
}

comptime {
    @bind_local_task(main, main_id);
    @activate(main_id);
}
```

## Pattern 2: Arrays and Pointers

Working with arrays, pointers, and function calls.

```csl
param memcpy_params: comptime_struct;
const sys_mod = @import_module("<memcpy/memcpy>", memcpy_params);

var result: [1]i16;
var result_ptr: [*]i16 = &result;

fn increment_and_sum(data_ptr: *[3]i16, result_ptr: *i16) void {
    (data_ptr.*)[0] += 1;
    (data_ptr.*)[1] += 1;
    (data_ptr.*)[2] += 1;
    result_ptr.* = (data_ptr.*)[0] + (data_ptr.*)[1] + (data_ptr.*)[2];
}

fn f_run() void {
    var data = [3]i16 { 1, 2, 3 };
    increment_and_sum(&data, &result[0]);
    sys_mod.unblock_cmd_stream();
}

comptime {
    @export_symbol(result_ptr, "result");
    @export_symbol(f_run);
}
```

## Pattern 3: Streaming Wavelet Data (Data Task)

Receive data from host via streaming, process it, send results back.

```csl
param memcpy_params: comptime_struct;
const sys_mod = @import_module("<memcpy/memcpy>", memcpy_params);

const h2d_iq: input_queue = @get_input_queue(2);
const d2h_oq: output_queue = @get_output_queue(3);

const main_task_id: data_task_id =
    if (@is_arch("wse2")) @get_data_task_id(sys_mod.MEMCPYH2D_1)
    else if (@is_arch("wse3")) @get_data_task_id(h2d_iq);

export var global: i16 = 0;

const out_dsd = @get_dsd(fabout_dsd, .{
    .extent = 1,
    .fabric_color = sys_mod.MEMCPYD2H_1,
    .output_queue = d2h_oq
});

// Wavelet-triggered task: fires when H2D data arrives
task main_task(wavelet_data: i16) void {
    global = wavelet_data;
    @mov16(out_dsd, global);
}

comptime {
    @bind_data_task(main_task, main_task_id);
    if (@is_arch("wse3")) {
        @initialize_queue(h2d_iq, .{ .color = sys_mod.MEMCPYH2D_1 });
        @initialize_queue(d2h_oq, .{ .color = sys_mod.MEMCPYD2H_1 });
    }
}
```

## Pattern 4: Multi-PE with Inter-PE Communication

Two PEs: left computes and sends EAST, right receives from WEST.

### layout.csl
```csl
const send_color: color = @get_color(0);

const memcpy = @import_module("<memcpy/get_params>", .{
    .width = 2, .height = 1
});

layout {
    @set_rectangle(2, 1);

    // Left PE: sends east
    @set_tile_code(0, 0, "sender.csl", .{
        .memcpy_params = memcpy.get_params(0),
        .send_color = send_color,
    });
    @set_color_config(0, 0, send_color, .{
        .routes = .{ .rx = .{ RAMP }, .tx = .{ EAST } }
    });

    // Right PE: receives from west
    @set_tile_code(1, 0, "receiver.csl", .{
        .memcpy_params = memcpy.get_params(1),
        .recv_color = send_color,
    });
    @set_color_config(1, 0, send_color, .{
        .routes = .{ .rx = .{ WEST }, .tx = .{ RAMP } }
    });

    @export_name("result", [*]f32, false);
    @export_name("f_send", fn()void);
}
```

### sender.csl
```csl
param memcpy_params: comptime_struct;
param send_color: color;

const sys_mod = @import_module("<memcpy/memcpy>", memcpy_params);

var data: [4]f32 = .{ 1.0, 2.0, 3.0, 4.0 };
const send_oq: output_queue = @get_output_queue(2);

const out_dsd = @get_dsd(fabout_dsd, .{
    .extent = 4,
    .fabric_color = send_color,
    .output_queue = send_oq
});

const data_dsd = @get_dsd(mem1d_dsd, .{
    .base_address = &data,
    .extent = 4
});

fn f_send() void {
    @mov32(out_dsd, data_dsd, .{ .async = true });
    sys_mod.unblock_cmd_stream();
}

comptime {
    @export_symbol(f_send);
    if (@is_arch("wse3")) {
        @initialize_queue(send_oq, .{ .color = send_color });
    }
}
```

### receiver.csl
```csl
param memcpy_params: comptime_struct;
param recv_color: color;

const sys_mod = @import_module("<memcpy/memcpy>", memcpy_params);

var result: [4]f32;
var result_ptr: [*]f32 = &result;
const recv_iq: input_queue = @get_input_queue(2);

const recv_task_id: data_task_id =
    if (@is_arch("wse2")) @get_data_task_id(recv_color)
    else if (@is_arch("wse3")) @get_data_task_id(recv_iq);

var idx: u16 = 0;

task recv_task(data: f32) void {
    result[idx] = data;
    idx += 1;
}

comptime {
    @bind_data_task(recv_task, recv_task_id);
    @export_symbol(result_ptr, "result");
    if (@is_arch("wse3")) {
        @initialize_queue(recv_iq, .{ .color = recv_color });
    }
}
```

## Pattern 5: Switches for Dynamic Routing

Center PE sends to 4 neighbors by cycling through switch positions.

### layout.csl (relevant section)
```csl
const channel: color = @get_color(0);

// Sender PE at center (1,1) of 3x3 grid
@set_tile_code(1, 1, "send.csl", .{
    .memcpy_params = memcpy_params_1,
    .tx_color = channel,
});

const sender_routes = .{
    .rx = .{ RAMP },
    .tx = .{ NORTH }  // Default direction
};

const sender_switches = .{
    .pos1 = .{ .tx = WEST },
    .pos2 = .{ .tx = EAST },
    .pos3 = .{ .tx = SOUTH },
    .current_switch_pos = 1,
    .ring_mode = true,
};

@set_color_config(1, 1, channel, .{
    .routes = sender_routes,
    .switches = sender_switches
});

// Receiver PEs at (1,0), (0,1), (2,1), (1,2)
@set_tile_code(1, 0, "recv.csl", .{ ... });
@set_color_config(1, 0, channel, .{
    .routes = .{ .rx = .{ SOUTH }, .tx = .{ RAMP } }
});
// ... similar for other 3 receivers
```

## Pattern 6: Color Swap (Alternating Tasks)

Red and blue colors swap on each eastward hop:

```csl
param memcpy_params: comptime_struct;
const sys_mod = @import_module("<memcpy/memcpy>", memcpy_params);

param red: color;
param blue: color;

const blue_oq: output_queue = @get_output_queue(2);
const h2d_task_id: data_task_id = @get_data_task_id(sys_mod.MEMCPYH2D_1);
const red_task_id: data_task_id = @get_data_task_id(red);
const blue_task_id: data_task_id = @get_data_task_id(blue);

var sum = @zeros([1]u32);
var ptr_sum: [*]u32 = &sum;

// Different tasks for different colors
task red_task(in_data: u32) void {
    sum[0] += in_data;           // Red: add 1x
}

task blue_task(in_data: u32) void {
    sum[0] += in_data * 2;       // Blue: add 2x
}

// Forward H2D data into the color-swap chain
var buf = @zeros([1]u32);
const buf_dsd = @get_dsd(mem1d_dsd, .{ .tensor_access = |i|{1} -> buf[i] });
const out_dsd = @get_dsd(fabout_dsd, .{
    .extent = 1, .fabric_color = blue, .output_queue = blue_oq
});

task wtt_h2d(data: u32) void {
    @block(h2d_task_id);
    buf[0] = data;
    @mov16(out_dsd, buf_dsd, .{ .async = true, .unblock = h2d_task_id });
}

comptime {
    @bind_data_task(red_task, red_task_id);
    @bind_data_task(blue_task, blue_task_id);
    @bind_data_task(wtt_h2d, h2d_task_id);
    @export_symbol(ptr_sum, "sum");
}
```

## Pattern 7: Host Program (run.py)

Standard Python host program structure:

```python
import argparse
import numpy as np
from cerebras.sdk.runtime.sdkruntimepybind import (
    SdkRuntime, MemcpyDataType, MemcpyOrder
)

parser = argparse.ArgumentParser()
parser.add_argument('--name', help='Compiled output directory')
parser.add_argument('--cmaddr', help='IP:port for CS system')
args = parser.parse_args()

# Create runtime
runner = SdkRuntime(args.name, cmaddr=args.cmaddr)

# Get symbol IDs
data_sym = runner.get_id('data')
result_sym = runner.get_id('result')

# Load and run
runner.load()
runner.run()

# Transfer data to device
input_data = np.array([1.0, 2.0, 3.0, 4.0], dtype=np.float32)
runner.memcpy_h2d(
    data_sym, input_data,
    0, 0,                    # start PE (x, y)
    1, 1,                    # width, height in PEs
    4,                       # elements per PE
    streaming=False,
    data_type=MemcpyDataType.MEMCPY_32BIT,
    order=MemcpyOrder.ROW_MAJOR,
    nonblock=False
)

# Launch kernel
runner.launch('f_compute', nonblock=False)

# Read results
output = np.zeros(4, dtype=np.float32)
runner.memcpy_d2h(
    output, result_sym,
    0, 0, 1, 1, 4,
    streaming=False,
    data_type=MemcpyDataType.MEMCPY_32BIT,
    order=MemcpyOrder.ROW_MAJOR,
    nonblock=False
)

runner.stop()

# Verify
expected = np.array([2.0, 3.0, 4.0, 5.0], dtype=np.float32)
np.testing.assert_allclose(output, expected, atol=0.01)
print("SUCCESS!")
```

## Pattern 8: Cholesky-Style Complex Multi-PE (Benchmark Reference)

Key patterns from the Cholesky benchmark — relevant to iDB for complex
multi-phase computations:

1. **Role-based PE assignment**: Different CSL files for different PE roles
   (diagonal, left-edge, interior, launcher)
2. **Phase-based execution**: Global iteration counter, tasks check
   `if (iter / Nt == my_column)` to determine active role
3. **Row + Column routing**: Two colors for orthogonal data flow
4. **Fabric switches**: Dynamic route changes between phases
5. **Continuation tasks**: `cont_task` activated after async receives complete,
   chains into next iteration
