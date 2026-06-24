# iDB Visual Guide (Plain-English)

If DB internals feel confusing, use this page as your "map."
You can ignore implementation details and just track the flows.

Last synced with implementation: `2026-02-28`

## Sync Rule (keep diagrams current)

Update this file and [DB_DIAGRAMS_ASCII.md](./DB_DIAGRAMS_ASCII.md) whenever any of these change:
- request path (parser/planner/runtime/backend stages)
- write path (mutation events, WAL/state behavior)
- watch lifecycle semantics
- durable stream poll/commit behavior
- transport behavior (HTTP/WS/TCP normalization or parity)

## 1) Big Picture

```mermaid
flowchart TD
    A["Apps / SDKs"] --> B["Server Adapters (HTTP / WebSocket / TCP)"]
    B --> C["Gateway Normalization (same canonical commands)"]
    C --> D["Gateway Runtime"]
    D --> E["CPU Backend (reference engine)"]
    E --> F["Unified Storage State"]
    E --> G["Ordered Log (durable stream)"]
    F --> H["Query Results / Watch Snapshots"]
    G --> I["Replayable Mutation Stream"]
```

What this means:
- clients can talk with different protocols
- server turns everything into one internal command shape
- backend runs queries/mutations
- storage holds current state, ordered log holds replayable mutation history

## 2) Query Flow (Read Path)

```mermaid
flowchart LR
    A["Query Text"] --> B["Parser (AST)"]
    B --> C["Planner (logical plan)"]
    C --> D["QueryRequest Bridge"]
    D --> E["Candidate Generation"]
    E --> F["Structured Filters"]
    F --> G["Semantic Scoring / Ranking"]
    G --> H["Hydration (batch fetch full records)"]
    H --> I["Response Rows"]
```

Mental model:
- find possible matches
- remove invalid ones
- rank best ones
- batch-fetch and return complete records

## 3) Write Flow (Ingest / Delete)

```mermaid
flowchart LR
    A["Mutation Command (ingest/delete)"] --> B["Auth + Tenant Check"]
    B --> C["WAL / Durable State Update"]
    C --> D["Unified Storage State Updated"]
    C --> E["Mutation Event Emitted"]
    E --> F["Ordered Log Append (durable stream mirror)"]
    D --> G["Queryable Current State"]
    F --> H["Replay Stream for Consumers"]
```

Mental model:
- every write updates current state
- every write also emits an event
- events are mirrored into ordered durable stream

## 4) Watch Flow (Live Updates)

```mermaid
sequenceDiagram
    participant Client
    participant Gateway
    participant Backend
    participant Storage

    Client->>Gateway: watch start (query text)
    Gateway->>Backend: canonical watch command
    Backend->>Storage: build initial snapshot + subscription
    Storage-->>Backend: snapshot + subscription id
    Backend-->>Gateway: watch started payload
    Gateway-->>Client: snapshot + resume token

    Client->>Gateway: watch poll (subscription id)
    Gateway->>Backend: poll watch updates
    Backend->>Storage: collect matching events
    Storage-->>Backend: update batch
    Backend-->>Gateway: watch updates
    Gateway-->>Client: updates + next resume token
```

Mental model:
- start watch once
- poll updates repeatedly
- updates are query-aware (not random global noise)
- poll bounds must be positive (`max_events > 0`)

## 5) Durable Mutation Stream (Replay + Commit Offsets)

```mermaid
sequenceDiagram
    participant Worker
    participant Server
    participant Backend
    participant OrderedLog

    Worker->>Server: poll mutations (tenant, group)
    Server->>Backend: durable poll command
    Backend->>OrderedLog: poll consumer group offsets
    OrderedLog-->>Backend: records after committed offsets
    Backend-->>Server: durable mutation records
    Server-->>Worker: records

    Worker->>Server: commit offsets (partition, sequence)
    Server->>Backend: durable commit command
    Backend->>OrderedLog: commit consumer offsets
    OrderedLog-->>Backend: ack
    Backend-->>Server: committed count
    Server-->>Worker: committed
```

Mental model:
- poll gets unprocessed events
- commit marks progress
- next poll resumes from committed position
- per-partition poll bounds must be positive (`max_events_per_partition > 0`)
- `consumer_group` must be non-empty

## 6) Why "One Storage" (Not DB vs Files)

```mermaid
flowchart TD
    A["Input Data"] --> B["Structured Fields (price, date, flags)"]
    A --> C["Semantic Embeddings (meaning vectors)"]
    A --> D["Relations (edges between records)"]
    B --> E["Unified Record in N-dimensional space"]
    C --> E
    D --> E
    E --> F["One query surface: structured + semantic + traversal"]
```

Mental model:
- different "data shapes" still end up in one system
- query language can mix these modes in one flow

## 7) N-Space Kernel (Theory Track)

```mermaid
flowchart LR
    A["Record"] --> B["Structured Block (x_s)"]
    A --> C["Semantic Block (x_m)"]
    A --> D["Topology Block (x_t)"]
    B --> E["S_s (structured similarity)"]
    C --> F["S_m (semantic similarity)"]
    D --> G["S_t (topology similarity)"]
    E --> H["Weighted Fusion"]
    F --> H
    G --> H
    H --> I["Fused Similarity S(x,y) in [0,1]"]
```

Kernel:
- `S(x,y) = (w_s*S_s + w_m*S_m + w_t*S_t) / (w_s + w_m + w_t)`
- `D(x,y) = 1 - S(x,y)`
- Deterministic per `nspace_version`

## 8) Transport Parity (Same Behavior Everywhere)

```mermaid
flowchart LR
    A["HTTP Request"] --> D["Normalization"]
    B["WebSocket Event"] --> D
    C["TCP Frame"] --> D
    D --> E["Canonical Gateway Command"]
    E --> F["One Runtime Dispatch Path"]
    F --> G["Equivalent Semantics"]
```

Mental model:
- protocol changes, behavior should not
