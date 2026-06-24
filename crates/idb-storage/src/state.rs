use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use idb_index::{lower_bound, train_linear_model, upper_bound, LearnedPositionModel};

use idb_core::{
    CoreError, CoreResult, DimensionRegistry, FieldValue, InterleavedKeyMapper, KeyRange,
    RecordEnvelope, RecordId, SpaceKeyMapper, TenantId,
};

use crate::events::{MutationEvent, MutationType};
use crate::wal::{Wal, WalEntry, WalMutation};

#[derive(Debug)]
pub struct DurableState {
    wal: Wal,
    visibility_path: PathBuf,
    spatial_indexer: Option<SpatialIndexer>,
    last_sequence: u64,
    cold_records: HashMap<(TenantId, RecordId), RecordEnvelope>,
    hot_records: HashMap<(TenantId, RecordId), CompactRecord>,
    tenant_space_indexes: HashMap<TenantId, TenantSpaceIndex>,
    mutation_events: Vec<MutationEvent>,
}

#[derive(Debug, Clone)]
pub struct SpatialIndexer {
    registry: DimensionRegistry,
    mapper: InterleavedKeyMapper,
}

impl SpatialIndexer {
    pub fn new(registry: DimensionRegistry, mapper: InterleavedKeyMapper) -> CoreResult<Self> {
        registry.validate()?;
        Ok(Self { registry, mapper })
    }

