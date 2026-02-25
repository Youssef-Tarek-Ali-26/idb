# iDB (NornDB) — Full Architecture Plan

## Project Identity

```
Name:        iDB (internal) / NornDB (product name)
Tagline:     "Every query is geometry"
License:     BSL 1.1 → Apache 2.0 (4 year conversion)
Patent:      Provisional patent on N-dimensional spatial 
             tiling on wafer-scale mesh architectures
```

---

## 1. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     CLIENT LAYER                            │
│                                                             │
│  norndb-client-ts    norndb-client-rs    norndb-client-py   │
│  (TypeScript SDK)    (Rust SDK)          (Python SDK)       │
│                                                             │
│  REST API            WebSocket           Wire Protocol      │
│  (HTTP/JSON)         (Reactive subs)     (Binary, fast)     │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                     GATEWAY LAYER                           │
│                        (Rust)                               │
│                                                             │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │
│  │  Auth &   │ │  Query   │ │  Rate    │ │  Tenant      │   │
│  │  Session  │ │  Router  │ │  Limiter │ │  Resolver    │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────────┘   │
└───────────────────────────┬─────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                     QUERY ENGINE                            │
│                        (Rust)                               │
│                                                             │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐   │
│  │  Parser  │→│ Analyzer │→│ Planner  │→│  Optimizer   │   │
│  │  (iQL)   │ │ (types,  │ │ (logical │ │  (physical   │   │
│  │          │ │  scope)  │ │  plan)   │ │   plan)      │   │
│  └──────────┘ └──────────┘ └──────────┘ └──────┬───────┘   │
│                                                 │           │
│                                    ┌────────────▼────────┐  │
│                                    │   Hardware Router   │  │
│                                    │   "What can I use?" │  │
│                                    └────────────┬────────┘  │
│                                                 │           │
│                            ┌────────────────────┼────────┐  │
│                            ▼                    ▼        ▼  │
│                     ┌──────────┐ ┌──────────┐ ┌────────┐   │
│                     │   CPU    │ │   GPU    │ │Cerebras│   │
│                     │ Executor │ │ Executor │ │Executor│   │
│                     └──────────┘ └──────────┘ └────────┘   │
└─────────────────────────────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                    STORAGE ENGINE                           │
│                       (Rust)                                │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │               Hilbert Spatial Layer                  │   │
│  │  N-dim coordinates → Hilbert key → tile assignment   │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │                               │
│  ┌──────────────┐ ┌────────▼────┐ ┌─────────────────────┐  │
│  │   WAL        │ │  Tile       │ │  Learned Index      │  │
│  │   (Write-    │ │  Manager    │ │  Manager            │  │
│  │   Ahead Log) │ │  (48KB      │ │  (per-tile linear   │  │
│  │              │ │   chunks)   │ │   models)           │  │
│  └──────────────┘ └─────────────┘ └─────────────────────┘  │
│                                                             │
│  ┌──────────────┐ ┌─────────────┐ ┌─────────────────────┐  │
│  │   Arrow      │ │  Parquet    │ │  Blob Store         │  │
│  │   Record     │ │  Cold       │ │  (large files,      │  │
│  │   Batches    │ │  Storage    │ │   images, etc.)     │  │
│  │   (hot data) │ │  (history)  │ │                     │  │
│  └──────────────┘ └─────────────┘ └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────────┐
│                  CEREBRAS RUNTIME                           │
│              (CSL + Python host code)                       │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Host Controller (Python, talks to Rust via FFI)     │   │
│  │  - Loads compiled CSL binaries onto chip             │   │
│  │  - Manages data transfer (memcpy framework)          │   │
│  │  - Dispatches query plans to wafer                   │   │
│  │  - Collects results from egress PEs                  │   │
│  └──────────────────────────┬───────────────────────────┘   │
│                             │ SdkRuntime API                │
│  ┌──────────────────────────▼───────────────────────────┐   │
│  │  WSE Kernels (CSL)                                   │   │
│  │  - spatial_scan.csl      (full scan with predicates) │   │
│  │  - point_lookup.csl      (single record by key)      │   │
│  │  - range_scan.csl        (Hilbert range query)       │   │
│  │  - knn_search.csl        (vector similarity)         │   │
│  │  - aggregate.csl         (count/sum/avg/min/max)     │   │
│  │  - topk.csl              (top-K with sort)           │   │
│  │  - data_load.csl         (bulk load tiles to PEs)    │   │
│  │  - index_build.csl       (fit learned indexes)       │   │
│  │  - graph_traverse.csl    (edge following)            │   │
│  └──────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Repository Structure

```
norndb/
├── Cargo.toml                    # Workspace root
├── LICENSE.md                    # BSL 1.1
├── README.md
├── ARCHITECTURE.md               # This document
│
├── crates/                       # Rust workspace members
│   ├── norndb-core/              # Core types, traits, errors
│   ├── norndb-parser/            # iQL query language parser
│   ├── norndb-planner/           # Query planning & optimization
│   ├── norndb-executor-cpu/      # CPU execution (DataFusion)
│   ├── norndb-executor-gpu/      # GPU execution (wgpu)
│   ├── norndb-executor-cerebras/ # Cerebras dispatch
│   ├── norndb-storage/           # Storage engine (Arrow, Parquet, WAL)
│   ├── norndb-hilbert/           # Hilbert curve encoding/decoding
│   ├── norndb-index/             # Learned indexes, secondary indexes
│   ├── norndb-auth/              # Authentication, tenant scoping, RLS
│   ├── norndb-reactive/          # Subscription engine, changefeed
│   ├── norndb-server/            # TCP/WebSocket/HTTP server
│   ├── norndb-wire/              # Binary wire protocol
│   ├── norndb-blob/              # Large file storage
│   └── norndb-cluster/           # Multi-node (future)
│
├── cerebras/                     # Cerebras-specific code
│   ├── kernels/                  # CSL source files
│   │   ├── common/               # Shared CSL libraries
│   │   │   ├── types.csl         # Record layout, constants
│   │   │   ├── hilbert.csl       # Hilbert key operations
│   │   │   ├── predicate.csl     # Predicate evaluation
│   │   │   └── reduce.csl        # Reduction primitives
│   │   ├── spatial_scan.csl
│   │   ├── point_lookup.csl
│   │   ├── range_scan.csl
│   │   ├── knn_search.csl
│   │   ├── aggregate.csl
│   │   ├── topk.csl
│   │   ├── data_load.csl
│   │   ├── index_build.csl
│   │   └── graph_traverse.csl
│   ├── host/                     # Python host code
│   │   ├── controller.py         # Main host ↔ device interface
│   │   ├── data_loader.py        # Bulk load data to wafer
│   │   ├── query_dispatch.py     # Send queries, collect results
│   │   └── perf_estimator.py     # Analytical performance model
│   ├── simulator/                # Simulator test harness
│   │   ├── test_spatial_scan.py
│   │   ├── test_point_lookup.py
│   │   ├── test_knn.py
│   │   └── benchmarks.py
│   └── docs/                     # CSL-specific documentation
│       ├── MEMORY_LAYOUT.md
│       ├── ROUTING_PATTERNS.md
│       ├── KERNEL_API.md
│       └── PERFORMANCE_MODEL.md
│
├── gpu/                          # GPU kernel code
│   ├── shaders/                  # WGSL compute shaders
│   │   ├── predicate_eval.wgsl
│   │   ├── vector_distance.wgsl
│   │   └── aggregate.wgsl
│   └── src/                      # Rust GPU dispatch
│       └── lib.rs
│
├── clients/                      # Client SDKs
│   ├── typescript/               # @norndb/client
│   ├── python/                   # norndb-python
│   └── rust/                     # norndb-client (Rust)
│
├── tools/                        # CLI and utilities
│   ├── norndb-cli/               # Command-line client
│   ├── norndb-bench/             # Benchmarking suite
│   └── norndb-migrate/           # Data migration tools
│
├── docs/                         # Documentation
│   ├── book/                     # mdBook user guide
│   ├── rfcs/                     # Design decisions
│   ├── paper/                    # Academic paper (LaTeX)
│   └── api/                      # API reference (generated)
│
└── tests/                        # Integration tests
    ├── correctness/
    ├── performance/
    └── compatibility/
```

