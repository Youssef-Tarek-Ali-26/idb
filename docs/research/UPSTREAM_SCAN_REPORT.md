# iDB Upstream Repo Scan Report

Produced by scanning: RethinkDB, Kuzu, ArangoDB, Apache AGE, JanusGraph, NebulaGraph.

---

## 1. Executive Summary

Six upstream database engines were analyzed for patterns directly applicable to iDB's two critical subsystems: **live updates (watch/changefeed)** and **graph traversal execution**.

**Key findings:**

- iDB's existing `ChangefeedEngine` already captures the right primitives (subscription, resume token, dependency filter, poll-based delivery). The gaps are: backpressure, squashing, and durable per-subscriber offsets.
- For traversal, the consensus pattern across all engines is **frontier-based iterative expansion** with queue-type determining BFS/DFS/weighted behavior. ArangoDB's enumerator/executor split and Kuzu's sparse-to-dense frontier switching are the two most directly translatable patterns.
- Every engine that handles live updates at scale separates the **change detection** (storage layer) from **change distribution** (subscription layer). RethinkDB's hub-and-spoke model is the canonical reference but its non-durable queue is a known weakness — iDB's WAL-based approach is already better.
- Graph traversal correctness hinges on three controls: **uniqueness level** (none/path/global), **depth bounds**, and **memory budgets**. All six engines implement these differently but the invariants are the same.

---

## 2. Pattern Catalog

### 2.1 Live Updates / Changefeed Patterns

| Pattern | Source | Problem Solved | Core Invariant | Failure Modes | iDB Translation | Priority |
|---------|--------|---------------|----------------|---------------|-----------------|----------|
| **Per-shard monotonic stamping** | RethinkDB | Ordered delivery within partition | stamp(n+1) = stamp(n) + 1; priority queue enforces order | Resharding invalidates stamps; cross-shard ordering not guaranteed | Use WAL sequence as stamp. iDB already has `commit_sequence` — ensure monotonic per-partition in `idb-ordered-log` | `now` |
| **Resume token as sequence cursor** | RethinkDB | Reconnect without replay from zero | Client stores last-seen token; server purges events below token | Token invalid after compaction; stale token may miss events if gap too large | Already implemented in `ResumeToken(u64)`. Add: token validation against log low-watermark, return error if token is behind compacted range | `now` |
| **Queue overflow → drop + error signal** | RethinkDB | Slow consumer doesn't OOM server | Queue has hard limit; when exceeded, clear queue, increment `skipped` counter, send error object | Lost events are unrecoverable; client must re-snapshot | Implement bounded `VecDeque` per subscription in `ChangefeedEngine`. On overflow: clear, set `skipped` flag on next `ChangeBatch`. Client re-snapshots via `include_initial` | `now` |
| **Squashing (latest-value-per-key)** | RethinkDB | Reduce bandwidth for rapid updates to same record | HashMap keyed by record ID; only latest `(old_val, new_val)` pair kept | Must be disabled during initial snapshot delivery; squash timer adds latency | Add `SquashMode { Off, Interval(Duration) }` to `Subscription`. Use `HashMap<RecordId, MutationEvent>` that collapses on flush. Disable until initial snapshot delivered | `next` |
| **Filtered changefeed (point vs range vs query)** | RethinkDB | Reduce server-side fan-out | Subscription carries filter expression; events matched before enqueue | Transform errors silently drop events; secondary index changefeeds can duplicate | Already have `DependencyFilter`. Extend to support predicate expressions (reuse `FilterExpr` from planner). Apply filter in `poll()` before collecting | `next` |
| **Include-initial splice stream** | RethinkDB | Atomic snapshot + live tail without gap | Read current state, buffer concurrent changes, splice together; track which key ranges have been read | Race between read and concurrent writes; stamps must be captured before read starts | Add `include_initial: bool` to subscribe. On first poll: snapshot current state via `DurableState` scan, capture current WAL head as resume point, then switch to incremental | `next` |
| **Durable per-subscriber offset** | JanusGraph/Kafka pattern | Survive server restart without losing position | Offset persisted atomically with acknowledgment; compaction respects lowest live offset | Offset commit races (ack before process); zombie subscribers block compaction | Persist `Subscription` structs to a sidecar file or dedicated WAL partition. On restart, reload subscriptions and resume from stored offsets. Add `ack()` method that advances durable offset | `now` |

