use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use chrono::Utc;
use idb_core::{
    compare_field_values, fields_match_predicates, AuthAction, AuthRuntime, BackendCapabilities,
    CallerContext, CoreError, CoreResult, DimensionRegistry, EngineMetrics, FieldValue,
    HydratedRecord, InterleavedKeyMapper, KeyRange, Predicate, QueryOrderDirection, QueryRequest,
    QueryTrace, RecordEnvelope, RecordId, ScoredRecord, StageTrace, StageType, StorageBackend,
    TenantId,
};
use idb_ordered_log::{CompactionPolicy, OrderedLog, OrderedLogError, OrderedTopicConfig};
use idb_planner::{
    explain_query_text as planner_explain_query_text, logical_plan_to_query_request_for_execution,
    plan_query_text, LiteralExpr, PlanMode, PlanSource, PlanTraversalDirection, PlanTraversalStep,
    QueryExplain, QueryRequestBridgeOptions,
};
use idb_storage::{
    ChangeBatch, ChangefeedEngine, CompactRecord, DurableState, MutationEvent, ResumeToken,
    SpatialIndexer, SubscriptionId,
};

const DURABLE_MUTATION_TOPIC: &str = "mutation-events-v1";
const DURABLE_MUTATION_TOPIC_PARTITIONS: u32 = 8;