---

## 3. Rust Crates — Detailed Breakdown

### 3.1 norndb-core

The foundation crate. Zero dependencies on other norndb crates.

```rust
// Core types used everywhere

/// A tenant-scoped identifier
pub struct TenantId(pub Ulid);

/// N-dimensional point in the data space
pub struct SpatialPoint {
    pub structured_dims: Vec<f64>,      // price, date, quantity, etc.
    pub embedding: Option<Vec<f32>>,     // semantic vector (128-1536 dims)
    pub hilbert_key: u128,               // computed Hilbert curve position
}

/// A record in the database
pub struct Record {
    pub id: RecordId,
    pub tenant: TenantId,
    pub entity_type: EntityType,         // "Product", "Order", etc.
    pub spatial_point: SpatialPoint,
    pub data: ArrowRecordBatch,          // actual field values in Arrow format
    pub edges: Vec<Edge>,                // graph relationships
}

/// Query plan that executors consume
pub enum PhysicalPlan {
    PointLookup { key: HilbertKey },
    RangeScan { min: HilbertKey, max: HilbertKey, predicates: Vec<Predicate> },
    FullScan { predicates: Vec<Predicate> },
    KnnSearch { query_vector: Vec<f32>, k: usize, predicates: Vec<Predicate> },
    Aggregate { op: AggOp, group_by: Vec<Column>, predicates: Vec<Predicate> },
    TopK { k: usize, order_by: Column, predicates: Vec<Predicate> },
    GraphTraverse { start: RecordId, edge_type: String, depth: usize },
}

/// What hardware to target
pub enum ExecutionTarget {
    Cpu,
    Gpu,
    Cerebras,
    Auto,  // planner decides
}

/// Trait that all executors implement
pub trait QueryExecutor: Send + Sync {
    fn execute(&self, plan: &PhysicalPlan, tenant: TenantId) 
        -> Result<RecordStream>;
    fn capabilities(&self) -> ExecutorCapabilities;
    fn estimate_cost(&self, plan: &PhysicalPlan) -> CostEstimate;
}

/// Reactive subscription handle
pub struct Subscription {
    pub id: SubscriptionId,
    pub query: LogicalPlan,
    pub tenant: TenantId,
    pub affected_ranges: Vec<HilbertRange>,  // for invalidation
}
```

**Key dependencies:** `arrow-rs`, `ulid`, `serde`, `thiserror`, `bytes`

### 3.2 norndb-parser

Parses iQL (the query language) into an AST.

```
iQL grammar (Python-inspired, not SQL):

// Entity access
Product                           → all products
Product("ring-001")               → by ID
Product where price < 500         → filtered
Product where meaning("luxury")   → semantic search

// Graph traversal
Company("Norn")->Brand->Product   → follow edges
Company("Norn")->Brand->Product where price < 500

// Aggregation
Product | count
Product | avg(price)
Product | group_by(category) | count

// Mutations
create Product { name = "Ring", price = 500 }
update Product("ring-001") { price = 450 }
delete Product("ring-001")

// Reactive
watch Product where stock < 10
watch Company("Norn")->Brand->Product | count

// Time travel
Product("ring-001") @ "2025-06-01"
Product("ring-001") @ history
```

**Implementation approach:**
- Hand-written recursive descent parser (not a parser generator)
- Clear error messages with source spans
- AST types in `norndb-core`

**Key dependencies:** `logos` (lexer), `miette` (error reporting)

### 3.3 norndb-planner

Converts AST → logical plan → physical plan.

```
Pipeline:

AST (from parser)
  → Name resolution (what entity? what fields?)
  → Type checking (is price a number? is category a string?)
  → Tenant scoping (inject WHERE tenant_id = X)
  → Logical plan (abstract operations)
  → Optimization (predicate pushdown, join reordering)
  → Hardware routing (CPU vs GPU vs Cerebras)
  → Physical plan (concrete execution steps)
```

**The hardware router is the key differentiator:**

```rust
fn route_to_hardware(
    logical: &LogicalPlan, 
    available: &[ExecutorCapabilities]
) -> PhysicalPlan {
    // Decision tree:
    // 1. Point lookup → CPU always (too small for GPU/Cerebras overhead)
    // 2. Small range scan (<1000 records) → CPU
    // 3. Large scan with predicates → GPU if available, else CPU
    // 4. Full scan with aggregation → Cerebras if available, else GPU, else CPU
    // 5. KNN search → Cerebras if available (massively parallel), else GPU
    // 6. Graph traversal → CPU (sequential hops, cache-friendly)
    
    // Cost-based: each executor estimates cost, pick cheapest
    available.iter()
        .map(|exec| (exec, exec.estimate_cost(logical)))
        .min_by_key(|(_, cost)| cost.estimated_microseconds)
        .map(|(exec, _)| exec.to_physical_plan(logical))
}
```

**Key dependencies:** `norndb-core`, `petgraph` (for plan DAGs)

### 3.4 norndb-hilbert

The spatial foundation. Maps N-dimensional coordinates to 1D Hilbert keys.

