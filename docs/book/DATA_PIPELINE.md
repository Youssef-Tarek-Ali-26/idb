# The Full Data Pipeline

Every step, from "user creates a record" to "user queries it back."

## Part 1: Schema Definition (One-Time Setup)

Before anything, you define what your data looks like:

```
define entity Product {
  // Structured fields
  name: string            // stored on host, NOT on PE
  description: string     // stored on host, NOT on PE
  price: f32              // → PE structured dim
  stock: u16              // → PE structured dim
  category: enum(rings, necklaces, bracelets)  // → PE structured dim
  created_at: timestamp   // → PE structured dim

  // Semantic fields
  image: file             // stored in blob store
  image_embedding: vector(384)  // auto-generated from image
  text_embedding: vector(384)   // auto-generated from name+description

  // Graph relationships
  belongs_to: -> Brand
  supplied_by: -> Supplier

  // Spatial configuration
  hilbert_dims: [price, stock, category, created_at, text_embedding]
  //            ↑ these fields determine WHERE in N-space this record lives

  scoped by tenant
}
```

From this schema, the system derives:

```
PE record format: 64 bytes (fixed, computed from schema)
  - Which fields are structured dims (stored on PE)
  - Which fields are embeddings (quantized, stored on PE)
  - Which fields are large (stored on host/blob store)

Hilbert configuration:
  - 4 structured dims + 20 projected embedding dims = 24 total dims
  - Bits per dim: 5 (enough for reasonable precision)
  - Total Hilbert key: 24 × 5 = 120 bits → u128

Random projection matrix:
  - 384-dim → 20-dim
  - Generated once, stored permanently
  - Same matrix used for all records and queries
```

---

## Part 2: Data Ingestion (Record Creation)

User sends:
```
create Product {
  name = "18K Gold Signet Ring"
  description = "Classic men's signet ring in 18 karat gold"
  price = 4500
  stock = 12
  category = "rings"
  image = file("signet.jpg")
  belongs_to = Brand("Norn Gold")
  supplied_by = Supplier("Cairo Goldworks")
}
```

### Step 1: Auth + Tenant Scoping (Rust — norndb-auth)

```
Request comes in with JWT token
Extract: tenant_id = "tenant_norn", user_id = "youssef"
Check: does this user have write permission on Product?
Tag record with tenant_id (this NEVER leaves the record)
```

### Step 2: Generate Embeddings (Rust — norndb-core)

```
text_embedding = embed("18K Gold Signet Ring Classic men's signet ring in 18 karat gold")
  → calls local sentence-transformers model
  → returns: float32[384] = [0.23, -0.41, 0.87, ...]

image_embedding = embed_image("signet.jpg")
  → calls local CLIP or similar
  → returns: float32[384] = [0.11, 0.55, -0.32, ...]
```

### Step 3: Store Large Data (Rust — norndb-storage + norndb-blob)

```
name, description → stored in Arrow record batch on host
  (too large / variable-length for PE SRAM)

image file → stored in blob store
  blob_id = hash("signet.jpg") → content-addressed storage

Full embeddings (float32[384]) → stored in Arrow batch on host
  (needed for re-ranking, re-indexing later)
```

### Step 4: Random Projection (Rust — norndb-hilbert)

```
Take text_embedding: float32[384]

Multiply by projection matrix R (384 × 20):
  projected = text_embedding × R
  → float32[20] = [1.42, -0.87, 2.31, ...]

This 20-dim vector captures most of the semantic meaning
Johnson-Lindenstrauss lemma guarantees distance preservation
```

### Step 5: Normalize Dimensions (Rust — norndb-hilbert)

```
Hilbert curves need integer coordinates.
Each dimension must be in range [0, 2^bits_per_dim).

For structured dims (known ranges):
  price: 4500 → normalize to [0, 31] → 14
    (assuming price range 0-10000, 5 bits)
  stock: 12 → normalize to [0, 31] → 12
  category: "rings" → enum index → 0
  created_at: 2026-02-25 → days since epoch → normalized → 28

For projected embedding dims:
  Each of 20 dims: float → normalize to [0, 31]
  projected[0]: 1.42 → normalized → 22
  projected[1]: -0.87 → normalized → 8
  ... etc for all 20

Final coordinates: [14, 12, 0, 28, 22, 8, 17, ...]
  = 24 integers, each in [0, 31]
```

