# Comptime and Layout Configuration

> Source: Cerebras SDK 1.4.0

## Comptime Blocks

`comptime` blocks execute at compile time. They are used for:
1. Binding tasks to task IDs
2. Configuring colors and routing
3. Exporting symbols
4. Activating initial tasks
5. Initializing queues (WSE-3)

```csl
comptime {
    // Task binding
    @bind_local_task(my_fn, my_local_task_id);
    @bind_data_task(my_wtt, my_data_task_id);
    @bind_control_task(my_ctrl, my_ctrl_task_id);

    // Symbol export
    @export_symbol(ptr, "name");
    @export_symbol(function_name);

    // Task activation (runs at program start)
    @activate(main_task_id);

    // WSE-3 queue initialization
    if (@is_arch("wse3")) {
        @initialize_queue(my_iq, .{ .color = my_color });
        @initialize_queue(my_oq, .{ .color = my_color });
    }
}
```

## Layout Programs

The layout block defines the PE mesh configuration. It is the top-level entry point
for a CSL program.

### Basic Layout

```csl
// layout.csl — Top-level program

// Import memcpy parameters
const memcpy = @import_module("<memcpy/get_params>", .{
    .width = 10,
    .height = 10
});

// Define colors
const query_color: color = @get_color(0);
const result_color: color = @get_color(1);

layout {
    // Define the PE rectangle
    @set_rectangle(10, 10);

    // Assign programs to PEs
    for (var y: u16 = 0; y < 10; y += 1) {
        for (var x: u16 = 0; x < 10; x += 1) {
            @set_tile_code(x, y, "pe_program.csl", .{
                .memcpy_params = memcpy.get_params(x),
                .pe_x = x,
                .pe_y = y,
                .query_color = query_color,
                .result_color = result_color,
            });

            // Configure color routing for this PE
            @set_color_config(x, y, query_color, .{
                .routes = .{ .rx = .{ NORTH }, .tx = .{ RAMP, SOUTH } }
            });
        }
    }

    // Export symbols (must match PE program exports)
    @export_name("result", [*]f32, false);
    @export_name("f_run", fn()void);
}
```

### Key Layout Builtins

| Builtin | Purpose |
|---------|---------|
| `@set_rectangle(w, h)` | Define the PE grid dimensions |
| `@set_tile_code(x, y, file, params)` | Assign a CSL program + params to a PE |
| `@set_color_config(x, y, color, config)` | Configure color routing for a PE |
| `@export_name(name, type, writable)` | Export a named symbol for host access |
| `@get_color(id)` | Get a color by numeric ID |

### Comptime Parameters

Parameters flow from layout → PE program at compile time:

```csl
// layout.csl
@set_tile_code(x, y, "pe_program.csl", .{
    .pe_x = x,
    .pe_y = y,
    .tile_id = y * width + x,
    .query_color = my_color,
    .memcpy_params = memcpy.get_params(x),
});

// pe_program.csl
param pe_x: u16;
param pe_y: u16;
param tile_id: u32;
param query_color: color;
param memcpy_params: comptime_struct;
```

### Architecture Detection

```csl
// Compile-time architecture check
if (@is_arch("wse2")) {
    // WSE-2 specific code
    const task_id = @get_data_task_id(some_color);
} else if (@is_arch("wse3")) {
    // WSE-3 specific code
    const task_id = @get_data_task_id(some_input_queue);
}
```

## Layout Patterns for iDB

### Data PE Grid with Ingress/Egress/Reducer Roles

```csl
const GRID_W: u16 = 750;
const GRID_H: u16 = 994;

const QUERY_COLOR: color = @get_color(0);
const RESULT_COLOR: color = @get_color(1);

layout {
    @set_rectangle(GRID_W, GRID_H);

    for (var y: u16 = 0; y < GRID_H; y += 1) {
        for (var x: u16 = 0; x < GRID_W; x += 1) {

            if (y == 0) {
                // Row 0: Ingress PEs
                @set_tile_code(x, y, "ingress.csl", .{
                    .memcpy_params = memcpy.get_params(x),
                });
                @set_color_config(x, y, QUERY_COLOR, .{
                    .routes = .{ .rx = .{ RAMP }, .tx = .{ SOUTH } }
                });

            } else if (y == GRID_H - 1) {
                // Last row: Egress PEs
                @set_tile_code(x, y, "egress.csl", .{
                    .memcpy_params = memcpy.get_params(x),
                });

            } else if (x == GRID_W - 1) {
                // Rightmost column: Reducer PEs
                @set_tile_code(x, y, "reducer.csl", .{
                    .memcpy_params = memcpy.get_params(x),
                    .pe_y = y,
                });
                @set_color_config(x, y, RESULT_COLOR, .{
                    .routes = .{ .rx = .{ WEST }, .tx = .{ RAMP, SOUTH } }
                });

            } else {
                // Data PEs (the vast majority)
                @set_tile_code(x, y, "data_pe.csl", .{
                    .memcpy_params = memcpy.get_params(x),
                    .pe_x = x,
                    .pe_y = y,
                    .tile_id = (y - 1) * (GRID_W - 1) + x,
                    .query_color = QUERY_COLOR,
                    .result_color = RESULT_COLOR,
                });

                // Query: receive from north, forward south + deliver locally
                @set_color_config(x, y, QUERY_COLOR, .{
                    .routes = .{ .rx = .{ NORTH }, .tx = .{ RAMP, SOUTH } }
                });

                // Results: send east toward reducer column
                @set_color_config(x, y, RESULT_COLOR, .{
                    .routes = .{ .rx = .{ RAMP }, .tx = .{ EAST } }
                });
            }
        }
    }
}
```

### Comptime Color Map

```
Color Map for iDB:

  ID  Purpose
  ─────────────────────────────
   0  QUERY_COLOR     (broadcast queries south)
   1  RESULT_COLOR    (collect results east)
   2  CONTROL_COLOR   (phase control, sync)
   3  KNN_QUERY       (vector search broadcast)
   4  KNN_RESULT      (KNN results collection)
   5  AGGREGATE       (partial aggregation)
   ...
  21  reserved (memcpy)
  22  reserved (memcpy)
  23  reserved (memcpy)
```