```rust
/// Encode N-dimensional point to Hilbert key
pub fn encode(coords: &[u64], bits_per_dim: u32) -> u128 {
    // Bit-interleaving algorithm
    // Well-documented, reference implementations available
    // Returns a single u128 key preserving spatial locality
}

/// Decode Hilbert key back to N-dimensional coordinates
pub fn decode(key: u128, dims: usize, bits_per_dim: u32) -> Vec<u64> {
    // Inverse of encode
}

/// Map a query region to Hilbert key ranges
pub fn region_to_ranges(
    min_corner: &[u64], 
    max_corner: &[u64], 
    bits_per_dim: u32
) -> Vec<HilbertRange> {
    // An N-dimensional box maps to MULTIPLE Hilbert ranges
    // because the curve weaves in and out of the region
    // Returns sorted, non-overlapping ranges
}

/// Assign a Hilbert key range to a PE on the Cerebras mesh
pub fn key_to_pe(key: u128, total_pes: usize) -> (u16, u16) {
    // Simple: divide key space evenly across PEs
    // PE position = (key / keys_per_pe) mapped to 2D grid
    let pe_index = (key as usize) / (u128::MAX as usize / total_pes);
    let cols = 750; // WSE-3 grid width
    (pe_index / cols, pe_index % cols)
}

/// For high-dimensional embeddings: project to lower dimensions first
pub fn project_embedding(
    embedding: &[f32],        // 128-1536 dims
    projection_matrix: &[f32], // random projection (Johnson-Lindenstrauss)
    target_dims: usize,        // ~20-40 dims
) -> Vec<f64> {
    // Matrix multiply: embedding × projection = lower-dim vector
    // Then quantize to integer coordinates for Hilbert encoding
}
```

**Key dependencies:** `ndarray` (matrix ops), `rand` (for random projection matrices)

### 3.5 norndb-index

Learned indexes and secondary indexes.

```rust
/// A learned index for one tile/PE
pub struct LearnedIndex {
    pub slope: f64,
    pub intercept: f64,
    pub min_key: u128,
    pub max_key: u128,
    pub max_error: u32,  // max positions off the prediction can be
}

impl LearnedIndex {
    /// Train from sorted data
    pub fn fit(keys: &[u128], positions: &[u32]) -> Self {
        // Simple linear regression
        // slope = cov(keys, positions) / var(keys)
        // intercept = mean(positions) - slope * mean(keys)
        // max_error = max absolute residual
    }
    
    /// Predict position of a key
    pub fn predict(&self, key: u128) -> u32 {
        let pos = (self.slope * key as f64 + self.intercept) as i64;
        pos.clamp(0, self.max_position as i64) as u32
    }
    
    /// Lookup with error correction
    pub fn lookup(&self, key: u128, data: &[Record]) -> Option<&Record> {
        let predicted = self.predict(key) as usize;
        let lo = predicted.saturating_sub(self.max_error as usize);
        let hi = (predicted + self.max_error as usize).min(data.len());
        // Binary search within error bounds
        data[lo..hi].binary_search_by_key(&key, |r| r.hilbert_key).ok()
            .map(|i| &data[lo + i])
    }
    
    /// Serialize for loading onto Cerebras PE (must be tiny)
    pub fn to_bytes(&self) -> [u8; 40] {
        // slope: 8 bytes (f64)
        // intercept: 8 bytes (f64)
        // min_key: 16 bytes (u128)  [can truncate]
        // max_error: 4 bytes (u32)
        // padding: 4 bytes
    }
}

/// Hierarchical index for routing queries to PEs
pub struct GlobalRoutingIndex {
    pub stage1: Vec<LearnedIndex>,  // ~100 models, coarse routing
    pub stage2: Vec<LearnedIndex>,  // ~10,000 models, PE-level routing
}

/// Secondary index (non-Hilbert access patterns)
pub enum SecondaryIndex {
    /// Inverted index for categorical fields
    Inverted { field: String, postings: HashMap<Value, Vec<RecordId>> },
    /// Sorted index for range queries on non-Hilbert dimensions
    BTree { field: String, tree: BTreeMap<Value, Vec<RecordId>> },
    /// Bitmap index for low-cardinality fields
    Bitmap { field: String, bitmaps: HashMap<Value, RoaringBitmap> },
}
```

**Key dependencies:** `ndarray`, `roaring` (bitmap indexes)

### 3.6 norndb-storage

Arrow-based storage engine.

```rust
/// A spatial tile — the unit of data that maps to one Cerebras PE
pub struct Tile {
    pub id: TileId,
    pub hilbert_range: HilbertRange,       // [min_key, max_key)
    pub bounds: NdimBounds,                 // N-dim bounding box
    pub records: ArrowRecordBatch,          // actual data, columnar
    pub learned_index: LearnedIndex,        // 40 bytes
    pub secondary_indexes: Vec<SecondaryIndex>,
    pub record_count: usize,
    pub size_bytes: usize,                  // must fit in 48KB for Cerebras
}

/// The storage manager
pub struct StorageEngine {
    pub wal: WriteAheadLog,
    pub hot_tiles: DashMap<TileId, Tile>,   // in-memory, Arrow format
    pub cold_store: ParquetStore,            // on-disk, compressed
    pub blob_store: BlobStore,               // large files
    pub tile_map: TileMap,                   // Hilbert key → TileId
}

impl StorageEngine {
    /// Insert a record
    pub fn insert(&self, tenant: TenantId, record: Record) -> Result<RecordId> {
        // 1. Compute Hilbert key from record dimensions
        // 2. Write to WAL
        // 3. Find target tile via tile_map
        // 4. Append to tile's Arrow batch
        // 5. If tile > 48KB, split into two tiles
        // 6. Update learned index
        // 7. Notify reactive subscription engine
    }
    
    /// Prepare tiles for Cerebras loading
    pub fn export_tiles_for_cerebras(&self, tenant: TenantId) -> Vec<CerebrasTile> {
        // Serialize each tile into exactly the format
        // that the CSL data_load kernel expects
        // Each CerebrasTile is ≤ 48KB
    }
}
```

**Key dependencies:** `arrow-rs`, `parquet`, `dashmap`, `tokio`

### 3.7 norndb-reactive

RethinkDB-inspired changefeed engine.

```rust
/// Track which key ranges a subscription depends on
pub struct DependencyTracker {
    /// Interval tree: Hilbert range → set of subscriptions
    pub range_tree: IntervalTree<HilbertKey, SubscriptionId>,
}

impl DependencyTracker {
    /// When a write happens, find affected subscriptions
    pub fn find_affected(&self, key: HilbertKey) -> Vec<SubscriptionId> {
        self.range_tree.query_point(key).collect()
    }
    
    /// Re-evaluate affected subscriptions and push diffs
    pub async fn process_write(&self, write: &Write) -> Vec<SubscriptionDiff> {
        let affected = self.find_affected(write.hilbert_key);
        let mut diffs = Vec::new();
        for sub_id in affected {
            let sub = self.subscriptions.get(&sub_id);
            let old_result = sub.cached_result();
            let new_result = self.re_evaluate(sub).await;
            if let Some(diff) = compute_diff(old_result, new_result) {
                diffs.push(diff);
            }
        }
        diffs
    }
}
```

**Key dependencies:** `tokio`, `interval-tree` (or custom), `dashmap`

### 3.8 norndb-auth

Authentication and tenant-scoped Row-Level Security.

