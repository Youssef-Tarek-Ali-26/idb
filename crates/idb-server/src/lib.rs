use std::collections::HashMap;
use std::path::Path;

use idb_gateway::{
    establish_tcp_session, normalize_http, normalize_tcp, normalize_websocket, CpuGatewayRuntime,
    GatewayResponse, HttpGatewayEnvelope, TcpGatewayFrame, TcpGatewaySession,
    WebSocketGatewayEnvelope,
};
use idb_wire::{ProtocolRange, ProtocolVersion, SessionHello, SessionWelcome};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("unknown TCP session id: {0}")]
    UnknownTcpSession(u64),
    #[error("unknown script TCP session key: {0}")]
    UnknownScriptSessionKey(String),
    #[error("invalid json payload: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error(transparent)]
    Gateway(#[from] idb_gateway::GatewayError),
    #[error(transparent)]
    Core(#[from] idb_core::CoreError),
}

pub type ServerResult<T> = Result<T, ServerError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TcpSessionId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerConfig {
    pub supported_protocol: ProtocolRange,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            supported_protocol: ProtocolRange {
                min: ProtocolVersion::new(1, 0),
                max: ProtocolVersion::new(1, 0),
            },
        }
    }
}

#[derive(Debug)]
pub struct IdbServer {
    runtime: CpuGatewayRuntime,
    config: ServerConfig,
    next_tcp_session_id: u64,
    tcp_sessions: HashMap<TcpSessionId, TcpGatewaySession>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenTcpSessionResponse {
    pub session_id: TcpSessionId,
    pub welcome: SessionWelcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerScriptStep {
    Http {
        request_id: String,
        method: String,
        path: String,
        body: Value,
    },
    WebSocket {
        request_id: String,
        event: String,
        payload: Value,
    },
    TcpOpen {
        session_key: String,
        hello: SessionHello,
    },
    TcpFrame {
        session_key: String,
        request_id: String,
        opcode: u16,
        payload: Value,
    },
    TcpClose {
        session_key: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerScriptEvent {
    Response {
        request_id: String,
        response: GatewayResponse,
    },
    TcpOpened {
        session_key: String,
        session_id: TcpSessionId,
        welcome: SessionWelcome,
    },
    TcpClosed {
        session_key: String,
        closed: bool,
    },
}

impl IdbServer {
    pub fn new(data_dir: impl AsRef<Path>) -> ServerResult<Self> {
        Self::new_with_config(data_dir, ServerConfig::default())
    }

    pub fn new_with_config(data_dir: impl AsRef<Path>, config: ServerConfig) -> ServerResult<Self> {
        Ok(Self {
            runtime: CpuGatewayRuntime::new(data_dir)?,
            config,
            next_tcp_session_id: 0,
            tcp_sessions: HashMap::new(),
        })
    }

    pub fn from_runtime(runtime: CpuGatewayRuntime, config: ServerConfig) -> Self {
        Self {
            runtime,
            config,
            next_tcp_session_id: 0,
            tcp_sessions: HashMap::new(),
        }
    }

    pub fn runtime(&self) -> &CpuGatewayRuntime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut CpuGatewayRuntime {
        &mut self.runtime
    }

    pub fn handle_http_envelope(
        &mut self,
        envelope: HttpGatewayEnvelope,
    ) -> ServerResult<GatewayResponse> {
        let request = normalize_http(envelope)?;
        Ok(self.runtime.handle_request(request)?)
    }

    pub fn handle_websocket_envelope(
        &mut self,
        envelope: WebSocketGatewayEnvelope,
    ) -> ServerResult<GatewayResponse> {
        let request = normalize_websocket(envelope)?;
        Ok(self.runtime.handle_request(request)?)
    }

    pub fn open_tcp_session(
        &mut self,
        hello: &SessionHello,
    ) -> ServerResult<OpenTcpSessionResponse> {
        let (session, welcome) = establish_tcp_session(hello, self.config.supported_protocol)?;
        self.next_tcp_session_id += 1;
        let session_id = TcpSessionId(self.next_tcp_session_id);
        self.tcp_sessions.insert(session_id, session);

        Ok(OpenTcpSessionResponse {
            session_id,
            welcome,
        })
    }

    pub fn close_tcp_session(&mut self, session_id: TcpSessionId) -> bool {
        self.tcp_sessions.remove(&session_id).is_some()
    }

    pub fn handle_tcp_frame(
        &mut self,
        session_id: TcpSessionId,
        frame: TcpGatewayFrame,
    ) -> ServerResult<GatewayResponse> {
        let session = self
            .tcp_sessions
            .get(&session_id)
            .ok_or(ServerError::UnknownTcpSession(session_id.0))?;
        let request = normalize_tcp(session, frame)?;
        Ok(self.runtime.handle_request(request)?)
    }

    pub fn handle_http_json(
        &mut self,
        request_id: impl Into<String>,
        method: impl Into<String>,
        path: impl Into<String>,
        body_json: &str,
    ) -> ServerResult<String> {
        let body: Value = serde_json::from_str(body_json)?;
        let response = self.handle_http_envelope(HttpGatewayEnvelope {
            request_id: request_id.into(),
            method: method.into(),
            path: path.into(),
            body,
        })?;
        Ok(serde_json::to_string(&response)?)
    }

    pub fn handle_websocket_json(
        &mut self,
        request_id: impl Into<String>,
        event: impl Into<String>,
        payload_json: &str,
    ) -> ServerResult<String> {
        let payload: Value = serde_json::from_str(payload_json)?;
        let response = self.handle_websocket_envelope(WebSocketGatewayEnvelope {
            request_id: request_id.into(),
            event: event.into(),
            payload,
        })?;
        Ok(serde_json::to_string(&response)?)
    }

    pub fn open_tcp_session_json(&mut self, hello_json: &str) -> ServerResult<String> {
        let hello: SessionHello = serde_json::from_str(hello_json)?;
        let response = self.open_tcp_session(&hello)?;
        Ok(serde_json::to_string(&response)?)
    }

    pub fn handle_tcp_json_frame(
        &mut self,
        session_id: TcpSessionId,
        request_id: impl Into<String>,
        opcode: u16,
        payload_json: &str,
    ) -> ServerResult<String> {
        let payload_value: Value = serde_json::from_str(payload_json)?;
        let frame = TcpGatewayFrame {
            request_id: request_id.into(),
            opcode,
            payload: serde_json::to_vec(&payload_value)?,
        };
        let response = self.handle_tcp_frame(session_id, frame)?;
        Ok(serde_json::to_string(&response)?)
    }

    pub fn run_script(
        &mut self,
        steps: &[ServerScriptStep],
    ) -> ServerResult<Vec<ServerScriptEvent>> {
        let mut session_map: HashMap<String, TcpSessionId> = HashMap::new();
        let mut events = Vec::new();

        for step in steps {
            match step {
                ServerScriptStep::Http {
                    request_id,
                    method,
                    path,
                    body,
                } => {
                    let response = self.handle_http_envelope(HttpGatewayEnvelope {
                        request_id: request_id.clone(),
                        method: method.clone(),
                        path: path.clone(),
                        body: body.clone(),
                    })?;
                    events.push(ServerScriptEvent::Response {
                        request_id: request_id.clone(),
                        response,
                    });
                }
                ServerScriptStep::WebSocket {
                    request_id,
                    event,
                    payload,
                } => {
                    let response = self.handle_websocket_envelope(WebSocketGatewayEnvelope {
                        request_id: request_id.clone(),
                        event: event.clone(),
                        payload: payload.clone(),
                    })?;
                    events.push(ServerScriptEvent::Response {
                        request_id: request_id.clone(),
                        response,
                    });
                }
                ServerScriptStep::TcpOpen { session_key, hello } => {
                    let opened = self.open_tcp_session(hello)?;
                    session_map.insert(session_key.clone(), opened.session_id);
                    events.push(ServerScriptEvent::TcpOpened {
                        session_key: session_key.clone(),
                        session_id: opened.session_id,
                        welcome: opened.welcome,
                    });
                }
                ServerScriptStep::TcpFrame {
                    session_key,
                    request_id,
                    opcode,
                    payload,
                } => {
                    let session_id = session_map
                        .get(session_key)
                        .copied()
                        .ok_or_else(|| ServerError::UnknownScriptSessionKey(session_key.clone()))?;
                    let response = self.handle_tcp_frame(
                        session_id,
                        TcpGatewayFrame {
                            request_id: request_id.clone(),
                            opcode: *opcode,
                            payload: serde_json::to_vec(payload)?,
                        },
                    )?;
                    events.push(ServerScriptEvent::Response {
                        request_id: request_id.clone(),
                        response,
                    });
                }
                ServerScriptStep::TcpClose { session_key } => {
                    let session_id = session_map
                        .remove(session_key)
                        .ok_or_else(|| ServerError::UnknownScriptSessionKey(session_key.clone()))?;
                    let closed = self.close_tcp_session(session_id);
                    events.push(ServerScriptEvent::TcpClosed {
                        session_key: session_key.clone(),
                        closed,
                    });
                }
            }
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use idb_core::{CallerContext, FieldValue, RecordEnvelope, TenantId};
    use idb_gateway::{
        GatewayError, GatewayResponsePayload, QueryTextPayload, TextQueryCompileConfig,
        OPCODE_DURABLE_MUTATION_COMMIT, OPCODE_DURABLE_MUTATION_POLL, OPCODE_QUERY_TEXT,
    };
    use idb_wire::{ProtocolVersion, SessionHello};

    use super::{IdbServer, ServerError, ServerScriptEvent, ServerScriptStep, TcpSessionId};

    fn temp_dir(test_name: &str) -> std::path::PathBuf {
        let base = env::temp_dir().join(format!(
            "idb_server_{}_{}_{}",
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

    fn ingest_seed_record(server: &mut IdbServer) {
        let mut record = RecordEnvelope::new(1, "tenant_a", "Product");
        record
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));

        let body = serde_json::json!({
            "caller": sample_caller(),
            "records": [record],
        });

        let response = server
            .handle_http_json("ingest-1", "POST", "/v1/records/ingest", &body.to_string())
            .expect("ingest response");

        let parsed: idb_gateway::GatewayResponse =
            serde_json::from_str(&response).expect("deserialize ingest response");
        assert!(matches!(
            parsed.payload,
            GatewayResponsePayload::Ingested { .. }
        ));
    }

    #[test]
    fn http_ws_tcp_adapters_return_consistent_query_rows() {
        let base = temp_dir("http_ws_tcp_consistency");
        let mut server = IdbServer::new(&base).expect("server");
        ingest_seed_record(&mut server);

        let query_payload = QueryTextPayload {
            caller: Some(sample_caller()),
            tenant_id: "tenant_a".to_string(),
            query_text: "Product where price < 500 | top(5)".to_string(),
            compile: Some(TextQueryCompileConfig::default()),
        };
        let query_json = serde_json::to_string(&query_payload).expect("query json");

        let http = server
            .handle_http_json("http-q", "POST", "/v1/query/text", &query_json)
            .expect("http query");
        let ws = server
            .handle_websocket_json("ws-q", "query.text", &query_json)
            .expect("ws query");

        let hello = SessionHello {
            client_name: "sdk-rust".to_string(),
            min_version: ProtocolVersion::new(1, 0),
            max_version: ProtocolVersion::new(1, 0),
        };
        let session = server.open_tcp_session(&hello).expect("open tcp session");

        let tcp = server
            .handle_tcp_json_frame(session.session_id, "tcp-q", OPCODE_QUERY_TEXT, &query_json)
            .expect("tcp query");

        let parse_ids = |data: &str| -> Vec<u64> {
            let parsed: idb_gateway::GatewayResponse =
                serde_json::from_str(data).expect("response json");
            match parsed.payload {
                GatewayResponsePayload::QueryRows { rows } => rows
                    .into_iter()
                    .map(|row| row.envelope.record_id.0)
                    .collect(),
                other => panic!("expected query rows, got {other:?}"),
            }
        };

        assert_eq!(parse_ids(&http), vec![1]);
        assert_eq!(parse_ids(&ws), vec![1]);
        assert_eq!(parse_ids(&tcp), vec![1]);

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn durable_stream_poll_and_commit_work_through_all_adapters() {
        let base = temp_dir("durable_stream_poll_and_commit_all_adapters");
        let mut server = IdbServer::new(&base).expect("server");
        ingest_seed_record(&mut server);

        let poll_body = |group: &str| {
            serde_json::json!({
                "caller": sample_caller(),
                "tenant_id": "tenant_a",
                "consumer_group": group,
                "max_events_per_partition": 50
            })
        };

        let commit_body = |group: &str, partition: u32, sequence: u64| {
            serde_json::json!({
                "caller": sample_caller(),
                "tenant_id": "tenant_a",
                "consumer_group": group,
                "offsets": [{
                    "partition": partition,
                    "committed_sequence": sequence
                }]
            })
        };

        let http_poll = server
            .handle_http_json(
                "http-durable-poll",
                "POST",
                "/v1/streams/mutations/poll",
                &poll_body("group-http").to_string(),
            )
            .expect("http poll");
        let http_poll_parsed: idb_gateway::GatewayResponse =
            serde_json::from_str(&http_poll).expect("http poll response");
        let http_records = match http_poll_parsed.payload {
            GatewayResponsePayload::DurableMutationRecords { records } => records,
            other => panic!("expected durable mutation records, got {other:?}"),
        };
        assert_eq!(http_records.len(), 1);

        let http_commit = server
            .handle_http_json(
                "http-durable-commit",
                "POST",
                "/v1/streams/mutations/commit",
                &commit_body(
                    "group-http",
                    http_records[0].partition,
                    http_records[0].sequence,
                )
                .to_string(),
            )
            .expect("http commit");
        let http_commit_parsed: idb_gateway::GatewayResponse =
            serde_json::from_str(&http_commit).expect("http commit response");
        assert!(matches!(
            http_commit_parsed.payload,
            GatewayResponsePayload::DurableMutationOffsetsCommitted {
                partitions_committed: 1
            }
        ));

        let http_repoll = server
            .handle_http_json(
                "http-durable-repoll",
                "POST",
                "/v1/streams/mutations/poll",
                &poll_body("group-http").to_string(),
            )
            .expect("http repoll");
        let http_repoll_parsed: idb_gateway::GatewayResponse =
            serde_json::from_str(&http_repoll).expect("http repoll response");
        match http_repoll_parsed.payload {
            GatewayResponsePayload::DurableMutationRecords { records } => {
                assert!(records.is_empty())
            }
            other => panic!("expected durable mutation records, got {other:?}"),
        }

        let ws_poll = server
            .handle_websocket_json(
                "ws-durable-poll",
                "streams.mutations.poll",
                &poll_body("group-ws").to_string(),
            )
            .expect("ws poll");
        let ws_poll_parsed: idb_gateway::GatewayResponse =
            serde_json::from_str(&ws_poll).expect("ws poll response");
        let ws_records = match ws_poll_parsed.payload {
            GatewayResponsePayload::DurableMutationRecords { records } => records,
            other => panic!("expected durable mutation records, got {other:?}"),
        };
        assert_eq!(ws_records.len(), 1);

        let ws_commit = server
            .handle_websocket_json(
                "ws-durable-commit",
                "streams.mutations.commit",
                &commit_body("group-ws", ws_records[0].partition, ws_records[0].sequence)
                    .to_string(),
            )
            .expect("ws commit");
        let ws_commit_parsed: idb_gateway::GatewayResponse =
            serde_json::from_str(&ws_commit).expect("ws commit response");
        assert!(matches!(
            ws_commit_parsed.payload,
            GatewayResponsePayload::DurableMutationOffsetsCommitted {
                partitions_committed: 1
            }
        ));

        let ws_repoll = server
            .handle_websocket_json(
                "ws-durable-repoll",
                "streams.mutations.poll",
                &poll_body("group-ws").to_string(),
            )
            .expect("ws repoll");
        let ws_repoll_parsed: idb_gateway::GatewayResponse =
            serde_json::from_str(&ws_repoll).expect("ws repoll response");
        match ws_repoll_parsed.payload {
            GatewayResponsePayload::DurableMutationRecords { records } => {
                assert!(records.is_empty())
            }
            other => panic!("expected durable mutation records, got {other:?}"),
        }

        let hello = SessionHello {
            client_name: "sdk-rust".to_string(),
            min_version: ProtocolVersion::new(1, 0),
            max_version: ProtocolVersion::new(1, 0),
        };
        let tcp_session = server.open_tcp_session(&hello).expect("open tcp session");

        let tcp_poll = server
            .handle_tcp_json_frame(
                tcp_session.session_id,
                "tcp-durable-poll",
                OPCODE_DURABLE_MUTATION_POLL,
                &poll_body("group-tcp").to_string(),
            )
            .expect("tcp poll");
        let tcp_poll_parsed: idb_gateway::GatewayResponse =
            serde_json::from_str(&tcp_poll).expect("tcp poll response");
        let tcp_records = match tcp_poll_parsed.payload {
            GatewayResponsePayload::DurableMutationRecords { records } => records,
            other => panic!("expected durable mutation records, got {other:?}"),
        };
        assert_eq!(tcp_records.len(), 1);

        let tcp_commit = server
            .handle_tcp_json_frame(
                tcp_session.session_id,
                "tcp-durable-commit",
                OPCODE_DURABLE_MUTATION_COMMIT,
                &commit_body(
                    "group-tcp",
                    tcp_records[0].partition,
                    tcp_records[0].sequence,
                )
                .to_string(),
            )
            .expect("tcp commit");
        let tcp_commit_parsed: idb_gateway::GatewayResponse =
            serde_json::from_str(&tcp_commit).expect("tcp commit response");
        assert!(matches!(
            tcp_commit_parsed.payload,
            GatewayResponsePayload::DurableMutationOffsetsCommitted {
                partitions_committed: 1
            }
        ));

        let tcp_repoll = server
            .handle_tcp_json_frame(
                tcp_session.session_id,
                "tcp-durable-repoll",
                OPCODE_DURABLE_MUTATION_POLL,
                &poll_body("group-tcp").to_string(),
            )
            .expect("tcp repoll");
        let tcp_repoll_parsed: idb_gateway::GatewayResponse =
            serde_json::from_str(&tcp_repoll).expect("tcp repoll response");
        match tcp_repoll_parsed.payload {
            GatewayResponsePayload::DurableMutationRecords { records } => {
                assert!(records.is_empty())
            }
            other => panic!("expected durable mutation records, got {other:?}"),
        }

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn unknown_tcp_session_is_rejected() {
        let base = temp_dir("unknown_tcp_session");
        let mut server = IdbServer::new(&base).expect("server");

        let err = server
            .handle_tcp_json_frame(TcpSessionId(999), "tcp-q", OPCODE_QUERY_TEXT, "{}")
            .expect_err("must fail");
        assert!(matches!(err, ServerError::UnknownTcpSession(999)));

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn closing_tcp_session_prevents_further_use() {
        let base = temp_dir("close_tcp_session");
        let mut server = IdbServer::new(&base).expect("server");

        let hello = SessionHello {
            client_name: "sdk-rust".to_string(),
            min_version: ProtocolVersion::new(1, 0),
            max_version: ProtocolVersion::new(1, 0),
        };
        let opened = server.open_tcp_session(&hello).expect("open session");
        assert!(server.close_tcp_session(opened.session_id));

        let err = server
            .handle_tcp_json_frame(opened.session_id, "tcp-q", OPCODE_QUERY_TEXT, "{}")
            .expect_err("must fail");
        assert!(matches!(
            err,
            ServerError::UnknownTcpSession(id) if id == opened.session_id.0
        ));

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn malformed_http_json_payload_is_rejected() {
        let base = temp_dir("malformed_http_json_payload");
        let mut server = IdbServer::new(&base).expect("server");

        let err = server
            .handle_http_json("http-q", "POST", "/v1/query/text", "{not-valid-json")
            .expect_err("must fail");
        assert!(matches!(err, ServerError::InvalidJson(_)));

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn durable_stream_invalid_payload_is_rejected_across_adapters() {
        let base = temp_dir("durable_stream_invalid_payload_across_adapters");
        let mut server = IdbServer::new(&base).expect("server");

        let invalid_payload = serde_json::json!({
            "tenant_id": "tenant_a",
            "consumer_group": "",
            "max_events_per_partition": 10
        });
        let invalid_json = invalid_payload.to_string();

        let http_err = server
            .handle_http_json(
                "durable-http-invalid",
                "POST",
                "/v1/streams/mutations/poll",
                &invalid_json,
            )
            .expect_err("http should reject invalid payload");
        assert!(matches!(
            http_err,
            ServerError::Gateway(GatewayError::InvalidPayload { .. })
        ));

        let ws_err = server
            .handle_websocket_json(
                "durable-ws-invalid",
                "streams.mutations.poll",
                &invalid_json,
            )
            .expect_err("ws should reject invalid payload");
        assert!(matches!(
            ws_err,
            ServerError::Gateway(GatewayError::InvalidPayload { .. })
        ));

        let hello = SessionHello {
            client_name: "sdk-rust".to_string(),
            min_version: ProtocolVersion::new(1, 0),
            max_version: ProtocolVersion::new(1, 0),
        };
        let session = server.open_tcp_session(&hello).expect("open tcp session");
        let tcp_err = server
            .handle_tcp_json_frame(
                session.session_id,
                "durable-tcp-invalid",
                OPCODE_DURABLE_MUTATION_POLL,
                &invalid_json,
            )
            .expect_err("tcp should reject invalid payload");
        assert!(matches!(
            tcp_err,
            ServerError::Gateway(GatewayError::InvalidPayload { .. })
        ));

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn scripted_flow_executes_mixed_transports() {
        let base = temp_dir("scripted_flow_executes_mixed_transports");
        let mut server = IdbServer::new(&base).expect("server");

        let mut record = RecordEnvelope::new(1, "tenant_a", "Product");
        record
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(100.0));

        let query_payload = serde_json::json!({
            "caller": sample_caller(),
            "tenant_id": "tenant_a",
            "query_text": "Product where price < 500 | top(5)",
            "compile": TextQueryCompileConfig::default(),
        });

        let steps = vec![
            ServerScriptStep::Http {
                request_id: "ingest-http".to_string(),
                method: "POST".to_string(),
                path: "/v1/records/ingest".to_string(),
                body: serde_json::json!({
                    "caller": sample_caller(),
                    "records": [record],
                }),
            },
            ServerScriptStep::WebSocket {
                request_id: "query-ws".to_string(),
                event: "query.text".to_string(),
                payload: query_payload.clone(),
            },
            ServerScriptStep::TcpOpen {
                session_key: "s1".to_string(),
                hello: SessionHello {
                    client_name: "sdk-rust".to_string(),
                    min_version: ProtocolVersion::new(1, 0),
                    max_version: ProtocolVersion::new(1, 0),
                },
            },
            ServerScriptStep::TcpFrame {
                session_key: "s1".to_string(),
                request_id: "query-tcp".to_string(),
                opcode: OPCODE_QUERY_TEXT,
                payload: query_payload,
            },
            ServerScriptStep::TcpClose {
                session_key: "s1".to_string(),
            },
        ];

        let events = server.run_script(&steps).expect("run script");
        assert_eq!(events.len(), 5);

        let mut saw_ws_rows = false;
        let mut saw_tcp_rows = false;
        let mut saw_open = false;
        let mut saw_close = false;

        for event in events {
            match event {
                ServerScriptEvent::Response {
                    request_id,
                    response,
                } if request_id == "query-ws" => {
                    if let GatewayResponsePayload::QueryRows { rows } = response.payload {
                        saw_ws_rows = rows.iter().any(|r| r.envelope.record_id.0 == 1);
                    }
                }
                ServerScriptEvent::Response {
                    request_id,
                    response,
                } if request_id == "query-tcp" => {
                    if let GatewayResponsePayload::QueryRows { rows } = response.payload {
                        saw_tcp_rows = rows.iter().any(|r| r.envelope.record_id.0 == 1);
                    }
                }
                ServerScriptEvent::TcpOpened { .. } => saw_open = true,
                ServerScriptEvent::TcpClosed { closed, .. } => saw_close = closed,
                _ => {}
            }
        }

        assert!(saw_ws_rows);
        assert!(saw_tcp_rows);
        assert!(saw_open);
        assert!(saw_close);

        fs::remove_dir_all(&base).expect("cleanup");
    }

    #[test]
    fn scripted_flow_rejects_unknown_session_key() {
        let base = temp_dir("scripted_flow_rejects_unknown_session_key");
        let mut server = IdbServer::new(&base).expect("server");

        let steps = vec![ServerScriptStep::TcpFrame {
            session_key: "missing".to_string(),
            request_id: "query-tcp".to_string(),
            opcode: OPCODE_QUERY_TEXT,
            payload: serde_json::json!({}),
        }];

        let err = server.run_script(&steps).expect_err("must fail");
        assert!(matches!(
            err,
            ServerError::UnknownScriptSessionKey(key) if key == "missing"
        ));

        fs::remove_dir_all(&base).expect("cleanup");
    }
}
