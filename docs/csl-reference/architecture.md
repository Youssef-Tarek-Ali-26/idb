# WSE Architecture Overview

> Source: Cerebras SDK 1.4.0 Documentation — "A Conceptual View"

## Wafer-Scale Engine (WSE)

The Cerebras WSE is a wafer-parallel compute accelerator containing hundreds of thousands
of independent Processing Elements (PEs) interconnected in a 2D rectangular mesh on a
single silicon wafer.

### WSE Generations

| Feature | WSE-2 | WSE-3 |
|---------|-------|-------|
| PE Count | ~850,000 | ~900,000 |
| SRAM per PE | 48 KB | 48 KB |
| Total SRAM | ~40 GB | ~44 GB |
| Grid (approx) | 750 × 994 | 750 × 994 |
| I/O Channels per edge | 60 | 62 |
| Routable color IDs | 0-23 | 0-7 (routable) |
| Task ID range | 0-63 | 0-63 |
| Routable task IDs | 0-23 | 0-7 |
| Clock speed | ~1 GHz | ~1 GHz |

## Processing Element (PE)

Each PE contains three components:

```
┌──────────────────────────────────┐
│           PE (x, y)              │
│                                  │
│  ┌──────────┐   ┌────────────┐  │
│  │ Compute  │   │   48 KB    │  │
│  │ Engine   │◄─►│   SRAM     │  │
│  │  (CE)    │   │ (1-cycle   │  │
│  │          │   │  access)   │  │
│  └────┬─────┘   └────────────┘  │
│       │ RAMP                     │
│  ┌────▼─────────────────────┐   │
│  │     Fabric Router        │   │
│  │                          │   │
│  │  NORTH ↕ SOUTH           │   │
│  │  EAST  ↔ WEST            │   │
│  └──────────────────────────┘   │
└──────────────────────────────────┘
```

### Key Properties

1. **Isolation**: Neither the CE nor the local memory of a PE is directly accessible
   by other PEs. All communication happens through wavelets via the fabric router.

2. **SRAM**: 48 KB per PE with single-cycle read/write access latency. This is the
   only memory available to each PE — there is no cache hierarchy or shared memory.

3. **Router**: Connects bidirectionally to the CE via the RAMP link and to four
   neighboring PEs (North, South, East, West) in the mesh.

4. **Compute Engine**: General-purpose processor that executes tasks. A PE must be
   able to quickly (within a few cycles) respond to wavelet arrival, update internal
   state, and send out wavelets.

## Communication: Wavelets

Wavelets are the fundamental communication unit on the WSE.

- **Size**: 32 bits payload
- **Tag**: 5-bit color tag determines routing and task activation
- **Channels**: 24 virtual communication channels (routable colors) identified by IDs 0-23

### Wavelet Structure

```
┌─────────┬──────────────────────────────┐
│  5-bit  │       32-bit payload          │
│  color  │                               │
│  tag    │                               │
└─────────┴──────────────────────────────┘
```

The color tag determines:
1. **Routing**: Which direction(s) the wavelet travels through the fabric
2. **Task activation**: Which task (if any) is activated when the wavelet arrives

## Fabric Mesh Topology

```
PE(0,0) ─── PE(1,0) ─── PE(2,0) ─── ... ─── PE(749,0)
  │           │           │                      │
PE(0,1) ─── PE(1,1) ─── PE(2,1) ─── ... ─── PE(749,1)
  │           │           │                      │
PE(0,2) ─── PE(1,2) ─── PE(2,2) ─── ... ─── PE(749,2)
  │           │           │                      │
  ⋮           ⋮           ⋮                      ⋮
  │           │           │                      │
PE(0,993) ── PE(1,993) ── PE(2,993) ── ... ── PE(749,993)
```

### Routing Directions

Each PE's router can send/receive wavelets in 5 directions:
- **RAMP**: To/from the local CE
- **NORTH**: To PE at (x, y-1)
- **SOUTH**: To PE at (x, y+1)
- **EAST**: To PE at (x+1, y)
- **WEST**: To PE at (x-1, y)

### Hop Latency

Wavelet traversal between adjacent PEs takes approximately 1 nanosecond per hop.
Maximum hops across the full wafer: ~750 (X) + ~994 (Y) = ~1744 hops worst case.

## Host Interface

The Cerebras System (CS) connects to host CPU clusters via parallel 100 Gigabit
Ethernet connections through "host I/O."

### I/O Channel Layout

- WSE-2: 60 channels at each edge
- WSE-3: 62 channels at each edge
- The memcpy infrastructure consumes 3 columns west + 2 columns east of the kernel,
  plus additional support PEs around it as a "halo"

### Fabric Dimension Constraints

When compiling with memcpy:
```
fabric_dims_x >= 7 + kernel_width
fabric_dims_y >= 2 + kernel_height
fabric_offset_x >= 4
fabric_offset_y >= 1
```

## Memory Architecture (for iDB)

### PE SRAM Budget (48 KB = 49,152 bytes)

For iDB, the PE memory is partitioned as:

```
Offset    Size     Contents
────────────────────────────────
0x0000    64B      Tile metadata (tile_id, hilbert_range, record_count)
0x0040    40B      Primary learned index (slope, intercept, max_error)
0x0068    40B      Secondary index 1 (optional)
0x0090    40B      Secondary index 2 (optional)
0x00B8    200B     Category bitmap index (optional)
0x0180    1KB      HNSW neighbor list (optional, for vector search)
0x0580    ~46KB    Record data (sorted by Hilbert key)
0xBFFF             End of 48KB SRAM
```

With 64-byte records: ~700 records per PE
With ~742,000 data PEs: ~519 million records per wafer