```rust
/// Tenant-scoped RLS — auto-injected into every query
pub struct SecurityPolicy {
    pub entity: EntityType,
    pub rules: Vec<RlsRule>,
}

pub struct RlsRule {
    pub action: Action,            // Read, Write, Delete
    pub condition: Predicate,       // e.g., $auth.role == "admin"
}

/// The auth context injected into every query
pub struct AuthContext {
    pub tenant: TenantId,
    pub user_id: UserId,
    pub role: String,
    pub permissions: Vec<Permission>,
}

/// Every query gets scoped automatically
pub fn scope_query(plan: &mut LogicalPlan, auth: &AuthContext) {
    // 1. Inject: WHERE tenant_id = auth.tenant (ALWAYS)
    // 2. Inject: RLS predicates based on entity type and role
    // 3. Filter accessible fields based on permissions
    // This happens in the planner, before execution
    // The executor NEVER sees unscoped data
}
```

**Key dependencies:** `jsonwebtoken`, `argon2` (password hashing)

### 3.9 norndb-server

The network-facing layer.

```rust
/// Server configuration
pub struct ServerConfig {
    pub bind_addr: SocketAddr,          // e.g., 0.0.0.0:5432
    pub tls: Option<TlsConfig>,
    pub max_connections: usize,
    pub available_executors: Vec<ExecutorConfig>,
}

/// Supported protocols
/// All protocols converge to the same query engine
pub async fn start_server(config: ServerConfig) -> Result<()> {
    let engine = QueryEngine::new(config.available_executors);
    
    tokio::join!(
        // HTTP/REST for simple queries and admin
        start_http_server(config.bind_addr, engine.clone()),
        // WebSocket for reactive subscriptions  
        start_ws_server(config.bind_addr, engine.clone()),
        // Binary wire protocol for high-performance clients
        start_wire_server(config.bind_addr, engine.clone()),
    );
}
```

**Key dependencies:** `tokio`, `axum` (HTTP), `tokio-tungstenite` (WebSocket)

---

## 4. CSL Kernels — Detailed Design

### 4.1 PE Memory Layout (48KB)

```
Offset    Size     Contents
──────────────────────────────────
0x0000    64B      Tile metadata
                     - tile_id (8B)
                     - hilbert_range_min (16B)
                     - hilbert_range_max (16B)
                     - record_count (4B)
                     - record_size (4B)
                     - padding (16B)

0x0040    40B      Primary learned index
                     - slope (8B, f64)
                     - intercept (8B, f64)
                     - min_key (8B, truncated u64)
                     - max_key (8B, truncated u64)
                     - max_error (4B, u32)
                     - padding (4B)

0x0068    40B      Secondary index 1 (optional)
0x0090    40B      Secondary index 2 (optional)

0x00B8    200B     Category bitmap index (optional)
                     - Up to 8 categories × 25B each
                     - Each is a bitfield over local records

0x0180    1KB      HNSW neighbor list (optional)
                     - For vector search
                     - Local graph edges to nearby PEs

0x0580    ~46KB    Record data (sorted by Hilbert key)
                     - Each record: fixed-size struct
                     - Layout depends on entity schema
                     
0xBFFF              End of 48KB SRAM
```

### 4.2 Record Format on PE

```c
// CSL struct — fixed size per entity type
// This example: 64 bytes per record = ~700 records per PE

struct Record {
    hilbert_key: u64,        // 8B  — truncated from u128
    id: u32,                 // 4B  — local record ID
    
    // Structured dimensions (used for predicates)
    price: f32,              // 4B
    stock: u16,              // 2B
    category: u8,            // 1B  — enum index
    flags: u8,               // 1B  — bitfield
    
    // Compact embedding (quantized)
    // Full embedding lives in MemoryX or host
    // This is the projected, quantized version for KNN
    embedding_q: [i8; 32],   // 32B — quantized to int8
    
    // Graph edges (compact)
    edge_count: u8,          // 1B
    edges: [u16; 5],         // 10B — up to 5 edges, PE-local IDs
    
    padding: u8,             // 1B
};
// Total: 64 bytes
```

### 4.3 Kernel: spatial_scan.csl

```c
// spatial_scan.csl — Full scan with predicate evaluation
// Every PE evaluates predicates on its local records
// Results flow EAST to reducer column

const RECORD_SIZE: u16 = 64;
const MAX_RECORDS: u16 = 700;

// Tile metadata (loaded during data_load)
var tile_id: u32 = 0;
var record_count: u16 = 0;
var records: [MAX_RECORDS]Record = undefined;

// Learned index
var idx_slope: f64 = 0.0;
var idx_intercept: f64 = 0.0;
var idx_max_error: u32 = 0;

// Query parameters (received via wavelet)
var query_op: u8 = 0;          // 0=scan, 1=point, 2=range, 3=knn
var pred_field: u8 = 0;        // which field to filter
var pred_op: u8 = 0;           // 0=eq, 1=lt, 2=gt, 3=le, 4=ge, 5=ne
var pred_value: f32 = 0.0;     // comparison value

// Colors for routing
const QUERY_COLOR: color = @get_color(0);    // incoming query
const RESULT_COLOR: color = @get_color(1);   // outgoing results
const SOUTH_COLOR: color = @get_color(2);    // forward query south

// Task: receive query, evaluate, send matches
task recv_query(wavelet_data: u32) void {
    // Decode query parameters from wavelet
    query_op = @as(u8, wavelet_data & 0xFF);
    pred_field = @as(u8, (wavelet_data >> 8) & 0xFF);
    pred_op = @as(u8, (wavelet_data >> 16) & 0xFF);
    
    // Second wavelet carries the comparison value
    // (simplified — real implementation uses multi-wavelet protocol)
    
    // Evaluate predicate on all local records
    var match_count: u16 = 0;
    for (var i: u16 = 0; i < record_count; i += 1) {
        if (eval_predicate(records[i])) {
            // Send matching record east toward reducer
            @fmovs(RESULT_COLOR, records[i].id);
            match_count += 1;
        }
    }
    
    // Forward query south (broadcast pattern)
    @fmovs(SOUTH_COLOR, wavelet_data);
}

fn eval_predicate(rec: Record) bool {
    var field_val: f32 = 0.0;
    
    // Select field value
    switch (pred_field) {
        0 => field_val = rec.price,
        1 => field_val = @as(f32, rec.stock),
        // ... more fields
    }
    
    // Evaluate comparison
    switch (pred_op) {
        0 => return field_val == pred_value,    // eq
        1 => return field_val < pred_value,     // lt
        2 => return field_val > pred_value,     // gt
        3 => return field_val <= pred_value,    // le
        4 => return field_val >= pred_value,    // ge
        5 => return field_val != pred_value,    // ne
    }
    return false;
}

// Bind task to query color
comptime {
    @bind_data_task(QUERY_COLOR, recv_query);
    @set_local_color_config(QUERY_COLOR, .{ .routes = .{ .rx = .WEST, .tx = .RAMP } });
    @set_local_color_config(RESULT_COLOR, .{ .routes = .{ .rx = .RAMP, .tx = .EAST } });
    @set_local_color_config(SOUTH_COLOR, .{ .routes = .{ .rx = .RAMP, .tx = .SOUTH } });
}
```

