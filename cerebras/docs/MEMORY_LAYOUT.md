# PE Memory Layout for iDB

## 48KB SRAM Partitioning

Each Processing Element has exactly 48KB (49,152 bytes) of SRAM with single-cycle
access. For iDB, this memory is partitioned as follows:

```
Offset    Size     Contents
────────────────────────────────────────────
0x0000    64B      Tile metadata
                     - tile_id       (8B)
                     - hilbert_min   (16B, u128)
                     - hilbert_max   (16B, u128)
                     - record_count  (4B, u32)
                     - record_size   (4B, u32)
                     - entity_type   (2B, u16)
                     - flags         (2B, u16)
                     - padding       (12B)

0x0040    40B      Primary learned index
                     - slope         (8B, f64)
                     - intercept     (8B, f64)
                     - min_key       (8B, truncated u64)
                     - max_key       (8B, truncated u64)
                     - max_error     (4B, u32)
                     - padding       (4B)

0x0068    40B      Secondary index 1 (optional)
                     - Same format as primary learned index
                     - Used for non-Hilbert dimension queries

0x0090    40B      Secondary index 2 (optional)

0x00B8    200B     Category bitmap index (optional)
                     - Up to 8 categories × 25B each
                     - Each is a bitfield over local records
                     - For low-cardinality fields (status, type)

0x0180    1KB      HNSW neighbor list (optional)
(1,408B)           - For vector search operations
                     - Local graph edges to nearby PEs
                     - Format: [neighbor_pe_x: u16, neighbor_pe_y: u16,
                                distance: f32] × max_neighbors

0x0580    ~46KB    Record data (sorted by Hilbert key)
(46,207B)          - Each record: fixed-size struct
                     - Layout depends on entity schema
                     - Sorted by hilbert_key for binary search

0xBFFF             End of 48KB SRAM
```

## Record Format (Default: 64 bytes)

```
Offset  Size  Field            Type    Description
─────────────────────────────────────────────────────
0x00    8B    hilbert_key      u64     Truncated from u128
0x08    4B    id               u32     Local record ID
0x0C    4B    price            f32     Structured dim: price
0x10    2B    stock            u16     Structured dim: stock
0x12    1B    category         u8      Enum index
0x13    1B    flags            u8      Bitfield
0x14    32B   embedding_q      [32]i8  Quantized embedding (int8)
0x34    1B    edge_count       u8      Number of graph edges
0x35    10B   edges            [5]u16  PE-local edge IDs
0x3F    1B    padding          u8      Alignment
─────────────────────────────────────────────────────
Total:  64 bytes
```

## Capacity Calculations

| Record Size | Records per PE | Records per Wafer (742K PEs) |
|-------------|----------------|------------------------------|
| 32 bytes    | ~1,444         | ~1.07 billion                |
| 64 bytes    | ~722           | ~536 million                 |
| 128 bytes   | ~361           | ~268 million                 |
| 256 bytes   | ~180           | ~134 million                 |

## Memory Budget Calculator

```
Available for records = 48KB - metadata_overhead
metadata_overhead = 64 + 40 + 40 + 40 + 200 + 1408 = 1,792 bytes (max)
                  = 64 + 40 = 104 bytes (minimum, no optional indexes)

Usable SRAM (max indexes)  = 49,152 - 1,792 = 47,360 bytes
Usable SRAM (min indexes)  = 49,152 - 104   = 49,048 bytes

Records per PE = usable_sram / record_size
```

## Adding New Fields

To add a new field to the record format:

1. **Check the budget**: Ensure `new_record_size × records_per_pe ≤ usable_sram`
2. **Update the CSL struct** in `common/types.csl`
3. **Update predicate evaluation** in `spatial_scan.csl` (add switch case)
4. **Update the host packing code** in `cerebras/host/controller.py`
5. **Update the Rust record format** in `norndb-core/src/types.rs`
6. **Rebuild learned indexes** (key positions may shift)

## Variable-Length Data Strategy

Since PEs only have 48KB, variable-length data lives off-chip:

| Data Type | On-PE Storage | Off-PE Storage |
|-----------|---------------|----------------|
| Numeric fields | Full value | — |
| Short strings | First 8 chars (truncated) | Full string on host |
| Embeddings | Quantized int8 (32B) | Full float32 on host/MemoryX |
| Text blobs | Hash/ID only (8B) | Full blob in blob store |
| Edge targets | PE-local IDs (10B) | Full edge list on host |