### Step 6: Hilbert Encode (Rust — norndb-hilbert)

```
Input: 24 coordinates, each 5 bits
Algorithm: bit-interleaving (standard Hilbert curve)
Output: u128 hilbert_key = 284_719_553_281

This single number encodes the record's position
in 24-dimensional space. Records with similar keys
are NEARBY in all dimensions simultaneously.
```

### Step 7: Quantize Embedding for PE Storage (Rust — norndb-hilbert)

```
Take text_embedding: float32[384]

Project to fewer dims for PE storage:
  384 → 32 dims (different from the 20 used for Hilbert)
  (more dims = better KNN accuracy, costs more PE memory)

Quantize to int8:
  scale = 127.0 / max(abs(projected_32))
  embedding_q = (projected_32 * scale).round().to_int8()
  → int8[32] = [47, -103, 82, ...]

32 bytes instead of 1,536 bytes
KNN accuracy drops ~5-10% but fits in PE SRAM
```

### Step 8: Encode Graph Edges (Rust — norndb-storage)

```
belongs_to = Brand("Norn Gold")
  → Brand's record has hilbert_key = 991_283_441
  → Edge stored as: (edge_type=BELONGS_TO, target_key=991_283_441)
  → On PE: compact form, 2 bytes per edge (local PE offset or overflow flag)

supplied_by = Supplier("Cairo Goldworks")
  → Same treatment
```

### Step 9: Pack PE Record (Rust — norndb-storage)

```
Now we have everything for the 64-byte PE record:

Offset  Size  Field              Value
0       8B    hilbert_key        284_719_553_281 (truncated to u64)
8       4B    record_id          42
12      4B    price              4500.0 (f32)
16      2B    stock              12
18      1B    category           0 (rings)
19      1B    flags              0x00
20      32B   embedding_q        [47, -103, 82, ...] (int8 * 32)
52      1B    edge_count         2
53      10B   edges              [(BELONGS_TO, offset), (SUPPLIED_BY, offset)]
63      1B    padding            0x00

Total: 64 bytes exactly
```

### Step 10: Assign to Tile (Rust — norndb-storage)

```
hilbert_key = 284_719_553_281

tile_map lookup:
  Tile 4821 covers key range [284_700_000_000, 284_800_000_000)
  This record falls in tile 4821

Tile 4821 is assigned to PE (347, 512) on the Cerebras mesh

Append record to tile 4821's sorted record array
If tile now > 48KB → split into two tiles, reassign
```

### Step 11: Write to WAL (Rust — norndb-storage)

```
Before anything is committed:
  WAL entry = {
    transaction_id: 98712,
    operation: INSERT,
    tenant: "tenant_norn",
    entity: Product,
    record_id: 42,
    full_data: { ... all fields ... },
    hilbert_key: 284_719_553_281,
    tile_id: 4821,
    timestamp: 2026-02-25T03:42:17Z
  }

Write to WAL file on disk → fsync → confirmed durable
```

### Step 12: Update Learned Index (Rust — norndb-index)

```
Tile 4821 now has a new record inserted at sorted position 308

Re-fit the linear model:
  keys = [all hilbert keys in tile, sorted]
  positions = [0, 1, 2, ..., 700]
  slope, intercept = linear_regression(keys, positions)
  max_error = max(|predicted - actual|) across all records

  Updated: slope=0.02301, intercept=-47.18, max_error=3

This takes microseconds (it's just linear regression on 700 points)
```

### Step 13: Notify Reactive Subscriptions (Rust — norndb-reactive)

```
Check dependency tracker:
  "Does hilbert_key 284_719_553_281 fall in any subscription's range?"

  Subscription #17: "watch Product where price < 5000"
    → mapped to Hilbert ranges [..., (280_000_000_000, 290_000_000_000), ...]
    → YES, this record matches

  Re-evaluate subscription #17:
    Old result: 143 products
    New result: 144 products (this new ring added)
    Diff: { added: [Record(id=42, "18K Gold Signet Ring")] }

  Push diff to subscriber via WebSocket
```

### Step 14: Sync to Cerebras (if loaded) (Python — controller.py)

```
If Cerebras chip is live and this tenant's data is loaded:

  Pack the 64-byte record
  memcpy_h2d to PE (347, 512) at correct SRAM offset
  Update learned index on PE (memcpy 40 bytes)

  If chip is not loaded (cold start):
    Record lives in host memory (Arrow batch)
    Will be bulk-loaded next time tenant's data is loaded to chip
```