### 4.4 Kernel: knn_search.csl

```c
// knn_search.csl — K-nearest-neighbor on quantized embeddings
// Each PE computes distances to query vector for local records
// Maintains local top-K heap, merges with neighbor results

const K: u16 = 10;
const EMBED_DIM: u16 = 32;  // quantized dimension

// Local top-K heap
var heap_ids: [K]u32 = undefined;
var heap_dists: [K]f32 = undefined;
var heap_size: u16 = 0;

// Query vector (received from host)
var query_embed: [EMBED_DIM]i8 = undefined;

task recv_knn_query(wavelet_data: u32) void {
    // Query embedding loaded via multi-wavelet transfer
    // (simplified here)
    
    // Compute distance to every local record
    for (var i: u16 = 0; i < record_count; i += 1) {
        var dist: f32 = compute_l2_distance(
            query_embed, 
            records[i].embedding_q
        );
        
        // Insert into heap if closer than current worst
        if (heap_size < K) {
            heap_insert(records[i].id, dist);
        } else if (dist < heap_dists[0]) {  // heap[0] = max (worst)
            heap_replace_max(records[i].id, dist);
        }
    }
    
    // Send local top-K east for merging
    for (var i: u16 = 0; i < heap_size; i += 1) {
        @fmovs(RESULT_COLOR, heap_ids[i]);
        @fmovs(RESULT_COLOR, @bitcast(u32, heap_dists[i]));
    }
}

fn compute_l2_distance(a: [EMBED_DIM]i8, b: [EMBED_DIM]i8) f32 {
    var sum: i32 = 0;
    for (var d: u16 = 0; d < EMBED_DIM; d += 1) {
        var diff: i32 = @as(i32, a[d]) - @as(i32, b[d]);
        sum += diff * diff;
    }
    return @as(f32, sum);  // skip sqrt, relative order preserved
}

// Max-heap operations (standard, small K so array-based is fine)
fn heap_insert(id: u32, dist: f32) void { /* ... */ }
fn heap_replace_max(id: u32, dist: f32) void { /* ... */ }
```

### 4.5 Kernel: aggregate.csl

```c
// aggregate.csl — Distributed aggregation with reduce tree
// Each PE computes local partial, reduces down columns

const AGG_COUNT: u8 = 0;
const AGG_SUM: u8 = 1;
const AGG_MIN: u8 = 2;
const AGG_MAX: u8 = 3;

var agg_op: u8 = 0;
var local_count: u32 = 0;
var local_sum: f64 = 0.0;
var local_min: f32 = 3.4e38;   // f32 max
var local_max: f32 = -3.4e38;  // f32 min

task recv_agg_query(wavelet_data: u32) void {
    agg_op = @as(u8, wavelet_data & 0xFF);
    pred_field = @as(u8, (wavelet_data >> 8) & 0xFF);
    
    // Compute local aggregate
    for (var i: u16 = 0; i < record_count; i += 1) {
        if (eval_predicate(records[i])) {
            var val: f32 = get_field(records[i], pred_field);
            local_count += 1;
            local_sum += @as(f64, val);
            if (val < local_min) local_min = val;
            if (val > local_max) local_max = val;
        }
    }
    
    // Send partial result east for column reduction
    // Format: [op, count, sum_hi, sum_lo, min, max]
    @fmovs(RESULT_COLOR, local_count);
    @fmovs(RESULT_COLOR, @bitcast(u32, local_sum));  // simplified
}

// Reducer PE (rightmost column) accumulates partials
task recv_partial(wavelet_data: u32) void {
    // Accumulate into running total
    // When all PEs in row have reported, send down column
    // Final PE sends to egress
}
```

### 4.6 Layout Program (comptime mesh configuration)

```c
// layout.csl — Configures the PE mesh for iDB
// This is the top-level program that assigns roles to PEs

const GRID_WIDTH: u16 = 750;
const GRID_HEIGHT: u16 = 994;

// PE roles
const ROLE_INGRESS: u8 = 0;    // Row 0: receive queries from host
const ROLE_DATA: u8 = 1;       // Rows 1-992: hold spatial tiles
const ROLE_REDUCER: u8 = 2;    // Column 749: reduce results
const ROLE_EGRESS: u8 = 3;     // Row 993: send results to host

layout {
    @set_rectangle(GRID_WIDTH, GRID_HEIGHT);
    
    // Row 0: Ingress PEs
    for (var x: u16 = 0; x < GRID_WIDTH; x += 1) {
        @set_tile_code(x, 0, "ingress.csl", .{
            .pe_x = x,
            .pe_y = 0,
            .role = ROLE_INGRESS,
        });
    }
    
    // Rows 1-992, Columns 0-748: Data PEs
    for (var y: u16 = 1; y < GRID_HEIGHT - 1; y += 1) {
        for (var x: u16 = 0; x < GRID_WIDTH - 1; x += 1) {
            @set_tile_code(x, y, "data_pe.csl", .{
                .pe_x = x,
                .pe_y = y,
                .role = ROLE_DATA,
                .tile_id = (y - 1) * (GRID_WIDTH - 1) + x,
            });
        }
    }
    
    // Column 749: Reducer PEs
    for (var y: u16 = 1; y < GRID_HEIGHT - 1; y += 1) {
        @set_tile_code(GRID_WIDTH - 1, y, "reducer.csl", .{
            .pe_x = GRID_WIDTH - 1,
            .pe_y = y,
            .role = ROLE_REDUCER,
        });
    }
    
    // Row 993: Egress PEs
    for (var x: u16 = 0; x < GRID_WIDTH; x += 1) {
        @set_tile_code(x, GRID_HEIGHT - 1, "egress.csl", .{
            .pe_x = x,
            .pe_y = GRID_HEIGHT - 1,
            .role = ROLE_EGRESS,
        });
    }
}
```

---

## 5. GPU Kernels (WGSL via wgpu)

```wgsl
// predicate_eval.wgsl — Parallel predicate evaluation on GPU
// Fallback when Cerebras not available

struct Record {
    hilbert_key: u32,
    price: f32,
    stock: u32,
    category: u32,
}

struct Query {
    pred_field: u32,
    pred_op: u32,
    pred_value: f32,
    result_count: atomic<u32>,
}

@group(0) @binding(0) var<storage, read> records: array<Record>;
@group(0) @binding(1) var<storage, read_write> query: Query;
@group(0) @binding(2) var<storage, read_write> results: array<u32>;

@compute @workgroup_size(256)
fn eval_predicate(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= arrayLength(&records)) { return; }
    
    let rec = records[idx];
    var field_val: f32;
    
    switch query.pred_field {
        case 0u: { field_val = rec.price; }
        case 1u: { field_val = f32(rec.stock); }
        default: { return; }
    }
    
    var matches: bool = false;
    switch query.pred_op {
        case 0u: { matches = field_val == query.pred_value; }
        case 1u: { matches = field_val < query.pred_value; }
        case 2u: { matches = field_val > query.pred_value; }
        default: { matches = false; }
    }
    
    if (matches) {
        let pos = atomicAdd(&query.result_count, 1u);
        results[pos] = idx;
    }
}
```