#[derive(Debug)]
pub struct CpuBackend {
    state: DurableState,
    ordered_log: OrderedLog,
    changefeed: ChangefeedEngine,
    active_watches: HashMap<SubscriptionId, ActiveWatchState>,
    auth_runtime: AuthRuntime,
    metrics: EngineMetrics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WatchQuerySession {
    pub subscription_id: SubscriptionId,
    pub resume_token: ResumeToken,
    pub snapshot: Vec<HydratedRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WatchQueryUpdate {
    pub event: MutationEvent,
    pub current: Option<HydratedRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WatchQueryUpdateBatch {
    pub subscription_id: SubscriptionId,
    pub updates: Vec<WatchQueryUpdate>,
    pub next_resume_token: ResumeToken,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DurableMutationRecord {
    pub partition: u32,
    pub sequence: u64,
    pub event: MutationEvent,
}

#[derive(Debug, Clone)]
struct ActiveWatchState {
    request: QueryRequest,
}

impl CpuBackend {
    pub fn new(data_dir: impl AsRef<Path>) -> CoreResult<Self> {
        Self::new_with_auth(data_dir, AuthRuntime::disabled())
    }

    pub fn new_with_auth(
        data_dir: impl AsRef<Path>,
        auth_runtime: AuthRuntime,
    ) -> CoreResult<Self> {
        Self::new_inner(data_dir, None, auth_runtime)
    }

    pub fn new_with_spatial_indexer(
        data_dir: impl AsRef<Path>,
        spatial_indexer: SpatialIndexer,
    ) -> CoreResult<Self> {
        Self::new_with_spatial_indexer_and_auth(data_dir, spatial_indexer, AuthRuntime::disabled())
    }

    pub fn new_with_spatial_indexer_and_auth(
        data_dir: impl AsRef<Path>,
        spatial_indexer: SpatialIndexer,
        auth_runtime: AuthRuntime,
    ) -> CoreResult<Self> {
        Self::new_inner(data_dir, Some(spatial_indexer), auth_runtime)
    }

    pub fn new_with_spatial_registry(
        data_dir: impl AsRef<Path>,
        registry: DimensionRegistry,
        bits_per_dimension: u8,
    ) -> CoreResult<Self> {
        Self::new_with_spatial_registry_and_auth(
            data_dir,
            registry,
            bits_per_dimension,
            AuthRuntime::disabled(),
        )
    }

    pub fn new_with_spatial_registry_and_auth(
        data_dir: impl AsRef<Path>,
        registry: DimensionRegistry,
        bits_per_dimension: u8,
        auth_runtime: AuthRuntime,
    ) -> CoreResult<Self> {
        let mapper = InterleavedKeyMapper::new(bits_per_dimension)?;
        let indexer = SpatialIndexer::new(registry, mapper)?;
        Self::new_with_spatial_indexer_and_auth(data_dir, indexer, auth_runtime)
    }

    fn new_inner(
        data_dir: impl AsRef<Path>,
        spatial_indexer: Option<SpatialIndexer>,
        auth_runtime: AuthRuntime,
    ) -> CoreResult<Self> {
        let data_dir = data_dir.as_ref();
        std::fs::create_dir_all(data_dir)
            .map_err(|e| CoreError::Storage(format!("failed creating data dir: {e}")))?;

        let wal_path = data_dir.join("wal.jsonl");
        let visibility_path = data_dir.join("visibility.txt");
        let state = DurableState::new_with_indexer(wal_path, visibility_path, spatial_indexer)?;
        let ordered_log_dir = data_dir.join("ordered-log");
        let ordered_log =
            OrderedLog::new(ordered_log_dir).map_err(Self::ordered_log_error_to_core)?;
        Ok(Self {
            state,
            ordered_log,
            changefeed: ChangefeedEngine::new(),
            active_watches: HashMap::new(),
            auth_runtime,
            metrics: EngineMetrics::default(),
        })
    }

    pub fn run_query(&mut self, query: &QueryRequest) -> CoreResult<Vec<HydratedRecord>> {
        let caller = CallerContext::system_for_tenant(query.tenant_id.clone());
        let (results, _) = self.run_query_with_trace_with_context(&caller, query)?;
        Ok(results)
    }

    pub fn run_query_with_context(
        &mut self,
        caller: &CallerContext,
        query: &QueryRequest,
    ) -> CoreResult<Vec<HydratedRecord>> {
        let (results, _) = self.run_query_with_trace_with_context(caller, query)?;
        Ok(results)
    }

    pub fn run_query_text(
        &mut self,
        tenant_id: TenantId,
        query_text: &str,
    ) -> CoreResult<Vec<HydratedRecord>> {
        let caller = CallerContext::system_for_tenant(tenant_id.clone());
        let options = QueryRequestBridgeOptions {
            tenant_id,
            top_k_default: 100,
            score_policy: Default::default(),
            semantic_embedding_field: "text_embedding".to_string(),
            semantic_embedding_dims: 16,
        };
        self.run_query_text_with_options_and_context(query_text, options, &caller)
    }

    pub fn run_query_text_with_options(
        &mut self,
        query_text: &str,
        options: QueryRequestBridgeOptions,
    ) -> CoreResult<Vec<HydratedRecord>> {
        let caller = CallerContext::system_for_tenant(options.tenant_id.clone());
        self.run_query_text_with_options_and_context(query_text, options, &caller)
    }

    pub fn run_query_text_with_options_and_context(
        &mut self,
        query_text: &str,
        options: QueryRequestBridgeOptions,
        caller: &CallerContext,
    ) -> CoreResult<Vec<HydratedRecord>> {
        self.authorize_caller(caller, AuthAction::Query, &options.tenant_id)?;
        let plan =
            plan_query_text(query_text).map_err(|e| CoreError::QueryPlanning(e.to_string()))?;
        let request = logical_plan_to_query_request_for_execution(&plan, options)
            .map_err(|e| CoreError::QueryPlanning(e.to_string()))?;
        self.execute_compiled_text_plan(caller, &plan, &request)
    }

    pub fn explain_query_text(
        &self,
        tenant_id: TenantId,
        query_text: &str,
    ) -> CoreResult<QueryExplain> {
        let caller = CallerContext::system_for_tenant(tenant_id.clone());
        let options = QueryRequestBridgeOptions {
            tenant_id,
            top_k_default: 100,
            score_policy: Default::default(),
            semantic_embedding_field: "text_embedding".to_string(),
            semantic_embedding_dims: 16,
        };
        self.explain_query_text_with_options_and_context(query_text, options, &caller)
    }

    pub fn explain_query_text_with_options(
        &self,
        query_text: &str,
        options: QueryRequestBridgeOptions,
    ) -> CoreResult<QueryExplain> {
        let caller = CallerContext::system_for_tenant(options.tenant_id.clone());
        self.explain_query_text_with_options_and_context(query_text, options, &caller)
    }

    pub fn explain_query_text_with_options_and_context(
        &self,
        query_text: &str,
        options: QueryRequestBridgeOptions,
        caller: &CallerContext,
    ) -> CoreResult<QueryExplain> {
        self.authorize_caller(caller, AuthAction::Explain, &options.tenant_id)?;
        planner_explain_query_text(query_text, options)
            .map_err(|e| CoreError::QueryPlanning(e.to_string()))
    }

    pub fn start_watch_query_text(
        &mut self,
        tenant_id: TenantId,
        query_text: &str,
    ) -> CoreResult<WatchQuerySession> {
        let caller = CallerContext::system_for_tenant(tenant_id.clone());
        let options = QueryRequestBridgeOptions {
            tenant_id,
            top_k_default: 100,
            score_policy: Default::default(),
            semantic_embedding_field: "text_embedding".to_string(),
            semantic_embedding_dims: 16,
        };
        self.start_watch_query_text_with_options_and_context(query_text, options, &caller)
    }

    pub fn start_watch_query_text_with_options(
        &mut self,
        query_text: &str,
        options: QueryRequestBridgeOptions,
    ) -> CoreResult<WatchQuerySession> {
        let caller = CallerContext::system_for_tenant(options.tenant_id.clone());
        self.start_watch_query_text_with_options_and_context(query_text, options, &caller)
    }

    pub fn start_watch_query_text_with_options_and_context(
        &mut self,
        query_text: &str,
        options: QueryRequestBridgeOptions,
        caller: &CallerContext,
    ) -> CoreResult<WatchQuerySession> {
        self.authorize_caller(caller, AuthAction::Watch, &options.tenant_id)?;
        let mut plan =
            plan_query_text(query_text).map_err(|e| CoreError::QueryPlanning(e.to_string()))?;
        if !matches!(plan.mode, PlanMode::Watch) {
            return Err(CoreError::QueryPlanning(
                "watch session API requires query text prefixed with `watch`".to_string(),
            ));
        }

        plan.mode = PlanMode::Once;
        let request = logical_plan_to_query_request_for_execution(&plan, options.clone())
            .map_err(|e| CoreError::QueryPlanning(e.to_string()))?;
        let snapshot = self.execute_compiled_text_plan(caller, &plan, &request)?;
        let dependencies = snapshot
            .iter()
            .map(|row| row.envelope.record_id.clone())
            .collect::<Vec<_>>();

        let resume_token = ResumeToken(self.state.last_sequence());
        let subscription_id = self.changefeed.subscribe_with_resume_and_dependencies(
            options.tenant_id,
            resume_token,
            Some(dependencies),
        );
        self.active_watches.insert(
            subscription_id,
            ActiveWatchState {
                request: request.clone(),
            },
        );

        Ok(WatchQuerySession {
            subscription_id,
            resume_token,
            snapshot,
        })
    }

    pub fn poll_watch(
        &mut self,
        subscription_id: SubscriptionId,
        max_events: usize,
    ) -> CoreResult<ChangeBatch> {
        self.changefeed
            .poll(subscription_id, &self.state, max_events)
    }

    pub fn stop_watch(&mut self, subscription_id: SubscriptionId) -> bool {
        let removed_active = self.active_watches.remove(&subscription_id).is_some();
        let removed_subscription = self.changefeed.unsubscribe(subscription_id);
        removed_active || removed_subscription
    }

    pub fn poll_watch_query_updates(
        &mut self,
        subscription_id: SubscriptionId,
        max_events: usize,
    ) -> CoreResult<WatchQueryUpdateBatch> {
        if max_events == 0 {
            return Err(CoreError::QueryPlanning(
                "watch poll max_events must be > 0".to_string(),
            ));
        }

        let request = self
            .active_watches
            .get(&subscription_id)
            .map(|watch| watch.request.clone())
            .ok_or_else(|| {
                CoreError::QueryPlanning(format!(
                    "unknown active watch subscription id: {}",
                    subscription_id.0
                ))
            })?;

        let batch = self.poll_watch(subscription_id, max_events)?;
        let updates = batch
            .events
            .iter()
            .map(|event| {
                let current = self.resolve_watch_current(&request, event)?;
                Ok(WatchQueryUpdate {
                    event: event.clone(),
                    current,
                })
            })
            .collect::<CoreResult<Vec<_>>>()?;

        Ok(WatchQueryUpdateBatch {
            subscription_id,
            updates,
            next_resume_token: batch.next_resume_token,
        })
    }

    pub fn poll_durable_mutation_stream(
        &mut self,
        tenant_id: &TenantId,
        consumer_group: &str,
        max_events_per_partition: usize,
    ) -> CoreResult<Vec<DurableMutationRecord>> {
        if consumer_group.trim().is_empty() {
            return Err(CoreError::QueryPlanning(
                "durable stream consumer_group must be non-empty".to_string(),
            ));
        }
        if max_events_per_partition == 0 {
            return Err(CoreError::QueryPlanning(
                "durable stream poll max_events_per_partition must be > 0".to_string(),
            ));
        }

        self.ensure_durable_mutation_topic(tenant_id)?;
        let events = self
            .ordered_log
            .poll_consumer_group(
                tenant_id,
                DURABLE_MUTATION_TOPIC,
                consumer_group,
                max_events_per_partition,
            )
            .map_err(Self::ordered_log_error_to_core)?;

        events
            .into_iter()
            .map(|record| {
                let event =
                    serde_json::from_value::<MutationEvent>(record.payload).map_err(|e| {
                        CoreError::Serialization(format!(
                            "failed decoding durable mutation event payload: {e}"
                        ))
                    })?;
                Ok(DurableMutationRecord {
                    partition: record.partition,
                    sequence: record.sequence,
                    event,
                })
            })
            .collect()
    }

    pub fn poll_durable_mutation_stream_with_context(
        &mut self,
        caller: &CallerContext,
        tenant_id: &TenantId,
        consumer_group: &str,
        max_events_per_partition: usize,
    ) -> CoreResult<Vec<DurableMutationRecord>> {
        self.authorize_caller(caller, AuthAction::Watch, tenant_id)?;
        self.poll_durable_mutation_stream(tenant_id, consumer_group, max_events_per_partition)
    }

    pub fn commit_durable_mutation_offsets(
        &mut self,
        tenant_id: &TenantId,
        consumer_group: &str,
        offsets: &[(u32, u64)],
    ) -> CoreResult<()> {
        if consumer_group.trim().is_empty() {
            return Err(CoreError::QueryPlanning(
                "durable stream consumer_group must be non-empty".to_string(),
            ));
        }
        self.ensure_durable_mutation_topic(tenant_id)?;
        for (partition, committed_sequence) in offsets {
            self.ordered_log
                .commit_consumer_group_offset(
                    tenant_id,
                    DURABLE_MUTATION_TOPIC,
                    consumer_group,
                    *partition,
                    *committed_sequence,
                )
                .map_err(Self::ordered_log_error_to_core)?;
        }
        Ok(())
    }

    pub fn commit_durable_mutation_offsets_with_context(
        &mut self,
        caller: &CallerContext,
        tenant_id: &TenantId,
        consumer_group: &str,
        offsets: &[(u32, u64)],
    ) -> CoreResult<()> {
        self.authorize_caller(caller, AuthAction::Watch, tenant_id)?;
        self.commit_durable_mutation_offsets(tenant_id, consumer_group, offsets)
    }

    pub fn run_query_with_trace(
        &mut self,
        query: &QueryRequest,
    ) -> CoreResult<(Vec<HydratedRecord>, QueryTrace)> {
        let caller = CallerContext::system_for_tenant(query.tenant_id.clone());
        self.run_query_with_trace_with_context(&caller, query)
    }

    pub fn run_query_with_trace_with_context(
        &mut self,
        caller: &CallerContext,
        query: &QueryRequest,
    ) -> CoreResult<(Vec<HydratedRecord>, QueryTrace)> {
        self.authorize_caller(caller, AuthAction::Query, &query.tenant_id)?;
        self.run_query_with_trace_internal(query)
    }

    fn run_query_with_trace_internal(
        &mut self,
        query: &QueryRequest,
    ) -> CoreResult<(Vec<HydratedRecord>, QueryTrace)> {
        let query_started_at = Utc::now();
        let query_started = Instant::now();

        let stage_start = Instant::now();
        let candidates = self.query_candidates(query)?;
        let candidate_stage = StageTrace {
            stage: StageType::CandidateGeneration,
            input_count: self.state.hot_record_count() as usize,
            output_count: candidates.len(),
            elapsed_micros: stage_start.elapsed().as_micros(),
        };

        let stage_start = Instant::now();
        let scored = self.score_and_rank(query, candidates)?;
        let score_stage = StageTrace {
            stage: StageType::ScoringAndRanking,
            input_count: candidate_stage.output_count,
            output_count: scored.len(),
            elapsed_micros: stage_start.elapsed().as_micros(),
        };

        let stage_start = Instant::now();
        let hydrated = self.hydrate(&query.tenant_id, scored)?;
        let hydration_stage = StageTrace {
            stage: StageType::Hydration,
            input_count: score_stage.output_count,
            output_count: hydrated.len(),
            elapsed_micros: stage_start.elapsed().as_micros(),
        };

        let elapsed = query_started.elapsed().as_micros();
        self.metrics.record_query(elapsed);
        self.metrics.update_record_counts(
            self.state.hot_record_count(),
            self.state.cold_record_count(),
        );

        Ok((
            hydrated,
            QueryTrace {
                started_at: query_started_at,
                finished_at: Utc::now(),
                stages: vec![candidate_stage, score_stage, hydration_stage],
                score_policy_version: query.score_policy.policy_version,
            },
        ))
    }

    pub fn set_auth_runtime(&mut self, auth_runtime: AuthRuntime) {
        self.auth_runtime = auth_runtime;
    }

    pub fn metrics(&self) -> &EngineMetrics {
        &self.metrics
    }

    fn ordered_log_error_to_core(error: OrderedLogError) -> CoreError {
        CoreError::Storage(format!("ordered log error: {error}"))
    }

    fn durable_mutation_topic_config() -> OrderedTopicConfig {
        OrderedTopicConfig {
            partition_count: DURABLE_MUTATION_TOPIC_PARTITIONS,
            retention_max_events_per_partition: None,
            retention_max_age_seconds: None,
            compaction_policy: CompactionPolicy::None,
        }
    }

    fn ensure_durable_mutation_topic(&mut self, tenant_id: &TenantId) -> CoreResult<()> {
        match self
            .ordered_log
            .topic_config(tenant_id, DURABLE_MUTATION_TOPIC)
        {
            Ok(_) => Ok(()),
            Err(OrderedLogError::TopicNotFound { .. }) => self
                .ordered_log
                .create_topic(
                    tenant_id.clone(),
                    DURABLE_MUTATION_TOPIC,
                    Self::durable_mutation_topic_config(),
                )
                .map_err(Self::ordered_log_error_to_core),
            Err(error) => Err(Self::ordered_log_error_to_core(error)),
        }
    }

    fn mirror_mutation_event(&mut self, event: &MutationEvent) -> CoreResult<()> {
        self.ensure_durable_mutation_topic(&event.tenant_id)?;
        let payload = serde_json::to_value(event).map_err(|e| {
            CoreError::Serialization(format!("failed serializing durable mutation event: {e}"))
        })?;

        self.ordered_log
            .append(
                &event.tenant_id,
                DURABLE_MUTATION_TOPIC,
                &event.record_id.0.to_string(),
                Some(event.record_id.0.to_string()),
                payload,
            )
            .map_err(Self::ordered_log_error_to_core)?;
        Ok(())
    }

    fn mutation_event_for_sequence(&self, sequence: u64) -> Option<MutationEvent> {
        self.state
            .mutation_events()
            .iter()
            .rev()
            .find(|event| event.commit_sequence == sequence)
            .cloned()
    }

    fn authorize_caller(
        &self,
        caller: &CallerContext,
        action: AuthAction,
        tenant_id: &TenantId,
    ) -> CoreResult<()> {
        self.auth_runtime.authorize(caller, action, tenant_id)
    }

    fn compute_vector_score(query_vector: &[f32], record_vector: &[f32]) -> f32 {
        if query_vector.is_empty() || record_vector.is_empty() {
            return 0.0;
        }

        let len = query_vector.len().min(record_vector.len());
        let mut dot = 0.0f32;
        let mut qnorm = 0.0f32;
        let mut rnorm = 0.0f32;

        for i in 0..len {
            let q = query_vector[i];
            let r = record_vector[i];
            dot += q * r;
            qnorm += q * q;
            rnorm += r * r;
        }

        if qnorm <= f32::EPSILON || rnorm <= f32::EPSILON {
            return 0.0;
        }

        dot / (qnorm.sqrt() * rnorm.sqrt())
    }

    fn record_matches_predicates_hot(record: &CompactRecord, predicates: &[Predicate]) -> bool {
        fields_match_predicates(&record.structured_fields, predicates)
    }

    fn key_in_ranges(space_key: u128, ranges: &[KeyRange]) -> bool {
        ranges
            .iter()
            .any(|range| space_key >= range.min && space_key <= range.max)
    }

    fn candidate_hint_matches(record: &CompactRecord, query: &QueryRequest) -> bool {
        let Some(hint) = &query.candidate_hint else {
            return true;
        };

        if hint.key_ranges.is_empty() {
            return true;
        }

        match record.space_key {
            Some(space_key) => Self::key_in_ranges(space_key, &hint.key_ranges),
            None => true,
        }
    }

    fn compare_order_field_values(
        left: Option<&FieldValue>,
        right: Option<&FieldValue>,
        direction: QueryOrderDirection,
    ) -> Ordering {
        match (left, right) {
            (Some(left), Some(right)) => {
                let ord = compare_field_values(left, right).unwrap_or(Ordering::Equal);
                match direction {
                    QueryOrderDirection::Asc => ord,
                    QueryOrderDirection::Desc => ord.reverse(),
                }
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }

    fn execute_compiled_text_plan(
        &mut self,
        caller: &CallerContext,
        plan: &idb_planner::LogicalPlan,
        request: &QueryRequest,
    ) -> CoreResult<Vec<HydratedRecord>> {
        match &plan.source {
            PlanSource::EntityScan { .. } => self.run_query_with_context(caller, request),
            PlanSource::Traversal { steps, directions } => {
                self.run_traversal_text_query(request, steps, directions)
            }
        }
    }

    fn resolve_watch_current(
        &self,
        request: &QueryRequest,
        event: &MutationEvent,
    ) -> CoreResult<Option<HydratedRecord>> {
        if event.tenant_id != request.tenant_id {
            return Ok(None);
        }

        let key = (request.tenant_id.clone(), event.record_id.clone());
        let Some(record) = self.state.records().get(&key) else {
            return Ok(None);
        };
        let Some(hot) = self.state.hot_records().get(&key) else {
            return Ok(None);
        };

        if !Self::record_matches_predicates_hot(hot, &request.predicates)
            || !Self::candidate_hint_matches(hot, request)
        {
            return Ok(None);
        }

        let structured_score = if request.predicates.is_empty() {
            0.0
        } else {
            1.0
        };
        let vector_score = match &request.vector_query {
            Some(vq) => hot
                .embedding_fields
                .get(&vq.field)
                .map(|embedding| Self::compute_vector_score(&vq.vector, embedding))
                .unwrap_or(0.0),
            None => 0.0,
        };

        if request.vector_query.is_some()
            && request
                .min_vector_score
                .is_some_and(|min| vector_score < min)
        {
            return Ok(None);
        }

        let score = if request.vector_query.is_some() {
            request.score_policy.structured_weight * structured_score
                + request.score_policy.vector_weight * vector_score
        } else {
            structured_score.max(1.0)
        };

        Ok(Some(HydratedRecord {
            envelope: record.clone(),
            score,
        }))
    }

    fn run_traversal_text_query(
        &self,
        request: &QueryRequest,
        steps: &[PlanTraversalStep],
        directions: &[PlanTraversalDirection],
    ) -> CoreResult<Vec<HydratedRecord>> {
        let traversed = self.resolve_traversal_candidates(&request.tenant_id, steps, directions)?;
        let filtered = traversed
            .into_iter()
            .filter(|record_id| {
                let key = (request.tenant_id.clone(), record_id.clone());
                self.state.hot_records().get(&key).is_some_and(|record| {
                    Self::record_matches_predicates_hot(record, &request.predicates)
                        && Self::candidate_hint_matches(record, request)
                })
            })
            .collect::<Vec<_>>();

        let scored = self.score_and_rank(request, filtered)?;
        self.hydrate(&request.tenant_id, scored)
    }

    fn resolve_traversal_candidates(
        &self,
        tenant_id: &TenantId,
        steps: &[PlanTraversalStep],
        directions: &[PlanTraversalDirection],
    ) -> CoreResult<Vec<RecordId>> {
        if steps.is_empty() {
            return Ok(Vec::new());
        }
        if steps.len() != directions.len() + 1 {
            return Err(CoreError::QueryPlanning(
                "invalid traversal shape: steps must be exactly one more than directions"
                    .to_string(),
            ));
        }

        let mut frontier = self.resolve_step_records(tenant_id, &steps[0])?;
        for (hop, direction) in directions.iter().enumerate() {
            let next_step = &steps[hop + 1];
            frontier = self.expand_frontier(tenant_id, &frontier, *direction, next_step)?;
        }

        Ok(frontier.into_iter().collect())
    }

    fn resolve_step_records(
        &self,
        tenant_id: &TenantId,
        step: &PlanTraversalStep,
    ) -> CoreResult<BTreeSet<RecordId>> {
        let mut out = BTreeSet::new();
        for ((record_tenant_id, record_id), record) in self.state.records() {
            if record_tenant_id != tenant_id {
                continue;
            }
            if self.record_matches_step(record, step)? {
                out.insert(record_id.clone());
            }
        }
        Ok(out)
    }

    fn expand_frontier(
        &self,
        tenant_id: &TenantId,
        frontier: &BTreeSet<RecordId>,
        direction: PlanTraversalDirection,
        next_step: &PlanTraversalStep,
    ) -> CoreResult<BTreeSet<RecordId>> {
        let mut out = BTreeSet::new();

        match direction {
            PlanTraversalDirection::Outbound => {
                let mut target_ids = BTreeSet::new();
                for source_id in frontier {
                    let key = (tenant_id.clone(), source_id.clone());
                    let Some(source) = self.state.records().get(&key) else {
                        continue;
                    };
                    for edge in &source.edge_refs {
                        target_ids.insert(edge.target_record_id.clone());
                    }
                }

                if target_ids.is_empty() {
                    return Ok(out);
                }

                let target_ids = target_ids.into_iter().collect::<Vec<_>>();
                let target_records = self.state.get_many(tenant_id, &target_ids);
                for (target_id, target_record) in target_ids.into_iter().zip(target_records) {
                    let Some(target) = target_record else {
                        continue;
                    };
                    if self.record_matches_step(&target, next_step)? {
                        out.insert(target_id);
                    }
                }
            }
            PlanTraversalDirection::Inbound => {
                let frontier_set = frontier.iter().cloned().collect::<HashSet<_>>();
                for ((record_tenant_id, record_id), record) in self.state.records() {
                    if record_tenant_id != tenant_id {
                        continue;
                    }
                    if !self.record_matches_step(record, next_step)? {
                        continue;
                    }
                    if record
                        .edge_refs
                        .iter()
                        .any(|edge| frontier_set.contains(&edge.target_record_id))
                    {
                        out.insert(record_id.clone());
                    }
                }
            }
        }

        Ok(out)
    }

    fn record_matches_step(
        &self,
        record: &RecordEnvelope,
        step: &PlanTraversalStep,
    ) -> CoreResult<bool> {
        match step {
            PlanTraversalStep::EntityScan(entity) => Ok(record.entity_type.0 == *entity),
            PlanTraversalStep::EntityRef { entity, id } => {
                if record.entity_type.0 != *entity {
                    return Ok(false);
                }
                self.record_matches_literal_ref(record, id)
            }
        }
    }

    fn record_matches_literal_ref(
        &self,
        record: &RecordEnvelope,
        literal: &LiteralExpr,
    ) -> CoreResult<bool> {
        match literal {
            LiteralExpr::Number(v) => {
                if *v < 0.0 || v.fract() != 0.0 || *v > u64::MAX as f64 {
                    return Err(CoreError::QueryPlanning(format!(
                        "entity ref numeric literal must be non-negative integer, got {v}"
                    )));
                }
                Ok(record.record_id.0 == *v as u64)
            }
            LiteralExpr::String(value) | LiteralExpr::Ident(value) => Ok(record
                .structured_fields
                .get("name")
                .is_some_and(|field| field == &FieldValue::String(value.clone()))),
            LiteralExpr::Bool(value) => Ok(record
                .structured_fields
                .get("name")
                .is_some_and(|field| field == &FieldValue::Bool(*value))),
        }
    }

    pub fn ingest_batch_with_context(
        &mut self,
        caller: &CallerContext,
        records: Vec<RecordEnvelope>,
    ) -> CoreResult<Vec<RecordId>> {
        self.ingest_batch_inner(caller, records)
    }

    pub fn delete_or_tombstone_with_context(
        &mut self,
        caller: &CallerContext,
        tenant_id: &TenantId,
        record_id: RecordId,
    ) -> CoreResult<bool> {
        self.authorize_caller(caller, AuthAction::Delete, tenant_id)?;
        let (sequence, existed) = self.state.delete(tenant_id.clone(), record_id)?;
        let event = self.mutation_event_for_sequence(sequence).ok_or_else(|| {
            CoreError::Storage(format!(
                "missing mutation event for durable mirror sequence {sequence}"
            ))
        })?;
        self.mirror_mutation_event(&event)?;
        Ok(existed)
    }

    fn ingest_batch_inner(
        &mut self,
        caller: &CallerContext,
        records: Vec<RecordEnvelope>,
    ) -> CoreResult<Vec<RecordId>> {
        let started = Instant::now();
        let before_wal = self.state.wal_size_bytes().unwrap_or(0);
        let mut ids = Vec::with_capacity(records.len());
        for record in records {
            self.authorize_caller(caller, AuthAction::Ingest, &record.tenant_id)?;
            ids.push(record.record_id.clone());
            let sequence = self.state.upsert(record)?;
            let event = self.mutation_event_for_sequence(sequence).ok_or_else(|| {
                CoreError::Storage(format!(
                    "missing mutation event for durable mirror sequence {sequence}"
                ))
            })?;
            self.mirror_mutation_event(&event)?;
        }
        let after_wal = self.state.wal_size_bytes().unwrap_or(before_wal);
        let wal_written = after_wal.saturating_sub(before_wal);
        self.metrics
            .record_ingest(started.elapsed().as_micros(), wal_written, ids.len() as u64);
        self.metrics.update_record_counts(
            self.state.hot_record_count(),
            self.state.cold_record_count(),
        );
        Ok(ids)
    }
}

impl StorageBackend for CpuBackend {
    fn ingest_batch(&mut self, records: Vec<RecordEnvelope>) -> CoreResult<Vec<RecordId>> {
        let caller = CallerContext::internal_unscoped();
        self.ingest_batch_inner(&caller, records)
    }

    fn delete_or_tombstone(
        &mut self,
        tenant_id: &TenantId,
        record_id: RecordId,
    ) -> CoreResult<bool> {
        let caller = CallerContext::internal_unscoped();
        self.delete_or_tombstone_with_context(&caller, tenant_id, record_id)
    }

    fn query_candidates(&self, query: &QueryRequest) -> CoreResult<Vec<RecordId>> {
        if let Some(hint) = &query.candidate_hint {
            if !hint.key_ranges.is_empty() {
                let mut out = Vec::new();
                let mut seen = HashSet::new();

                let range_candidates = self
                    .state
                    .candidate_ids_for_key_ranges(&query.tenant_id, &hint.key_ranges);
                for record_id in range_candidates {
                    let key = (query.tenant_id.clone(), record_id.clone());
                    let Some(record) = self.state.hot_records().get(&key) else {
                        continue;
                    };
                    if Self::record_matches_predicates_hot(record, &query.predicates)
                        && Self::candidate_hint_matches(record, query)
                    {
                        seen.insert(record_id.clone());
                        out.push(record_id);
                    }
                }

                for ((tenant_id, record_id), record) in self.state.hot_records() {
                    if tenant_id != &query.tenant_id {
                        continue;
                    }
                    if record.space_key.is_some() || seen.contains(record_id) {
                        continue;
                    }
                    if Self::record_matches_predicates_hot(record, &query.predicates)
                        && Self::candidate_hint_matches(record, query)
                    {
                        out.push(record_id.clone());
                    }
                }

                return Ok(out);
            }
        }

        let mut out = Vec::new();
        for ((tenant_id, record_id), record) in self.state.hot_records() {
            if tenant_id == &query.tenant_id
                && Self::record_matches_predicates_hot(record, &query.predicates)
                && Self::candidate_hint_matches(record, query)
            {
                out.push(record_id.clone());
            }
        }

        Ok(out)
    }

    fn score_and_rank(
        &self,
        query: &QueryRequest,
        candidates: Vec<RecordId>,
    ) -> CoreResult<Vec<ScoredRecord>> {
        let mut scored = Vec::with_capacity(candidates.len());

        for candidate in candidates {
            let key = (query.tenant_id.clone(), candidate.clone());
            let record = self.state.hot_records().get(&key).ok_or_else(|| {
                CoreError::Storage(format!("record {:?} missing from state", candidate.0))
            })?;

            let structured_score = if query.predicates.is_empty() {
                0.0
            } else {
                1.0
            };

            let vector_score = match &query.vector_query {
                Some(vq) => record
                    .embedding_fields
                    .get(&vq.field)
                    .map(|embedding| Self::compute_vector_score(&vq.vector, embedding))
                    .unwrap_or(0.0),
                None => 0.0,
            };

            if query.vector_query.is_some() {
                if let Some(min_vector_score) = query.min_vector_score {
                    if vector_score < min_vector_score {
                        continue;
                    }
                }
            }

            let final_score = if query.vector_query.is_some() {
                query.score_policy.structured_weight * structured_score
                    + query.score_policy.vector_weight * vector_score
            } else {
                structured_score.max(1.0)
            };

            scored.push(ScoredRecord {
                record_id: candidate,
                score: final_score,
            });
        }

        scored.sort_by(|a, b| {
            if let Some(order_by) = &query.order_by {
                let a_key = (query.tenant_id.clone(), a.record_id.clone());
                let b_key = (query.tenant_id.clone(), b.record_id.clone());
                let a_field = self
                    .state
                    .hot_records()
                    .get(&a_key)
                    .and_then(|r| r.structured_fields.get(&order_by.field));
                let b_field = self
                    .state
                    .hot_records()
                    .get(&b_key)
                    .and_then(|r| r.structured_fields.get(&order_by.field));

                let field_ord =
                    Self::compare_order_field_values(a_field, b_field, order_by.direction);
                if field_ord != Ordering::Equal {
                    return field_ord;
                }
            }

            b.score
                .partial_cmp(&a.score)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.record_id.0.cmp(&b.record_id.0))
        });

        scored.truncate(query.top_k);
        Ok(scored)
    }

    fn hydrate(
        &self,
        tenant_id: &TenantId,
        scored: Vec<ScoredRecord>,
    ) -> CoreResult<Vec<HydratedRecord>> {
        let record_ids = scored
            .iter()
            .map(|scored_record| scored_record.record_id.clone())
            .collect::<Vec<_>>();
        let records = self.state.get_many(tenant_id, &record_ids);
        let mut hydrated = Vec::with_capacity(records.len());

        for (scored_record, record) in scored.into_iter().zip(records.into_iter()) {
            let record = record.ok_or_else(|| {
                CoreError::Storage(format!(
                    "record {:?} missing for hydrate",
                    scored_record.record_id.0
                ))
            })?;

            hydrated.push(HydratedRecord {
                envelope: record,
                score: scored_record.score,
            });
        }

        Ok(hydrated)
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities::cpu_reference()
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;
    use std::collections::BTreeMap;
    use std::env;
    use std::fs;

    use chrono::Utc;
    use idb_core::{
        fields_match_predicates, AuthRuntime, AuthorizationDecision, AuthorizationProvider,
        AuthorizationRequest, CallerContext, CandidateGenerationHint, CoreError,
        DimensionDefinition, DimensionRegistry, DimensionSource, DimensionType, EdgeRef,
        FallbackBackend, FieldValue, HybridScorePolicy, InterleavedKeyMapper, KeyRange,
        MissingValuePolicy, NormalizationPolicy, Predicate, PredicateOp, QueryRequest,
        RecordEnvelope, RecordId, ScoredRecord, SpaceKeyMapper, StorageBackend, TenantId,
        VectorQuery,
    };
    use idb_executor_cerebras_stub::CerebrasStubBackend;
    use idb_planner::RequestProjectionStatus;

    use crate::CpuBackend;

    struct DenyQueryAuthorizationProvider;

    impl AuthorizationProvider for DenyQueryAuthorizationProvider {
        fn decide(&self, request: &AuthorizationRequest) -> AuthorizationDecision {
            match request.action {
                idb_core::AuthAction::Query => AuthorizationDecision::Deny {
                    reason: "query denied by policy".to_string(),
                },
                _ => AuthorizationDecision::Allow,
            }
        }
    }

    fn reference_vector_score(query_vector: &[f32], record_vector: &[f32]) -> f32 {
        if query_vector.is_empty() || record_vector.is_empty() {
            return 0.0;
        }
        let len = query_vector.len().min(record_vector.len());
        let mut dot = 0.0f32;
        let mut qnorm = 0.0f32;
        let mut rnorm = 0.0f32;
        for i in 0..len {
            dot += query_vector[i] * record_vector[i];
            qnorm += query_vector[i] * query_vector[i];
            rnorm += record_vector[i] * record_vector[i];
        }
        if qnorm <= f32::EPSILON || rnorm <= f32::EPSILON {
            return 0.0;
        }
        dot / (qnorm.sqrt() * rnorm.sqrt())
    }

    fn run_reference_query(records: &[RecordEnvelope], query: &QueryRequest) -> Vec<(u64, f32)> {
        let mut scored: Vec<(u64, f32)> = records
            .iter()
            .filter(|record| {
                record.tenant_id == query.tenant_id
                    && fields_match_predicates(&record.structured_fields, &query.predicates)
            })
            .map(|record| {
                let structured_score = if query.predicates.is_empty() {
                    0.0
                } else {
                    1.0
                };
                let vector_score = match &query.vector_query {
                    Some(vq) => record
                        .embedding_fields
                        .get(&vq.field)
                        .map(|emb| reference_vector_score(&vq.vector, emb))
                        .unwrap_or(0.0),
                    None => 0.0,
                };
                let score = if query.vector_query.is_some() {
                    query.score_policy.structured_weight * structured_score
                        + query.score_policy.vector_weight * vector_score
                } else {
                    structured_score.max(1.0)
                };
                (record.record_id.0, score)
            })
            .collect();

        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });
        scored.truncate(query.top_k);
        scored
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
                    max: 2000.0,
                    bins: 32,
                },
                missing_value: MissingValuePolicy::Error,
            }],
        }
    }

