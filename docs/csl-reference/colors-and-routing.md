# Colors and Routing

> Source: Cerebras SDK 1.4.0

## Colors

Colors are the fundamental routing mechanism on the WSE. Each wavelet carries a 5-bit
color tag that determines both its routing path through the fabric and what task it
activates upon arrival.

### Color Allocation

| Color IDs | Purpose |
|-----------|---------|
| 0-20 | User-available routable colors |
| 21-23 | Reserved by memcpy infrastructure |
| 24+ | Non-routable (control tasks only) |

```csl
// In layout file: allocate a color
const my_color: color = @get_color(0);

// In PE program: receive as parameter
param my_color: color;
```

### Color Configuration

Each PE can independently configure how each color routes wavelets:

```csl
// In layout file
@set_color_config(pe_x, pe_y, my_color, .{
    .routes = .{
        .rx = .{ WEST },      // Receive from west
        .tx = .{ RAMP, EAST } // Send to local CE and east
    }
});
```

### Route Directions

- `RAMP` — To/from the local Compute Engine
- `NORTH` — To PE at (x, y-1)
- `SOUTH` — To PE at (x, y+1)
- `EAST` — To PE at (x+1, y)
- `WEST` — To PE at (x-1, y)

## Routing Patterns

### 1. Broadcast (West → East)

Send a wavelet to all PEs in a row:

```csl
// Layout: configure each PE in the row
for (var x: u16 = 0; x < width; x += 1) {
    if (x == 0) {
        // First PE: receive from ramp, send east
        @set_color_config(x, y, color, .{
            .routes = .{ .rx = .{ RAMP }, .tx = .{ RAMP, EAST } }
        });
    } else if (x == width - 1) {
        // Last PE: receive from west, no forward
        @set_color_config(x, y, color, .{
            .routes = .{ .rx = .{ WEST }, .tx = .{ RAMP } }
        });
    } else {
        // Middle PEs: receive from west, forward east + deliver locally
        @set_color_config(x, y, color, .{
            .routes = .{ .rx = .{ WEST }, .tx = .{ RAMP, EAST } }
        });
    }
}
```

### 2. Column Reduction (North → South)

Collect results from a column of PEs:

```csl
// Each PE sends results south
for (var y: u16 = 0; y < height; y += 1) {
    if (y == height - 1) {
        // Bottom PE: receive from ramp and north
        @set_color_config(x, y, result_color, .{
            .routes = .{ .rx = .{ RAMP, NORTH }, .tx = .{ RAMP } }
        });
    } else {
        // Other PEs: send south
        @set_color_config(x, y, result_color, .{
            .routes = .{ .rx = .{ RAMP }, .tx = .{ SOUTH } }
        });
    }
}
```

### 3. Row Reduction (West → East)

```csl
// Each PE sends results east; rightmost column collects
@set_color_config(x, y, result_color, .{
    .routes = .{ .rx = .{ RAMP }, .tx = .{ EAST } }
});

// Rightmost column (reducer)
@set_color_config(width - 1, y, result_color, .{
    .routes = .{ .rx = .{ WEST }, .tx = .{ RAMP } }
});
```

### 4. Point-to-Point Routing

Route a wavelet to a specific PE by configuring intermediate PEs to forward:

```csl
// Route from (0,0) to (3,2): go east 3 hops, then south 2 hops
// PE (0,0) to (2,0): forward east
// PE (3,0) to (3,1): forward south
// PE (3,2): deliver to ramp
```

## Fabric Switches

Switches allow dynamic routing changes at runtime. A PE can change which direction
a color transmits to based on control wavelets.

```csl
const sender_routes = .{
    .rx = .{ RAMP },
    .tx = .{ NORTH }  // Default: send north
};

const sender_switches = .{
    .pos1 = .{ .tx = WEST },    // After 1st control: send west
    .pos2 = .{ .tx = EAST },    // After 2nd control: send east
    .pos3 = .{ .tx = SOUTH },   // After 3rd control: send south
    .current_switch_pos = 1,     // Start at position 1
    .ring_mode = true,           // Wrap around after pos3
};

@set_color_config(x, y, channel, .{
    .routes = sender_routes,
    .switches = sender_switches
});
```

Control wavelets are sent as: `(5 << 22) | (4 << 25)` encoding to trigger switch changes.

## Color Swap

When two colors differ only in their lowest bit, enabling `color_swap_x` causes
alternating color transformation as wavelets travel east:

```csl
// Red (color 0) and Blue (color 1) swap on each hop east
const routes = .{
    .rx = .{ WEST },
    .tx = .{ RAMP, EAST },
    .color_swap_x = true
};
```

This means:
- PE 0 receives red → PE 1 receives blue → PE 2 receives red → ...
- Useful for alternating computation patterns

## Sending Wavelets

### Using Fabric Output DSDs

```csl
const out_dsd = @get_dsd(fabout_dsd, .{
    .extent = 1,
    .fabric_color = my_color,
    .output_queue = my_oq
});

// Send a single value
@mov16(out_dsd, value);

// Send asynchronously
@mov16(out_dsd, buf_dsd, .{ .async = true, .unblock = task_id });
```

### Using @fmovs (Single Wavelet)

```csl
// Send a single 32-bit value on a color
@fmovs(my_color, value);
```

## Receiving Wavelets

### Via Data Tasks

```csl
task recv_data(wavelet_data: u32) void {
    // Process received wavelet
    buffer[write_idx] = wavelet_data;
    write_idx += 1;
}

comptime {
    @bind_data_task(recv_data, my_data_task_id);
}
```

### Via Fabric Input DSDs

```csl
const in_dsd = @get_dsd(fabin_dsd, .{
    .extent = num_elements,
    .fabric_color = my_color,
    .input_queue = my_iq
});

// Receive into memory
@mov16(mem_dsd, in_dsd, .{ .async = true, .activate = process_task_id });
```

## Queue Initialization (WSE-3)

On WSE-3, input and output queues must be explicitly initialized:

```csl
const h2d_iq: input_queue = @get_input_queue(2);
const d2h_oq: output_queue = @get_output_queue(3);

comptime {
    @initialize_queue(h2d_iq, .{ .color = sys_mod.MEMCPYH2D_1 });
    @initialize_queue(d2h_oq, .{ .color = sys_mod.MEMCPYD2H_1 });
}
```

## Routing for iDB Patterns

### Query Broadcast (Ingress → All Data PEs)

```
Row 0 (Ingress): receive from host, broadcast south
  ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓
Data PEs: receive from north, forward south + deliver to ramp
  ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓
  ... (992 rows of data PEs)
```

### Result Collection (Data PEs → Reducer → Egress)

```
Data PEs → send results EAST →
  → → → → → → → Column 749 (Reducer PEs)
                      ↓
                  send results SOUTH
                      ↓
                  Row 993 (Egress) → back to host
```