---

## 6. Host Code (Python — Cerebras Interface)

```python
# cerebras/host/controller.py
# Bridge between Rust database and CSL kernels on the wafer

from cerebras.sdk.runtime import SdkRuntime
import numpy as np

class CerebrasController:
    """Manages communication with the WSE chip."""
    
    def __init__(self, binary_path: str, simulated: bool = True):
        self.runtime = SdkRuntime(binary_path)
        self.simulated = simulated
        self.loaded = False
    
    def load_tiles(self, tiles: list[dict]):
        """Load spatial tiles onto PE SRAM.
        
        Each tile dict contains:
          - tile_id: int
          - pe_x, pe_y: int (target PE coordinates)  
          - records: np.ndarray (fixed-size record array)
          - learned_index: dict (slope, intercept, max_error)
        """
        for tile in tiles:
            pe = (tile['pe_x'], tile['pe_y'])
            
            # Pack tile metadata (64 bytes)
            metadata = self._pack_metadata(tile)
            self.runtime.memcpy_h2d(pe, metadata, offset=0x0000)
            
            # Pack learned index (40 bytes)
            index = self._pack_learned_index(tile['learned_index'])
            self.runtime.memcpy_h2d(pe, index, offset=0x0040)
            
            # Pack record data
            records = self._pack_records(tile['records'])
            self.runtime.memcpy_h2d(pe, records, offset=0x0580)
        
        self.loaded = True
    
    def execute_scan(self, predicates: list, agg_op: str = None):
        """Execute a spatial scan across all PEs.
        
        Returns: list of matching record IDs or aggregate result
        """
        # Encode query as wavelet data
        query_wavelet = self._encode_query(predicates, agg_op)
        
        # Send to ingress PEs (row 0)
        self.runtime.launch("recv_query", query_wavelet)
        
        # Collect results from egress PEs (last row)
        results = self.runtime.memcpy_d2h(
            egress_pes, 
            result_buffer_size
        )
        
        return self._decode_results(results)
    
    def execute_knn(self, query_vector: np.ndarray, k: int):
        """Execute KNN search across all PEs."""
        # Quantize query vector to int8
        query_q = self._quantize_embedding(query_vector)
        
        # Load query vector onto all PEs
        # (broadcast via ingress row)
        self.runtime.launch("recv_knn_query", query_q, k)
        
        # Collect and merge top-K results
        raw_results = self.runtime.memcpy_d2h(...)
        return self._merge_topk(raw_results, k)
    
    def _pack_metadata(self, tile):
        """Pack tile metadata into 64-byte buffer."""
        buf = np.zeros(64, dtype=np.uint8)
        # ... struct packing
        return buf
    
    def _quantize_embedding(self, vec: np.ndarray) -> np.ndarray:
        """Quantize float32 embedding to int8 for PE storage."""
        scale = 127.0 / np.max(np.abs(vec))
        return (vec * scale).astype(np.int8)
```

```python
# cerebras/host/perf_estimator.py
# Analytical performance model — no hardware needed

class CerebrasEstimator:
    """Estimate query performance on WSE without running it."""
    
    CLOCK_HZ = 1_000_000_000       # 1 GHz
    NUM_DATA_PES = 742_252          # 748 * 992 data PEs
    SRAM_PER_PE = 48 * 1024         # 48KB
    RECORD_OVERHEAD = 0x580         # bytes reserved for metadata/indexes
    USABLE_SRAM = SRAM_PER_PE - RECORD_OVERHEAD  # ~46.6KB
    HOP_LATENCY_NS = 1
    MAX_HOPS_X = 750
    MAX_HOPS_Y = 994
    
    def estimate(self, query_type: str, record_size: int, 
                 total_records: int, **kwargs) -> dict:
        
        records_per_pe = self.USABLE_SRAM // record_size
        pes_needed = total_records // records_per_pe
        
        if query_type == "full_scan":
            pred_cycles = kwargs.get('predicate_cycles', 10)
            scan_time = records_per_pe * pred_cycles / self.CLOCK_HZ
            reduce_time = self.MAX_HOPS_Y * self.HOP_LATENCY_NS * 1e-9
            total = scan_time + reduce_time
            
        elif query_type == "point_lookup":
            # Route to PE + binary search locally
            route_time = 500 * self.HOP_LATENCY_NS * 1e-9  # avg hops
            search_cycles = 10 * 5  # log2(700) * 5 cycles/compare
            search_time = search_cycles / self.CLOCK_HZ
            total = route_time + search_time
            
        elif query_type == "knn":
            k = kwargs.get('k', 10)
            embed_dim = kwargs.get('embed_dim', 32)
            dist_cycles = embed_dim * 3  # multiply + subtract + accumulate
            scan_time = records_per_pe * dist_cycles / self.CLOCK_HZ
            merge_time = self.MAX_HOPS_Y * self.HOP_LATENCY_NS * 1e-9
            total = scan_time + merge_time
            
        elif query_type == "aggregate":
            pred_cycles = kwargs.get('predicate_cycles', 10)
            agg_cycles = 5  # accumulate
            scan_time = records_per_pe * (pred_cycles + agg_cycles) / self.CLOCK_HZ
            reduce_time = self.MAX_HOPS_Y * self.HOP_LATENCY_NS * 1e-9
            total = scan_time + reduce_time
        
        # Compare with CPU and GPU
        cpu_time = total_records * kwargs.get('predicate_cycles', 10) / 3e9
        gpu_time = total_records * kwargs.get('predicate_cycles', 10) / (3e9 * 5000)
        
        return {
            'cerebras_us': total * 1e6,
            'cpu_estimated_ms': cpu_time * 1e3,
            'gpu_estimated_ms': gpu_time * 1e3,
            'speedup_vs_cpu': cpu_time / total,
            'speedup_vs_gpu': gpu_time / total,
            'pes_used': min(pes_needed, self.NUM_DATA_PES),
            'records_per_pe': records_per_pe,
        }
```

---

## 7. Development Phases & Milestones

### Phase 0: Foundation (Weeks 1-4)
**Goal:** Learn CSL, validate basic spatial concept

