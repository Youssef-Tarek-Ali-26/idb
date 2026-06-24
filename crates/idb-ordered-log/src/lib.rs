use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use idb_core::TenantId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrderedLogError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("topic already exists: tenant={tenant}, topic={topic}")]
    TopicExists { tenant: String, topic: String },
    #[error("topic not found: tenant={tenant}, topic={topic}")]
    TopicNotFound { tenant: String, topic: String },
    #[error("invalid partition count: {0}")]
    InvalidPartitionCount(u32),
    #[error(
        "invalid partition id {partition} for topic {topic} with {partition_count} partitions"
    )]
    InvalidPartition {
        topic: String,
        partition: u32,
        partition_count: u32,
    },
}

pub type OrderedLogResult<T> = Result<T, OrderedLogError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionPolicy {
    None,
    LatestByKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderedTopicConfig {
    pub partition_count: u32,
    pub retention_max_events_per_partition: Option<usize>,
    pub retention_max_age_seconds: Option<u64>,
    pub compaction_policy: CompactionPolicy,
}

impl OrderedTopicConfig {
    pub fn validate(&self) -> OrderedLogResult<()> {
        if self.partition_count == 0 {
            return Err(OrderedLogError::InvalidPartitionCount(self.partition_count));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderedEvent {
    pub tenant_id: TenantId,
    pub topic: String,
    pub partition: u32,
    pub sequence: u64,
    pub partition_key: String,
    pub key: Option<String>,
    pub payload: Value,
    pub committed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerOffset {
    pub group: String,
    pub partition: u32,
    pub committed_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicStats {
    pub partition_count: u32,
    pub event_count: usize,
    pub next_sequence_per_partition: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TopicKey {
    tenant: String,
    topic: String,
}

impl TopicKey {
    fn new(tenant_id: &TenantId, topic: &str) -> Self {
        Self {
            tenant: tenant_id.0.clone(),
            topic: topic.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TopicMetadata {
    tenant_id: TenantId,
    topic: String,
    config: OrderedTopicConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroupOffsetsFile {
    offsets: Vec<ConsumerOffset>,
}

#[derive(Debug, Clone)]
struct TopicState {
    config: OrderedTopicConfig,
    partitions: Vec<Vec<OrderedEvent>>,
    next_sequence_per_partition: Vec<u64>,
}

#[derive(Debug)]
pub struct OrderedLog {
    data_dir: PathBuf,
    topics: HashMap<TopicKey, TopicState>,
    group_offsets: HashMap<(TopicKey, String, u32), u64>,
}

impl OrderedLog {
    pub fn new(data_dir: impl AsRef<Path>) -> OrderedLogResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        fs::create_dir_all(Self::topics_root(&data_dir))?;

        let mut log = Self {
            data_dir,
            topics: HashMap::new(),
            group_offsets: HashMap::new(),
        };
        log.load_from_disk()?;
        Ok(log)
    }

    pub fn create_topic(
        &mut self,
        tenant_id: TenantId,
        topic: impl Into<String>,
        config: OrderedTopicConfig,
    ) -> OrderedLogResult<()> {
        config.validate()?;
        let topic = topic.into();
        let key = TopicKey::new(&tenant_id, &topic);
        if self.topics.contains_key(&key) {
            return Err(OrderedLogError::TopicExists {
                tenant: tenant_id.0,
                topic,
            });
        }

        let topic_path = self.topic_path_from_key(&key);
        fs::create_dir_all(&topic_path)?;

        let metadata = TopicMetadata {
            tenant_id,
            topic: key.topic.clone(),
            config: config.clone(),
        };
        Self::write_json_file(topic_path.join("metadata.json"), &metadata)?;

        for partition in 0..config.partition_count {
            Self::ensure_partition_file(&topic_path, partition)?;
        }
        Self::write_json_file(
            topic_path.join("groups.json"),
            &GroupOffsetsFile { offsets: vec![] },
        )?;

        let partition_count = config.partition_count as usize;
        self.topics.insert(
            key,
            TopicState {
                config,
                partitions: vec![Vec::new(); partition_count],
                next_sequence_per_partition: vec![1; partition_count],
            },
        );

        Ok(())
    }

    pub fn append(
        &mut self,
        tenant_id: &TenantId,
        topic: &str,
        partition_key: &str,
        key: Option<String>,
        payload: Value,
    ) -> OrderedLogResult<OrderedEvent> {
        let topic_key = TopicKey::new(tenant_id, topic);
        let topic_path = self.topic_path_from_key(&topic_key);
        let state =
            self.topics
                .get_mut(&topic_key)
                .ok_or_else(|| OrderedLogError::TopicNotFound {
                    tenant: tenant_id.0.clone(),
                    topic: topic.to_string(),
                })?;

        let partition = partition_for_key(partition_key, state.config.partition_count);
        let partition_index = partition as usize;
        let sequence = state.next_sequence_per_partition[partition_index];
        state.next_sequence_per_partition[partition_index] += 1;

        let event = OrderedEvent {
            tenant_id: tenant_id.clone(),
            topic: topic.to_string(),
            partition,
            sequence,
            partition_key: partition_key.to_string(),
            key,
            payload,
            committed_at: Utc::now(),
        };

        state.partitions[partition_index].push(event.clone());
        Self::append_event_line(&topic_path, partition, &event)?;

        let changed =
            Self::apply_partition_policies(&mut state.partitions[partition_index], &state.config);
        if changed {
            Self::rewrite_partition_file(
                &topic_path,
                partition,
                &state.partitions[partition_index],
            )?;
        }

        Ok(event)
    }

    pub fn replay_from(
        &self,
        tenant_id: &TenantId,
        topic: &str,
        partition: u32,
        from_sequence: u64,
        limit: usize,
    ) -> OrderedLogResult<Vec<OrderedEvent>> {
        let topic_key = TopicKey::new(tenant_id, topic);
        let state = self
            .topics
            .get(&topic_key)
            .ok_or_else(|| OrderedLogError::TopicNotFound {
                tenant: tenant_id.0.clone(),
                topic: topic.to_string(),
            })?;

        Self::validate_partition(state, topic, partition)?;

        let events = state.partitions[partition as usize]
            .iter()
            .filter(|event| event.sequence >= from_sequence)
            .take(limit)
            .cloned()
            .collect();

        Ok(events)
    }

    pub fn poll_consumer_group(
        &self,
        tenant_id: &TenantId,
        topic: &str,
        group: &str,
        max_events_per_partition: usize,
    ) -> OrderedLogResult<Vec<OrderedEvent>> {
        let topic_key = TopicKey::new(tenant_id, topic);
        let state = self
            .topics
            .get(&topic_key)
            .ok_or_else(|| OrderedLogError::TopicNotFound {
                tenant: tenant_id.0.clone(),
                topic: topic.to_string(),
            })?;

        let mut out = Vec::new();
        for partition in 0..state.config.partition_count {
            let committed = self
                .group_offsets
                .get(&(topic_key.clone(), group.to_string(), partition))
                .copied()
                .unwrap_or(0);
            let next_sequence = committed.saturating_add(1);

            out.extend(
                state.partitions[partition as usize]
                    .iter()
                    .filter(|event| event.sequence >= next_sequence)
                    .take(max_events_per_partition)
                    .cloned(),
            );
        }

        out.sort_by_key(|event| (event.partition, event.sequence));
        Ok(out)
    }

    pub fn commit_consumer_group_offset(
        &mut self,
        tenant_id: &TenantId,
        topic: &str,
        group: &str,
        partition: u32,
        committed_sequence: u64,
    ) -> OrderedLogResult<()> {
        let topic_key = TopicKey::new(tenant_id, topic);
        let topic_path = self.topic_path_from_key(&topic_key);
        let state = self
            .topics
            .get(&topic_key)
            .ok_or_else(|| OrderedLogError::TopicNotFound {
                tenant: tenant_id.0.clone(),
                topic: topic.to_string(),
            })?;

        Self::validate_partition(state, topic, partition)?;

        let offset_key = (topic_key.clone(), group.to_string(), partition);
        let current = self.group_offsets.get(&offset_key).copied().unwrap_or(0);
        let next = current.max(committed_sequence);
        self.group_offsets.insert(offset_key, next);

        self.persist_group_offsets_for_topic(&topic_key, &topic_path)?;
        Ok(())
    }

    pub fn topic_config(
        &self,
        tenant_id: &TenantId,
        topic: &str,
    ) -> OrderedLogResult<OrderedTopicConfig> {
        let key = TopicKey::new(tenant_id, topic);
        let state = self
            .topics
            .get(&key)
            .ok_or_else(|| OrderedLogError::TopicNotFound {
                tenant: tenant_id.0.clone(),
                topic: topic.to_string(),
            })?;
        Ok(state.config.clone())
    }

    pub fn topic_stats(&self, tenant_id: &TenantId, topic: &str) -> OrderedLogResult<TopicStats> {
        let key = TopicKey::new(tenant_id, topic);
        let state = self
            .topics
            .get(&key)
            .ok_or_else(|| OrderedLogError::TopicNotFound {
                tenant: tenant_id.0.clone(),
                topic: topic.to_string(),
            })?;

        let event_count = state.partitions.iter().map(Vec::len).sum();
        Ok(TopicStats {
            partition_count: state.config.partition_count,
            event_count,
            next_sequence_per_partition: state.next_sequence_per_partition.clone(),
        })
    }

    pub fn consumer_offsets(
        &self,
        tenant_id: &TenantId,
        topic: &str,
        group: &str,
    ) -> OrderedLogResult<Vec<ConsumerOffset>> {
        let key = TopicKey::new(tenant_id, topic);
        let state = self
            .topics
            .get(&key)
            .ok_or_else(|| OrderedLogError::TopicNotFound {
                tenant: tenant_id.0.clone(),
                topic: topic.to_string(),
            })?;

        let mut out = Vec::with_capacity(state.config.partition_count as usize);
        for partition in 0..state.config.partition_count {
            let committed = self
                .group_offsets
                .get(&(key.clone(), group.to_string(), partition))
                .copied()
                .unwrap_or(0);
            out.push(ConsumerOffset {
                group: group.to_string(),
                partition,
                committed_sequence: committed,
            });
        }
        Ok(out)
    }

    fn load_from_disk(&mut self) -> OrderedLogResult<()> {
        let topics_root = Self::topics_root(&self.data_dir);
        if !topics_root.exists() {
            return Ok(());
        }

        for tenant_entry in fs::read_dir(&topics_root)? {
            let tenant_entry = tenant_entry?;
            if !tenant_entry.file_type()?.is_dir() {
                continue;
            }

            for topic_entry in fs::read_dir(tenant_entry.path())? {
                let topic_entry = topic_entry?;
                if !topic_entry.file_type()?.is_dir() {
                    continue;
                }

                let topic_path = topic_entry.path();
                let metadata_path = topic_path.join("metadata.json");
                if !metadata_path.exists() {
                    continue;
                }

                let metadata: TopicMetadata = Self::read_json_file(metadata_path)?;
                metadata.config.validate()?;

                let key = TopicKey::new(&metadata.tenant_id, &metadata.topic);
                let mut partitions = Vec::with_capacity(metadata.config.partition_count as usize);
                let mut next_sequence_per_partition =
                    Vec::with_capacity(metadata.config.partition_count as usize);

                for partition in 0..metadata.config.partition_count {
                    let events = Self::read_partition_file(&topic_path, partition)?;
                    let next_sequence = events
                        .iter()
                        .map(|event| event.sequence)
                        .max()
                        .unwrap_or(0)
                        .saturating_add(1);
                    partitions.push(events);
                    next_sequence_per_partition.push(next_sequence.max(1));
                }

                self.topics.insert(
                    key.clone(),
                    TopicState {
                        config: metadata.config,
                        partitions,
                        next_sequence_per_partition,
                    },
                );

                self.load_group_offsets_for_topic(&key, &topic_path)?;
            }
        }

        Ok(())
    }

    fn load_group_offsets_for_topic(
        &mut self,
        topic_key: &TopicKey,
        topic_path: &Path,
    ) -> OrderedLogResult<()> {
        let groups_path = topic_path.join("groups.json");
        if !groups_path.exists() {
            return Ok(());
        }

        let data: GroupOffsetsFile = Self::read_json_file(groups_path)?;
        for offset in data.offsets {
            self.group_offsets.insert(
                (topic_key.clone(), offset.group, offset.partition),
                offset.committed_sequence,
            );
        }
        Ok(())
    }

    fn persist_group_offsets_for_topic(
        &self,
        topic_key: &TopicKey,
        topic_path: &Path,
    ) -> OrderedLogResult<()> {
        let mut offsets = self
            .group_offsets
            .iter()
            .filter(|((key, _, _), _)| key == topic_key)
            .map(
                |((_, group, partition), committed_sequence)| ConsumerOffset {
                    group: group.clone(),
                    partition: *partition,
                    committed_sequence: *committed_sequence,
                },
            )
            .collect::<Vec<_>>();

        offsets.sort_by(|a, b| {
            a.group
                .cmp(&b.group)
                .then_with(|| a.partition.cmp(&b.partition))
        });

        Self::write_json_file(
            topic_path.join("groups.json"),
            &GroupOffsetsFile { offsets },
        )
    }

    fn validate_partition(state: &TopicState, topic: &str, partition: u32) -> OrderedLogResult<()> {
        if partition >= state.config.partition_count {
            return Err(OrderedLogError::InvalidPartition {
                topic: topic.to_string(),
                partition,
                partition_count: state.config.partition_count,
            });
        }
        Ok(())
    }

    fn apply_partition_policies(
        events: &mut Vec<OrderedEvent>,
        config: &OrderedTopicConfig,
    ) -> bool {
        let mut changed = false;

        if matches!(config.compaction_policy, CompactionPolicy::LatestByKey) {
            let compacted = compact_latest_by_key(events);
            if compacted.len() != events.len() {
                *events = compacted;
                changed = true;
            }
        }

        if let Some(max_events) = config.retention_max_events_per_partition {
            if events.len() > max_events {
                let start = events.len().saturating_sub(max_events);
                events.drain(0..start);
                changed = true;
            }
        }

        if let Some(max_age_seconds) = config.retention_max_age_seconds {
            let cutoff = Utc::now() - Duration::seconds(max_age_seconds as i64);
            let before = events.len();
            events.retain(|event| event.committed_at >= cutoff);
            changed |= events.len() != before;
        }

        changed
    }

    fn topics_root(data_dir: &Path) -> PathBuf {
        data_dir.join("topics")
    }

    fn topic_path_from_key(&self, key: &TopicKey) -> PathBuf {
        Self::topics_root(&self.data_dir)
            .join(hex_component(&key.tenant))
            .join(hex_component(&key.topic))
    }

    fn ensure_partition_file(topic_path: &Path, partition: u32) -> OrderedLogResult<()> {
        let path = partition_file_path(topic_path, partition);
        if !path.exists() {
            File::create(path)?;
        }
        Ok(())
    }

    fn read_partition_file(
        topic_path: &Path,
        partition: u32,
    ) -> OrderedLogResult<Vec<OrderedEvent>> {
        let path = partition_file_path(topic_path, partition);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            events.push(serde_json::from_str::<OrderedEvent>(&line)?);
        }
        events.sort_by_key(|event| event.sequence);
        Ok(events)
    }

    fn append_event_line(
        topic_path: &Path,
        partition: u32,
        event: &OrderedEvent,
    ) -> OrderedLogResult<()> {
        let path = partition_file_path(topic_path, partition);
        let mut file = OpenOptions::new().append(true).create(true).open(path)?;
        let line = serde_json::to_string(event)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.flush()?;
        Ok(())
    }

    fn rewrite_partition_file(
        topic_path: &Path,
        partition: u32,
        events: &[OrderedEvent],
    ) -> OrderedLogResult<()> {
        let path = partition_file_path(topic_path, partition);
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        for event in events {
            let line = serde_json::to_string(event)?;
            writer.write_all(line.as_bytes())?;
            writer.write_all(b"\n")?;
        }
        writer.flush()?;
        Ok(())
    }

    fn write_json_file<T: Serialize>(path: PathBuf, value: &T) -> OrderedLogResult<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, value)?;
        writer.flush()?;
        Ok(())
    }

    fn read_json_file<T: for<'de> Deserialize<'de>>(path: PathBuf) -> OrderedLogResult<T> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }
}

fn partition_file_path(topic_path: &Path, partition: u32) -> PathBuf {
    topic_path.join(format!("partition-{partition}.jsonl"))
}

fn partition_for_key(partition_key: &str, partition_count: u32) -> u32 {
    if partition_count <= 1 {
        return 0;
    }
    (stable_fnv1a_hash(partition_key.as_bytes()) % partition_count as u64) as u32
}

fn stable_fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn hex_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for byte in input.as_bytes() {
        out.push(hex_digit(byte >> 4));
        out.push(hex_digit(byte & 0x0f));
    }
    out
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => '0',
    }
}

fn compact_latest_by_key(events: &[OrderedEvent]) -> Vec<OrderedEvent> {
    let mut kept = Vec::with_capacity(events.len());
    let mut seen_keys: HashSet<String> = HashSet::new();

    for event in events.iter().rev() {
        match &event.key {
            Some(key) => {
                if seen_keys.insert(key.clone()) {
                    kept.push(event.clone());
                }
            }
            None => kept.push(event.clone()),
        }
    }

    kept.reverse();
    kept
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{CompactionPolicy, OrderedLog, OrderedLogResult, OrderedTopicConfig};
    use idb_core::TenantId;
    use serde_json::json;

    fn temp_path(test_name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!(
            "idb_ordered_log_{}_{}_{}",
            test_name,
            std::process::id(),
            nanos
        ))
    }

    fn config(partitions: u32) -> OrderedTopicConfig {
        OrderedTopicConfig {
            partition_count: partitions,
            retention_max_events_per_partition: None,
            retention_max_age_seconds: None,
            compaction_policy: CompactionPolicy::None,
        }
    }

    #[test]
    fn append_and_replay_preserves_partition_order() -> OrderedLogResult<()> {
        let path = temp_path("append_replay_order");
        let tenant = TenantId("tenant_a".to_string());
        let mut log = OrderedLog::new(&path)?;
        log.create_topic(tenant.clone(), "orders", config(4))?;

        let a = log.append(
            &tenant,
            "orders",
            "order-123",
            Some("a".to_string()),
            json!({"v": 1}),
        )?;
        let b = log.append(
            &tenant,
            "orders",
            "order-123",
            Some("b".to_string()),
            json!({"v": 2}),
        )?;
        let c = log.append(
            &tenant,
            "orders",
            "order-123",
            Some("c".to_string()),
            json!({"v": 3}),
        )?;

        assert_eq!(a.partition, b.partition);
        assert_eq!(b.partition, c.partition);

        let replay = log.replay_from(&tenant, "orders", a.partition, 1, 10)?;
        let sequences = replay.iter().map(|e| e.sequence).collect::<Vec<_>>();
        assert_eq!(sequences, vec![a.sequence, b.sequence, c.sequence]);

        std::fs::remove_dir_all(path).ok();
        Ok(())
    }

    #[test]
    fn consumer_group_poll_and_commit_advances_offsets() -> OrderedLogResult<()> {
        let path = temp_path("consumer_group_offsets");
        let tenant = TenantId("tenant_a".to_string());
        let mut log = OrderedLog::new(&path)?;
        log.create_topic(tenant.clone(), "jobs", config(1))?;

        log.append(&tenant, "jobs", "job-1", None, json!({"job": 1}))?;
        log.append(&tenant, "jobs", "job-2", None, json!({"job": 2}))?;
        let last = log.append(&tenant, "jobs", "job-3", None, json!({"job": 3}))?;

        let first_poll = log.poll_consumer_group(&tenant, "jobs", "workers", 2)?;
        assert_eq!(first_poll.len(), 2);

        log.commit_consumer_group_offset(&tenant, "jobs", "workers", 0, first_poll[1].sequence)?;

        let second_poll = log.poll_consumer_group(&tenant, "jobs", "workers", 5)?;
        assert_eq!(second_poll.len(), 1);
        assert_eq!(second_poll[0].sequence, last.sequence);

        std::fs::remove_dir_all(path).ok();
        Ok(())
    }

    #[test]
    fn retention_max_events_enforced() -> OrderedLogResult<()> {
        let path = temp_path("retention_max_events");
        let tenant = TenantId("tenant_a".to_string());
        let mut log = OrderedLog::new(&path)?;
        let mut topic_config = config(1);
        topic_config.retention_max_events_per_partition = Some(2);
        log.create_topic(tenant.clone(), "events", topic_config)?;

        log.append(&tenant, "events", "k", None, json!({"n": 1}))?;
        log.append(&tenant, "events", "k", None, json!({"n": 2}))?;
        let third = log.append(&tenant, "events", "k", None, json!({"n": 3}))?;
        let fourth = log.append(&tenant, "events", "k", None, json!({"n": 4}))?;

        let replay = log.replay_from(&tenant, "events", 0, 1, 10)?;
        assert_eq!(replay.len(), 2);
        assert_eq!(replay[0].sequence, third.sequence);
        assert_eq!(replay[1].sequence, fourth.sequence);

        std::fs::remove_dir_all(path).ok();
        Ok(())
    }

    #[test]
    fn compaction_latest_by_key_keeps_latest_versions() -> OrderedLogResult<()> {
        let path = temp_path("compaction_latest");
        let tenant = TenantId("tenant_a".to_string());
        let mut log = OrderedLog::new(&path)?;
        let mut topic_config = config(1);
        topic_config.compaction_policy = CompactionPolicy::LatestByKey;
        log.create_topic(tenant.clone(), "events", topic_config)?;

        log.append(
            &tenant,
            "events",
            "k",
            Some("a".to_string()),
            json!({"v": 1}),
        )?;
        log.append(
            &tenant,
            "events",
            "k",
            Some("b".to_string()),
            json!({"v": 1}),
        )?;
        let newest_a = log.append(
            &tenant,
            "events",
            "k",
            Some("a".to_string()),
            json!({"v": 2}),
        )?;
        let no_key = log.append(&tenant, "events", "k", None, json!({"v": 3}))?;

        let replay = log.replay_from(&tenant, "events", 0, 1, 10)?;
        assert_eq!(replay.len(), 3);
        assert!(replay.iter().any(|e| e.sequence == newest_a.sequence));
        assert!(replay.iter().any(|e| e.sequence == no_key.sequence));
        assert!(!replay.iter().any(|e| e.sequence == 1));

        std::fs::remove_dir_all(path).ok();
        Ok(())
    }

    #[test]
    fn state_persists_across_reopen() -> OrderedLogResult<()> {
        let path = temp_path("persist_reopen");
        let tenant = TenantId("tenant_a".to_string());

        {
            let mut log = OrderedLog::new(&path)?;
            log.create_topic(tenant.clone(), "events", config(1))?;
            log.append(&tenant, "events", "pk", None, json!({"v": 1}))?;
            log.append(&tenant, "events", "pk", None, json!({"v": 2}))?;
            log.commit_consumer_group_offset(&tenant, "events", "g1", 0, 1)?;
        }

        {
            let log = OrderedLog::new(&path)?;
            let replay = log.replay_from(&tenant, "events", 0, 1, 10)?;
            assert_eq!(replay.len(), 2);

            let polled = log.poll_consumer_group(&tenant, "events", "g1", 10)?;
            assert_eq!(polled.len(), 1);
            assert_eq!(polled[0].sequence, 2);
        }

        std::fs::remove_dir_all(path).ok();
        Ok(())
    }
}
