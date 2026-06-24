use std::num::NonZeroUsize;

use idb_core::{CallerContext, RecordEnvelope, RecordId, TenantId};
use idb_wire::{negotiate_handshake, ProtocolRange, ProtocolVersion, SessionHello, SessionWelcome};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::error::{GatewayError, GatewayResult};
use crate::model::{
    GatewayCommand, GatewayDurableMutationOffset, GatewayMetadata, GatewayRequest,
    TextQueryCompileConfig, TransportKind,
};

pub const OPCODE_QUERY_TEXT: u16 = 0x01;
pub const OPCODE_EXPLAIN_TEXT: u16 = 0x02;
pub const OPCODE_WATCH_START_TEXT: u16 = 0x03;
pub const OPCODE_WATCH_POLL: u16 = 0x04;
pub const OPCODE_WATCH_STOP: u16 = 0x05;
pub const OPCODE_INGEST: u16 = 0x06;
pub const OPCODE_DELETE: u16 = 0x07;
pub const OPCODE_DURABLE_MUTATION_POLL: u16 = 0x08;
pub const OPCODE_DURABLE_MUTATION_COMMIT: u16 = 0x09;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpGatewayEnvelope {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub body: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSocketGatewayEnvelope {
    pub request_id: String,
    pub event: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TcpGatewayFrame {
    pub request_id: String,
    pub opcode: u16,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpGatewaySession {
    pub negotiated_version: ProtocolVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryTextPayload {
    pub caller: Option<CallerContext>,
    pub tenant_id: String,
    pub query_text: String,
    pub compile: Option<TextQueryCompileConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchPollPayload {
    pub subscription_id: u64,
    pub max_events: Option<NonZeroUsize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchStopPayload {
    pub subscription_id: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IngestPayload {
    pub caller: Option<CallerContext>,
    pub records: Vec<RecordEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletePayload {
    pub caller: Option<CallerContext>,
    pub tenant_id: String,
    pub record_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableMutationPollPayload {
    pub caller: Option<CallerContext>,
    pub tenant_id: String,
    #[serde(deserialize_with = "deserialize_non_empty_trimmed_string")]
    pub consumer_group: String,
    pub max_events_per_partition: Option<NonZeroUsize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableMutationOffsetPayload {
    pub partition: u32,
    pub committed_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableMutationCommitPayload {
    pub caller: Option<CallerContext>,
    pub tenant_id: String,
    #[serde(deserialize_with = "deserialize_non_empty_trimmed_string")]
    pub consumer_group: String,
    pub offsets: Vec<DurableMutationOffsetPayload>,
}

fn deserialize_non_empty_trimmed_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(serde::de::Error::custom(
            "expected non-empty string for consumer_group",
        ));
    }
    Ok(trimmed.to_string())
}

pub fn establish_tcp_session(
    hello: &SessionHello,
    server_supported: ProtocolRange,
) -> GatewayResult<(TcpGatewaySession, SessionWelcome)> {
    let welcome = negotiate_handshake(hello, server_supported)
        .map_err(|e| GatewayError::ProtocolNegotiation(e.to_string()))?;
    Ok((
        TcpGatewaySession {
            negotiated_version: welcome.negotiated_version,
        },
        welcome,
    ))
}

pub fn normalize_http(envelope: HttpGatewayEnvelope) -> GatewayResult<GatewayRequest> {
    let method = envelope.method.to_ascii_uppercase();
    let metadata = GatewayMetadata {
        request_id: envelope.request_id,
        transport: TransportKind::Http,
    };

    let command = match (method.as_str(), envelope.path.as_str()) {
        ("POST", "/v1/query/text") | ("POST", "/query/text") => {
            parse_text_command(envelope.body, "http query.text", GatewayCommandKind::Query)?
        }
        ("POST", "/v1/query/explain") | ("POST", "/query/explain") => parse_text_command(
            envelope.body,
            "http query.explain",
            GatewayCommandKind::Explain,
        )?,
        ("POST", "/v1/watch/start") | ("POST", "/watch/start") => parse_text_command(
            envelope.body,
            "http watch.start",
            GatewayCommandKind::WatchStart,
        )?,
        ("POST", "/v1/watch/poll") | ("POST", "/watch/poll") => {
            parse_watch_poll(envelope.body, "http watch.poll")?
        }
        ("POST", "/v1/watch/stop") | ("POST", "/watch/stop") => {
            parse_watch_stop(envelope.body, "http watch.stop")?
        }
        ("POST", "/v1/records/ingest") | ("POST", "/records/ingest") => {
            parse_ingest(envelope.body, "http records.ingest")?
        }
        ("POST", "/v1/records/delete") | ("POST", "/records/delete") => {
            parse_delete(envelope.body, "http records.delete")?
        }
        ("POST", "/v1/streams/mutations/poll") | ("POST", "/streams/mutations/poll") => {
            parse_durable_mutation_poll(envelope.body, "http streams.mutations.poll")?
        }
        ("POST", "/v1/streams/mutations/commit") | ("POST", "/streams/mutations/commit") => {
            parse_durable_mutation_commit(envelope.body, "http streams.mutations.commit")?
        }
        _ => {
            return Err(GatewayError::UnsupportedHttpRoute {
                method,
                path: envelope.path,
            });
        }
    };

    Ok(GatewayRequest { metadata, command })
}

pub fn normalize_websocket(envelope: WebSocketGatewayEnvelope) -> GatewayResult<GatewayRequest> {
    let metadata = GatewayMetadata {
        request_id: envelope.request_id,
        transport: TransportKind::WebSocket,
    };

    let command = match envelope.event.as_str() {
        "query.text" => {
            parse_text_command(envelope.payload, "ws query.text", GatewayCommandKind::Query)?
        }
        "query.explain" => parse_text_command(
            envelope.payload,
            "ws query.explain",
            GatewayCommandKind::Explain,
        )?,
        "watch.start" => parse_text_command(
            envelope.payload,
            "ws watch.start",
            GatewayCommandKind::WatchStart,
        )?,
        "watch.poll" => parse_watch_poll(envelope.payload, "ws watch.poll")?,
        "watch.stop" => parse_watch_stop(envelope.payload, "ws watch.stop")?,
        "records.ingest" => parse_ingest(envelope.payload, "ws records.ingest")?,
        "records.delete" => parse_delete(envelope.payload, "ws records.delete")?,
        "streams.mutations.poll" => {
            parse_durable_mutation_poll(envelope.payload, "ws streams.mutations.poll")?
        }
        "streams.mutations.commit" => {
            parse_durable_mutation_commit(envelope.payload, "ws streams.mutations.commit")?
        }
        other => return Err(GatewayError::UnsupportedWebSocketEvent(other.to_string())),
    };

    Ok(GatewayRequest { metadata, command })
}

pub fn normalize_tcp(
    _session: &TcpGatewaySession,
    frame: TcpGatewayFrame,
) -> GatewayResult<GatewayRequest> {
    let metadata = GatewayMetadata {
        request_id: frame.request_id,
        transport: TransportKind::Tcp,
    };

    let payload_value: Value =
        serde_json::from_slice(&frame.payload).map_err(|source| GatewayError::InvalidPayload {
            context: format!("tcp opcode 0x{:02X}", frame.opcode),
            source,
        })?;

    let command = match frame.opcode {
        OPCODE_QUERY_TEXT => {
            parse_text_command(payload_value, "tcp query.text", GatewayCommandKind::Query)?
        }
        OPCODE_EXPLAIN_TEXT => parse_text_command(
            payload_value,
            "tcp query.explain",
            GatewayCommandKind::Explain,
        )?,
        OPCODE_WATCH_START_TEXT => parse_text_command(
            payload_value,
            "tcp watch.start",
            GatewayCommandKind::WatchStart,
        )?,
        OPCODE_WATCH_POLL => parse_watch_poll(payload_value, "tcp watch.poll")?,
        OPCODE_WATCH_STOP => parse_watch_stop(payload_value, "tcp watch.stop")?,
        OPCODE_INGEST => parse_ingest(payload_value, "tcp records.ingest")?,
        OPCODE_DELETE => parse_delete(payload_value, "tcp records.delete")?,
        OPCODE_DURABLE_MUTATION_POLL => {
            parse_durable_mutation_poll(payload_value, "tcp streams.mutations.poll")?
        }
        OPCODE_DURABLE_MUTATION_COMMIT => {
            parse_durable_mutation_commit(payload_value, "tcp streams.mutations.commit")?
        }
        other => return Err(GatewayError::UnsupportedTcpOpcode(other)),
    };

    Ok(GatewayRequest { metadata, command })
}

enum GatewayCommandKind {
    Query,
    Explain,
    WatchStart,
}

fn parse_text_command(
    payload: Value,
    context: &'static str,
    kind: GatewayCommandKind,
) -> GatewayResult<GatewayCommand> {
    let payload = from_value::<QueryTextPayload>(payload, context)?;
    let caller = payload.caller.unwrap_or_else(CallerContext::anonymous);
    let tenant_id = TenantId(payload.tenant_id);
    let compile = payload.compile.unwrap_or_default();

    let command = match kind {
        GatewayCommandKind::Query => GatewayCommand::QueryText {
            caller,
            tenant_id,
            query_text: payload.query_text,
            compile,
        },
        GatewayCommandKind::Explain => GatewayCommand::ExplainText {
            caller,
            tenant_id,
            query_text: payload.query_text,
            compile,
        },
        GatewayCommandKind::WatchStart => GatewayCommand::WatchStartText {
            caller,
            tenant_id,
            query_text: payload.query_text,
            compile,
        },
    };

    Ok(command)
}

fn parse_watch_poll(payload: Value, context: &'static str) -> GatewayResult<GatewayCommand> {
    let payload = from_value::<WatchPollPayload>(payload, context)?;
    Ok(GatewayCommand::WatchPoll {
        subscription_id: payload.subscription_id,
        max_events: payload.max_events.map(NonZeroUsize::get).unwrap_or(100),
    })
}

fn parse_watch_stop(payload: Value, context: &'static str) -> GatewayResult<GatewayCommand> {
    let payload = from_value::<WatchStopPayload>(payload, context)?;
    Ok(GatewayCommand::WatchStop {
        subscription_id: payload.subscription_id,
    })
}

fn parse_ingest(payload: Value, context: &'static str) -> GatewayResult<GatewayCommand> {
    let payload = from_value::<IngestPayload>(payload, context)?;
    Ok(GatewayCommand::Ingest {
        caller: payload.caller.unwrap_or_else(CallerContext::anonymous),
        records: payload.records,
    })
}

fn parse_delete(payload: Value, context: &'static str) -> GatewayResult<GatewayCommand> {
    let payload = from_value::<DeletePayload>(payload, context)?;
    Ok(GatewayCommand::Delete {
        caller: payload.caller.unwrap_or_else(CallerContext::anonymous),
        tenant_id: TenantId(payload.tenant_id),
        record_id: RecordId(payload.record_id),
    })
}

fn parse_durable_mutation_poll(
    payload: Value,
    context: &'static str,
) -> GatewayResult<GatewayCommand> {
    let payload = from_value::<DurableMutationPollPayload>(payload, context)?;
    Ok(GatewayCommand::DurableMutationPoll {
        caller: payload.caller.unwrap_or_else(CallerContext::anonymous),
        tenant_id: TenantId(payload.tenant_id),
        consumer_group: payload.consumer_group,
        max_events_per_partition: payload
            .max_events_per_partition
            .map(NonZeroUsize::get)
            .unwrap_or(100),
    })
}

fn parse_durable_mutation_commit(
    payload: Value,
    context: &'static str,
) -> GatewayResult<GatewayCommand> {
    let payload = from_value::<DurableMutationCommitPayload>(payload, context)?;
    Ok(GatewayCommand::DurableMutationCommit {
        caller: payload.caller.unwrap_or_else(CallerContext::anonymous),
        tenant_id: TenantId(payload.tenant_id),
        consumer_group: payload.consumer_group,
        offsets: payload
            .offsets
            .into_iter()
            .map(|offset| GatewayDurableMutationOffset {
                partition: offset.partition,
                committed_sequence: offset.committed_sequence,
            })
            .collect(),
    })
}

fn from_value<T>(value: Value, context: &'static str) -> GatewayResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(value).map_err(|source| GatewayError::InvalidPayload {
        context: context.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use idb_core::{CallerContext, FieldValue, RecordEnvelope, TenantId};
    use idb_wire::{ProtocolRange, ProtocolVersion, SessionHello};
    use serde_json::json;

    use crate::model::{GatewayCommand, TextQueryCompileConfig};

    use super::{
        establish_tcp_session, normalize_http, normalize_tcp, normalize_websocket, DeletePayload,
        DurableMutationCommitPayload, DurableMutationOffsetPayload, DurableMutationPollPayload,
        HttpGatewayEnvelope, IngestPayload, QueryTextPayload, TcpGatewayFrame, WatchPollPayload,
        WebSocketGatewayEnvelope, OPCODE_DURABLE_MUTATION_POLL, OPCODE_QUERY_TEXT,
    };

    fn sample_query_payload() -> QueryTextPayload {
        QueryTextPayload {
            caller: Some(CallerContext::service(
                "sdk",
                Some(TenantId("tenant_a".to_string())),
            )),
            tenant_id: "tenant_a".to_string(),
            query_text: "Product where price < 1000 | top(5)".to_string(),
            compile: Some(TextQueryCompileConfig {
                top_k_default: 50,
                semantic_embedding_field: "embed".to_string(),
                semantic_embedding_dims: 24,
            }),
        }
    }

    #[test]
    fn query_payload_normalizes_equivalently_across_transports() {
        let payload = sample_query_payload();

        let http = normalize_http(HttpGatewayEnvelope {
            request_id: "r1".to_string(),
            method: "POST".to_string(),
            path: "/v1/query/text".to_string(),
            body: serde_json::to_value(&payload).expect("payload"),
        })
        .expect("http normalize");

        let ws = normalize_websocket(WebSocketGatewayEnvelope {
            request_id: "r2".to_string(),
            event: "query.text".to_string(),
            payload: serde_json::to_value(&payload).expect("payload"),
        })
        .expect("ws normalize");

        let hello = SessionHello {
            client_name: "sdk-rust".to_string(),
            min_version: ProtocolVersion::new(1, 0),
            max_version: ProtocolVersion::new(1, 1),
        };
        let (tcp_session, _) = establish_tcp_session(
            &hello,
            ProtocolRange {
                min: ProtocolVersion::new(1, 0),
                max: ProtocolVersion::new(2, 0),
            },
        )
        .expect("tcp session");

        let tcp = normalize_tcp(
            &tcp_session,
            TcpGatewayFrame {
                request_id: "r3".to_string(),
                opcode: OPCODE_QUERY_TEXT,
                payload: serde_json::to_vec(&payload).expect("payload bytes"),
            },
        )
        .expect("tcp normalize");

        assert_eq!(http.command, ws.command);
        assert_eq!(ws.command, tcp.command);
    }

    #[test]
    fn unsupported_http_route_is_rejected() {
        let err = normalize_http(HttpGatewayEnvelope {
            request_id: "r1".to_string(),
            method: "GET".to_string(),
            path: "/v1/query/text".to_string(),
            body: json!({}),
        })
        .expect_err("must fail");

        let msg = err.to_string();
        assert!(msg.contains("unsupported HTTP route"));
    }

    #[test]
    fn tcp_payload_decode_error_is_reported() {
        let hello = SessionHello {
            client_name: "sdk-rust".to_string(),
            min_version: ProtocolVersion::new(1, 0),
            max_version: ProtocolVersion::new(1, 0),
        };
        let (tcp_session, _) = establish_tcp_session(
            &hello,
            ProtocolRange {
                min: ProtocolVersion::new(1, 0),
                max: ProtocolVersion::new(1, 0),
            },
        )
        .expect("session");

        let err = normalize_tcp(
            &tcp_session,
            TcpGatewayFrame {
                request_id: "r3".to_string(),
                opcode: OPCODE_QUERY_TEXT,
                payload: vec![0xFF, 0x00, 0x11],
            },
        )
        .expect_err("must fail");

        let msg = err.to_string();
        assert!(msg.contains("invalid payload"));
    }

    #[test]
    fn protocol_negotiation_error_is_surfaceable() {
        let hello = SessionHello {
            client_name: "sdk-rust".to_string(),
            min_version: ProtocolVersion::new(1, 0),
            max_version: ProtocolVersion::new(1, 2),
        };

        let err = establish_tcp_session(
            &hello,
            ProtocolRange {
                min: ProtocolVersion::new(2, 0),
                max: ProtocolVersion::new(2, 2),
            },
        )
        .expect_err("must fail");

        assert!(err.to_string().contains("protocol negotiation failed"));
    }

    #[test]
    fn ingest_and_delete_payloads_parse_for_transport_reuse() {
        let mut record = RecordEnvelope::new(12, "tenant_a", "Product");
        record
            .structured_fields
            .insert("price".to_string(), FieldValue::Float(120.0));

        let ingest_payload = IngestPayload {
            caller: Some(CallerContext::service(
                "ingest-worker",
                Some(TenantId("tenant_a".to_string())),
            )),
            records: vec![record],
        };
        let delete_payload = DeletePayload {
            caller: Some(CallerContext::service(
                "ingest-worker",
                Some(TenantId("tenant_a".to_string())),
            )),
            tenant_id: "tenant_a".to_string(),
            record_id: 12,
        };
        let poll_payload = WatchPollPayload {
            subscription_id: 88,
            max_events: NonZeroUsize::new(50),
        };

        let ingest = normalize_http(HttpGatewayEnvelope {
            request_id: "ingest-1".to_string(),
            method: "POST".to_string(),
            path: "/v1/records/ingest".to_string(),
            body: serde_json::to_value(ingest_payload).expect("ingest json"),
        })
        .expect("ingest normalized");

        let delete = normalize_websocket(WebSocketGatewayEnvelope {
            request_id: "delete-1".to_string(),
            event: "records.delete".to_string(),
            payload: serde_json::to_value(delete_payload).expect("delete json"),
        })
        .expect("delete normalized");

        let poll = normalize_http(HttpGatewayEnvelope {
            request_id: "poll-1".to_string(),
            method: "POST".to_string(),
            path: "/v1/watch/poll".to_string(),
            body: serde_json::to_value(poll_payload).expect("poll json"),
        })
        .expect("poll normalized");

        assert!(matches!(ingest.command, GatewayCommand::Ingest { .. }));
        assert!(matches!(delete.command, GatewayCommand::Delete { .. }));
        assert!(matches!(poll.command, GatewayCommand::WatchPoll { .. }));
    }

    #[test]
    fn durable_stream_payloads_parse_for_transport_reuse() {
        let caller =
            CallerContext::service("stream-worker", Some(TenantId("tenant_a".to_string())));
        let poll_payload = DurableMutationPollPayload {
            caller: Some(caller.clone()),
            tenant_id: "tenant_a".to_string(),
            consumer_group: "workers".to_string(),
            max_events_per_partition: NonZeroUsize::new(25),
        };
        let commit_payload = DurableMutationCommitPayload {
            caller: Some(caller.clone()),
            tenant_id: "tenant_a".to_string(),
            consumer_group: "workers".to_string(),
            offsets: vec![
                DurableMutationOffsetPayload {
                    partition: 0,
                    committed_sequence: 7,
                },
                DurableMutationOffsetPayload {
                    partition: 2,
                    committed_sequence: 11,
                },
            ],
        };

        let poll_http = normalize_http(HttpGatewayEnvelope {
            request_id: "durable-poll-http".to_string(),
            method: "POST".to_string(),
            path: "/v1/streams/mutations/poll".to_string(),
            body: serde_json::to_value(&poll_payload).expect("poll json"),
        })
        .expect("poll http normalized");

        let commit_ws = normalize_websocket(WebSocketGatewayEnvelope {
            request_id: "durable-commit-ws".to_string(),
            event: "streams.mutations.commit".to_string(),
            payload: serde_json::to_value(&commit_payload).expect("commit json"),
        })
        .expect("commit ws normalized");

        let hello = SessionHello {
            client_name: "sdk-rust".to_string(),
            min_version: ProtocolVersion::new(1, 0),
            max_version: ProtocolVersion::new(1, 1),
        };
        let (tcp_session, _) = establish_tcp_session(
            &hello,
            ProtocolRange {
                min: ProtocolVersion::new(1, 0),
                max: ProtocolVersion::new(2, 0),
            },
        )
        .expect("tcp session");
        let poll_tcp = normalize_tcp(
            &tcp_session,
            TcpGatewayFrame {
                request_id: "durable-poll-tcp".to_string(),
                opcode: OPCODE_DURABLE_MUTATION_POLL,
                payload: serde_json::to_vec(&poll_payload).expect("poll bytes"),
            },
        )
        .expect("poll tcp normalized");

        assert!(matches!(
            poll_http.command,
            GatewayCommand::DurableMutationPoll { .. }
        ));
        assert!(matches!(
            commit_ws.command,
            GatewayCommand::DurableMutationCommit { .. }
        ));
        assert!(matches!(
            poll_tcp.command,
            GatewayCommand::DurableMutationPoll { .. }
        ));

        let anonymous_poll = normalize_http(HttpGatewayEnvelope {
            request_id: "durable-poll-anon".to_string(),
            method: "POST".to_string(),
            path: "/v1/streams/mutations/poll".to_string(),
            body: json!({
                "tenant_id": "tenant_a",
                "consumer_group": "workers",
                "max_events_per_partition": 25
            }),
        })
        .expect("anonymous poll normalized");
        match anonymous_poll.command {
            GatewayCommand::DurableMutationPoll { caller, .. } => {
                assert_eq!(caller, CallerContext::anonymous())
            }
            other => panic!("expected durable mutation poll, got {other:?}"),
        }
    }

    #[test]
    fn watch_poll_rejects_zero_bound() {
        let err = normalize_http(HttpGatewayEnvelope {
            request_id: "watch-poll-zero".to_string(),
            method: "POST".to_string(),
            path: "/v1/watch/poll".to_string(),
            body: json!({
                "subscription_id": 88,
                "max_events": 0
            }),
        })
        .expect_err("zero max_events must fail");
        assert!(err.to_string().contains("invalid payload"));
    }

    #[test]
    fn durable_poll_rejects_zero_bound() {
        let err = normalize_http(HttpGatewayEnvelope {
            request_id: "durable-poll-zero".to_string(),
            method: "POST".to_string(),
            path: "/v1/streams/mutations/poll".to_string(),
            body: json!({
                "tenant_id": "tenant_a",
                "consumer_group": "workers",
                "max_events_per_partition": 0
            }),
        })
        .expect_err("zero max_events_per_partition must fail");
        assert!(err.to_string().contains("invalid payload"));
    }

    #[test]
    fn durable_poll_rejects_empty_consumer_group() {
        let err = normalize_http(HttpGatewayEnvelope {
            request_id: "durable-poll-empty-group".to_string(),
            method: "POST".to_string(),
            path: "/v1/streams/mutations/poll".to_string(),
            body: json!({
                "tenant_id": "tenant_a",
                "consumer_group": "   ",
                "max_events_per_partition": 10
            }),
        })
        .expect_err("empty consumer_group must fail");
        assert!(err.to_string().contains("invalid payload"));
    }

    #[test]
    fn durable_commit_rejects_empty_consumer_group() {
        let err = normalize_http(HttpGatewayEnvelope {
            request_id: "durable-commit-empty-group".to_string(),
            method: "POST".to_string(),
            path: "/v1/streams/mutations/commit".to_string(),
            body: json!({
                "tenant_id": "tenant_a",
                "consumer_group": "",
                "offsets": []
            }),
        })
        .expect_err("empty consumer_group must fail");
        assert!(err.to_string().contains("invalid payload"));
    }
}