```
Week 1:
  □ Install Cerebras SDK simulator on i7 laptop (WSL2)
  □ Run all SDK example programs
  □ Understand: tasks, colors, wavelets, routing
  □ Write: "hello world" — each PE stores a number,
    broadcast a query, PEs respond if match

Week 2:
  □ Implement Hilbert curve in Python (reference impl)
  □ Port Hilbert encoding to CSL (runs on PE)
  □ Test: encode/decode roundtrip on 5x5 grid
  □ Understand: memcpy framework, host↔device data transfer

Week 3:
  □ Design record format (64 bytes, fixed layout)
  □ Load 100 records onto a 5x5 grid (4 records per PE)
  □ Implement point lookup: host sends key → 
    route to PE → binary search → return record
  □ Benchmark: measure cycles on simulator

Week 4:  
  □ Implement spatial scan with predicate
  □ All PEs evaluate "price < 500" on local records
  □ Results flow east, collected by rightmost column
  □ First end-to-end query working on simulator
  □ RECORD EPISODE 1 of YouTube series
```

### Phase 1: Core Kernels (Weeks 5-12)
**Goal:** All query types working on simulator

```
Week 5-6: Range scan with Hilbert ranges
  □ Query region → Hilbert ranges → target PEs
  □ Only relevant PEs evaluate (skip others)
  □ Performance model: compare full scan vs range scan

Week 7-8: Aggregation with reduce tree
  □ count, sum, avg, min, max
  □ Column-wise reduction pattern
  □ Verify correctness against Python reference

Week 9-10: KNN vector search
  □ Quantized embeddings stored per PE
  □ L2 distance computation
  □ Local top-K heap
  □ Merge results across PEs

Week 11-12: Performance estimation + paper draft
  □ Build analytical perf model (Python)
  □ Run all queries on simulator, verify correctness
  □ Extrapolate to full wafer performance
  □ Begin writing paper
  □ FILE PROVISIONAL PATENT
```

### Phase 2: Rust Database Shell (Weeks 13-24)
**Goal:** Working database with CPU backend

```
Week 13-14: norndb-core + norndb-parser
  □ Core types: Record, Tile, SpatialPoint
  □ iQL lexer and parser
  □ AST types
  □ Basic test suite

Week 15-16: norndb-hilbert + norndb-index
  □ Hilbert encoding/decoding in Rust
  □ Learned index training (linear regression)
  □ Secondary index structures (BTree, Bitmap)

Week 17-18: norndb-storage
  □ Arrow record batch management
  □ Tile-based storage (in-memory)
  □ WAL for durability
  □ Insert/update/delete operations

Week 19-20: norndb-planner + norndb-executor-cpu
  □ Logical plan generation
  □ Tenant scoping injection
  □ CPU executor using DataFusion
  □ All query types working: scan, point, range, 
    aggregate, KNN (using hora or hnsw)

Week 21-22: norndb-server + norndb-auth
  □ TCP server with binary wire protocol
  □ WebSocket for reactive subscriptions
  □ HTTP REST API
  □ JWT authentication
  □ Tenant scoping and RLS

Week 23-24: norndb-reactive + TypeScript client
  □ Changefeed engine (dependency tracking)
  □ Subscription management
  □ TypeScript SDK: @norndb/client
  □ Integration tests
  □ SUBMIT PAPER to arXiv
  □ EMAIL developer@cerebras.ai
```

### Phase 3: Cerebras Integration (Weeks 25-36)
**Goal:** Connect Rust DB to CSL kernels

```
Week 25-28: norndb-executor-cerebras
  □ Rust → Python FFI for Cerebras host code
  □ Tile export: Rust storage → Cerebras PE format
  □ Query dispatch: physical plan → CSL kernel invocation
  □ Result collection: CSL output → Arrow record batches
  □ Hardware router: auto-select CPU vs Cerebras

Week 29-32: GPU executor
  □ WGSL compute shaders for predicate eval
  □ wgpu integration in Rust
  □ GPU path for scans and KNN
  □ Three-way comparison: CPU vs GPU vs Cerebras

Week 33-36: Integration + benchmarking
  □ End-to-end benchmarks: TPC-H adapted for iDB
  □ Vector benchmarks: SIFT1M, GloVe
  □ Multi-tenant benchmark: 100 simulated tenants
  □ Update paper with real/estimated numbers
  □ SUBMIT PAPER to SIGMOD/VLDB
```

### Phase 4: Production Hardening (Weeks 37-52)
**Goal:** Usable by real applications

```
  □ Crash recovery from WAL
  □ Backup and restore
  □ Query timeout and cancellation
  □ Memory pressure management
  □ Connection pooling
  □ Monitoring and metrics (Prometheus)
  □ Python SDK
  □ Documentation (mdBook)
  □ Docker image
  □ norndb-cli tool
  □ Norn ERP integration as first real user
```

---

## 8. Documentation Plan

### 8.1 CSL-Specific Documentation (cerebras/docs/)

```
MEMORY_LAYOUT.md
  - PE SRAM map (offset table above)
  - Record format specification
  - How to add new fields / change record size
  - Memory budget calculator

ROUTING_PATTERNS.md  
  - Broadcast: query from ingress to all PEs
  - Row reduction: results flow east to reducer
  - Column reduction: partials flow south to egress
  - Point routing: host → specific PE via coordinates
  - Multi-wavelet transfer: sending large data (embeddings)

KERNEL_API.md
  - List of all kernels with signatures
  - Input/output wavelet formats
  - Expected behavior and invariants
  - Error handling patterns

PERFORMANCE_MODEL.md
  - Cycle counts per operation
  - Network latency model
  - How to estimate wall-clock time
  - Comparison methodology vs CPU/GPU
  
ADDING_A_KERNEL.md
  - Step-by-step: how to add a new query type
  - CSL kernel template
  - Host-side dispatch code
  - Rust executor integration
  - Testing on simulator
```

### 8.2 User Documentation (docs/book/)

```
mdBook structure:

1. Getting Started
   1.1 Installation
   1.2 Quick Start (5-minute tutorial)
   1.3 Concepts (entities, dimensions, spatial tiling)

2. iQL Query Language
   2.1 Creating entities
   2.2 Inserting data
   2.3 Querying (filters, ranges)
   2.4 Aggregations
   2.5 Graph traversal
   2.6 Vector/semantic search
   2.7 Reactive subscriptions
   2.8 Time travel

3. Data Modeling
   3.1 Entities and fields
   3.2 Choosing dimensions (what goes into Hilbert key)
   3.3 Embeddings and semantic fields
   3.4 Graph relationships
   3.5 Tenant scoping

4. Architecture
   4.1 How spatial tiling works
   4.2 Learned indexes
   4.3 CPU/GPU/Cerebras execution paths
   4.4 Storage engine internals

5. Client SDKs
   5.1 TypeScript
   5.2 Python
   5.3 Rust

6. Operations
   6.1 Configuration
   6.2 Monitoring
   6.3 Backup/Restore
   6.4 Scaling
```

### 8.3 Academic Paper Structure

