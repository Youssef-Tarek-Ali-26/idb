use std::collections::{BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use idb_core::{CoreError, CoreResult, RecordId, TenantId};

use crate::events::MutationEvent;
use crate::state::DurableState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResumeToken(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub id: SubscriptionId,
    pub tenant_id: TenantId,
    pub last_sequence: u64,
    pub dependency_filter: DependencyFilter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyFilter {
    AllRecords,
    RecordIds(BTreeSet<RecordId>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeBatch {
    pub subscription_id: SubscriptionId,
    pub events: Vec<MutationEvent>,
    pub next_resume_token: ResumeToken,
}

#[derive(Debug, Default)]
pub struct ChangefeedEngine {
    next_id: u64,
    subscriptions: HashMap<SubscriptionId, Subscription>,
}

impl ChangefeedEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&mut self, tenant_id: TenantId) -> SubscriptionId {
        self.subscribe_with_resume_and_dependencies(tenant_id, ResumeToken(0), None)
    }

    pub fn subscribe_with_resume(
        &mut self,
        tenant_id: TenantId,
        resume_token: ResumeToken,
    ) -> SubscriptionId {
        self.subscribe_with_resume_and_dependencies(tenant_id, resume_token, None)
    }

    pub fn subscribe_with_resume_and_dependencies(
        &mut self,
        tenant_id: TenantId,
        resume_token: ResumeToken,
        dependencies: Option<Vec<RecordId>>,
    ) -> SubscriptionId {
        self.next_id += 1;
        let id = SubscriptionId(self.next_id);
        let dependency_filter = match dependencies {
            Some(ids) => DependencyFilter::RecordIds(ids.into_iter().collect()),
            None => DependencyFilter::AllRecords,
        };
        self.subscriptions.insert(
            id,
            Subscription {
                id,
                tenant_id,
                last_sequence: resume_token.0,
                dependency_filter,
            },
        );
        id
    }

    pub fn get_subscription(&self, id: SubscriptionId) -> Option<&Subscription> {
        self.subscriptions.get(&id)
    }

    pub fn unsubscribe(&mut self, id: SubscriptionId) -> bool {
        self.subscriptions.remove(&id).is_some()
    }

    pub fn poll(
        &mut self,
        id: SubscriptionId,
        state: &DurableState,
        max_events: usize,
    ) -> CoreResult<ChangeBatch> {
        if max_events == 0 {
            return Err(CoreError::Storage(
                "changefeed poll max_events must be > 0".to_string(),
            ));
        }

        let subscription = self
            .subscriptions
            .get_mut(&id)
            .ok_or_else(|| CoreError::Storage(format!("unknown subscription id: {}", id.0)))?;

        let scan_limit = max_events.saturating_mul(4).max(max_events);
        let scanned_events = state.mutation_events_since(
            &subscription.tenant_id,
            subscription.last_sequence,
            scan_limit,
        );
        let last_scanned_sequence = scanned_events
            .last()
            .map(|event| event.commit_sequence)
            .unwrap_or(subscription.last_sequence);

        let mut events: Vec<MutationEvent> = scanned_events
            .into_iter()
            .filter(|event| subscription.matches_dependency(event))
            .take(max_events)
            .collect();

        // Keep deterministic sequence ordering even after filtering.
        events.sort_by_key(|event| event.commit_sequence);

        let next_sequence = events
            .last()
            .map(|event| event.commit_sequence)
            .unwrap_or(last_scanned_sequence);
        subscription.last_sequence = next_sequence;

        Ok(ChangeBatch {
            subscription_id: id,
            events,
            next_resume_token: ResumeToken(next_sequence),
        })
    }
}

impl Subscription {
    fn matches_dependency(&self, event: &MutationEvent) -> bool {
        match &self.dependency_filter {
            DependencyFilter::AllRecords => true,
            DependencyFilter::RecordIds(ids) => ids.contains(&event.record_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;

    use idb_core::RecordEnvelope;

    use super::{ChangefeedEngine, ResumeToken};
    use crate::DurableState;

    #[test]
    fn reconnect_with_resume_token_continues_from_next_sequence() {
        let base = env::temp_dir().join(format!(
            "idb_changefeed_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create tmp dir");

        let wal_path = base.join("wal.jsonl");
        let vis_path = base.join("visibility.txt");
        let mut state = DurableState::new(&wal_path, &vis_path).expect("state");

        state
            .upsert(RecordEnvelope::new(1, "tenant_a", "Product"))
            .expect("event 1");
        state
            .upsert(RecordEnvelope::new(2, "tenant_a", "Product"))
            .expect("event 2");

        let mut feed = ChangefeedEngine::new();
        let sub = feed.subscribe(idb_core::TenantId("tenant_a".to_string()));

        let first_batch = feed.poll(sub, &state, 10).expect("first poll");
        assert_eq!(first_batch.events.len(), 2);
        assert_eq!(first_batch.events[0].commit_sequence, 1);
        assert_eq!(first_batch.events[1].commit_sequence, 2);

        let resume = first_batch.next_resume_token;

        state
            .upsert(RecordEnvelope::new(3, "tenant_a", "Product"))
            .expect("event 3");

        let reconnect_sub = feed.subscribe_with_resume(
            idb_core::TenantId("tenant_a".to_string()),
            ResumeToken(resume.0),
        );

        let reconnect_batch = feed
            .poll(reconnect_sub, &state, 10)
            .expect("reconnect poll");
        assert_eq!(reconnect_batch.events.len(), 1);
        assert_eq!(reconnect_batch.events[0].commit_sequence, 3);

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn tenant_subscriptions_are_isolated() {
        let base = env::temp_dir().join(format!(
            "idb_changefeed_tenant_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create tmp dir");

        let wal_path = base.join("wal.jsonl");
        let vis_path = base.join("visibility.txt");
        let mut state = DurableState::new(&wal_path, &vis_path).expect("state");

        state
            .upsert(RecordEnvelope::new(11, "tenant_a", "Product"))
            .expect("tenant a event");
        state
            .upsert(RecordEnvelope::new(22, "tenant_b", "Product"))
            .expect("tenant b event");

        let mut feed = ChangefeedEngine::new();
        let a_sub = feed.subscribe(idb_core::TenantId("tenant_a".to_string()));
        let b_sub = feed.subscribe(idb_core::TenantId("tenant_b".to_string()));

        let a_batch = feed.poll(a_sub, &state, 10).expect("a poll");
        let b_batch = feed.poll(b_sub, &state, 10).expect("b poll");

        assert_eq!(a_batch.events.len(), 1);
        assert_eq!(b_batch.events.len(), 1);
        assert_eq!(a_batch.events[0].record_id.0, 11);
        assert_eq!(b_batch.events[0].record_id.0, 22);

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn dependency_filter_only_delivers_tracked_records() {
        let base = env::temp_dir().join(format!(
            "idb_changefeed_deps_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create tmp dir");

        let wal_path = base.join("wal.jsonl");
        let vis_path = base.join("visibility.txt");
        let mut state = DurableState::new(&wal_path, &vis_path).expect("state");

        state
            .upsert(RecordEnvelope::new(100, "tenant_a", "Product"))
            .expect("event 100");
        state
            .upsert(RecordEnvelope::new(200, "tenant_a", "Product"))
            .expect("event 200");

        let mut feed = ChangefeedEngine::new();
        let sub = feed.subscribe_with_resume_and_dependencies(
            idb_core::TenantId("tenant_a".to_string()),
            ResumeToken(0),
            Some(vec![idb_core::RecordId(200)]),
        );

        let batch = feed.poll(sub, &state, 10).expect("poll with deps");
        assert_eq!(batch.events.len(), 1);
        assert_eq!(batch.events[0].record_id.0, 200);

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn sparse_dependency_filter_poll_progresses_to_later_matches() {
        let base = env::temp_dir().join(format!(
            "idb_changefeed_sparse_deps_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create tmp dir");

        let wal_path = base.join("wal.jsonl");
        let vis_path = base.join("visibility.txt");
        let mut state = DurableState::new(&wal_path, &vis_path).expect("state");

        for id in 1..=12 {
            state
                .upsert(RecordEnvelope::new(id, "tenant_a", "Product"))
                .expect("seed event");
        }
        state
            .upsert(RecordEnvelope::new(999, "tenant_a", "Product"))
            .expect("tracked event");

        let mut feed = ChangefeedEngine::new();
        let sub = feed.subscribe_with_resume_and_dependencies(
            idb_core::TenantId("tenant_a".to_string()),
            ResumeToken(0),
            Some(vec![idb_core::RecordId(999)]),
        );

        let mut delivered = false;
        let mut prev_token = 0;
        for _ in 0..10 {
            let batch = feed.poll(sub, &state, 1).expect("poll with sparse deps");
            assert!(batch.next_resume_token.0 >= prev_token);
            prev_token = batch.next_resume_token.0;
            if let Some(event) = batch.events.first() {
                assert_eq!(event.record_id.0, 999);
                delivered = true;
                break;
            }
        }

        assert!(delivered, "expected to eventually reach tracked event");
        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn poll_rejects_zero_max_events() {
        let base = env::temp_dir().join(format!(
            "idb_changefeed_zero_max_events_test_{}_{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create tmp dir");

        let wal_path = base.join("wal.jsonl");
        let vis_path = base.join("visibility.txt");
        let state = DurableState::new(&wal_path, &vis_path).expect("state");

        let mut feed = ChangefeedEngine::new();
        let sub = feed.subscribe(idb_core::TenantId("tenant_a".to_string()));

        let err = feed
            .poll(sub, &state, 0)
            .expect_err("zero max_events must fail");
        assert!(err.to_string().contains("max_events must be > 0"));

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn unsubscribe_removes_subscription() {
        let mut feed = ChangefeedEngine::new();
        let sub = feed.subscribe(idb_core::TenantId("tenant_a".to_string()));
        assert!(feed.get_subscription(sub).is_some());

        assert!(feed.unsubscribe(sub));
        assert!(feed.get_subscription(sub).is_none());
        assert!(!feed.unsubscribe(sub));
    }
}
