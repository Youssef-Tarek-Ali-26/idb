use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use idb_core::{RecordId, TenantId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationType {
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutationEvent {
    pub tenant_id: TenantId,
    pub record_id: RecordId,
    pub mutation_type: MutationType,
    pub commit_sequence: u64,
    pub committed_at: DateTime<Utc>,
}
