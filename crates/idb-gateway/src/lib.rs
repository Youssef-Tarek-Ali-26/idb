pub mod error;
pub mod model;
pub mod runtime;
pub mod transport;

pub use error::{GatewayError, GatewayResult};
pub use model::{
    GatewayCommand, GatewayDurableMutationOffset, GatewayDurableMutationRecord, GatewayMetadata,
    GatewayRequest, GatewayResponse, GatewayResponsePayload, GatewayWatchSession,
    GatewayWatchUpdate, GatewayWatchUpdateBatch, TextQueryCompileConfig, TransportKind,
};
pub use runtime::CpuGatewayRuntime;
pub use transport::{
    establish_tcp_session, normalize_http, normalize_tcp, normalize_websocket, DeletePayload,
    DurableMutationCommitPayload, DurableMutationOffsetPayload, DurableMutationPollPayload,
    HttpGatewayEnvelope, IngestPayload, QueryTextPayload, TcpGatewayFrame, TcpGatewaySession,
    WatchPollPayload, WatchStopPayload, WebSocketGatewayEnvelope, OPCODE_DELETE,
    OPCODE_DURABLE_MUTATION_COMMIT, OPCODE_DURABLE_MUTATION_POLL, OPCODE_EXPLAIN_TEXT,
    OPCODE_INGEST, OPCODE_QUERY_TEXT, OPCODE_WATCH_POLL, OPCODE_WATCH_START_TEXT,
    OPCODE_WATCH_STOP,
};
