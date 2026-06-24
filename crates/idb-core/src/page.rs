use serde::{Deserialize, Serialize};

use crate::types::TenantId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageMetadata {
    pub page_id: u64,
    pub tenant_id: TenantId,
    pub key_min: u128,
    pub key_max: u128,
    pub record_count: u32,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageConfig {
    pub max_records_per_page: u32,
    pub min_records_for_merge: u32,
}

impl Default for PageConfig {
    fn default() -> Self {
        Self {
            max_records_per_page: 1024,
            min_records_for_merge: 128,
        }
    }
}

impl PageMetadata {
    pub fn should_split(&self, config: &PageConfig) -> bool {
        self.record_count > config.max_records_per_page
    }

    pub fn merge_eligible(&self, config: &PageConfig) -> bool {
        self.record_count < config.min_records_for_merge
    }
}
