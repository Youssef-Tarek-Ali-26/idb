use std::path::Path;

use idb_core::{CoreResult, HybridScorePolicy};
use idb_executor_cpu::CpuBackend;
use idb_planner::QueryRequestBridgeOptions;
use idb_storage::SubscriptionId;

use crate::error::GatewayResult;
use crate::model::{
    GatewayCommand, GatewayDurableMutationRecord, GatewayRequest, GatewayResponse,
    GatewayResponsePayload, GatewayWatchSession, GatewayWatchUpdate, GatewayWatchUpdateBatch,
    TextQueryCompileConfig,
};

#[derive(Debug)]
pub struct CpuGatewayRuntime {
    backend: CpuBackend,
}

impl CpuGatewayRuntime {
    pub fn new(data_dir: impl AsRef<Path>) -> CoreResult<Self> {
        Ok(Self {
            backend: CpuBackend::new(data_dir)?,
        })
    }

    pub fn from_backend(backend: CpuBackend) -> Self {
        Self { backend }
    }

    pub fn backend(&self) -> &CpuBackend {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut CpuBackend {
        &mut self.backend
    }

    pub fn handle_request(&mut self, request: GatewayRequest) -> GatewayResult<GatewayResponse> {
        let request_id = request.metadata.request_id;
        let payload = match request.command {
            GatewayCommand::QueryText {
                caller,
                tenant_id,
                query_text,
                compile,
            } => {
                let options = bridge_options(tenant_id, compile);
                let rows = self.backend.run_query_text_with_options_and_context(
                    &query_text,
                    options,
                    &caller,
                )?;
                GatewayResponsePayload::QueryRows { rows }
            }
            GatewayCommand::ExplainText {
                caller,
                tenant_id,
                query_text,
                compile,
            } => {
                let options = bridge_options(tenant_id, compile);
                let explain = self.backend.explain_query_text_with_options_and_context(
                    &query_text,
                    options,
                    &caller,
                )?;
                GatewayResponsePayload::ExplainPlan { explain }
            }
            GatewayCommand::WatchStartText {
                caller,
                tenant_id,
                query_text,
                compile,
            } => {
                let options = bridge_options(tenant_id, compile);
                let session = self
                    .backend
                    .start_watch_query_text_with_options_and_context(
                        &query_text,
                        options,
                        &caller,
                    )?;
                GatewayResponsePayload::WatchStarted {
                    session: GatewayWatchSession {
                        subscription_id: session.subscription_id.0,
                        resume_token: session.resume_token.0,
                        snapshot: session.snapshot,
                    },
                }
            }
            GatewayCommand::WatchPoll {
                subscription_id,
                max_events,
            } => {
                let batch = self
                    .backend
                    .poll_watch_query_updates(SubscriptionId(subscription_id), max_events)?;
                let updates = batch
                    .updates
                    .into_iter()
                    .map(|update| GatewayWatchUpdate {
                        commit_sequence: update.event.commit_sequence,
                        record_id: update.event.record_id,
                        current: update.current,
                    })
                    .collect::<Vec<_>>();

                GatewayResponsePayload::WatchUpdates {
                    batch: GatewayWatchUpdateBatch {
                        subscription_id: batch.subscription_id.0,
                        updates,
                        next_resume_token: batch.next_resume_token.0,
                    },
                }
            }
            GatewayCommand::WatchStop { subscription_id } => {
                let stopped = self.backend.stop_watch(SubscriptionId(subscription_id));
                GatewayResponsePayload::WatchStopped { stopped }
            }
            GatewayCommand::Ingest { caller, records } => {
                let record_ids = self.backend.ingest_batch_with_context(&caller, records)?;
                GatewayResponsePayload::Ingested { record_ids }
            }
            GatewayCommand::Delete {
                caller,
                tenant_id,
                record_id,
            } => {
                let deleted = self
                    .backend
                    .delete_or_tombstone_with_context(&caller, &tenant_id, record_id)?;
                GatewayResponsePayload::Deleted { deleted }
            }
            GatewayCommand::DurableMutationPoll {
                caller,
                tenant_id,
                consumer_group,
                max_events_per_partition,
            } => {
                let records = self
                    .backend
                    .poll_durable_mutation_stream_with_context(
                        &caller,
                        &tenant_id,
                        &consumer_group,
                        max_events_per_partition,
                    )?
                    .into_iter()
                    .map(|record| GatewayDurableMutationRecord {
                        partition: record.partition,
                        sequence: record.sequence,
                        commit_sequence: record.event.commit_sequence,
                        record_id: record.event.record_id,
                        mutation_type: record.event.mutation_type,
                    })
                    .collect::<Vec<_>>();
                GatewayResponsePayload::DurableMutationRecords { records }
            }
            GatewayCommand::DurableMutationCommit {
                caller,
                tenant_id,
                consumer_group,
                offsets,
            } => {
                let mapped_offsets = offsets
                    .iter()
                    .map(|offset| (offset.partition, offset.committed_sequence))
                    .collect::<Vec<_>>();
                self.backend.commit_durable_mutation_offsets_with_context(
                    &caller,
                    &tenant_id,
                    &consumer_group,
                    &mapped_offsets,
                )?;
                GatewayResponsePayload::DurableMutationOffsetsCommitted {
                    partitions_committed: mapped_offsets.len(),
                }
            }
        };

        Ok(GatewayResponse {
            request_id,
            payload,
        })
    }
}

fn bridge_options(
    tenant_id: idb_core::TenantId,
    compile: TextQueryCompileConfig,
) -> QueryRequestBridgeOptions {
    QueryRequestBridgeOptions {
        tenant_id,
        top_k_default: compile.top_k_default,
        score_policy: HybridScorePolicy::default(),
        semantic_embedding_field: compile.semantic_embedding_field,
        semantic_embedding_dims: compile.semantic_embedding_dims,
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use idb_core::{
        AuthAction, AuthRuntime, AuthorizationDecision, AuthorizationProvider,
        AuthorizationRequest, CallerContext, FieldValue, RecordEnvelope, TenantId,
    };
    use idb_executor_cpu::CpuBackend;
    use serde_json::json;

    use crate::error::GatewayError;
    use crate::model::{GatewayResponsePayload, TextQueryCompileConfig};
    use crate::transport::{
        establish_tcp_session, normalize_http, normalize_tcp, normalize_websocket,
        HttpGatewayEnvelope, QueryTextPayload, TcpGatewayFrame, WebSocketGatewayEnvelope,
        OPCODE_QUERY_TEXT,
    };

    use super::CpuGatewayRuntime;

    struct DenyQueryProvider;
    struct DenyWatchProvider;

    impl AuthorizationProvider for DenyQueryProvider {
        fn decide(&self, request: &AuthorizationRequest) -> AuthorizationDecision {
            match request.action {
                AuthAction::Query => AuthorizationDecision::Deny {
                    reason: "policy denied query".to_string(),
                },
                _ => AuthorizationDecision::Allow,
            }
        }
    }

    impl AuthorizationProvider for DenyWatchProvider {
        fn decide(&self, request: &AuthorizationRequest) -> AuthorizationDecision {
            match request.action {
                AuthAction::Watch => AuthorizationDecision::Deny {
                    reason: "policy denied watch".to_string(),
                },
                _ => AuthorizationDecision::Allow,
            }
        }
    }

    fn temp_dir(test_name: &str) -> std::path::PathBuf {
        let base = env::temp_dir().join(format!(
            "idb_gateway_{}_{}_{}",
            test_name,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        fs::create_dir_all(&base).expect("create temp directory");
        base
    }

    fn sample_caller() -> CallerContext {
        CallerContext::service("sdk", Some(TenantId("tenant_a".to_string())))
    }

    fn ingest_sample_record(runtime: &mut CpuGatewayRuntime) {
        let mut record = RecordEnvelope::new(1, "tenant_a", "Product");
        record
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));

        let request = normalize_http(HttpGatewayEnvelope {
            request_id: "ingest-1".to_string(),
            method: "POST".to_string(),
            path: "/v1/records/ingest".to_string(),
            body: json!({
                "caller": sample_caller(),
                "records": [record],
            }),
        })
        .expect("normalize ingest");

        let response = runtime.handle_request(request).expect("ingest response");
        assert!(matches!(
            response.payload,
            GatewayResponsePayload::Ingested { .. }
        ));
    }

    #[test]
    fn equivalent_transport_queries_return_same_rows() {
        let base = temp_dir("equivalent_transport_queries");
        let mut runtime = CpuGatewayRuntime::new(&base).expect("runtime");
        ingest_sample_record(&mut runtime);

        let query_payload = QueryTextPayload {
            caller: Some(sample_caller()),
            tenant_id: "tenant_a".to_string(),
            query_text: "Product where price < 1000 | top(5)".to_string(),
            compile: Some(TextQueryCompileConfig::default()),
        };

        let http_request = normalize_http(HttpGatewayEnvelope {
            request_id: "http-q".to_string(),
            method: "POST".to_string(),
            path: "/v1/query/text".to_string(),
            body: serde_json::to_value(&query_payload).expect("payload"),
        })
        .expect("http normalize");

        let ws_request = normalize_websocket(WebSocketGatewayEnvelope {
            request_id: "ws-q".to_string(),
            event: "query.text".to_string(),
            payload: serde_json::to_value(&query_payload).expect("payload"),
        })
        .expect("ws normalize");

        let hello = idb_wire::SessionHello {
            client_name: "sdk-rust".to_string(),
            min_version: idb_wire::ProtocolVersion::new(1, 0),
            max_version: idb_wire::ProtocolVersion::new(1, 0),
        };
        let (tcp_session, _) = establish_tcp_session(
            &hello,
            idb_wire::ProtocolRange {
                min: idb_wire::ProtocolVersion::new(1, 0),
                max: idb_wire::ProtocolVersion::new(1, 2),
            },
        )
        .expect("session");
        let tcp_request = normalize_tcp(
            &tcp_session,
            TcpGatewayFrame {
                request_id: "tcp-q".to_string(),
                opcode: OPCODE_QUERY_TEXT,
                payload: serde_json::to_vec(&query_payload).expect("payload bytes"),
            },
        )
        .expect("tcp normalize");

        let http_response = runtime.handle_request(http_request).expect("http response");
        let ws_response = runtime.handle_request(ws_request).expect("ws response");
        let tcp_response = runtime.handle_request(tcp_request).expect("tcp response");

        let extract_ids = |payload: GatewayResponsePayload| -> Vec<u64> {
            match payload {
                GatewayResponsePayload::QueryRows { rows } => rows
                    .into_iter()
                    .map(|row| row.envelope.record_id.0)
                    .collect(),
                other => panic!("expected query rows, got {other:?}"),
            }
        };

        assert_eq!(extract_ids(http_response.payload), vec![1]);
        assert_eq!(extract_ids(ws_response.payload), vec![1]);
        assert_eq!(extract_ids(tcp_response.payload), vec![1]);

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn watch_start_poll_stop_works_through_gateway_runtime() {
        let base = temp_dir("watch_start_poll_stop");
        let mut runtime = CpuGatewayRuntime::new(&base).expect("runtime");
        ingest_sample_record(&mut runtime);

        let watch_start = normalize_http(HttpGatewayEnvelope {
            request_id: "watch-start".to_string(),
            method: "POST".to_string(),
            path: "/v1/watch/start".to_string(),
            body: json!({
                "caller": sample_caller(),
                "tenant_id": "tenant_a",
                "query_text": "watch Product where price < 1000 | top(10)",
                "compile": TextQueryCompileConfig::default(),
            }),
        })
        .expect("normalize watch start");

        let start_resp = runtime
            .handle_request(watch_start)
            .expect("watch start response");
        let subscription_id = match start_resp.payload {
            GatewayResponsePayload::WatchStarted { session } => session.subscription_id,
            other => panic!("expected watch started payload, got {other:?}"),
        };

        let mut updated = RecordEnvelope::new(1, "tenant_a", "Product");
        updated
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(90.0));
        let ingest_update = normalize_http(HttpGatewayEnvelope {
            request_id: "ingest-update".to_string(),
            method: "POST".to_string(),
            path: "/v1/records/ingest".to_string(),
            body: json!({
                "caller": sample_caller(),
                "records": [updated],
            }),
        })
        .expect("normalize update");
        runtime
            .handle_request(ingest_update)
            .expect("ingest update response");

        let poll = normalize_http(HttpGatewayEnvelope {
            request_id: "watch-poll".to_string(),
            method: "POST".to_string(),
            path: "/v1/watch/poll".to_string(),
            body: json!({
                "subscription_id": subscription_id,
                "max_events": 10,
            }),
        })
        .expect("normalize poll");
        let poll_resp = runtime.handle_request(poll).expect("poll response");

        match poll_resp.payload {
            GatewayResponsePayload::WatchUpdates { batch } => {
                assert_eq!(batch.subscription_id, subscription_id);
                assert!(!batch.updates.is_empty());
            }
            other => panic!("expected watch updates, got {other:?}"),
        }

        let stop = normalize_http(HttpGatewayEnvelope {
            request_id: "watch-stop".to_string(),
            method: "POST".to_string(),
            path: "/v1/watch/stop".to_string(),
            body: json!({
                "subscription_id": subscription_id,
            }),
        })
        .expect("normalize stop");
        let stop_resp = runtime.handle_request(stop).expect("stop response");
        assert!(matches!(
            stop_resp.payload,
            GatewayResponsePayload::WatchStopped { stopped: true }
        ));

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn durable_stream_poll_and_commit_work_through_gateway_runtime() {
        let base = temp_dir("durable_stream_poll_and_commit");
        let mut runtime = CpuGatewayRuntime::new(&base).expect("runtime");
        ingest_sample_record(&mut runtime);

        let poll = normalize_http(HttpGatewayEnvelope {
            request_id: "durable-poll".to_string(),
            method: "POST".to_string(),
            path: "/v1/streams/mutations/poll".to_string(),
            body: json!({
                "caller": sample_caller(),
                "tenant_id": "tenant_a",
                "consumer_group": "group-a",
                "max_events_per_partition": 50
            }),
        })
        .expect("normalize durable poll");

        let poll_resp = runtime.handle_request(poll).expect("poll response");
        let records = match poll_resp.payload {
            GatewayResponsePayload::DurableMutationRecords { records } => records,
            other => panic!("expected durable mutation records, got {other:?}"),
        };
        assert_eq!(records.len(), 1);

        let commit = normalize_http(HttpGatewayEnvelope {
            request_id: "durable-commit".to_string(),
            method: "POST".to_string(),
            path: "/v1/streams/mutations/commit".to_string(),
            body: json!({
                "caller": sample_caller(),
                "tenant_id": "tenant_a",
                "consumer_group": "group-a",
                "offsets": [
                    {
                        "partition": records[0].partition,
                        "committed_sequence": records[0].sequence
                    }
                ]
            }),
        })
        .expect("normalize durable commit");

        let commit_resp = runtime.handle_request(commit).expect("commit response");
        assert!(matches!(
            commit_resp.payload,
            GatewayResponsePayload::DurableMutationOffsetsCommitted {
                partitions_committed: 1
            }
        ));

        let repoll = normalize_http(HttpGatewayEnvelope {
            request_id: "durable-repoll".to_string(),
            method: "POST".to_string(),
            path: "/v1/streams/mutations/poll".to_string(),
            body: json!({
                "caller": sample_caller(),
                "tenant_id": "tenant_a",
                "consumer_group": "group-a",
                "max_events_per_partition": 50
            }),
        })
        .expect("normalize durable repoll");
        let repoll_resp = runtime.handle_request(repoll).expect("repoll response");
        match repoll_resp.payload {
            GatewayResponsePayload::DurableMutationRecords { records } => {
                assert!(records.is_empty())
            }
            other => panic!("expected durable mutation records, got {other:?}"),
        }

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn auth_denial_is_returned_from_gateway_runtime() {
        let base = temp_dir("auth_denial");
        let mut backend = CpuBackend::new(&base).expect("backend");
        backend.set_auth_runtime(AuthRuntime::with_provider(DenyQueryProvider));
        let mut runtime = CpuGatewayRuntime::from_backend(backend);
        ingest_sample_record(&mut runtime);

        let query = normalize_http(HttpGatewayEnvelope {
            request_id: "query-denied".to_string(),
            method: "POST".to_string(),
            path: "/v1/query/text".to_string(),
            body: json!({
                "caller": sample_caller(),
                "tenant_id": "tenant_a",
                "query_text": "Product where price < 1000 | top(5)",
                "compile": TextQueryCompileConfig::default(),
            }),
        })
        .expect("normalize query");

        let err = runtime
            .handle_request(query)
            .expect_err("query should fail");
        assert!(matches!(
            err,
            GatewayError::Core(idb_core::CoreError::AuthorizationDenied(_))
        ));

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn durable_stream_auth_denial_is_returned_from_gateway_runtime() {
        let base = temp_dir("durable_stream_auth_denial");
        let mut backend = CpuBackend::new(&base).expect("backend");
        backend.set_auth_runtime(AuthRuntime::with_provider(DenyWatchProvider));
        let mut runtime = CpuGatewayRuntime::from_backend(backend);
        ingest_sample_record(&mut runtime);

        let poll = normalize_http(HttpGatewayEnvelope {
            request_id: "durable-poll-denied".to_string(),
            method: "POST".to_string(),
            path: "/v1/streams/mutations/poll".to_string(),
            body: json!({
                "caller": sample_caller(),
                "tenant_id": "tenant_a",
                "consumer_group": "group-a",
                "max_events_per_partition": 50
            }),
        })
        .expect("normalize durable poll");

        let err = runtime.handle_request(poll).expect_err("poll should fail");
        assert!(matches!(
            err,
            GatewayError::Core(idb_core::CoreError::AuthorizationDenied(_))
        ));

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn durable_stream_tenant_scope_mismatch_is_rejected() {
        let base = temp_dir("durable_stream_tenant_scope_mismatch");
        let mut runtime = CpuGatewayRuntime::new(&base).expect("runtime");
        ingest_sample_record(&mut runtime);

        let scoped_to_other_tenant =
            CallerContext::service("sdk", Some(TenantId("tenant_b".to_string())));
        let commit = normalize_http(HttpGatewayEnvelope {
            request_id: "durable-commit-scope-mismatch".to_string(),
            method: "POST".to_string(),
            path: "/v1/streams/mutations/commit".to_string(),
            body: json!({
                "caller": scoped_to_other_tenant,
                "tenant_id": "tenant_a",
                "consumer_group": "group-a",
                "offsets": []
            }),
        })
        .expect("normalize durable commit");

        let err = runtime
            .handle_request(commit)
            .expect_err("commit should fail for tenant mismatch");
        assert!(matches!(
            err,
            GatewayError::Core(idb_core::CoreError::AuthorizationDenied(_))
        ));

        fs::remove_dir_all(&base).expect("cleanup");
    }
}
