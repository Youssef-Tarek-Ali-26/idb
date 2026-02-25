# CSL Libraries and Builtins

> Source: Cerebras SDK 1.4.0

## Core Builtins

### Task Management

| Builtin | Description |
|---------|-------------|
| `@activate(local_task_id)` | Activate a local task |
| `@block(task_id)` | Block a task from being scheduled |
| `@unblock(task_id)` | Unblock a previously blocked task |
| `@bind_local_task(fn, id)` | Bind a function to a local task ID |
| `@bind_data_task(fn, id)` | Bind a function to a data task ID |
| `@bind_control_task(fn, id)` | Bind a function to a control task ID |
| `@get_local_task_id(n)` | Create a local task ID from number n |
| `@get_data_task_id(color_or_queue)` | Create a data task ID |
| `@get_control_task_id(n)` | Create a control task ID |

### Color and Queue Management

| Builtin | Description |
|---------|-------------|
| `@get_color(n)` | Get color by numeric ID |
| `@get_input_queue(n)` | Get input queue by ID (WSE-3) |
| `@get_output_queue(n)` | Get output queue by ID (WSE-3) |
| `@initialize_queue(queue, config)` | Initialize a queue (WSE-3) |
| `@set_color_config(x, y, color, config)` | Configure color routing (layout) |

### DSD Operations

| Builtin | Description |
|---------|-------------|
| `@get_dsd(type, config)` | Create a DSD |
| `@increment_dsd_offset(dsd, offset, type)` | Shift DSD base address |
| `@allocate_fifo(buffer)` | Allocate a FIFO from a buffer |

### Data Movement

| Builtin | Description |
|---------|-------------|
| `@mov16(dest, src, [opts])` | 16-bit move |
| `@mov32(dest, src, [opts])` | 32-bit move |
| `@fmovs(color, value)` | Send single wavelet |
| `@fadds(dest, src1, src2, [opts])` | Float add |
| `@fmacs(dest, src1, src2, scalar, [opts])` | Float multiply-accumulate |
| `@fnegs(dest, src, [opts])` | Float negate |
| `@faddh(dest, src1, src2, [opts])` | f16 add |
| `@fmach(dest, src1, src2, [opts])` | f16 MAC |
| `@add16(dest, src1, src2, [opts])` | 16-bit integer add |
| `@map(callback, dest, src)` | Apply custom function over DSDs |

### Type and Memory

| Builtin | Description |
|---------|-------------|
| `@as(type, value)` | Type cast |
| `@bitcast(type, value)` | Reinterpret bits as different type |
| `@zeros([N]T)` | Zero-initialized array |
| `@is_arch(arch_string)` | Check target architecture |

### Layout

| Builtin | Description |
|---------|-------------|
| `@set_rectangle(w, h)` | Define PE grid |
| `@set_tile_code(x, y, file, params)` | Assign program to PE |
| `@export_symbol(sym, [name])` | Export symbol for host |
| `@export_name(name, type, writable)` | Named export (layout) |

### Module System

| Builtin | Description |
|---------|-------------|
| `@import_module(path, [params])` | Import a CSL module |

## Standard Libraries

### Math (`<math>`)

```csl
const math = @import_module("<math>");

var result = math.sqrt(x);      // Square root
// Additional: sin, cos, exp, log, abs, etc.
```

### Random (`<random>`)

```csl
const random = @import_module("<random>");

var rand_val = random.random_u16();   // Random 16-bit unsigned
var rand_f32 = random.random_f32();   // Random float [0, 1)
```

### Debug (`<debug>`)

```csl
const debug = @import_module("<debug>");

// Simulator-only debugging
debug.trace_i32(value);
debug.trace_f32(value);
```

### Simprint (`<simprint>`)

```csl
const simprint = @import_module("<simprint>");

// Print to simulator console (not available on hardware)
simprint.print_i32(value);
```

### Timestamp (`<timestamp>`)

```csl
const timestamp = @import_module("<timestamp>");

var start = timestamp.get_timestamp();
// ... do work ...
var end = timestamp.get_timestamp();
var cycles = end - start;
```

### Collectives (`<collectives_2d>`)

MPI-like collective operations across PE grid:

```csl
const collectives = @import_module("<collectives_2d>", .{
    .width = width,
    .height = height,
    // ... color assignments for rows/columns
});

// Broadcast from one PE to all in row
collectives.broadcast(data_dsd, root_pe_x);

// Scatter data from root to all PEs
collectives.scatter(recv_dsd, send_dsd, root_pe_x);

// Gather data from all PEs to root
collectives.gather(recv_dsd, send_dsd, root_pe_x);

// Reduce (sum) across a row
collectives.reduce_fadds(result_dsd, data_dsd, root_pe_x);
```

### Memcpy (`<memcpy/memcpy>`)

```csl
const sys_mod = @import_module("<memcpy/memcpy>", memcpy_params);

// Signal kernel completion to host
sys_mod.unblock_cmd_stream();

// System streaming colors
sys_mod.MEMCPYH2D_1  // Host-to-device streaming color 1
sys_mod.MEMCPYD2H_1  // Device-to-host streaming color 1
```

### Layout Module (`<layout>`)

Access PE coordinates at runtime:

```csl
const layout_mod = @import_module("<layout>");

// Get this PE's coordinates (set at compile time)
var my_x = layout_mod.get_x_coord();
var my_y = layout_mod.get_y_coord();
```

## WSE-3 Specific Libraries

### Microthreads

WSE-3 supports explicit microthread management:

```csl
// Get microthread ID
const mt_id = @get_microthread_id(n);

// Microthread-specific builtins
@set_microthread(mt_id);
```

## Async Operation Options

All DSD operations accept an options struct:

```csl
@fadds(dest, src1, src2, .{
    .async = true,              // Non-blocking
    .unblock = task_id,         // Unblock task on completion
    .activate = task_id,        // Activate task on completion
    // .microthread = mt_id,    // WSE-3: specify microthread
});
```
