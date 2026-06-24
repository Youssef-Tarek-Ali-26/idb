use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("missing field: {0}")]
    MissingField(String),
    #[error("invalid field type for {field}: expected {expected}")]
    InvalidFieldType {
        field: String,
        expected: &'static str,
    },
    #[error("missing embedding field: {0}")]
    MissingEmbedding(String),
    #[error("invalid dimension definition: {0}")]
    InvalidDimension(String),
    #[error("normalization error: {0}")]
    Normalization(String),
    #[error("tenant mismatch in operation")]
    TenantMismatch,
    #[error("storage error: {0}")]
    Storage(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("query planning error: {0}")]
    QueryPlanning(String),
    #[error("authorization denied: {0}")]
    AuthorizationDenied(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