    #[test]
    fn ranking_is_deterministic_with_tie_break() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut a = RecordEnvelope::new(2, "tenant_a", "Product");
        let mut b = RecordEnvelope::new(1, "tenant_a", "Product");

        a.structured_fields
            .insert("price".to_string(), FieldValue::Float(1200.0));
        b.structured_fields
            .insert("price".to_string(), FieldValue::Float(1200.0));

        a.embedding_fields = BTreeMap::from([("text_embedding".to_string(), vec![1.0, 0.0])]);
        b.embedding_fields = BTreeMap::from([("text_embedding".to_string(), vec![1.0, 0.0])]);

        backend.ingest_batch(vec![a, b]).expect("ingest");

        let query = QueryRequest {
            tenant_id: TenantId("tenant_a".to_string()),
            predicates: vec![Predicate {
                field: "price".to_string(),
                op: PredicateOp::Lte,
                value: FieldValue::Float(2000.0),
            }],
            vector_query: Some(VectorQuery {
                field: "text_embedding".to_string(),
                vector: vec![1.0, 0.0],
            }),
            min_vector_score: None,
            order_by: None,
            candidate_hint: None,
            top_k: 10,
            score_policy: HybridScorePolicy::default(),
        };

        let result = backend.run_query(&query).expect("run query");
        assert_eq!(result.len(), 2);