### INGESTION COMPLETE

The record now exists in:
- WAL (durability)
- Arrow batch on host (full data, variable-length fields)
- Blob store (image file)
- Tile 4821 in memory (compact 64-byte format)
- PE (347, 512) on Cerebras (if loaded)
- Reactive subscribers notified

---

## Part 3: Query Execution

User sends:
```
Product where price < 3000 and meaning("elegant traditional") | top(5, price desc)
```

### Step 1: Parse (Rust — norndb-parser)

```
Lexer tokens:
  IDENT("Product"), WHERE, IDENT("price"), LT, NUMBER(3000),
  AND, IDENT("meaning"), LPAREN, STRING("elegant traditional"), RPAREN,
  PIPE, IDENT("top"), LPAREN, NUMBER(5), COMMA, IDENT("price"),
  IDENT("desc"), RPAREN

AST:
  TopK {
    k: 5,
    order_by: (price, Desc),
    source: Filter {
      predicates: [
        StructuredPred { field: price, op: Lt, value: 3000 },
        SemanticPred { query: "elegant traditional" },
      ],
      source: EntityScan { entity: Product }
    }
  }
```

### Step 2: Analyze + Scope (Rust — norndb-planner)

```
Type check:
  price is f32
  3000 is numeric
  meaning() returns similarity score
  top(5) needs order_by field

Tenant scope injection:
  Add: WHERE tenant_id = "tenant_norn" (from auth context)
  This is injected BEFORE the optimizer sees the query
  No executor will ever see cross-tenant data

RLS check:
  User "youssef" has role "admin" → full read access
```

### Step 3: Embed Semantic Query (Rust — norndb-core)

```
"elegant traditional" → sentence-transformers
  → float32[384] = [0.55, -0.12, 0.78, ...]

Quantize for PE comparison:
  → project to 32 dims → quantize to int8
  → int8[32] = [71, -15, 99, ...]
  This is what gets broadcast to PEs for KNN
```

### Step 4: Map to Hilbert Ranges (Rust — norndb-hilbert)

```
price < 3000 constrains the price dimension to [0, 3000]
  → Hilbert normalized: [0, 9] (out of [0, 31])

meaning("elegant traditional") constrains embedding dims
  → The projected embedding defines a REGION in semantic space
  → But we don't know exact region, so: use ALL PEs for semantic,
    or use the price constraint to narrow to ~30% of PEs

Hilbert range mapping:
  price in [0, 9] across all other dims
  → generates ~50 Hilbert key ranges covering ~30% of key space
  → these ranges map to ~30% of PEs (about 220K PEs)

  Remaining 70% of PEs can skip this query entirely
```

### Step 5: Build Physical Plan (Rust — norndb-planner)

```
Hardware router checks:
  Cerebras available? YES → use it
  Record count for this tenant: 500,000
  Estimated Cerebras time: ~35us
  Estimated CPU time: ~50ms
  Decision: CEREBRAS

Physical plan:
  CerebrasExec {
    kernel: "topk_scan"    // combined scan + KNN + top-K
    predicates: [price < 3000]
    knn_query: int8[32]    // quantized "elegant traditional"
    knn_weight: 0.5        // balance structured vs semantic
    k: 5
    order_by: price DESC
    pe_ranges: [(0, 100_000), (200_000, 350_000), ...]  // Hilbert ranges → PEs
  }
```

### Step 6: Dispatch to Cerebras (Python — controller.py via Rust FFI)

```
Rust calls Python via pyo3:
  cerebras_controller.execute_topk_scan(
    predicates=[(PRICE, LT, 3000.0)],
    knn_query=int8[32],
    knn_weight=0.5,
    k=5,
    order_by=(PRICE, DESC),
    pe_mask=bitfield[745000]  // which PEs to activate
  )

Python host code:
  1. Encode query parameters into wavelet format
  2. Encode KNN query vector (multi-wavelet transfer)
  3. Set PE activation mask (skip irrelevant PEs)
  4. Launch kernel via SdkRuntime
```

### Step 7: Execute on Wafer (CSL kernels)

