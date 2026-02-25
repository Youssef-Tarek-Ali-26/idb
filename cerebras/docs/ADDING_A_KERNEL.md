# Adding a New Kernel to iDB

Step-by-step guide for adding a new query type or operation to iDB.

## Overview

Adding a kernel requires changes in four layers:
1. **CSL kernel** — The PE-level computation
2. **Python host code** — Dispatching and result collection
3. **Rust executor** — Integration with the query engine
4. **Tests** — Simulator-based correctness verification

## Step 1: Define the Operation

Before writing code, document:
- **Input format**: What data does the kernel receive? (wavelet protocol)
- **Output format**: What does the kernel produce?
- **Algorithm**: Step-by-step logic on each PE
- **Routing pattern**: Which existing pattern (broadcast, point, etc.) does it use?
- **Performance target**: Expected cycles per record

## Step 2: Write the CSL Kernel

Create `cerebras/kernels/<kernel_name>.csl`:

```csl
// Template for a new iDB kernel
// cerebras/kernels/my_new_op.csl

// Import common types and utilities
const types = @import_module("common/types.csl");
const hilbert = @import_module("common/hilbert.csl");
const predicate = @import_module("common/predicate.csl");

// Parameters from layout
param memcpy_params: comptime_struct;
param pe_x: u16;
param pe_y: u16;
param tile_id: u32;
param query_color: color;
param result_color: color;

const sys_mod = @import_module("<memcpy/memcpy>", memcpy_params);

// Queues (WSE-3)
const query_iq: input_queue = @get_input_queue(2);
const result_oq: output_queue = @get_output_queue(3);

// Tile data (loaded by data_load kernel)
var tile_meta: types.TileMetadata = undefined;
var learned_idx: types.LearnedIndex = undefined;
var records: [types.MAX_RECORDS]types.Record = undefined;
var record_count: u16 = 0;

// Task IDs
const query_task_id: data_task_id =
    if (@is_arch("wse2")) @get_data_task_id(query_color)
    else if (@is_arch("wse3")) @get_data_task_id(query_iq);

// Result output DSD
const result_dsd = @get_dsd(fabout_dsd, .{
    .extent = 1,
    .fabric_color = result_color,
    .output_queue = result_oq
});

// ─── YOUR KERNEL LOGIC ───

task handle_query(wavelet_data: u32) void {
    // 1. Decode the query wavelet
    var op = @as(u8, wavelet_data & 0xFF);
    // ... decode other fields

    // 2. Process local records
    for (var i: u16 = 0; i < record_count; i += 1) {
        // Your per-record logic here
        // if (matches_criteria(records[i])) {
        //     @fmovs(result_color, records[i].id);
        // }
    }
}

// ─── BINDING ───

comptime {
    @bind_data_task(handle_query, query_task_id);

    if (@is_arch("wse3")) {
        @initialize_queue(query_iq, .{ .color = query_color });
        @initialize_queue(result_oq, .{ .color = result_color });
    }
}
```

## Step 3: Update the Layout

Add the new kernel to `cerebras/kernels/layout.csl` or update the data_pe.csl
to handle the new operation code in its query dispatch:

```csl
// In data_pe.csl, extend the query handler
task handle_query(wavelet_data: u32) void {
    var op = @as(u8, wavelet_data & 0xFF);

    switch (op) {
        0 => spatial_scan(wavelet_data),
        1 => point_lookup(wavelet_data),
        2 => range_scan(wavelet_data),
        3 => knn_search(wavelet_data),
        4 => aggregate(wavelet_data),
        5 => topk(wavelet_data),
        6 => my_new_op(wavelet_data),      // ← Add new case
        else => {},
    }
}
```

## Step 4: Write Python Host Code

Add dispatch and result collection to `cerebras/host/controller.py`:

```python
def execute_my_new_op(self, params: dict):
    """Execute the new operation across all PEs."""

    # 1. Encode query as wavelet data
    query_wavelet = self._encode_query(
        op=6,                    # New op code
        field=params['field'],
        pred_op=params['pred_op'],
    )

    # 2. Send additional data if needed (multi-wavelet)
    if 'extra_data' in params:
        extra_wavelets = self._pack_extra_data(params['extra_data'])

    # 3. Send to ingress PEs
    self.runtime.launch("recv_query", query_wavelet)

    # 4. Collect results from egress
    results = self.runtime.memcpy_d2h(
        self.egress_pes,
        self.result_buffer_size
    )

    # 5. Decode and return
    return self._decode_results(results)
```

## Step 5: Write Rust Executor Integration

In `crates/norndb-executor-cerebras/src/lib.rs`:

```rust
impl QueryExecutor for CerebrasExecutor {
    fn execute(&self, plan: &PhysicalPlan, tenant: TenantId) -> Result<RecordStream> {
        match plan {
            // ... existing cases
            PhysicalPlan::MyNewOp { params } => {
                let py_result = self.host_controller.execute_my_new_op(params)?;
                Ok(self.arrow_from_cerebras(py_result))
            }
        }
    }
}
```

## Step 6: Write Tests

Create `cerebras/simulator/test_my_new_op.py`:

```python
import numpy as np
from cerebras.sdk.runtime.sdkruntimepybind import SdkRuntime

def test_my_new_op_basic():
    """Test basic functionality on small grid."""
    # 1. Set up small grid (e.g., 5x5)
    # 2. Load known test data
    # 3. Execute the new operation
    # 4. Verify results against Python reference implementation

    # Load test data
    records = generate_test_records(count=100)

    # Run on simulator
    runner = SdkRuntime(compiled_dir, cmaddr=None)
    runner.load()
    runner.run()
    # ... load data, execute query, collect results
    runner.stop()

    # Verify against reference
    expected = reference_implementation(records, query_params)
    np.testing.assert_equal(actual_results, expected)

def test_my_new_op_empty():
    """Test with no matching records."""
    # ...

def test_my_new_op_all_match():
    """Test where all records match."""
    # ...

def test_my_new_op_boundary():
    """Test boundary conditions."""
    # ...
```

## Step 7: Benchmark

Add to `cerebras/simulator/benchmarks.py`:

```python
def benchmark_my_new_op():
    """Measure performance on simulator."""
    # Time the operation with increasing data sizes
    # Compare with analytical performance model
    # Record results for the paper
```

## Step 8: Update Documentation

1. Add kernel to the table in `cerebras/docs/KERNEL_API.md`
2. Add performance estimates to `cerebras/docs/PERFORMANCE_MODEL.md`
3. Update `docs/csl-reference/code-patterns.md` if it introduces a new pattern

## Checklist

- [ ] CSL kernel written and compiles
- [ ] Handles WSE-2 and WSE-3 differences (`@is_arch()`)
- [ ] Query wavelet protocol documented
- [ ] Python host dispatch code
- [ ] Rust executor integration
- [ ] Simulator tests passing
- [ ] Performance benchmark recorded
- [ ] Documentation updated
- [ ] Common types/utilities used (don't duplicate)