        // Scores tie, so record_id=1 must appear before record_id=2.
        assert_eq!(result[0].envelope.record_id.0, 1);
        assert_eq!(result[1].envelope.record_id.0, 2);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn auth_runtime_can_deny_query_context() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_auth_deny_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut record = RecordEnvelope::new(1, "tenant_a", "Product");
        record
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        backend.ingest_batch(vec![record]).expect("ingest");

        backend.set_auth_runtime(AuthRuntime::with_provider(DenyQueryAuthorizationProvider));

        let query = QueryRequest {
            tenant_id: TenantId("tenant_a".to_string()),
            predicates: vec![],
            vector_query: None,
            min_vector_score: None,
            order_by: None,
            candidate_hint: None,
            top_k: 10,
            score_policy: HybridScorePolicy::default(),
        };

        let caller = CallerContext::service("api", Some(TenantId("tenant_a".to_string())));
        let err = backend
            .run_query_with_context(&caller, &query)
            .expect_err("query should be denied");
        assert!(matches!(err, CoreError::AuthorizationDenied(_)));

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn caller_tenant_scope_blocks_cross_tenant_query() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_auth_scope_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut record = RecordEnvelope::new(1, "tenant_a", "Product");
        record
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        backend.ingest_batch(vec![record]).expect("ingest");

