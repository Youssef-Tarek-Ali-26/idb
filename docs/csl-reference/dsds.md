# Data Structure Descriptors (DSDs)

> Source: Cerebras SDK 1.4.0

## Overview

DSDs are CSL's mechanism for describing data access patterns. They decouple
*what data to access* from *what operation to perform*, enabling efficient
hardware-level data movement.

## DSD Types

### 1. Memory DSDs (`mem1d_dsd`)

Access data in PE local SRAM.

```csl
var buffer: [100]f32;

// Contiguous access (100 elements starting from &buffer)
const buf_dsd = @get_dsd(mem1d_dsd, .{
    .base_address = &buffer,
    .extent = 100
});

// Strided access via tensor_access expression
// Access every Nth element: buffer[0], buffer[N], buffer[2*N], ...
const strided_dsd = @get_dsd(mem1d_dsd, .{
    .tensor_access = |i|{M} -> buffer[i * N]
});

// 2D access pattern
var matrix: [10, 10]f32;
const col_dsd = @get_dsd(mem1d_dsd, .{
    .tensor_access = |i|{10} -> matrix[i * 10 + col_idx]
});
```

**tensor_access syntax**: `|loop_var|{extent} -> array[index_expr]`
- `loop_var`: Iterator variable
- `extent`: Number of iterations
- `index_expr`: Expression computing the memory address

### 2. Fabric Output DSDs (`fabout_dsd`)

Send wavelets to the fabric (to other PEs or host).

```csl
const out_dsd = @get_dsd(fabout_dsd, .{
    .extent = num_elements,      // How many wavelets to send
    .fabric_color = my_color,    // Which color to send on
    .output_queue = my_oq        // Output queue (WSE-3 required)
});
```

### 3. Fabric Input DSDs (`fabin_dsd`)

Receive wavelets from the fabric.

```csl
const in_dsd = @get_dsd(fabin_dsd, .{
    .extent = num_elements,      // How many wavelets to receive
    .fabric_color = my_color,    // Which color to receive on
    .input_queue = my_iq         // Input queue (WSE-3 required)
});
```

### 4. FIFO DSDs

Buffer data between producers and consumers.

```csl
var fifo_buffer = @zeros([1024]f32);
const fifo = @allocate_fifo(fifo_buffer);

// Write to FIFO (from fabric input)
@fadds(fifo, in_dsd, constant_dsd, .{ .async = true });

// Read from FIFO (to fabric output or memory)
@fnegs(out_dsd, fifo, .{ .async = true });
```

**Key constraint**: FIFO input size cannot exceed buffer size with serialized memcpy.

## DSD Operations

### Move Operations

```csl
// 16-bit move: dest = src
@mov16(dest_dsd, src_dsd);
@mov16(dest_dsd, src_dsd, .{ .async = true, .unblock = task_id });

// 32-bit move
@mov32(dest_dsd, src_dsd);

// Single value move to fabric
@fmovs(color, value);  // Send one 32-bit wavelet on color
```

### Arithmetic Operations

```csl
// Add: dest = src1 + src2
@fadds(dest_dsd, src1_dsd, src2_dsd);

// Multiply-accumulate: dest += src1 * src2 (scalar)
@fmacs(dest_dsd, src1_dsd, src2_dsd, scalar);

// Negate: dest = -src
@fnegs(dest_dsd, src_dsd);

// Half-precision operations
@faddh(dest_dsd, src1_dsd, src2_dsd);  // f16 add
@fmach(dest_dsd, src1_dsd, src2_dsd);  // f16 mac

// 16-bit add
@add16(dest_dsd, src1_dsd, src2_dsd);
```

### Map Operations

Custom operations over DSDs using callbacks:

```csl
// Apply a custom function element-wise
@map(callback_fn, dest_dsd, src_dsd);
```

## Modifying DSDs

### Offset Adjustment

```csl
// Shift a DSD to start at a different position
// Useful for processing data in chunks
var dsd = @get_dsd(mem1d_dsd, .{ .base_address = &buffer, .extent = 10 });

// After processing first 10 elements, shift to next 10
dsd = @increment_dsd_offset(dsd, 10, f32);
```

## Async Operations

All DSD operations can run asynchronously:

```csl
@fadds(dest_dsd, src1_dsd, src2_dsd, .{
    .async = true,           // Don't block the CE
    .unblock = task_id,      // Unblock this task when done
    .activate = next_task,   // Activate this task when done
});
```

**Flow control pattern:**

```csl
task process_data(data: u32) void {
    // Block self to prevent re-entry
    @block(my_task_id);

    buf[0] = data;

    // Async operation: forward data, then unblock self
    @mov16(out_dsd, buf_dsd, .{
        .async = true,
        .unblock = my_task_id
    });
}
```

## DSDs for iDB

### Tile Data Access

```csl
// Access record data starting at offset 0x0580
var records: [700]Record;  // at fixed SRAM offset

// Sequential scan of all records
const records_dsd = @get_dsd(mem1d_dsd, .{
    .base_address = &records,
    .extent = record_count
});

// Access specific field across all records (columnar pattern)
// e.g., access price field at offset 8 in each 64-byte record
const price_dsd = @get_dsd(mem1d_dsd, .{
    .tensor_access = |i|{record_count} -> records[i * RECORD_SIZE + PRICE_OFFSET]
});
```

### Result Streaming

```csl
// Stream matching record IDs east to reducer
const result_dsd = @get_dsd(fabout_dsd, .{
    .extent = match_count,
    .fabric_color = result_color,
    .output_queue = result_oq
});
```