```
"NornDB: N-Dimensional Spatial Indexing on 
 Wafer-Scale Architecture"

Venue target: SIGMOD, VLDB, or SOSP

1. Abstract (250 words)
2. Introduction (1.5 pages)
   - The data management fragmentation problem
   - Near-data processing opportunity
   - Contributions summary
3. Background (2 pages)
   - Hilbert curves and spatial indexing
   - Learned indexes
   - Cerebras WSE architecture
   - HTAP and multi-model databases
4. System Design (4 pages)
   - N-dimensional spatial tiling
   - PE memory layout and record format
   - Query execution on mesh (broadcast/reduce)
   - Learned index hierarchy
   - Tenant-scoped sharding
5. Implementation (2 pages)
   - CSL kernel design
   - Host ↔ device protocol
   - Rust database integration
6. Evaluation (3 pages)
   - Experimental setup (simulator + analytical model)
   - Micro-benchmarks: point lookup, scan, KNN, aggregate
   - Comparison: CPU (DataFusion), GPU (cuDF), Cerebras
   - Multi-tenant scalability
   - Memory efficiency analysis
7. Related Work (1 page)
   - GPU databases (HeavyDB, Sirius)
   - Learned indexes (RMI, ALEX, Tsunami)
   - Multi-model databases (SurrealDB, ArangoDB)
   - Near-data processing (PIM, SmartSSD)
8. Discussion & Future Work (1 page)
   - Limitations (48KB constraint, CSL complexity)
   - Multi-wafer scaling
   - Lattice-based encrypted computation
9. Conclusion (0.5 page)
```

---

## 9. Dependency Map

```
RUST CRATES:

arrow = "52"              # In-memory columnar format
parquet = "52"            # On-disk columnar storage  
datafusion = "41"         # CPU query execution
tokio = "1"               # Async runtime
axum = "0.7"              # HTTP server
tokio-tungstenite = "0.23" # WebSocket
serde = "1"               # Serialization
dashmap = "6"             # Concurrent hashmap
bytes = "1"               # Byte buffer management
ulid = "1"                # Unique IDs
logos = "0.14"            # Lexer generator
miette = "7"              # Error reporting
ndarray = "0.16"          # Matrix operations (Hilbert, projections)
roaring = "0.10"          # Bitmap indexes
wgpu = "23"               # GPU compute (optional)
hora = "0.1"              # HNSW approximate nearest neighbor
pyo3 = "0.22"             # Python FFI (for Cerebras host code)
jsonwebtoken = "9"        # JWT auth
tracing = "0.1"           # Structured logging
prometheus = "0.13"       # Metrics
criterion = "0.5"         # Benchmarking

PYTHON (Cerebras host):

cerebras-sdk = "2.9"      # Cerebras SDK
numpy                      # Array operations
pytest                     # Testing

CSL:

Cerebras SDK 1.4+          # CSL compiler + simulator
collective_comms library   # Broadcast, scatter, gather, reduce
```

---

## 10. Key Design Decisions (RFCs)

```
RFC-001: Why Hilbert curves over Z-order curves
  Hilbert has better locality preservation in 2D+
  Z-order has simpler computation but worse clustering
  Decision: Hilbert for primary, Z-order as fast fallback

RFC-002: Why 64-byte fixed records on Cerebras
  Variable-length records complicate PE memory management
  64B fits 700 records in 48KB with room for indexes
  Structured fields are fixed-size anyway
  Variable data (text, large embeddings) lives on host/MemoryX
  Decision: Fixed-size records on PEs, variable data off-chip

RFC-003: Why learned indexes over B-trees on PEs
  B-trees waste memory on pointers and internal nodes
  48KB is too small for tree overhead
  Learned index: 40 bytes for same functionality
  Linear model sufficient for sorted data within a PE
  Decision: Learned index on PEs, B-tree on host for secondary

RFC-004: Why separate Rust and CSL codebases
  CSL has no package ecosystem, no async, no networking
  Rust has the best systems programming ecosystem
  CSL is ONLY for the hot path on the chip
  Everything else (parsing, planning, networking, auth) in Rust
  Decision: Rust for 90% of code, CSL for PE kernels only

RFC-005: Why Arrow as the internal format
  Zero-copy between Rust and DataFusion
  Columnar = fast for analytics (OLAP path)
  Parquet on disk = Arrow in memory (same ecosystem)
  wgpu can read Arrow buffers directly
  Decision: Arrow everywhere, Parquet for cold storage

RFC-006: Why tenant scoping at the query planner level
  Injecting WHERE tenant_id = X in the planner means
  executors NEVER see cross-tenant data
  Even bugs in the executor can't leak data
  On Cerebras: each tenant's tiles on separate PEs (future)
  Decision: Planner-level scoping, not executor-level
```

---

## 11. Testing Strategy

```
UNIT TESTS (per crate):
  Every Rust crate has its own test suite
  Run with: cargo test --workspace
  Target: >80% coverage on core, parser, planner, hilbert

SIMULATOR TESTS (CSL):
  Each kernel has a Python test script
  Runs on Cerebras fabric simulator
  Verifies: correctness on small grids (5x5, 10x10, 20x20)
  Run with: cd cerebras/simulator && pytest

INTEGRATION TESTS:
  End-to-end: client → server → executor → storage → result
  Tests all three backends: CPU, GPU (if available), Cerebras (simulator)
  Multi-tenant isolation tests
  Reactive subscription tests

BENCHMARKS:
  TPC-H adapted for iDB (modified for single-node, multi-tenant)
  SIFT1M for vector search accuracy
  Custom: multi-tenant concurrent workload
  Analytical model comparison: predicted vs actual performance
  Run with: cargo bench (Criterion)

PROPERTY TESTS:
  Hilbert encoding/decoding roundtrip for arbitrary dimensions
  Learned index: lookup always finds the record (given max_error)
  Tenant scoping: no query ever returns cross-tenant data
  Run with: cargo test (proptest crate)
```

---

## 12. What to Build First (Priority Order)

```
1.  Cerebras simulator "hello world"          ← THIS WEEK
2.  Hilbert curve in Python + CSL             ← Week 1-2
3.  spatial_scan.csl on 5x5 grid              ← Week 3-4
4.  Performance estimator (Python)            ← Week 5
5.  point_lookup.csl                          ← Week 6
6.  knn_search.csl                            ← Week 7-8
7.  aggregate.csl                             ← Week 9-10
8.  Paper draft + patent filing               ← Week 11-12
9.  norndb-parser (iQL in Rust)               ← Week 13-14
10. norndb-storage (Arrow tiles)              ← Week 15-16
11. norndb-executor-cpu (DataFusion)          ← Week 17-20
12. norndb-server (TCP + WebSocket)           ← Week 21-22
13. TypeScript client SDK                     ← Week 23-24
14. Connect Rust ↔ Cerebras                   ← Week 25-28
15. GPU executor (wgpu)                       ← Week 29-32
```

Items 1-8 are the research track (CSL + paper).
Items 9-13 are the product track (Rust database).
Items 14-15 snap them together.

Both tracks can run in parallel if you have collaborators,
but solo, do the CSL/research track first — it's the novel
contribution that gets you Cerebras access and paper credit.
The Rust database is "just" good engineering.
```
