# Routing Patterns for iDB

## Overview

iDB uses the WSE fabric mesh for parallel query execution. Different query types
require different routing patterns across the PE grid.

## PE Grid Layout

```
         Col 0    Col 1    ...   Col 748   Col 749
       ┌────────┬────────┬─────┬─────────┬──────────┐
Row 0  │Ingress │Ingress │ ... │ Ingress │ Ingress  │  ← Query entry
       ├────────┼────────┼─────┼─────────┼──────────┤
Row 1  │ Data   │ Data   │ ... │  Data   │ Reducer  │
       ├────────┼────────┼─────┼─────────┼──────────┤
Row 2  │ Data   │ Data   │ ... │  Data   │ Reducer  │
       ├────────┼────────┼─────┼─────────┼──────────┤
  ...  │  ...   │  ...   │ ... │   ...   │   ...    │
       ├────────┼────────┼─────┼─────────┼──────────┤
Row 992│ Data   │ Data   │ ... │  Data   │ Reducer  │
       ├────────┼────────┼─────┼─────────┼──────────┤
Row 993│Egress  │Egress  │ ... │ Egress  │ Egress   │  ← Result exit
       └────────┴────────┴─────┴─────────┴──────────┘
```

**PE counts:**
- Ingress PEs: 750 (row 0)
- Data PEs: 748 × 992 = 742,016
- Reducer PEs: 992 (column 749)
- Egress PEs: 750 (row 993)

## Pattern 1: Query Broadcast (Top → Bottom)

Used for: full scan, spatial scan, aggregation, KNN

Query wavelets flow from ingress (row 0) southward through all data PEs.
Each PE receives the query, evaluates locally, and forwards south.

```
Ingress (Row 0): Host → RAMP → SOUTH
  ↓ query wavelet
Data PE (Row 1): NORTH → RAMP + SOUTH
  ↓ query wavelet (forwarded)
Data PE (Row 2): NORTH → RAMP + SOUTH
  ↓
  ... (992 rows)
  ↓
Data PE (Row 992): NORTH → RAMP (no further forwarding)
```

**Color configuration:**
```csl
// Ingress PE
@set_color_config(x, 0, QUERY_COLOR, .{
    .routes = .{ .rx = .{ RAMP }, .tx = .{ SOUTH } }
});

// Data PEs (rows 1 to 991)
@set_color_config(x, y, QUERY_COLOR, .{
    .routes = .{ .rx = .{ NORTH }, .tx = .{ RAMP, SOUTH } }
});

// Last data PE row (row 992)
@set_color_config(x, 992, QUERY_COLOR, .{
    .routes = .{ .rx = .{ NORTH }, .tx = .{ RAMP } }
});
```

**Latency**: ~992 hops × 1ns = ~1μs for query to reach all PEs

## Pattern 2: Result Collection (Left → Right → Down)

Used for: collecting matching records from data PEs

Results flow EAST from data PEs to the reducer column (col 749),
then SOUTH to egress.

```
Data PE → EAST → Data PE → EAST → ... → Reducer PE
                                              ↓ SOUTH
                                         Reducer PE
                                              ↓ SOUTH
                                              ...
                                              ↓
                                         Egress PE → Host
```

**Color configuration:**
```csl
// Data PEs: send results east
@set_color_config(x, y, RESULT_COLOR, .{
    .routes = .{ .rx = .{ RAMP }, .tx = .{ EAST } }
});

// Reducer PEs (col 749): receive from west, aggregate, send south
@set_color_config(749, y, RESULT_COLOR, .{
    .routes = .{ .rx = .{ WEST }, .tx = .{ RAMP, SOUTH } }
});

// Egress PEs: collect final results
@set_color_config(x, 993, RESULT_COLOR, .{
    .routes = .{ .rx = .{ NORTH }, .tx = .{ RAMP } }
});
```

**Latency**: ~748 hops east + ~992 hops south = ~1.7μs worst case

## Pattern 3: Point Routing (Targeted Query)

Used for: point lookup, updating a specific tile

Route query to a specific PE based on Hilbert key → PE mapping.

```
Host → Ingress PE(target_x, 0) → South to PE(target_x, target_y)
```

**Optimization**: Only activate the target column's ingress PE, and only
forward south until reaching the target row.

This can be achieved with:
1. Host sends to specific ingress PE via targeted memcpy
2. Ingress PE forwards south along the column
3. Each PE checks if it's the target (compare tile_id)
4. Target PE responds, others ignore

## Pattern 4: Aggregation Reduce Tree

Used for: count, sum, avg, min, max

Two-phase reduction:
1. **Row reduction** (EAST): Each row reduces partial results to reducer column
2. **Column reduction** (SOUTH): Reducer column reduces all rows to final result

```
Phase 1: Row Reduction
Data PE → partial → Data PE → partial → ... → Reducer PE (row accumulator)

Phase 2: Column Reduction
Reducer PE (row 1) → partial ↓
Reducer PE (row 2) → accumulate + forward ↓
  ...
Reducer PE (row 992) → final result → Egress
```

**On each data PE:**
```csl
// Local aggregation
var local_count: u32 = 0;
var local_sum: f64 = 0.0;

// After scanning local records, send partial east
@fmovs(RESULT_COLOR, local_count);
```

**On each reducer PE:**
```csl
// Accumulate partials from west
task recv_partial(data: u32) void {
    running_total += data;
    partials_received += 1;
    if (partials_received == expected_from_row) {
        // Forward accumulated result south
        @fmovs(RESULT_COLOR, running_total);
    }
}
```

## Pattern 5: KNN Merge

Used for: vector similarity search

Each PE maintains a local top-K heap. Results merge as they flow east.

```
PE(0,y): local top-K → send east
PE(1,y): merge with incoming → send merged top-K east
  ...
PE(748,y): final merged top-K for this row → send to reducer
Reducer: merge across rows → send to egress
```

**Key consideration**: Each PE sends K result pairs (id, distance), so
the merge at each hop must handle 2K elements and output K.

## Pattern 6: Multi-Wavelet Transfer

For operations requiring more than 32 bits of data (e.g., query vectors
for KNN, record updates), use multi-wavelet protocols:

```
Wavelet 1: operation code + metadata
Wavelet 2-N: data payload (chunked into 32-bit pieces)

Example: 32-dimension int8 query vector = 32 bytes = 8 wavelets
Total: 1 header + 8 data = 9 wavelets per query
```

## Latency Estimates

| Operation | Hops | Estimated Latency |
|-----------|------|-------------------|
| Query broadcast (all PEs) | ~992 vertical | ~1μs |
| Result collection (full row) | ~748 horizontal | ~0.75μs |
| Point routing (worst case) | ~992 vertical | ~1μs |
| Full query round-trip | ~992 + 748 + 992 | ~2.7μs |
| PE local scan (700 records × 10 cycles) | — | ~7μs |
| Total full scan query | — | ~10μs |