```
INGRESS PEs (Row 0):
  Receive query wavelets from host
  Broadcast south to all data PE rows
  Also broadcast KNN query vector (multi-wavelet)

  Time: ~1us (994 hops south)

DATA PEs (Rows 1-992):
  Each PE receives query wavelet

  First: check PE activation mask — am I needed?
    PE (347, 512): YES (falls in active Hilbert range)
    PE (600, 200): NO (skip, do nothing)

  For each local record (up to 700):
    // Structured predicate
    if (record.price >= 3000) continue;  // skip, fails predicate

    // Semantic similarity (L2 distance on quantized embeddings)
    dist = l2_distance(record.embedding_q, query_embedding_q)

    // Combined score
    score = (1.0 - normalize(record.price)) * 0.5   // price component
          + (1.0 - normalize(dist)) * 0.5            // semantic component

    // Insert into local top-5 heap (max-heap by score)
    if (heap.size < 5 || score > heap.min_score):
      heap.insert(record.id, record.price, score)

  Send local top-5 results EAST → reducer column

  Time per PE: ~30us (700 records * ~40 cycles each)

REDUCER PEs (Column 749):
  Receive top-5 from each PE in the row
  Merge into running top-5 (merge sort, keep best 5)
  Pass merged top-5 SOUTH to next reducer

  By the time it reaches the bottom:
    Top-5 across ALL ~220K active PEs

  Time: ~2us (994 hops, simple merge at each)

EGRESS PEs (Row 993):
  Collect final top-5 from reducer column
  DMA results back to host

  Time: ~0.5us

TOTAL WAFER TIME: ~33us
```

### Step 8: Collect Results (Python → Rust)

```
Python host receives from egress:
  5 results, each containing:
    record_id: u32
    price: f32
    score: f32

  results = [
    (id=42,  price=4500, score=0.91),  // our gold ring!
    (id=187, price=2800, score=0.87),
    (id=503, price=1200, score=0.85),
    (id=91,  price=2100, score=0.82),
    (id=334, price=900,  score=0.79),
  ]

Python returns to Rust via pyo3
```

### Step 9: Hydrate Full Records (Rust — norndb-storage)

```
The Cerebras only returned (id, price, score)
We need full records with all fields

Lookup in Arrow record batches on host:
  Record 42 → {
    name: "18K Gold Signet Ring",
    description: "Classic men's signet ring...",
    price: 4500,
    stock: 12,
    category: "rings",
    image: blob_ref("abc123"),
    image_embedding: float32[384],
    text_embedding: float32[384],
    belongs_to: Brand("Norn Gold"),
    supplied_by: Supplier("Cairo Goldworks"),
    _score: 0.91,
  }

  ... same for other 4 results

This hydration is fast — it's just a hashmap lookup by ID
```

### Step 10: Serialize + Send Response (Rust — norndb-server)

```
Serialize results to wire format (or JSON for HTTP)
Send to client via the same connection that sent the query

Total end-to-end latency:
  Parse:          ~10us
  Plan:           ~50us
  Embed query:    ~5ms (sentence-transformers, this is the bottleneck)
  Dispatch:       ~100us
  Wafer execution: ~33us
  Result hydration: ~20us
  Serialize:      ~30us
  Network:        ~1ms

  Total: ~6ms, dominated by the embedding model

  Without semantic search (pure structured query):
  Total: ~1.2ms

  On CPU instead of Cerebras:
  Total: ~55ms
```

### Step 11: If This Was a Reactive Subscription

```
If the query was:
  watch Product where price < 3000 and meaning("elegant traditional") | top(5)

Then:
  1. First execution returns initial results (same as above)
  2. Subscription registered in dependency tracker
  3. When any Product record is inserted/updated/deleted:
     - Check if it affects the Hilbert ranges for this subscription
     - If yes, re-execute query (33us on Cerebras)
     - Compute diff from previous results
     - Push diff to client via WebSocket

  Client sees: real-time updates to their top-5 elegant products
  Latency per update: ~33us Cerebras + ~1ms network = ~1ms
```

---

## Data Flow Summary

```
Create:  User → Auth → Embed → Project → Quantize → Hilbert
         → Tile → WAL → PE → Learned Index → Notify Subscribers

Query:   User → Auth → Parse → Scope → Embed Query → Hilbert Ranges
         → Plan → Dispatch → Wafer Execute → Collect → Hydrate → Return

Update:  Same as Create, but: find old record → update tile
         → re-fit index → notify subscribers with diff
```