        let query = QueryRequest {
            tenant_id: TenantId("tenant_a".to_string()),
            predicates: vec![],
            vector_query: None,
            min_vector_score: None,
            order_by: None,
            candidate_hint: None,
            top_k: 10,
            score_policy: HybridScorePolicy::default(),
        };

        let scoped_to_other = CallerContext::service("api", Some(TenantId("tenant_b".to_string())));
        let err = backend
            .run_query_with_context(&scoped_to_other, &query)
            .expect_err("cross tenant should be denied");
        assert!(matches!(err, CoreError::AuthorizationDenied(_)));

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn query_trace_and_metrics_are_recorded() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_trace_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut record = RecordEnvelope::new(7, "tenant_a", "Product");
        record
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(999.0));
        record.embedding_fields = BTreeMap::from([("text_embedding".to_string(), vec![0.9, 0.1])]);
        backend.ingest_batch(vec![record]).expect("ingest");

        let query = QueryRequest {
            tenant_id: TenantId("tenant_a".to_string()),
            predicates: vec![Predicate {
                field: "price".to_string(),
                op: PredicateOp::Lt,
                value: FieldValue::Float(2000.0),
            }],
            vector_query: Some(VectorQuery {
                field: "text_embedding".to_string(),
                vector: vec![0.9, 0.1],
            }),
            min_vector_score: None,
            order_by: None,
            candidate_hint: None,
            top_k: 5,
            score_policy: HybridScorePolicy::default(),
        };

        let (results, trace) = backend
            .run_query_with_trace(&query)
            .expect("run traced query");
        assert_eq!(results.len(), 1);
        assert_eq!(trace.stages.len(), 3);
        assert_eq!(
            trace.stages[0].stage,
            idb_core::StageType::CandidateGeneration
        );
        assert_eq!(trace.stages[2].stage, idb_core::StageType::Hydration);

        let metrics = backend.metrics();
        assert_eq!(metrics.ingest_latency.count, 1);
        assert_eq!(metrics.query_latency.count, 1);
        assert_eq!(metrics.hot_record_count, 1);
        assert_eq!(metrics.cold_record_count, 1);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn cpu_backend_matches_reference_execution() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_diff_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut records = Vec::new();
        for (id, price, emb) in [
            (1_u64, 1200.0_f64, vec![0.9_f32, 0.1_f32]),
            (2_u64, 950.0_f64, vec![0.5_f32, 0.5_f32]),
            (3_u64, 2600.0_f64, vec![0.2_f32, 0.8_f32]),
            (4_u64, 1100.0_f64, vec![0.9_f32, 0.1_f32]),
        ] {
            let mut record = RecordEnvelope::new(id, "tenant_a", "Product");
            record
                .structured_fields
                .insert("price".to_string(), FieldValue::Float(price));
            record.structured_fields.insert(
                "status".to_string(),
                FieldValue::String("active".to_string()),
            );
            record
                .embedding_fields
                .insert("text_embedding".to_string(), emb);
            records.push(record);
        }

        backend
            .ingest_batch(records.clone())
            .expect("ingest dataset");

        let query = QueryRequest {
            tenant_id: TenantId("tenant_a".to_string()),
            predicates: vec![
                Predicate {
                    field: "price".to_string(),
                    op: PredicateOp::Lt,
                    value: FieldValue::Float(2000.0),
                },
                Predicate {
                    field: "status".to_string(),
                    op: PredicateOp::Eq,
                    value: FieldValue::String("active".to_string()),
                },
            ],
            vector_query: Some(VectorQuery {
                field: "text_embedding".to_string(),
                vector: vec![0.9, 0.1],
            }),
            min_vector_score: None,
            order_by: None,
            candidate_hint: None,
            top_k: 3,
            score_policy: HybridScorePolicy::default(),
        };

        let cpu = backend.run_query(&query).expect("cpu query");
        let reference = run_reference_query(&records, &query);

        let cpu_pairs: Vec<(u64, f32)> = cpu
            .into_iter()
            .map(|row| (row.envelope.record_id.0, row.score))
            .collect();

        assert_eq!(cpu_pairs.len(), reference.len());
        for (cpu_row, ref_row) in cpu_pairs.iter().zip(reference.iter()) {
            assert_eq!(cpu_row.0, ref_row.0);
            assert!((cpu_row.1 - ref_row.1).abs() < 1e-6);
        }

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn fallback_wrapper_uses_cpu_when_cerebras_is_unavailable() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_fallback_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let cpu = CpuBackend::new(&base).expect("create cpu backend");
        let stub = CerebrasStubBackend::new();
        let mut backend = FallbackBackend::new(stub, cpu);

        let mut record = RecordEnvelope::new(9, "tenant_a", "Product");
        record
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(850.0));
        record
            .embedding_fields
            .insert("text_embedding".to_string(), vec![1.0, 0.0]);

        backend
            .ingest_batch(vec![record])
            .expect("ingest via fallback");

        let query = QueryRequest {
            tenant_id: TenantId("tenant_a".to_string()),
            predicates: vec![Predicate {
                field: "price".to_string(),
                op: PredicateOp::Lt,
                value: FieldValue::Float(1000.0),
            }],
            vector_query: Some(VectorQuery {
                field: "text_embedding".to_string(),
                vector: vec![1.0, 0.0],
            }),
            min_vector_score: None,
            order_by: None,
            candidate_hint: None,
            top_k: 5,
            score_policy: HybridScorePolicy::default(),
        };

        let candidates = backend.query_candidates(&query).expect("query candidates");
        let scored = backend
            .score_and_rank(&query, candidates)
            .expect("score and rank");
        let hydrated = backend
            .hydrate(&query.tenant_id, scored)
            .expect("hydrate results");
        assert_eq!(hydrated.len(), 1);
        assert_eq!(hydrated[0].envelope.record_id.0, 9);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn candidate_key_ranges_prune_results_when_spatial_indexing_is_enabled() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_keyrange_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let registry = price_registry();
        let mapper = InterleavedKeyMapper::new(5).expect("mapper");
        let mut backend =
            CpuBackend::new_with_spatial_registry(&base, registry.clone(), 5).expect("backend");

        let mut cheap = RecordEnvelope::new(21, "tenant_a", "Product");
        cheap
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        cheap
            .embedding_fields
            .insert("text_embedding".to_string(), vec![1.0, 0.0]);

        let mut expensive = RecordEnvelope::new(22, "tenant_a", "Product");
        expensive
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(1900.0));
        expensive
            .embedding_fields
            .insert("text_embedding".to_string(), vec![1.0, 0.0]);

        backend
            .ingest_batch(vec![cheap.clone(), expensive.clone()])
            .expect("ingest");

        let cheap_key = mapper
            .encode(&registry.map_record(&cheap).expect("map cheap"))
            .expect("encode cheap");

        let query = QueryRequest {
            tenant_id: TenantId("tenant_a".to_string()),
            predicates: vec![Predicate {
                field: "price".to_string(),
                op: PredicateOp::Lt,
                value: FieldValue::Float(5000.0),
            }],
            vector_query: Some(VectorQuery {
                field: "text_embedding".to_string(),
                vector: vec![1.0, 0.0],
            }),
            min_vector_score: None,
            order_by: None,
            candidate_hint: Some(CandidateGenerationHint {
                key_ranges: vec![KeyRange {
                    min: cheap_key,
                    max: cheap_key,
                }],
                ann_probe: None,
            }),
            top_k: 5,
            score_policy: HybridScorePolicy::default(),
        };

        let result = backend.run_query(&query).expect("run query");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].envelope.record_id.0, 21);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn hydrate_preserves_scored_input_order_with_batch_fetch() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_hydrate_batch_order_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        backend
            .ingest_batch(vec![
                RecordEnvelope::new(7001, "tenant_a", "Product"),
                RecordEnvelope::new(7002, "tenant_a", "Product"),
            ])
            .expect("ingest");

        let scored = vec![
            ScoredRecord {
                record_id: RecordId(7002),
                score: 0.9,
            },
            ScoredRecord {
                record_id: RecordId(7001),
                score: 0.8,
            },
        ];

        let hydrated = backend
            .hydrate(&TenantId("tenant_a".to_string()), scored)
            .expect("hydrate");
        assert_eq!(hydrated.len(), 2);
        assert_eq!(hydrated[0].envelope.record_id, RecordId(7002));
        assert_eq!(hydrated[1].envelope.record_id, RecordId(7001));
        assert!(hydrated[0].score > hydrated[1].score);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn candidate_key_ranges_track_updates_and_deletes() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_keyrange_update_delete_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let registry = price_registry();
        let mapper = InterleavedKeyMapper::new(5).expect("mapper");
        let mut backend =
            CpuBackend::new_with_spatial_registry(&base, registry.clone(), 5).expect("backend");

        let mut target = RecordEnvelope::new(31, "tenant_a", "Product");
        target
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(400.0));
        target
            .embedding_fields
            .insert("text_embedding".to_string(), vec![1.0, 0.0]);

        let mut sibling = RecordEnvelope::new(32, "tenant_a", "Product");
        sibling
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(900.0));
        sibling
            .embedding_fields
            .insert("text_embedding".to_string(), vec![1.0, 0.0]);

        backend
            .ingest_batch(vec![target.clone(), sibling])
            .expect("ingest");

        let old_key = mapper
            .encode(&registry.map_record(&target).expect("map target old"))
            .expect("encode target old");

        let query_for_key = |key: u128| QueryRequest {
            tenant_id: TenantId("tenant_a".to_string()),
            predicates: vec![Predicate {
                field: "price".to_string(),
                op: PredicateOp::Lt,
                value: FieldValue::Float(5000.0),
            }],
            vector_query: Some(VectorQuery {
                field: "text_embedding".to_string(),
                vector: vec![1.0, 0.0],
            }),
            min_vector_score: None,
            order_by: None,
            candidate_hint: Some(CandidateGenerationHint {
                key_ranges: vec![KeyRange { min: key, max: key }],
                ann_probe: None,
            }),
            top_k: 5,
            score_policy: HybridScorePolicy::default(),
        };

        let mut result = backend
            .run_query(&query_for_key(old_key))
            .expect("query old");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].envelope.record_id.0, 31);

        target
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(980.0));
        let new_key = mapper
            .encode(&registry.map_record(&target).expect("map target new"))
            .expect("encode target new");
        assert_ne!(old_key, new_key);
        backend.ingest_batch(vec![target]).expect("update target");

        result = backend
            .run_query(&query_for_key(old_key))
            .expect("query old after update");
        assert!(result.is_empty());

        result = backend
            .run_query(&query_for_key(new_key))
            .expect("query new after update");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].envelope.record_id.0, 31);

        backend
            .delete_or_tombstone(&TenantId("tenant_a".to_string()), RecordId(31))
            .expect("delete target");
        result = backend
            .run_query(&query_for_key(new_key))
            .expect("query new after delete");
        assert!(result.is_empty());

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn text_query_bridge_executes_supported_subset() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_text_query_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut a = RecordEnvelope::new(1, "tenant_a", "Product");
        a.structured_fields
            .insert("price".to_string(), FieldValue::Float(1200.0));

        let mut b = RecordEnvelope::new(2, "tenant_a", "Product");
        b.structured_fields
            .insert("price".to_string(), FieldValue::Float(900.0));

        let mut c = RecordEnvelope::new(3, "tenant_a", "Product");
        c.structured_fields
            .insert("price".to_string(), FieldValue::Float(2500.0));

        backend.ingest_batch(vec![a, b, c]).expect("ingest");

        let result = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Product where price < 2000 | top(2)",
            )
            .expect("run text query");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].envelope.record_id.0, 1);
        assert_eq!(result[1].envelope.record_id.0, 2);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn text_query_bridge_honors_topk_order_desc() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_text_query_order_desc_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut a = RecordEnvelope::new(401, "tenant_a", "Product");
        a.structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        let mut b = RecordEnvelope::new(402, "tenant_a", "Product");
        b.structured_fields
            .insert("price".to_string(), FieldValue::Float(300.0));
        let mut c = RecordEnvelope::new(403, "tenant_a", "Product");
        c.structured_fields
            .insert("price".to_string(), FieldValue::Float(200.0));
        backend.ingest_batch(vec![a, b, c]).expect("ingest");

        let result = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Product | top(2, price desc)",
            )
            .expect("run ordered desc");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].envelope.record_id.0, 402);
        assert_eq!(result[1].envelope.record_id.0, 403);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn text_query_bridge_honors_topk_order_asc() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_text_query_order_asc_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut a = RecordEnvelope::new(411, "tenant_a", "Product");
        a.structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        let mut b = RecordEnvelope::new(412, "tenant_a", "Product");
        b.structured_fields
            .insert("price".to_string(), FieldValue::Float(300.0));
        let mut c = RecordEnvelope::new(413, "tenant_a", "Product");
        c.structured_fields
            .insert("price".to_string(), FieldValue::Float(200.0));
        backend.ingest_batch(vec![a, b, c]).expect("ingest");

        let result = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Product | top(2, price asc)",
            )
            .expect("run ordered asc");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].envelope.record_id.0, 411);
        assert_eq!(result[1].envelope.record_id.0, 413);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn text_query_bridge_executes_outbound_traversal() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_text_query_traversal_out_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");

        let mut brand = RecordEnvelope::new(900, "tenant_a", "Brand");
        brand.structured_fields.insert(
            "name".to_string(),
            FieldValue::String("Norn Gold".to_string()),
        );
        brand.edge_refs = vec![
            EdgeRef {
                edge_type: "brand_product".to_string(),
                target_record_id: RecordId(901),
            },
            EdgeRef {
                edge_type: "brand_product".to_string(),
                target_record_id: RecordId(902),
            },
            EdgeRef {
                edge_type: "brand_product".to_string(),
                target_record_id: RecordId(904),
            },
        ];

        let mut p1 = RecordEnvelope::new(901, "tenant_a", "Product");
        p1.structured_fields
            .insert("price".to_string(), FieldValue::Float(1200.0));
        let mut p2 = RecordEnvelope::new(902, "tenant_a", "Product");
        p2.structured_fields
            .insert("price".to_string(), FieldValue::Float(2600.0));
        let mut p3 = RecordEnvelope::new(904, "tenant_a", "Product");
        p3.structured_fields
            .insert("price".to_string(), FieldValue::Float(1500.0));

        backend
            .ingest_batch(vec![brand, p1, p2, p3])
            .expect("ingest");

        let result = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Brand(\"Norn Gold\") -> Product where price < 2000 | top(2, price desc)",
            )
            .expect("run outbound traversal text query");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].envelope.record_id.0, 904);
        assert_eq!(result[1].envelope.record_id.0, 901);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn text_query_bridge_executes_inbound_traversal() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_text_query_traversal_in_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");

        let target = RecordEnvelope::new(1001, "tenant_a", "Product");
        let other_target = RecordEnvelope::new(1002, "tenant_a", "Product");

        let mut s1 = RecordEnvelope::new(1101, "tenant_a", "Supplier");
        s1.structured_fields
            .insert("rating".to_string(), FieldValue::Float(4.2));
        s1.edge_refs = vec![EdgeRef {
            edge_type: "supplies".to_string(),
            target_record_id: RecordId(1001),
        }];

        let mut s2 = RecordEnvelope::new(1102, "tenant_a", "Supplier");
        s2.structured_fields
            .insert("rating".to_string(), FieldValue::Float(4.8));
        s2.edge_refs = vec![EdgeRef {
            edge_type: "supplies".to_string(),
            target_record_id: RecordId(1001),
        }];

        let mut s3 = RecordEnvelope::new(1103, "tenant_a", "Supplier");
        s3.structured_fields
            .insert("rating".to_string(), FieldValue::Float(4.9));
        s3.edge_refs = vec![EdgeRef {
            edge_type: "supplies".to_string(),
            target_record_id: RecordId(1002),
        }];

        backend
            .ingest_batch(vec![target, other_target, s1, s2, s3])
            .expect("ingest");

        let result = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Product(1001) <- Supplier | top(2, rating desc)",
            )
            .expect("run inbound traversal text query");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].envelope.record_id.0, 1102);
        assert_eq!(result[1].envelope.record_id.0, 1101);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn watch_query_text_rejects_non_watch_mode() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_watch_query_mode_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let err = backend
            .start_watch_query_text(
                TenantId("tenant_a".to_string()),
                "Product where price < 3000 | top(5)",
            )
            .expect_err("non-watch query should fail");
        assert!(matches!(err, CoreError::QueryPlanning(_)));
        assert!(err
            .to_string()
            .contains("requires query text prefixed with `watch`"));

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn watch_query_text_returns_snapshot_and_dependency_filtered_events() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_watch_query_flow_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");

        let mut p1 = RecordEnvelope::new(2001, "tenant_a", "Product");
        p1.structured_fields
            .insert("price".to_string(), FieldValue::Float(1000.0));
        let mut p2 = RecordEnvelope::new(2002, "tenant_a", "Product");
        p2.structured_fields
            .insert("price".to_string(), FieldValue::Float(3000.0));
        backend.ingest_batch(vec![p1, p2]).expect("ingest seed");

        let session = backend
            .start_watch_query_text(
                TenantId("tenant_a".to_string()),
                "watch Product where price < 2000 | top(10)",
            )
            .expect("start watch");

        assert_eq!(session.snapshot.len(), 1);
        assert_eq!(session.snapshot[0].envelope.record_id.0, 2001);
        assert!(session.resume_token.0 >= 2);

        let mut p1_update = RecordEnvelope::new(2001, "tenant_a", "Product");
        p1_update
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(900.0));
        let mut p2_update = RecordEnvelope::new(2002, "tenant_a", "Product");
        p2_update
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(1500.0));
        backend
            .ingest_batch(vec![p1_update, p2_update])
            .expect("ingest updates");

        let batch = backend
            .poll_watch(session.subscription_id, 10)
            .expect("poll watch");

        assert_eq!(batch.events.len(), 1);
        assert_eq!(batch.events[0].record_id.0, 2001);
        assert!(batch.next_resume_token.0 >= session.resume_token.0);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn watch_query_update_batch_reports_current_state_transitions() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_watch_query_updates_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");

        let mut p1 = RecordEnvelope::new(3001, "tenant_a", "Product");
        p1.structured_fields
            .insert("price".to_string(), FieldValue::Float(1000.0));
        backend.ingest_batch(vec![p1]).expect("ingest seed");

        let session = backend
            .start_watch_query_text(
                TenantId("tenant_a".to_string()),
                "watch Product where price < 2000 | top(10)",
            )
            .expect("start watch");

        let mut drop_update = RecordEnvelope::new(3001, "tenant_a", "Product");
        drop_update
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(2500.0));
        backend
            .ingest_batch(vec![drop_update])
            .expect("ingest drop");

        let dropped_batch = backend
            .poll_watch_query_updates(session.subscription_id, 10)
            .expect("poll dropped update");
        assert_eq!(dropped_batch.updates.len(), 1);
        assert_eq!(dropped_batch.updates[0].event.record_id.0, 3001);
        assert!(dropped_batch.updates[0].current.is_none());

        let mut match_update = RecordEnvelope::new(3001, "tenant_a", "Product");
        match_update
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(900.0));
        backend
            .ingest_batch(vec![match_update])
            .expect("ingest match");

        let matched_batch = backend
            .poll_watch_query_updates(session.subscription_id, 10)
            .expect("poll matched update");
        assert_eq!(matched_batch.updates.len(), 1);
        assert_eq!(matched_batch.updates[0].event.record_id.0, 3001);
        let current = matched_batch.updates[0]
            .current
            .as_ref()
            .expect("current row");
        assert_eq!(current.envelope.record_id.0, 3001);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn stop_watch_removes_session_and_prevents_polling() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_watch_stop_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        backend
            .ingest_batch(vec![RecordEnvelope::new(4001, "tenant_a", "Product")])
            .expect("ingest");

        let session = backend
            .start_watch_query_text(TenantId("tenant_a".to_string()), "watch Product | top(10)")
            .expect("start watch");

        assert!(backend.stop_watch(session.subscription_id));
        assert!(!backend.stop_watch(session.subscription_id));

        let poll_err = backend
            .poll_watch(session.subscription_id, 10)
            .expect_err("stopped watch should not poll");
        assert!(poll_err.to_string().contains("unknown subscription id"));

        let updates_err = backend
            .poll_watch_query_updates(session.subscription_id, 10)
            .expect_err("stopped watch updates should fail");
        assert!(updates_err
            .to_string()
            .contains("unknown active watch subscription id"));

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn watch_poll_query_updates_rejects_zero_max_events() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_watch_zero_max_events_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        backend
            .ingest_batch(vec![RecordEnvelope::new(4501, "tenant_a", "Product")])
            .expect("ingest");
        let session = backend
            .start_watch_query_text(TenantId("tenant_a".to_string()), "watch Product | top(10)")
            .expect("start watch");

        let err = backend
            .poll_watch_query_updates(session.subscription_id, 0)
            .expect_err("zero max_events should fail");
        assert!(err.to_string().contains("max_events must be > 0"));

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn durable_mutation_stream_replays_and_respects_committed_offsets() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_durable_stream_offsets_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let tenant_id = TenantId("tenant_a".to_string());
        let mut backend = CpuBackend::new(&base).expect("create backend");

        let mut first = RecordEnvelope::new(5001, "tenant_a", "Product");
        first
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        backend.ingest_batch(vec![first]).expect("insert");

        let mut update = RecordEnvelope::new(5001, "tenant_a", "Product");
        update
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(120.0));
        backend.ingest_batch(vec![update]).expect("update");

        backend
            .delete_or_tombstone(&tenant_id, RecordId(5001))
            .expect("delete");

        let stream = backend
            .poll_durable_mutation_stream(&tenant_id, "group-a", 50)
            .expect("poll durable stream");
        assert_eq!(stream.len(), 3);
        assert!(matches!(
            stream[0].event.mutation_type,
            idb_storage::MutationType::Insert
        ));
        assert!(matches!(
            stream[1].event.mutation_type,
            idb_storage::MutationType::Update
        ));
        assert!(matches!(
            stream[2].event.mutation_type,
            idb_storage::MutationType::Delete
        ));

        let mut max_offsets = BTreeMap::<u32, u64>::new();
        for item in &stream {
            max_offsets
                .entry(item.partition)
                .and_modify(|existing| *existing = (*existing).max(item.sequence))
                .or_insert(item.sequence);
        }
        let offset_list = max_offsets.into_iter().collect::<Vec<_>>();

        backend
            .commit_durable_mutation_offsets(&tenant_id, "group-a", &offset_list)
            .expect("commit durable offsets");

        let next = backend
            .poll_durable_mutation_stream(&tenant_id, "group-a", 50)
            .expect("poll after commit");
        assert!(next.is_empty());

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn durable_mutation_stream_state_persists_across_reopen() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_durable_stream_reopen_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let tenant_id = TenantId("tenant_a".to_string());
        {
            let mut backend = CpuBackend::new(&base).expect("create backend");
            backend
                .ingest_batch(vec![RecordEnvelope::new(6001, "tenant_a", "Product")])
                .expect("ingest");

            let stream = backend
                .poll_durable_mutation_stream(&tenant_id, "group-reopen", 50)
                .expect("poll durable stream");
            assert_eq!(stream.len(), 1);

            let offsets = vec![(stream[0].partition, stream[0].sequence)];
            backend
                .commit_durable_mutation_offsets(&tenant_id, "group-reopen", &offsets)
                .expect("commit offsets");
        }

        {
            let mut reopened = CpuBackend::new(&base).expect("reopen backend");

            let already_consumed = reopened
                .poll_durable_mutation_stream(&tenant_id, "group-reopen", 50)
                .expect("poll committed group");
            assert!(already_consumed.is_empty());

            let new_group = reopened
                .poll_durable_mutation_stream(&tenant_id, "group-fresh", 50)
                .expect("poll fresh group");
            assert_eq!(new_group.len(), 1);
            assert!(matches!(
                new_group[0].event.mutation_type,
                idb_storage::MutationType::Insert
            ));
            assert_eq!(new_group[0].event.record_id.0, 6001);
        }

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn durable_mutation_stream_rejects_zero_max_events_per_partition() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_durable_stream_zero_max_events_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let tenant_id = TenantId("tenant_a".to_string());
        let mut backend = CpuBackend::new(&base).expect("create backend");
        backend
            .ingest_batch(vec![RecordEnvelope::new(6501, "tenant_a", "Product")])
            .expect("ingest");

        let err = backend
            .poll_durable_mutation_stream(&tenant_id, "group-zero", 0)
            .expect_err("zero max_events_per_partition should fail");
        assert!(err
            .to_string()
            .contains("max_events_per_partition must be > 0"));

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn durable_mutation_stream_rejects_empty_consumer_group() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_durable_stream_empty_group_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let tenant_id = TenantId("tenant_a".to_string());
        let mut backend = CpuBackend::new(&base).expect("create backend");
        backend
            .ingest_batch(vec![RecordEnvelope::new(6601, "tenant_a", "Product")])
            .expect("ingest");

        let poll_err = backend
            .poll_durable_mutation_stream(&tenant_id, "   ", 10)
            .expect_err("empty consumer_group should fail");
        assert!(poll_err
            .to_string()
            .contains("consumer_group must be non-empty"));

        let commit_err = backend
            .commit_durable_mutation_offsets(&tenant_id, "", &[])
            .expect_err("empty consumer_group should fail");
        assert!(commit_err
            .to_string()
            .contains("consumer_group must be non-empty"));

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn explain_query_text_reports_supported_projection() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_explain_query_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let backend = CpuBackend::new(&base).expect("create backend");
        let explain = backend
            .explain_query_text(
                TenantId("tenant_a".to_string()),
                "Brand(\"Norn Gold\") -> Product where price < 3000 | top(5)",
            )
            .expect("explain");

        match explain.request_projection {
            RequestProjectionStatus::Supported(request) => {
                assert_eq!(request.top_k, 5);
                assert_eq!(request.predicates.len(), 1);
            }
            RequestProjectionStatus::Unsupported { reason } => {
                panic!("expected supported projection, got {reason}")
            }
        }

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn explain_query_text_reports_unsupported_watch_projection() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_explain_query_watch_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let backend = CpuBackend::new(&base).expect("create backend");
        let explain = backend
            .explain_query_text(
                TenantId("tenant_a".to_string()),
                "watch Product where price < 3000 | top(5)",
            )
            .expect("explain");

        match explain.request_projection {
            RequestProjectionStatus::Supported(_) => panic!("expected unsupported"),
            RequestProjectionStatus::Unsupported { reason } => {
                assert!(reason.contains("watch mode"));
            }
        }

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn text_query_bridge_executes_semantic_query() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_text_query_semantic_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut semantic_match = RecordEnvelope::new(100, "tenant_a", "Product");
        semantic_match
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        semantic_match.embedding_fields.insert(
            "text_embedding".to_string(),
            idb_planner::deterministic_text_embedding("trending", 16).expect("embedding"),
        );

        let mut semantic_other = RecordEnvelope::new(101, "tenant_a", "Product");
        semantic_other
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(120.0));
        semantic_other.embedding_fields.insert(
            "text_embedding".to_string(),
            idb_planner::deterministic_text_embedding("formal", 16).expect("embedding"),
        );

        backend
            .ingest_batch(vec![semantic_match, semantic_other])
            .expect("ingest");

        let result = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Product where meaning(\"trending\") | top(1)",
            )
            .expect("semantic text query should execute");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].envelope.record_id.0, 100);
        assert!(result[0].score > 0.5);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn text_query_bridge_executes_multi_semantic_query() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_text_query_multi_semantic_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let query_centroid = idb_planner::compile_semantic_query_vector(
            &["trending".to_string(), "sport".to_string()],
            16,
        )
        .expect("centroid");

        let mut multi_match = RecordEnvelope::new(150, "tenant_a", "Product");
        multi_match
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        multi_match
            .embedding_fields
            .insert("text_embedding".to_string(), query_centroid);

        let mut semantic_other = RecordEnvelope::new(151, "tenant_a", "Product");
        semantic_other
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(120.0));
        semantic_other.embedding_fields.insert(
            "text_embedding".to_string(),
            idb_planner::deterministic_text_embedding("formal", 16).expect("embedding"),
        );

        backend
            .ingest_batch(vec![multi_match, semantic_other])
            .expect("ingest");

        let result = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Product where meaning(\"trending\") and meaning(\"sport\") | top(1)",
            )
            .expect("multi semantic text query should execute");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].envelope.record_id.0, 150);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn text_query_bridge_executes_hybrid_semantic_and_structured_query() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_text_query_hybrid_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut a = RecordEnvelope::new(201, "tenant_a", "Product");
        a.structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));
        a.embedding_fields.insert(
            "text_embedding".to_string(),
            idb_planner::deterministic_text_embedding("trending", 16).expect("embedding"),
        );

        let mut b = RecordEnvelope::new(202, "tenant_a", "Product");
        b.structured_fields
            .insert("price".to_string(), FieldValue::Float(3000.0));
        b.embedding_fields.insert(
            "text_embedding".to_string(),
            idb_planner::deterministic_text_embedding("trending", 16).expect("embedding"),
        );

        let mut c = RecordEnvelope::new(203, "tenant_a", "Product");
        c.structured_fields
            .insert("price".to_string(), FieldValue::Float(120.0));
        c.embedding_fields.insert(
            "text_embedding".to_string(),
            idb_planner::deterministic_text_embedding("formal", 16).expect("embedding"),
        );

        backend.ingest_batch(vec![a, b, c]).expect("ingest");

        let result = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Product where price < 500 and meaning(\"trending\") | top(2)",
            )
            .expect("hybrid text query should execute");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].envelope.record_id.0, 201);
        assert_eq!(result[1].envelope.record_id.0, 203);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn text_query_bridge_executes_sort_take_ordering() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_text_query_sort_take_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut a = RecordEnvelope::new(401, "tenant_a", "Product");
        a.structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));

        let mut b = RecordEnvelope::new(402, "tenant_a", "Product");
        b.structured_fields
            .insert("price".to_string(), FieldValue::Float(400.0));

        let mut c = RecordEnvelope::new(403, "tenant_a", "Product");
        c.structured_fields
            .insert("price".to_string(), FieldValue::Float(250.0));

        backend.ingest_batch(vec![a, b, c]).expect("ingest");

        let result = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Product | sort(price desc) | take(2)",
            )
            .expect("sort+take query should execute");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].envelope.record_id.0, 402);
        assert_eq!(result[1].envelope.record_id.0, 403);

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }

    #[test]
    fn text_query_bridge_enforces_semantic_threshold_and_rejects_invalid_bounds() {
        let base = env::temp_dir().join(format!(
            "idb_cpu_text_query_threshold_test_{}_{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");

        let mut backend = CpuBackend::new(&base).expect("create backend");
        let mut high = RecordEnvelope::new(301, "tenant_a", "Product");
        high.embedding_fields.insert(
            "text_embedding".to_string(),
            idb_planner::deterministic_text_embedding("trending", 16).expect("embedding"),
        );

        let mut low = RecordEnvelope::new(302, "tenant_a", "Product");
        low.embedding_fields.insert(
            "text_embedding".to_string(),
            idb_planner::deterministic_text_embedding("formal", 16).expect("embedding"),
        );

        backend.ingest_batch(vec![high, low]).expect("ingest");

        let filtered = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Product where meaning(\"trending\", threshold=0.8) | top(5)",
            )
            .expect("threshold semantic query should execute");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].envelope.record_id.0, 301);

        let invalid = backend
            .run_query_text(
                TenantId("tenant_a".to_string()),
                "Product where meaning(\"trending\", threshold=1.5) | top(5)",
            )
            .expect_err("invalid threshold should fail planning");
        assert!(matches!(invalid, CoreError::QueryPlanning(_)));
        assert!(invalid.to_string().contains("between -1.0 and 1.0"));

        fs::remove_dir_all(&base).expect("cleanup temp dir");
    }
}
