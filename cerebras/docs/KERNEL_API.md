# iDB Kernel API Reference

## Kernel Index

| Kernel | Purpose | Input | Output |
|--------|---------|-------|--------|
| `spatial_scan.csl` | Full scan with predicates | query wavelet | matching record IDs |
| `point_lookup.csl` | Single record by key | hilbert key | record data |
| `range_scan.csl` | Hilbert range query | key range + predicates | matching records |
| `knn_search.csl` | Vector similarity search | query vector + K | top-K (id, dist) pairs |
| `aggregate.csl` | count/sum/avg/min/max | agg op + predicates | partial aggregates |
| `topk.csl` | Top-K with sort | K + order_by field | top-K records |
| `data_load.csl` | Bulk load tiles to PEs | tile data from host | — |
| `index_build.csl` | Fit learned indexes | sorted records | index params |
| `graph_traverse.csl` | Edge following | start node + edge type | reachable nodes |

## Wavelet Protocol

### Query Wavelet Format (32 bits)

```
Bits 31-24: reserved
Bits 23-16: pred_op     (0=eq, 1=lt, 2=gt, 3=le, 4=ge, 5=ne)
Bits 15-8:  pred_field  (field index in record struct)
Bits 7-0:   query_op    (0=scan, 1=point, 2=range, 3=knn, 4=agg, 5=topk)
```

### Encoding/Decoding

```csl
// Encode
var wavelet: u32 = query_op | (pred_field << 8) | (pred_op << 16);

// Decode
var op = @as(u8, wavelet & 0xFF);
var field = @as(u8, (wavelet >> 8) & 0xFF);
var pred = @as(u8, (wavelet >> 16) & 0xFF);
```

### Multi-Wavelet Sequences

For operations needing more data:

```
Wavelet 0: query header (op, field, pred_op)
Wavelet 1: pred_value (f32 as u32 bitcast)
Wavelet 2+: additional data (query vector for KNN, key range for range scan)
```

## Kernel: spatial_scan.csl

**Purpose**: Evaluate predicates on all local records, send matching IDs east.

**Input**: Query wavelet (broadcast from north)
**Output**: Matching record IDs (sent east to reducer)

**Algorithm**:
1. Receive query wavelet → decode op, field, predicate
2. Receive comparison value (second wavelet)
3. Iterate all local records
4. For each record: evaluate predicate on specified field
5. If match: send record ID east via RESULT_COLOR

**Expected performance**: ~700 records × 10 cycles/record = ~7,000 cycles = ~7μs

## Kernel: point_lookup.csl

**Purpose**: Find a single record by Hilbert key using learned index.

**Input**: Target Hilbert key (2 wavelets: upper + lower 32 bits)
**Output**: Full record data or "not found" sentinel

**Algorithm**:
1. Receive target key
2. Use learned index to predict position: `pos = slope * key + intercept`
3. Search within error bounds: `[pos - max_error, pos + max_error]`
4. Binary search within bounds for exact key match
5. If found: send record data east

**Expected performance**: ~50 cycles (index predict + binary search of ~10 elements)

## Kernel: range_scan.csl

**Purpose**: Find all records within a Hilbert key range.

**Input**: min_key, max_key (4 wavelets) + optional predicates
**Output**: Matching record IDs

**Algorithm**:
1. Receive key range [min, max]
2. Check if PE's tile range overlaps query range
3. If no overlap: skip (don't scan)
4. If overlap: use learned index to find start position
5. Scan from start position while key ≤ max_key
6. Apply additional predicates if present
7. Send matching IDs east

## Kernel: knn_search.csl

**Purpose**: Find K nearest neighbors by vector distance.

**Input**: Query vector (quantized int8, multiple wavelets) + K
**Output**: Top-K (record_id, distance) pairs

**Algorithm**:
1. Receive query vector (32 bytes = 8 wavelets)
2. For each local record:
   a. Compute L2 distance between query and record's embedding_q
   b. If distance < heap maximum: insert into local top-K heap
3. Send local top-K pairs east for merging

**Distance computation**: ~32 dims × 3 ops (sub, mul, acc) = ~96 cycles per record

## Kernel: aggregate.csl

**Purpose**: Compute aggregations with optional predicates.

**Input**: Aggregation op (count/sum/avg/min/max) + field + predicates
**Output**: Partial aggregate (sent east for reduction)

**Algorithm**:
1. Receive query wavelet → decode agg_op, field, predicates
2. Initialize accumulators (count=0, sum=0.0, min=MAX, max=MIN)
3. Iterate all local records:
   a. Apply predicate filter
   b. Update relevant accumulator
4. Send partial result east to reducer

**Reducer merges**: count+=count, sum+=sum, min=min(min), max=max(max)
**Avg**: computed on host as sum/count

## Kernel: data_load.csl

**Purpose**: Receive tile data from host and store in PE SRAM.

**Input**: Tile metadata + learned index + record data (via streaming memcpy)
**Output**: None (data stored in local SRAM)

**Sequence**:
1. Receive tile metadata (64 bytes via memcpy_h2d)
2. Store at offset 0x0000
3. Receive learned index (40 bytes)
4. Store at offset 0x0040
5. Receive record data (variable, up to ~46KB)
6. Store at offset 0x0580
7. Signal completion

## Kernel: index_build.csl

**Purpose**: Fit a learned index from local sorted records.

**Input**: Activation signal (records must already be loaded)
**Output**: Updated learned index parameters at offset 0x0040

**Algorithm**:
1. Read all local records' hilbert_keys
2. Compute linear regression: positions vs keys
3. Compute max error (maximum deviation from prediction)
4. Store slope, intercept, max_error at index offset

## Kernel: graph_traverse.csl

**Purpose**: Follow graph edges from a starting record.

**Input**: Start record ID + edge type + max depth
**Output**: Set of reachable record IDs

**Algorithm** (BFS, depth-limited):
1. Receive start record ID
2. Look up record in local tile
3. Read edge list from record
4. For each edge: send traversal request to target PE
5. Target PE repeats (depth -= 1) until depth == 0

**Note**: Graph traversal is inherently sequential per hop.
Parallelism comes from fanning out at each level.

## Error Handling

All kernels use sentinel values for error conditions:

| Sentinel | Meaning |
|----------|---------|
| `0xFFFFFFFF` | Record not found |
| `0xFFFFFFFE` | Predicate type mismatch |
| `0xFFFFFFFD` | Tile range mismatch (range scan skip) |
| `0xFFFFFFFC` | Buffer overflow (too many results) |
