use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RecordId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TenantId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityType(pub String);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum FieldValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Timestamp(DateTime<Utc>),
    Bytes(Vec<u8>),
}

impl FieldValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Int(v) => Some(*v as f64),
            Self::Float(v) => Some(*v),
            Self::Timestamp(ts) => Some(ts.timestamp_millis() as f64),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobRef {
    pub field: String,
    pub blob_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeRef {
    pub edge_type: String,
    pub target_record_id: RecordId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordEnvelope {
    pub record_id: RecordId,
    pub tenant_id: TenantId,
    pub entity_type: EntityType,
    pub schema_version: u32,
    pub dimension_version: u32,
    pub structured_fields: BTreeMap<String, FieldValue>,
    pub embedding_fields: BTreeMap<String, Vec<f32>>,
    pub blob_refs: Vec<BlobRef>,
    pub edge_refs: Vec<EdgeRef>,
    pub event_time: DateTime<Utc>,
    pub ingest_time: DateTime<Utc>,
}

impl RecordEnvelope {
    pub fn new(
        record_id: u64,
        tenant_id: impl Into<String>,
        entity_type: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            record_id: RecordId(record_id),
            tenant_id: TenantId(tenant_id.into()),
            entity_type: EntityType(entity_type.into()),
            schema_version: 1,
            dimension_version: 1,
            structured_fields: BTreeMap::new(),
            embedding_fields: BTreeMap::new(),
            blob_refs: Vec::new(),
            edge_refs: Vec::new(),
            event_time: now,
            ingest_time: now,
        }
    }
}