### 2.2 Graph Traversal Patterns

| Pattern | Source | Problem Solved | Core Invariant | Failure Modes | iDB Translation | Priority |
|---------|--------|---------------|----------------|---------------|-----------------|----------|
| **Queue-type selects algorithm** | ArangoDB | Single traversal engine, multiple behaviors | FIFO→BFS, LIFO→DFS, MinHeap→Weighted | Wrong queue type silently produces wrong results | Create `TraversalQueue` trait with `push/pop`. Impl `FifoQueue`, `LifoQueue`, `WeightedQueue<K: Ord>`. Planner selects queue type based on iQL clause | `now` |
| **Enumerator/Executor split** | ArangoDB | Separate path generation from pipeline control | Executor feeds source vertices; enumerator yields (vertex, edge, path) tuples | Enumerator state must be reset per source vertex; leaked state → wrong paths | `TraversalEnumerator` trait in `idb-executor-cpu` with `next_path() -> Option<PathResult>`. Executor owns enumerator, calls `reset(source)` per input | `now` |
| **Frontier-based iterative expansion** | Kuzu | Efficient BFS with batch processing | Current/next frontier swap each iteration; nodes visited at most once per semantics | Frontier swap must be atomic; visited set must match path semantics | `FrontierPair { current: HashSet<RecordId>, next: HashSet<RecordId> }` with `swap()`. Iteration count = depth. Stop when next is empty or depth > max | `now` |
| **Sparse-to-dense frontier switching** | Kuzu | Adaptive memory for small vs large frontiers | Below threshold: HashSet (sparse). Above: BitVec sized to max node ID (dense) | Threshold choice affects memory; dense wastes memory on sparse graphs | Start with `HashSet`. If `frontier.len() > node_count / 4`, switch to `BitVec`. Profile on real data to tune threshold | `next` |
| **Three-layer filtering (vertex expr → PRUNE → POST-FILTER)** | ArangoDB | Early termination without missing valid paths | PRUNE blocks expansion (exponential savings); POST-FILTER checks terminal conditions | PRUNE too aggressive → misses valid paths; POST-FILTER too late → wasted work | Map iQL `where` on intermediate vertices → PRUNE evaluator. Map iQL `where` on final result → POST-FILTER. Both stored in `TraversalOptions` | `now` |
| **Uniqueness levels (none/path/global)** | ArangoDB, Kuzu | Cycle control without one-size-fits-all | NONE: allow revisits. PATH: per-path visited set. GLOBAL: single visited set across all paths | GLOBAL prevents finding all shortest paths; PATH has per-path memory cost | `enum Uniqueness { None, Path, Global }` on `TraversalOptions`. PATH: clone visited set per branch. GLOBAL: shared `HashSet` with read-only access during expansion | `now` |
| **Composite GraphID (type embedded in ID)** | AGE | Eliminate per-entity type lookup | Upper bits = label/type ID, lower bits = entity sequence | Limits max entities per type; ID not human-readable | Consider for iDB's `RecordId` if type discrimination is hot path. Currently `RecordId(u64)` — could reserve upper 16 bits for entity type without changing storage | `later` |
| **DFS with edge-used flag for VLE** | AGE | Simple cycle detection for variable-length paths | Each edge has `used_in_path` flag; backtrack resets flag | Only prevents edge cycles, not vertex cycles; DFS stack depth = path length | Implement as fallback for PATH uniqueness when edges are the dedup target. Use `HashSet<EdgeId>` per path instead of flag bits | `now` |
| **Bidirectional BFS for shortest path** | Nebula, ArangoDB | Halve search space for point-to-point paths | Expand from both endpoints; stop when frontiers meet; optimal when left_d + right_d >= best_path | Meeting detection must check ALL vertices in frontier, not just first | `BidirectionalBFS` struct with `left_frontier`, `right_frontier`, `left_visited`, `right_visited`. Expand smaller side first (Nebula's data-skew heuristic) | `next` |
| **Index-per-depth traversal** | ArangoDB, JanusGraph | Use different indexes at different hop levels | `depth_lookup_info: HashMap<u32, IndexCursor>` | Index miss at one depth falls back to scan; mixed index/scan hops | Store `Vec<Option<IndexHint>>` in traversal plan, one per depth. Planner fills from available indexes. Executor uses hint or falls back to scan | `next` |
| **Batch multi-key fetch** | JanusGraph | Amortize I/O across multiple vertex lookups | Group vertex IDs by storage shard; single batch RPC per shard | Partial failures leave some vertices unfetched; must handle gracefully | When expanding frontier, collect all neighbor IDs, batch-fetch from `DurableState` in one call. Add `get_many(ids: &[RecordId])` to storage | `now` |
| **Memory-bounded traversal** | ArangoDB, Nebula | Prevent OOM from graph explosion | ResourceMonitor tracks allocation; exception on exceed; queue cleared | Partial results returned; user may not know result is incomplete | Add `max_traversal_memory: usize` to `TraversalOptions`. Track allocation in frontier + visited sets. Return error with partial results if exceeded | `next` |
| **Path-as-linked-list** | Nebula, Kuzu | Memory-efficient path storage for all-paths queries | Linked `NPath` nodes; thread-local allocation pools; paths share prefix nodes | GC complexity; path materialization cost on output | Use `Arc<PathNode>` with `parent: Option<Arc<PathNode>>`. Shared prefixes reduce memory. Materialize to `Vec` only on output | `next` |

---

## 3. Edge Case Matrix

| Edge Case | RethinkDB | Kuzu | ArangoDB | AGE | JanusGraph | Nebula | iDB Recommendation |
|-----------|-----------|------|----------|-----|------------|--------|-------------------|
| **Subscriber reconnect + resume token** | Per-shard `(uuid, u64)` stamp pair; client resumes with last stamp; server purges below | N/A (embedded) | N/A (no changefeed) | N/A | N/A | N/A | Single monotonic `u64` from WAL sequence. Validate token >= log low-watermark. Error if behind compaction |
| **Exactly-once vs at-least-once** | At-least-once with possible loss on queue overflow | N/A | N/A | N/A | N/A | N/A | At-least-once with idempotent client apply. Add `event_id: u64` for client-side dedup. Exactly-once via ack-then-advance protocol |
| **Offset commit races** | No durable offsets; position lost on crash | N/A | N/A | N/A | N/A | N/A | Persist offset on `ack()`. Use WAL append for atomic offset update. On crash: replay from last acked offset (at-least-once) |
| **Backpressure / slow consumer** | Hard queue limit → drop all + `skipped` error; half-full → early squash signal | N/A | N/A | N/A | N/A | N/A | Bounded buffer per subscription. Three thresholds: 50% → squash early, 75% → warn client, 100% → drop + error. Never block writers |
| **Out-of-order / duplicate events** | Per-shard priority queue enforces order; no cross-shard ordering | N/A | N/A | N/A | N/A | N/A | WAL sequence is total order. No reordering possible in single-node. For future multi-node: Lamport timestamp or partition-local ordering |
| **Tombstones/deletes in live views** | Generates `{old_val: X, new_val: null}` change event | N/A | N/A | N/A | N/A | N/A | `MutationEvent` already has `MutationType::Delete`. Ensure watch updates emit `current: None` for deleted records |
| **Traversal fanout explosion** | N/A | Early termination when frontier empty; frontier size threshold | ResourceMonitor per queue; depth limit; memory exception | DFS stack depth = max path length | SliceQuery with limit per vertex | Memory watermark check; 150K row threshold for batching | Depth limit (mandatory). Memory budget (optional, default 64MB). If frontier exceeds budget → return partial + warning. Log metric |
| **Cycle handling** | N/A | Path semantic enum (WALK/TRAIL/PATH) | Uniqueness levels (NONE/PATH/GLOBAL) via template specialization | `used_in_path` edge flag | `ElementLifeCycle` visited tracking | `hasSameEdge()` dedup | `Uniqueness` enum. Default to `Path` (per-path visited set). Expose in iQL: `traverse ... unique edges` / `unique vertices` / `allow cycles` |
| **Depth limits** | N/A | `upperBound` on `RecJoin`; iteration counter check | `maxDepth` in `TraverserOptions`; step not added if depth exceeds | `uidx` bound in VLE; `uidx_infinite` flag if unset | N/A (application-level) | Max step count in Traverse executor | Mandatory `max_depth` in `TraversalOptions`. Default 10. Hard cap 100. iQL syntax: `traverse ... depth 1..5` |
| **Deterministic ordering** | Per-shard monotonic; no cross-shard guarantee | Table-order iteration over nodes | BFS=FIFO order, DFS=LIFO order, Weighted=heap order | DFS stack order | Sort-key in edge serialization format | N/A | Define: BFS returns breadth-level order; DFS returns discovery order; within level, order by record ID ascending. Document guarantee |
| **Multi-tenant isolation** | N/A | N/A | N/A | N/A | N/A | N/A | Already have `TenantId` on `Subscription` and filter in `poll()`. Ensure traversal also scopes to tenant: add `tenant_id` to `TraversalOptions`, filter at frontier expansion |
| **Partial failure (distributed)** | Resharding → `RESUMABLE_OP_FAILED` | N/A (embedded) | N/A (single-node traversal) | N/A (Postgres txn) | Backend-dependent consistency | `completeness` percentage; partial success flag | Single-node for v0 — no partial failure. For future: adopt Nebula's completeness percentage model. Return `result + completeness_pct` |

---

## 4. Phased iDB Implementation Plan

### Phase A: Stable v0 (must-have)

**Goal**: Reliable watch/changefeed + single-hop traversal execution with correctness guarantees.

#### A1. Harden ChangefeedEngine

**API/Contract changes:**
- Add `ack(subscription_id, resume_token)` to advance durable offset
- Add `backpressure_limit: usize` to `Subscription` (default 1000)
- Add `skipped: u64` field to `ChangeBatch`
- Add `include_initial: bool` parameter to subscribe methods

**Internal data structures:**
- Bounded `VecDeque<MutationEvent>` per subscription (replace unbounded collect)
- `SubscriptionPersistence` struct for sidecar file (subscription_id → last_acked_sequence)
- `SnapshotSpliceState` for include_initial: tracks read progress + buffered concurrent changes

**Runtime invariants:**
- Buffer never exceeds `backpressure_limit`; overflow → clear + set `skipped`
- `ack()` is monotonic: reject if token < current acked position
- `poll()` after overflow returns error batch with `skipped` count before resuming normal delivery
- `include_initial` snapshot is consistent: capture WAL head before scan, buffer changes during scan, splice after

**Test plan:**
- Unit: subscribe → write 10 events → poll → verify order and count
- Unit: subscribe → overflow buffer → verify `skipped` field set
- Unit: ack → crash → restart → verify resume from acked position
- Property: for any sequence of writes/polls/acks, `poll` never returns events below acked token
- Integration: concurrent writers + subscriber with include_initial → no gaps in delivered events

**Observability:**
- `idb.changefeed.subscriptions_active` (gauge)
- `idb.changefeed.events_delivered` (counter, per subscription)
- `idb.changefeed.events_skipped` (counter, per subscription)
- `idb.changefeed.poll_latency_us` (histogram)

#### A2. Traversal Execution Foundation

**API/Contract changes:**
- New `TraversalOptions` struct in `idb-core`:
  ```
  max_depth: u32 (default 10, cap 100)
  uniqueness: Uniqueness { None, Path, Global }
  direction: TraversalDirection { Outbound, Inbound, Both }
  vertex_filter: Option<Vec<FilterExpr>>
  prune_filter: Option<Vec<FilterExpr>>
  ```
- Extend `QueryRequest` to carry `TraversalOptions` when source is `Traversal`
- New `TraversalResult` type: `Vec<PathResult>` where `PathResult = Vec<(RecordId, Option<HydratedRecord>)>`

**Internal data structures:**
- `TraversalQueue` trait: `push(Step)`, `pop() -> Option<Step>`, `is_empty() -> bool`, `len() -> usize`
- `FifoQueue`, `LifoQueue` implementations (VecDeque-based)
- `Step { vertex_id: RecordId, depth: u32, path: Vec<RecordId> }`
- `VisitedSet` enum: `PerPath(Vec<HashSet<RecordId>>)` | `Global(HashSet<RecordId>)` | `None`

**Runtime invariants:**
- Depth never exceeds `max_depth` (enforced at push time)
- Uniqueness is checked before push (not after pop)
- Traversal scoped to `tenant_id` — every vertex lookup verifies tenant
- Empty frontier → traversal complete (no infinite loops possible)

**Test plan:**
- Unit: linear chain A→B→C, depth 1..3, verify path correctness
- Unit: cycle A→B→A, uniqueness=Path → no infinite loop
- Unit: cycle A→B→A, uniqueness=None, depth=3 → produces cyclic paths
- Unit: disconnected graph → returns only reachable paths
- Property: for any graph and depth limit, traversal terminates in finite steps
- Integration: iQL `traverse Product -> Category depth 1..2` end-to-end through parser → planner → executor

**Observability:**
- `idb.traversal.vertices_visited` (counter per query)
- `idb.traversal.depth_reached` (histogram)
- `idb.traversal.duration_us` (histogram)
- `idb.traversal.paths_returned` (counter per query)

#### A3. Batch Vertex Fetch

**API/Contract changes:**
- Add `get_many(tenant_id, ids: &[RecordId]) -> Vec<Option<HydratedRecord>>` to `DurableState`

**Internal data structures:**
- Reuse existing state scan but accept batch of IDs
- Return in input order (positional correspondence)

**Runtime invariants:**
- Missing records return `None` at their position (no panic)
- Tenant isolation enforced per record

**Test plan:**
- Unit: fetch 100 records, verify all returned in order
- Unit: fetch mix of existing and non-existing → correct None positions
- Property: `get_many(ids).len() == ids.len()` always

**Observability:**
- `idb.storage.batch_fetch_count` (histogram of batch sizes)
- `idb.storage.batch_fetch_latency_us` (histogram)

---

### Phase B: Performance Hardening

**Goal**: Handle large graphs, high-throughput changefeeds, and complex traversals without degradation.

#### B1. Squashing + Filtered Changefeeds

**API/Contract changes:**
- Add `squash: SquashMode { Off, Interval(Duration) }` to `Subscription`
- Extend `DependencyFilter` to accept `PredicateFilter(Vec<FilterExpr>)`

**Internal data structures:**
- `SquashingBuffer`: `HashMap<RecordId, MutationEvent>` that collapses updates; flush on timer or threshold
- Predicate evaluator reused from executor's `fields_match_predicates`

**Runtime invariants:**
- Squashing disabled during `include_initial` phase
- Squashing only collapses events with same `record_id`; deletes are terminal (not squashed with subsequent inserts)
- Filtered subscriptions never deliver events that don't match predicate

**Test plan:**
- Unit: 100 updates to same record with squash=1s → single event delivered
- Unit: insert then delete with squash → delivers delete (not squash to nothing)
- Unit: predicate filter `status = "active"` → only matching events delivered
- Property: squashed delivery is prefix-closed (no gaps in record history for any single record)

**Observability:**
- `idb.changefeed.events_squashed` (counter)
- `idb.changefeed.squash_flush_count` (counter)

#### B2. Bidirectional Shortest Path

**API/Contract changes:**
- iQL syntax: `shortest path from X to Y through EdgeType`
- New `ShortestPathOptions { source, target, max_depth, weighted: bool }`

**Internal data structures:**
- `BidirectionalBFS`: two frontiers expanding from source and target
- `left_visited: HashMap<RecordId, (u32, Vec<RecordId>)>` (depth + parent path)
- `right_visited`: same structure
- Meeting detection: after each expansion, check intersection of frontier with opposite visited set

**Runtime invariants:**
- Always expand smaller frontier first (Nebula's data-skew heuristic)
- Optimality: stop when `left_depth + right_depth >= best_found_length`
- For weighted: use min-heap instead of FIFO; stop when `left_min_cost + right_min_cost >= best_found_cost`

**Test plan:**
- Unit: known shortest path in 5-node graph → correct
- Unit: no path exists → empty result
- Unit: multiple shortest paths of same length → returns one (deterministic)
- Property: returned path length <= any other path between same endpoints
- Benchmark: 100K node random graph, compare unidirectional vs bidirectional timing

**Observability:**
- `idb.traversal.shortest_path.vertices_expanded` (counter)
- `idb.traversal.shortest_path.direction_switches` (counter)

#### B3. Memory-Bounded Traversal + Adaptive Frontier

**API/Contract changes:**
- Add `max_memory_bytes: Option<usize>` to `TraversalOptions` (default None = 64MB)
- Traversal result includes `truncated: bool` flag

**Internal data structures:**
- `MemoryTracker { used: usize, limit: usize }` — increment on every frontier/visited insert
- `AdaptiveFrontier` enum: `Sparse(HashSet<RecordId>)` | `Dense(BitVec)` — switch at threshold

**Runtime invariants:**
- Memory check before every insert; if exceeded, stop expansion, return partial results with `truncated = true`
- Dense switch threshold: `frontier.len() > estimated_node_count / 4`

**Test plan:**
- Unit: set memory limit to 1KB, traverse large graph → truncated result
- Unit: sparse frontier stays HashSet; large frontier switches to BitVec
- Benchmark: compare memory usage sparse vs dense on power-law graph

**Observability:**
- `idb.traversal.memory_used_bytes` (gauge per query)
- `idb.traversal.truncated` (counter)
- `idb.traversal.frontier_mode` (sparse vs dense, per query)

#### B4. Index-Aware Traversal

**API/Contract changes:**
- Planner emits `index_hints: Vec<Option<IndexHint>>` per traversal depth
- Executor uses index hint to narrow neighbor scan

**Internal data structures:**
- `IndexHint { field: String, value: FieldValue }` — used to build range scan on adjacency
- Falls back to full neighbor scan if no hint available

**Runtime invariants:**
- Index hint is advisory; incorrect hint produces correct (but slower) results
- Missing index → full scan (no error)

**Test plan:**
- Unit: traverse with index hint on edge type → only matching edges expanded
- Unit: traverse without index → full scan, same result
- Benchmark: index vs no-index on typed edge graph

---

### Phase C: Advanced Features

#### C1. Weighted Traversal + Priority Queue

- `WeightedQueue` using `BinaryHeap<Reverse<(Cost, Step)>>`
- Weight extracted from edge property specified in iQL: `traverse ... weight cost`
- Dijkstra-like semantics: only expand minimum-cost vertex

#### C2. All-Paths with Shared-Prefix Path Storage

- `Arc<PathNode { vertex: RecordId, parent: Option<Arc<PathNode>> }>` linked-list paths
- Thread-local allocation pools for `PathNode` (Nebula pattern)
- Output limit to prevent combinatorial explosion

#### C3. Streaming Traversal + Watch Integration

- `watch traverse ...` syntax: live-updating traversal results
- On mutation event affecting any vertex in traversal scope → re-evaluate affected paths
- Incremental: only re-traverse from mutation point, not from root

#### C4. Distributed Traversal Coordination

- Partition-aware frontier expansion (Nebula's scatter-gather)
- Completeness percentage tracking for partial results
- Cross-partition resume on failure

#### C5. Graph-Aware Query Optimizer

- Rule-based optimization passes (ArangoDB model):
  - Push filters into traversal (vertex/edge predicates)
  - Eliminate unused path variables
  - Merge adjacent filter nodes
- Cost-based index selection per depth level
- DP-based join ordering for multi-pattern queries (Kuzu model)

---

## 5. Anti-Patterns to Avoid

### 5.1 Silent Full Scans

**Observed in**: JanusGraph (when no index covers query), ArangoDB (when optimizer can't push filter).

**Problem**: User writes `traverse Product -> Category where category.name = "X"` expecting index use, but if no index exists on `category.name`, system silently scans all categories at each depth level. On a 10M-edge graph this turns a 1ms query into 30s.

**iDB recommendation**: Log a warning when traversal falls back to full neighbor scan at any depth. In `explain` output, mark each depth as `index_scan` or `full_scan`. Consider: if total estimated scan exceeds threshold, return error with suggestion to create index.

### 5.2 Unbounded Subscriptions

**Observed in**: RethinkDB (queue overflow → silent drop).

**Problem**: Server allocates unbounded buffer per subscriber. With 1000 subscribers and bursty writes, memory spikes can crash the process. RethinkDB's solution (drop everything) loses data.

**iDB recommendation**: Hard cap per-subscription buffer (already planned in A1). Add global memory budget across all subscriptions: `max_changefeed_memory_bytes`. When global budget hit, reject new subscriptions (not drop existing ones). Monitor `subscriptions_active * avg_buffer_size` as health metric.

### 5.3 Non-Deterministic Traversal Results

**Observed in**: Multiple engines when using hash-based visited sets.

**Problem**: `HashSet` iteration order varies across runs (Rust's `RandomState`). If traversal results depend on iteration order (e.g., which path is found first in DFS), results are non-deterministic across identical queries.

**iDB recommendation**: Use `BTreeSet` for visited sets when deterministic ordering is required. For BFS, process neighbors in sorted `RecordId` order within each depth level. Document: "BFS results are deterministic; DFS results are deterministic within a run but may vary across process restarts unless `deterministic: true` is set."

### 5.4 Weak Replay Guarantees

**Observed in**: RethinkDB (no durable per-subscriber offset; crash loses position).

**Problem**: If server crashes between delivering events and client acknowledging them, the delivery position is lost. On restart, either events are re-delivered (duplicates) or skipped (data loss).

**iDB recommendation**: Two-phase protocol: (1) deliver batch, (2) client calls `ack(resume_token)`, (3) server persists offset. On crash recovery, replay from last persisted offset (at-least-once). Add `event_id` to each `MutationEvent` for client-side idempotency.

### 5.5 Traversal State Leak Across Queries

**Observed in**: AGE (VLE SRF maintains state across invocations via grammar node ID cache).

**Problem**: If traversal state from a previous query leaks into a new query (via caching, pooling, or incorrect reset), results are corrupted silently.

**iDB recommendation**: Every `TraversalEnumerator` instance is query-scoped. Never pool or cache traversal state. `reset()` must clear ALL internal state (queue, visited set, depth counter). Add debug assertion: `assert!(queue.is_empty())` at start of `reset()`.

### 5.6 Dangling Edge References

**Observed in**: AGE (explicit two-phase delete check), JanusGraph (ghost vertices from eventual consistency).

**Problem**: Deleting a vertex without deleting its edges creates references to non-existent records. Traversal follows edge → tries to fetch deleted vertex → either crashes or returns phantom result.

**iDB recommendation**: On vertex delete: scan edges referencing vertex, either cascade-delete edges (default) or error if edges exist (`strict` mode). During traversal: if vertex fetch returns `None` for a valid edge target, skip silently and log warning (defensive). Never crash on dangling reference.

---

## 6. Ship-First Recommendation

### Build First (2-4 items)

1. **Harden ChangefeedEngine with backpressure + durable offsets (A1)**
   - Your changefeed already works. The gaps (backpressure, persistence, include_initial) are what separate "works in dev" from "works in production." This is low-risk, high-value.

2. **Traversal execution with queue-based BFS/DFS + uniqueness + depth limits (A2)**
   - Your planner already emits `PlanSource::Traversal` with steps and directions. The missing piece is the executor. The queue-based pattern from ArangoDB is clean and proven — a single `TraversalExecutor` with pluggable queue covers BFS and DFS.

3. **Batch vertex fetch (A3)**
   - Without this, every traversal step does N individual lookups. This is the single biggest performance lever for graph queries and it's a small API addition to `DurableState`.

4. **PRUNE filter in traversal (part of A2)**
   - Without early termination, even depth-limited traversals explode on dense graphs. This is a must-have for correctness, not just performance.

### Can Wait

- Squashing and filtered changefeeds (B1) — nice-to-have for bandwidth, not blocking correctness
- Bidirectional shortest path (B2) — useful but unidirectional BFS covers most use cases first
- Memory-bounded traversal (B3) — depth limits provide a coarser but functional safety net
- Index-aware traversal (B4) — full scan works at small scale; optimize later with real profiling data
- Weighted traversal (C1) — niche use case compared to basic BFS/DFS

### Likely Overengineering for Now

- Sparse-to-dense frontier switching — premature optimization; HashSet is fine for v0 graph sizes
- Path-as-linked-list with thread-local pools — only matters for all-paths queries on large graphs
- Distributed traversal coordination — single-node is the current target
- Graph-aware query optimizer with DP join ordering — rule-based is sufficient until multi-pattern queries are needed
- Streaming traversal + watch integration (C3) — fascinating but requires stable traversal + stable changefeed first

---

*Report generated from upstream scan of: rethinkdb, kuzu, arangodb, age, janusgraph, nebula*
*All patterns are architectural extractions — no verbatim code was copied*