    fn map_space_key(&self, record: &RecordEnvelope) -> CoreResult<u128> {
        let coordinates = self.registry.map_record(record)?;
        self.mapper.encode(&coordinates)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpaceIndexEntry {
    space_key: u128,
    record_id: RecordId,
}

#[derive(Debug, Clone, Default)]
struct TenantSpaceIndex {
    entries: Vec<SpaceIndexEntry>,
    keys: Vec<u128>,
    model: Option<LearnedPositionModel>,
}

impl TenantSpaceIndex {
    fn insert(&mut self, space_key: u128, record_id: &RecordId) {
        let insert_at = self.entries.partition_point(|entry| {
            entry.space_key < space_key
                || (entry.space_key == space_key && entry.record_id < *record_id)
        });
        self.entries.insert(
            insert_at,
            SpaceIndexEntry {
                space_key,
                record_id: record_id.clone(),
            },
        );
        self.keys.insert(insert_at, space_key);
        self.refresh_model();
    }

    fn remove(&mut self, space_key: u128, record_id: &RecordId) -> bool {
        let target = self
            .entries
            .binary_search_by(|entry| match entry.space_key.cmp(&space_key) {
                Ordering::Equal => entry.record_id.cmp(record_id),
                other => other,
            });

        let Ok(index) = target else {
            return false;
        };

        self.entries.remove(index);
        self.keys.remove(index);
        self.refresh_model();
        true
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn range_candidates(&self, min_key: u128, max_key: u128) -> Vec<RecordId> {
        if self.entries.is_empty() || min_key > max_key {
            return Vec::new();
        }

        let len = self.keys.len();
        let (mut lower, mut upper) = match &self.model {
            Some(model) => (
                lower_bound(&self.keys, model, min_key).min(len),
                upper_bound(&self.keys, model, max_key).min(len),
            ),
            None => (
                self.keys.partition_point(|key| *key < min_key),
                self.keys.partition_point(|key| *key <= max_key),
            ),
        };

        while lower > 0 && self.keys[lower - 1] >= min_key {
            lower -= 1;
        }
        while lower < len && self.keys[lower] < min_key {
            lower += 1;
        }

        while upper > 0 && self.keys[upper - 1] > max_key {
            upper -= 1;
        }
        while upper < len && self.keys[upper] <= max_key {
            upper += 1;
        }

        if lower >= upper || lower >= len {
            return Vec::new();
        }

        self.entries[lower..upper.min(len)]
            .iter()
            .filter(|entry| entry.space_key >= min_key && entry.space_key <= max_key)
            .map(|entry| entry.record_id.clone())
            .collect()
    }

    fn refresh_model(&mut self) {
        self.model = if self.keys.is_empty() {
            None
        } else {
            train_linear_model(&self.keys).ok()
        };
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompactRecord {
    pub tenant_id: TenantId,
    pub record_id: RecordId,
    pub schema_version: u32,
    pub dimension_version: u32,
    pub structured_fields: BTreeMap<String, FieldValue>,
    pub embedding_fields: BTreeMap<String, Vec<f32>>,
    pub space_key: Option<u128>,
}

impl CompactRecord {
    fn from_envelope(
        record: &RecordEnvelope,
        spatial_indexer: Option<&SpatialIndexer>,
    ) -> CoreResult<Self> {
        let space_key = match spatial_indexer {
            Some(indexer) => Some(indexer.map_space_key(record)?),
            None => None,
        };

        Ok(Self {
            tenant_id: record.tenant_id.clone(),
            record_id: record.record_id.clone(),
            schema_version: record.schema_version,
            dimension_version: record.dimension_version,
            structured_fields: record.structured_fields.clone(),
            embedding_fields: record.embedding_fields.clone(),
            space_key,
        })
    }
}

impl DurableState {
    pub fn new(wal_path: impl AsRef<Path>, visibility_path: impl AsRef<Path>) -> CoreResult<Self> {
        Self::new_with_indexer(wal_path, visibility_path, None)
    }

    pub fn new_with_indexer(
        wal_path: impl AsRef<Path>,
        visibility_path: impl AsRef<Path>,
        spatial_indexer: Option<SpatialIndexer>,
    ) -> CoreResult<Self> {
        let wal = Wal::new(wal_path)?;
        let visibility_path = visibility_path.as_ref().to_path_buf();
        if !visibility_path.exists() {
            fs::write(&visibility_path, b"0")
                .map_err(|e| CoreError::Storage(format!("failed creating visibility file: {e}")))?;
        }

        let mut state = Self {
            wal,
            visibility_path,
            spatial_indexer,
            last_sequence: 0,
            cold_records: HashMap::new(),
            hot_records: HashMap::new(),
            tenant_space_indexes: HashMap::new(),
            mutation_events: Vec::new(),
        };
        state.replay()?;
        Ok(state)
    }

    pub fn records(&self) -> &HashMap<(TenantId, RecordId), RecordEnvelope> {
        &self.cold_records
    }

    pub fn hot_records(&self) -> &HashMap<(TenantId, RecordId), CompactRecord> {
        &self.hot_records
    }

    pub fn get_many(&self, tenant_id: &TenantId, ids: &[RecordId]) -> Vec<Option<RecordEnvelope>> {
        ids.iter()
            .map(|record_id| {
                self.cold_records
                    .get(&(tenant_id.clone(), record_id.clone()))
                    .cloned()
            })
            .collect()
    }

    pub fn cold_record_count(&self) -> u64 {
        self.cold_records.len() as u64
    }

    pub fn hot_record_count(&self) -> u64 {
        self.hot_records.len() as u64
    }

    pub fn candidate_ids_for_key_ranges(
        &self,
        tenant_id: &TenantId,
        ranges: &[KeyRange],
    ) -> Vec<RecordId> {
        if ranges.is_empty() {
            return Vec::new();
        }

        if let Some(index) = self.tenant_space_indexes.get(tenant_id) {
            let mut out = BTreeSet::new();
            for range in ranges {
                for record_id in index.range_candidates(range.min, range.max) {
                    out.insert(record_id);
                }
            }
            return out.into_iter().collect();
        }

        let mut out = BTreeSet::new();
        for ((record_tenant_id, record_id), record) in &self.hot_records {
            if record_tenant_id != tenant_id {
                continue;
            }
            if let Some(space_key) = record.space_key {
                if ranges
                    .iter()
                    .any(|range| space_key >= range.min && space_key <= range.max)
                {
                    out.insert(record_id.clone());
                }
            }
        }
        out.into_iter().collect()
    }

    pub fn wal_size_bytes(&self) -> CoreResult<u64> {
        fs::metadata(self.wal.path())
            .map(|m| m.len())
            .map_err(|e| CoreError::Storage(format!("failed to read wal file size: {e}")))
    }

    pub fn mutation_events(&self) -> &[MutationEvent] {
        &self.mutation_events
    }

    pub fn mutation_events_since(
        &self,
        tenant_id: &TenantId,
        after_sequence: u64,
        limit: usize,
    ) -> Vec<MutationEvent> {
        self.mutation_events
            .iter()
            .filter(|event| &event.tenant_id == tenant_id && event.commit_sequence > after_sequence)
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub fn upsert(&mut self, record: RecordEnvelope) -> CoreResult<u64> {
        let seq = self.next_sequence();
        let entry = WalEntry {
            sequence: seq,
            committed_at: Utc::now(),
            mutation: WalMutation::Upsert {
                record: record.clone(),
            },
        };
        self.wal.append(&entry)?;
        self.apply_entry(entry)?;
        self.persist_visibility(seq)?;
        Ok(seq)
    }

    pub fn delete(&mut self, tenant_id: TenantId, record_id: RecordId) -> CoreResult<(u64, bool)> {
        let existed = self
            .cold_records
            .contains_key(&(tenant_id.clone(), record_id.clone()));
        let seq = self.next_sequence();
        let entry = WalEntry {
            sequence: seq,
            committed_at: Utc::now(),
            mutation: WalMutation::Delete {
                tenant_id: tenant_id.clone(),
                record_id: record_id.clone(),
            },
        };
        self.wal.append(&entry)?;
        self.apply_entry(entry)?;
        self.persist_visibility(seq)?;
        Ok((seq, existed))
    }

    pub fn replay(&mut self) -> CoreResult<()> {
        self.cold_records.clear();
        self.hot_records.clear();
        self.tenant_space_indexes.clear();
        self.mutation_events.clear();

        let visibility = self.read_visibility()?;
        let entries = self.wal.read_all()?;

        for entry in entries {
            if entry.sequence <= visibility {
                self.apply_entry(entry)?;
            }
        }
        Ok(())
    }

    fn apply_entry(&mut self, entry: WalEntry) -> CoreResult<()> {
        let WalEntry {
            sequence,
            committed_at,
            mutation,
        } = entry;

        match mutation {
            WalMutation::Upsert { record } => {
                let key = (record.tenant_id.clone(), record.record_id.clone());
                let mutation_type = if self.cold_records.contains_key(&key) {
                    MutationType::Update
                } else {
                    MutationType::Insert
                };
                let compact = CompactRecord::from_envelope(&record, self.spatial_indexer.as_ref())?;
                let previous = self.hot_records.insert(key.clone(), compact.clone());
                if let Some(previous_key) = previous.and_then(|old| old.space_key) {
                    self.remove_from_space_index(
                        &record.tenant_id,
                        &record.record_id,
                        previous_key,
                    );
                }
                if let Some(new_key) = compact.space_key {
                    self.insert_into_space_index(&record.tenant_id, &record.record_id, new_key);
                }
                self.cold_records.insert(key, record.clone());
                self.mutation_events.push(MutationEvent {
                    tenant_id: record.tenant_id,
                    record_id: record.record_id,
                    mutation_type,
                    commit_sequence: sequence,
                    committed_at,
                });
            }
            WalMutation::Delete {
                tenant_id,
                record_id,
            } => {
                let removed = self
                    .hot_records
                    .remove(&(tenant_id.clone(), record_id.clone()));
                if let Some(space_key) = removed.and_then(|record| record.space_key) {
                    self.remove_from_space_index(&tenant_id, &record_id, space_key);
                }
                self.cold_records
                    .remove(&(tenant_id.clone(), record_id.clone()));
                self.mutation_events.push(MutationEvent {
                    tenant_id,
                    record_id,
                    mutation_type: MutationType::Delete,
                    commit_sequence: sequence,
                    committed_at,
                });
            }
        }

        self.last_sequence = self.last_sequence.max(sequence);
        Ok(())
    }

    fn insert_into_space_index(
        &mut self,
        tenant_id: &TenantId,
        record_id: &RecordId,
        space_key: u128,
    ) {
        self.tenant_space_indexes
            .entry(tenant_id.clone())
            .or_default()
            .insert(space_key, record_id);
    }

    fn remove_from_space_index(
        &mut self,
        tenant_id: &TenantId,
        record_id: &RecordId,
        space_key: u128,
    ) {
        let should_remove_tenant =
            self.tenant_space_indexes
                .get_mut(tenant_id)
                .is_some_and(|index| {
                    index.remove(space_key, record_id);
                    index.is_empty()
                });
        if should_remove_tenant {
            self.tenant_space_indexes.remove(tenant_id);
        }
    }

    fn next_sequence(&self) -> u64 {
        self.last_sequence + 1
    }

    fn persist_visibility(&self, seq: u64) -> CoreResult<()> {
        fs::write(&self.visibility_path, seq.to_string())
            .map_err(|e| CoreError::Storage(format!("failed persisting visibility marker: {e}")))
    }

    fn read_visibility(&self) -> CoreResult<u64> {
        let data = fs::read_to_string(&self.visibility_path)
            .map_err(|e| CoreError::Storage(format!("failed reading visibility marker: {e}")))?;
        data.trim()
            .parse::<u64>()
            .map_err(|e| CoreError::Storage(format!("invalid visibility marker value: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::env;
    use std::fs;

    use idb_core::{
        DimensionDefinition, DimensionRegistry, DimensionSource, DimensionType, FieldValue,
        InterleavedKeyMapper, KeyRange, MissingValuePolicy, NormalizationPolicy, RecordEnvelope,
        RecordId, TenantId,
    };

    use super::{DurableState, SpatialIndexer};

    #[test]
    fn wal_replay_restores_committed_state() {
        let base = env::temp_dir().join(format!(
            "idb_storage_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create tmp dir");

        let wal_path = base.join("wal.jsonl");
        let vis_path = base.join("visibility.txt");

        let mut state = DurableState::new(&wal_path, &vis_path).expect("create state");

        let mut record = RecordEnvelope::new(1, "tenant_a", "Product");
        record
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        record.structured_fields.insert(
            "category".to_string(),
            FieldValue::String("rings".to_string()),
        );
        record.embedding_fields = BTreeMap::from([("text_embedding".to_string(), vec![0.1, 0.2])]);

        state.upsert(record).expect("upsert");
        assert_eq!(state.records().len(), 1);

        let restarted = DurableState::new(&wal_path, &vis_path).expect("restart state");
        assert_eq!(restarted.records().len(), 1);

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn spatial_indexer_populates_hot_record_space_key() {
        let base = env::temp_dir().join(format!(
            "idb_storage_spatial_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create tmp dir");

        let wal_path = base.join("wal.jsonl");
        let vis_path = base.join("visibility.txt");
        let registry = DimensionRegistry {
            version: 1,
            dimensions: vec![DimensionDefinition {
                name: "price".to_string(),
                source: DimensionSource::StructuredField("price".to_string()),
                dimension_type: DimensionType::Numeric,
                normalization: NormalizationPolicy::MinMax {
                    min: 0.0,
                    max: 1000.0,
                    bins: 32,
                },
                missing_value: MissingValuePolicy::Error,
            }],
        };
        let indexer = SpatialIndexer::new(registry, InterleavedKeyMapper::new(5).expect("mapper"))
            .expect("indexer");

        let mut state =
            DurableState::new_with_indexer(&wal_path, &vis_path, Some(indexer)).expect("state");

        let mut record = RecordEnvelope::new(2, "tenant_a", "Product");
        record
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(500.0));

        state.upsert(record).expect("upsert");
        let key = (
            idb_core::TenantId("tenant_a".to_string()),
            idb_core::RecordId(2),
        );
        let hot = state.hot_records().get(&key).expect("hot record");
        assert!(hot.space_key.is_some());

        fs::remove_dir_all(&base).expect("cleanup");
    }

    fn price_registry() -> DimensionRegistry {
        DimensionRegistry {
            version: 1,
            dimensions: vec![DimensionDefinition {
                name: "price".to_string(),
                source: DimensionSource::StructuredField("price".to_string()),
                dimension_type: DimensionType::Numeric,
                normalization: NormalizationPolicy::MinMax {
                    min: 0.0,
                    max: 1000.0,
                    bins: 32,
                },
                missing_value: MissingValuePolicy::Error,
            }],
        }
    }

    fn hot_space_key(state: &DurableState, tenant: &str, record_id: u64) -> u128 {
        let key = (TenantId(tenant.to_string()), RecordId(record_id));
        state
            .hot_records()
            .get(&key)
            .and_then(|record| record.space_key)
            .expect("space key")
    }

    #[test]
    fn key_range_candidates_stay_correct_across_updates_and_deletes() {
        let base = env::temp_dir().join(format!(
            "idb_storage_learned_range_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create tmp dir");

        let wal_path = base.join("wal.jsonl");
        let vis_path = base.join("visibility.txt");
        let indexer = SpatialIndexer::new(
            price_registry(),
            InterleavedKeyMapper::new(5).expect("mapper"),
        )
        .expect("indexer");
        let mut state =
            DurableState::new_with_indexer(&wal_path, &vis_path, Some(indexer)).expect("state");

        let mut a = RecordEnvelope::new(1, "tenant_a", "Product");
        a.structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        let mut b = RecordEnvelope::new(2, "tenant_a", "Product");
        b.structured_fields
            .insert("price".to_string(), FieldValue::Float(400.0));
        let mut c = RecordEnvelope::new(3, "tenant_a", "Product");
        c.structured_fields
            .insert("price".to_string(), FieldValue::Float(900.0));

        state.upsert(a).expect("upsert a");
        state.upsert(b.clone()).expect("upsert b");
        state.upsert(c).expect("upsert c");

        let old_key = hot_space_key(&state, "tenant_a", 2);
        let mut candidates = state.candidate_ids_for_key_ranges(
            &TenantId("tenant_a".to_string()),
            &[KeyRange {
                min: old_key,
                max: old_key,
            }],
        );
        assert_eq!(candidates, vec![RecordId(2)]);

        b.structured_fields
            .insert("price".to_string(), FieldValue::Float(950.0));
        state.upsert(b).expect("update b");
        let new_key = hot_space_key(&state, "tenant_a", 2);
        assert_ne!(old_key, new_key);

        candidates = state.candidate_ids_for_key_ranges(
            &TenantId("tenant_a".to_string()),
            &[KeyRange {
                min: old_key,
                max: old_key,
            }],
        );
        assert!(candidates.is_empty());

        candidates = state.candidate_ids_for_key_ranges(
            &TenantId("tenant_a".to_string()),
            &[KeyRange {
                min: new_key,
                max: new_key,
            }],
        );
        assert_eq!(candidates, vec![RecordId(2)]);

        state
            .delete(TenantId("tenant_a".to_string()), RecordId(2))
            .expect("delete b");
        candidates = state.candidate_ids_for_key_ranges(
            &TenantId("tenant_a".to_string()),
            &[KeyRange {
                min: new_key,
                max: new_key,
            }],
        );
        assert!(candidates.is_empty());

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn get_many_preserves_positions_with_missing_and_tenant_isolation() {
        let base = env::temp_dir().join(format!(
            "idb_storage_get_many_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create tmp dir");

        let wal_path = base.join("wal.jsonl");
        let vis_path = base.join("visibility.txt");
        let mut state = DurableState::new(&wal_path, &vis_path).expect("state");

        state
            .upsert(RecordEnvelope::new(10, "tenant_a", "Product"))
            .expect("upsert tenant_a id=10");
        state
            .upsert(RecordEnvelope::new(20, "tenant_a", "Product"))
            .expect("upsert tenant_a id=20");
        state
            .upsert(RecordEnvelope::new(10, "tenant_b", "Product"))
            .expect("upsert tenant_b id=10");

        let ids = vec![RecordId(20), RecordId(999), RecordId(10)];
        let fetched = state.get_many(&TenantId("tenant_a".to_string()), &ids);
        assert_eq!(fetched.len(), ids.len());

        assert_eq!(
            fetched[0].as_ref().map(|record| record.record_id.clone()),
            Some(RecordId(20))
        );
        assert!(fetched[1].is_none());
        assert_eq!(
            fetched[2].as_ref().map(|record| record.record_id.clone()),
            Some(RecordId(10))
        );
        assert!(fetched
            .iter()
            .flatten()
            .all(|record| record.tenant_id.0 == "tenant_a"));

        fs::remove_dir_all(&base).expect("cleanup");
    }
}
