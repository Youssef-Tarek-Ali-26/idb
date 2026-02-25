# Performance Model

## Hardware Constants

| Constant | Value | Source |
|----------|-------|--------|
| Clock frequency | 1 GHz | WSE spec |
| Num data PEs | 742,016 | 748 cols × 992 rows |
| SRAM per PE | 48 KB | WSE spec |
| Record overhead | 0x580 = 1,408 bytes | iDB memory layout |
| Usable SRAM | ~47,744 bytes | 48KB - overhead |
| Hop latency | ~1 ns | Wavelet traversal |
| Max hops X | 750 | Grid width |
| Max hops Y | 994 | Grid height |

## Per-Operation Cycle Counts

| Operation | Cycles | Notes |
|-----------|--------|-------|
| Integer compare | 1 | Single ALU op |
| Float compare | 2 | |
| Field access (fixed offset) | 1 | Direct SRAM read |
| Predicate eval (1 field) | ~5 | Load + compare + branch |
| Full predicate eval | ~10 | Average with multiple fields |
| L2 distance (32-dim int8) | ~96 | 32 × (sub + mul + acc) |
| Heap insert (K=10) | ~30 | Sift-up in 10-element heap |
| Heap replace max | ~30 | Replace root + sift-down |
| Linear regression predict | ~5 | Multiply + add |
| Binary search (10 elements) | ~20 | ~3-4 iterations × 5 cycles |
| Wavelet send | ~3 | Including fabric overhead |

## Query Latency Estimates

### Full Scan (predicate on all records)

```
Records per PE: 722 (64-byte records)
Predicate cycles per record: 10
Local scan time: 722 × 10 / 1GHz = 7.22μs

Query broadcast (top→bottom): 992 hops × 1ns = 0.99μs
Result collection (left→right): 748 hops × 1ns = 0.75μs
Result collection (top→bottom): 992 hops × 1ns = 0.99μs

Total wall-clock: max(scan_time, broadcast + collection)
                = 7.22μs + 0.99μs + 0.75μs + 0.99μs
                ≈ 10μs
```

### Point Lookup

```
Route to target PE: ~500 hops average × 1ns = 0.5μs
Learned index predict: 5 cycles = 5ns
Binary search: 20 cycles = 20ns
Route result back: ~500 hops × 1ns = 0.5μs

Total: ≈ 1μs
```

### KNN Search (K=10, 32-dim int8)

```
Distance per record: 96 cycles
Heap ops per record: ~15 cycles average (most records don't insert)
Per-record total: ~111 cycles
Local scan: 722 × 111 / 1GHz = 80μs

Merge overhead (per hop): K × 2 wavelets = 20 wavelets × 3 cycles = 60 cycles
Merge across 748 columns: 748 × 60 cycles = ~45μs

Total: 80μs + broadcast + merge ≈ 82μs
```

### Aggregation (count/sum)

```
Predicate + accumulate per record: 15 cycles
Local agg: 722 × 15 / 1GHz = 10.8μs

Row reduction (east): 748 hops, each adding 1 cycle = 748 cycles = 0.75μs
Column reduction (south): 992 hops × 1 cycle = 0.99μs

Total: ≈ 12.5μs
```

## Comparison with CPU and GPU

### Assumptions

| Platform | Parallelism | Clock | Memory BW |
|----------|-------------|-------|-----------|
| CPU (i7-10750H) | 12 threads | 2.6 GHz | 40 GB/s |
| GPU (RTX 3060) | 3,584 cores | 1.78 GHz | 360 GB/s |
| WSE-3 | 742,016 PEs | 1 GHz | ~44 TB/s (aggregate SRAM) |

### Full Scan: 500M records, 64 bytes each

```
CPU:
  Total data: 500M × 64B = 32 GB
  Throughput: 40 GB/s ÷ 64B = 625M records/s
  Time: 500M / 625M = 800ms
  With SIMD (4x): ~200ms

GPU:
  Throughput: 360 GB/s ÷ 64B = 5.6B records/s
  Transfer overhead: 32 GB at PCIe 16 GB/s = 2s
  Compute: 500M / 5.6B = 89ms
  Total (including transfer): ~2.1s
  (With data already on GPU: 89ms)

Cerebras:
  All data in SRAM (no memory hierarchy)
  Every PE scans in parallel: ~7μs
  Total with routing: ~10μs

Speedup vs CPU: ~80,000x
Speedup vs GPU (compute only): ~8,900x
```

### Point Lookup: single record

```
CPU: Hash index lookup ~1μs (cache hit) to ~100μs (cache miss)
GPU: Not applicable (transfer overhead dominates)
Cerebras: ~1μs

Note: For point lookups, CPU with hot cache is competitive.
Cerebras advantage is consistency (always ~1μs, no cache effects).
```

### KNN: 500M records, K=10, 32-dim

```
CPU (brute force):
  Distance per record: ~64 cycles (SIMD)
  Throughput: 12 threads × 2.6GHz / 64 = 487M records/s
  Time: 500M / 487M = 1.03s

CPU (HNSW index):
  Approximate: ~1ms (but requires building index, not exact)

GPU:
  Distance per record: ~1 cycle per core
  Throughput: 3584 × 1.78GHz = 6.4B records/s
  Time: 500M / 6.4B = 78ms

Cerebras:
  Total: ~82μs

Speedup vs CPU (exact): ~12,500x
Speedup vs GPU: ~950x
```

## Caveats

1. **Simulator vs hardware**: Actual hardware performance may differ from analytical model
2. **Data loading**: Initial data transfer from host to WSE is NOT included in query latency
3. **Multi-tenant overhead**: Tenant scoping adds ~2 cycles per predicate
4. **Result size**: Queries returning many results are limited by egress bandwidth
5. **WSE-3 specifics**: Some micro-architectural details may differ from WSE-2 estimates
