use idb_core::{CallerContext, HydratedRecord, RecordEnvelope, RecordId, TenantId};
use idb_planner::QueryExplain;
use idb_storage::MutationType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportKind {
    Http,
    WebSocket,
    Tcp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayMetadata {
    pub request_id: String,
    pub transport: TransportKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextQueryCompileConfig {
    pub top_k_default: usize,
    pub semantic_embedding_field: String,
    pub semantic_embedding_dims: usize,
}

impl Default for TextQueryCompileConfig {
    fn default() -> Self {
        Self {
            top_k_default: 100,
            semantic_embedding_field: "text_embedding".to_string(),
            semantic_embedding_dims: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GatewayCommand {
    QueryText {
        caller: CallerContext,
        tenant_id: TenantId,
        query_text: String,
        compile: TextQueryCompileConfig,
    },
    ExplainText {
        caller: CallerContext,
        tenant_id: TenantId,
        query_text: String,
        compile: TextQueryCompileConfig,
    },
    WatchStartText {
        caller: CallerContext,
        tenant_id: TenantId,
        query_text: String,
        compile: TextQueryCompileConfig,
    },
    WatchPoll {
        subscription_id: u64,
        max_events: usize,
    },
    WatchStop {
        subscription_id: u64,
    },
    Ingest {
        caller: CallerContext,
        records: Vec<RecordEnvelope>,
    },
    Delete {
        caller: CallerContext,
        tenant_id: TenantId,
        record_id: RecordId,
    },
    DurableMutationPoll {
        caller: CallerContext,
        tenant_id: TenantId,
        consumer_group: String,
        max_events_per_partition: usize,
    },
    DurableMutationCommit {
        caller: CallerContext,
        tenant_id: TenantId,
        consumer_group: String,
        offsets: Vec<GatewayDurableMutationOffset>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayRequest {
    pub metadata: GatewayMetadata,
    pub command: GatewayCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayWatchSession {
    pub subscription_id: u64,
    pub resume_token: u64,
    pub snapshot: Vec<HydratedRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayWatchUpdate {
    pub commit_sequence: u64,
    pub record_id: RecordId,
    pub current: Option<HydratedRecord>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayWatchUpdateBatch {
    pub subscription_id: u64,
    pub updates: Vec<GatewayWatchUpdate>,
    pub next_resume_token: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayDurableMutationOffset {
    pub partition: u32,
    pub committed_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GatewayDurableMutationRecord {
    pub partition: u32,
    pub sequence: u64,
    pub commit_sequence: u64,
    pub record_id: RecordId,
    pub mutation_type: MutationType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GatewayResponsePayload {
    QueryRows {
        rows: Vec<HydratedRecord>,
    },
    ExplainPlan {
        explain: QueryExplain,
    },
    WatchStarted {
        session: GatewayWatchSession,
    },
    WatchUpdates {
        batch: GatewayWatchUpdateBatch,
    },
    WatchStopped {
        stopped: bool,
    },
    Ingested {
        record_ids: Vec<RecordId>,
    },
    Deleted {
        deleted: bool,
    },
    DurableMutationRecords {
        records: Vec<GatewayDurableMutationRecord>,
    },
    DurableMutationOffsetsCommitted {
        partitions_committed: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GatewayResponse {
    pub request_id: String,
    pub payload: GatewayResponsePayload,
}
