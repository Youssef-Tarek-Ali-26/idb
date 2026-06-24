# iDB Visual Guide (ASCII)

No renderer needed. Just plain text diagrams.

Last synced with implementation: `2026-02-28`

## Sync Rule (keep diagrams current)

Update this file and [DB_DIAGRAMS.md](./DB_DIAGRAMS.md) whenever any of these change:
- request path (parser/planner/runtime/backend stages)
- write path (mutation events, WAL/state behavior)
- watch lifecycle semantics
- durable stream poll/commit behavior
- transport behavior (HTTP/WS/TCP normalization or parity)

## 1) Big Picture

```text
+------------------+      +-------------------------------+
| Apps / SDKs      | ---> | Server Adapters              |
| (ts/py/rs/etc)   |      | HTTP | WebSocket | TCP       |
+------------------+      +-------------------------------+
                                   |
                                   v
                         +-------------------------------+
                         | Gateway Normalization         |
                         | (one canonical command shape) |
                         +-------------------------------+
                                   |
                                   v
                         +-------------------------------+
                         | Gateway Runtime               |
                         +-------------------------------+
                                   |
                                   v
                         +-------------------------------+
                         | CPU Backend (reference)       |
                         +-------------------------------+
                            |                        |
                            v                        v
                 +-------------------+     +----------------------+
                 | Unified State     |     | Ordered Log          |
                 | (current records) |     | (durable event replay)|
                 +-------------------+     +----------------------+
```

## 2) Query Flow (Read Path)

```text
Query Text
   |
   v
+--------+   +---------+   +------------------+   +---------------------+
| Parser |-> | Planner |-> | QueryRequest     |-> | Candidate Generation |
| (AST)  |   |         |   | Bridge           |   |                     |
+--------+   +---------+   +------------------+   +---------------------+
                                                           |
                                                           v
                                               +---------------------+
                                               | Structured Filters  |
                                               +---------------------+
                                                           |
                                                           v
                                               +---------------------+
                                               | Semantic Score/Rank |
                                               +---------------------+
                                                           |
                                                           v
                                               +---------------------+
                                               | Hydration           |
                                               | (batch full records)|
                                               +---------------------+
                                                           |
                                                           v
                                                    Response Rows
```

## 3) Write Flow (Ingest/Delete)

```text
Mutation Command
   |
   v
+--------------------+      +---------------------------+
| Auth + Tenant Check| ---> | WAL / Durable State Write |
+--------------------+      +---------------------------+
                                     |                |
                                     v                v
                         +-------------------+   +--------------------+
                         | Unified State     |   | Mutation Event     |
                         | Updated           |   | Emitted            |
                         +-------------------+   +--------------------+
                                                       |
                                                       v
                                             +----------------------+
                                             | Ordered Log Append   |
                                             | (durable stream)     |
                                             +----------------------+
```

## 4) Watch Flow (Live Updates)

```text
Client                Gateway                Backend                Storage
  |                      |                      |                      |
  | watch start          |                      |                      |
  |--------------------->| canonical watch      |                      |
  |                      |--------------------->| build snapshot+sub   |
  |                      |                      |--------------------->|
  |                      |                      | snapshot + sub id     |
  |                      |<--------------------------------------------|
  | snapshot+token       |                      |                      |
  |<---------------------|                      |                      |
  |                      |                      |                      |
  | watch poll           |                      |                      |
  |--------------------->| poll updates         |                      |
  |                      |--------------------->| collect matching evts|
  |                      |                      |--------------------->|
  |                      |                      | update batch          |
  |                      |<--------------------------------------------|
  | updates+next token   |                      |                      |
  |<---------------------|                      |                      |
```

Guardrail: `max_events` must be `> 0`.

## 5) Durable Mutation Stream (Replay + Commit)

```text
Worker                Server                 Backend               OrderedLog
  |                     |                      |                      |
  | poll(group)         |                      |                      |
  |-------------------->| durable poll cmd     |                      |
  |                     |--------------------->| poll by offsets      |
  |                     |                      |--------------------->|
  |                     |                      | records after commit  |
  |                     |<--------------------------------------------|
  | records             |                      |                      |
  |<--------------------|                      |                      |
  |                     |                      |                      |
  | commit(partition,seq)|                     |                      |
  |-------------------->| durable commit cmd   |                      |
  |                     |--------------------->| commit offsets       |
  |                     |                      |--------------------->|
  |                     |                      | ack                  |
  |                     |<--------------------------------------------|
  | committed           |                      |                      |
  |<--------------------|                      |                      |
```

Guardrail: `max_events_per_partition` must be `> 0`.
Guardrail: `consumer_group` must be non-empty.

## 6) One Storage Model

```text
Input Data
   |------------------------|
   |                        |
   v                        v
Structured Fields       Semantic Embeddings
(price/date/etc)        (meaning vectors)
   |                        |
   |                        |
   +-----------+------------+
               |
               v
        Unified Record Space
               |
               v
      One Query Surface:
      structured + semantic + traversal
```

## 7) N-Space Kernel (Theory Track)

```text
Record
  |---------------------------|
  |             |             |
  v             v             v
x_s           x_m           x_t
(structured)  (semantic)    (topology)
  |             |             |
  v             v             v
S_s           S_m           S_t
  \             |             /
   \            |            /
    +-------- Weighted Fusion --------+
                    |
                    v
             Fused S(x,y) in [0,1]
             D(x,y) = 1 - S(x,y)
```

## 8) Transport Parity

```text
HTTP Request  ----\
WebSocket Event ---+--> Normalization --> Canonical Command --> One Runtime Path
TCP Frame     ----/                                              |
                                                              Equivalent
                                                              behavior
```
