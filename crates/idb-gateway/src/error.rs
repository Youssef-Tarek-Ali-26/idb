use idb_core::CoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("unsupported HTTP route: {method} {path}")]
    UnsupportedHttpRoute { method: String, path: String },
    #[error("unsupported WebSocket event: {0}")]
    UnsupportedWebSocketEvent(String),
    #[error("unsupported TCP opcode: {0}")]
    UnsupportedTcpOpcode(u16),
    #[error("invalid payload for {context}: {source}")]
    InvalidPayload {
        context: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("protocol negotiation failed: {0}")]
    ProtocolNegotiation(String),
    #[error(transparent)]
    Core(#[from] CoreError),
}

pub type GatewayResult<T> = Result<T, GatewayError>;
