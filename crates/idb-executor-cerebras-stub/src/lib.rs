use idb_core::{
    BackendCapabilities, CoreError, CoreResult, HydratedRecord, QueryRequest, RecordEnvelope,
    RecordId, ScoredRecord, StorageBackend, TenantId,
};
use serde::{Deserialize, Serialize};

pub const SUPPORTED_ENVELOPE_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelOperation {
    CandidateScan,
    ScoreAndRank,
    Hydrate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KernelInputEnvelope {
    pub version: u16,
    pub operation: KernelOperation,
    pub tenant_id: TenantId,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KernelScoredCandidate {
    pub record_id: RecordId,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KernelOutputEnvelope {
    pub version: u16,
    pub operation: KernelOperation,
    pub scored_candidates: Vec<KernelScoredCandidate>,
    pub host_hydration_required: bool,
}

#[derive(Debug, Default)]
pub struct CerebrasStubBackend;

impl CerebrasStubBackend {
    pub fn new() -> Self {
        Self
    }

    fn unsupported(op: &str) -> CoreError {
        CoreError::Storage(format!(
            "cerebras stub backend does not support operation: {}",
            op
        ))
    }

    pub fn validate_envelope(&self, envelope: &KernelInputEnvelope) -> CoreResult<()> {
        if envelope.version != SUPPORTED_ENVELOPE_VERSION {
            return Err(CoreError::Storage(format!(
                "unsupported kernel envelope version {}, expected {}",
                envelope.version, SUPPORTED_ENVELOPE_VERSION
            )));
        }
        Ok(())
    }

    pub fn dispatch_envelope(
        &self,
        envelope: &KernelInputEnvelope,
    ) -> CoreResult<KernelOutputEnvelope> {
        self.validate_envelope(envelope)?;

        match envelope.operation {
            // Stub contracts score/rank stage only. Hydration stays host-side.
            KernelOperation::ScoreAndRank => Ok(KernelOutputEnvelope {
                version: SUPPORTED_ENVELOPE_VERSION,
                operation: KernelOperation::ScoreAndRank,
                scored_candidates: Vec::new(),
                host_hydration_required: true,
            }),
            KernelOperation::CandidateScan => Err(Self::unsupported("CandidateScan")),
            KernelOperation::Hydrate => Err(Self::unsupported("Hydrate")),
        }
    }

    pub fn compare_with_cpu_oracle(
        &self,
        cpu: &[ScoredRecord],
        accel: &[KernelScoredCandidate],
        tolerance: f32,
    ) -> CoreResult<()> {
        if cpu.len() != accel.len() {
            return Err(CoreError::Storage(format!(
                "conformance mismatch: cpu {} rows vs accel {} rows",
                cpu.len(),
                accel.len()
            )));
        }

        for (cpu_row, accel_row) in cpu.iter().zip(accel.iter()) {
            if cpu_row.record_id != accel_row.record_id {
                return Err(CoreError::Storage(format!(
                    "conformance mismatch: record_id {} != {}",
                    cpu_row.record_id.0, accel_row.record_id.0
                )));
            }
            if (cpu_row.score - accel_row.score).abs() > tolerance {
                return Err(CoreError::Storage(format!(
                    "conformance mismatch on record {}: {} vs {} (tol {})",
                    cpu_row.record_id.0, cpu_row.score, accel_row.score, tolerance
                )));
            }
        }

        Ok(())
    }
}

impl StorageBackend for CerebrasStubBackend {
    fn ingest_batch(&mut self, _records: Vec<RecordEnvelope>) -> CoreResult<Vec<RecordId>> {
        Err(Self::unsupported("ingest_batch"))
    }

    fn delete_or_tombstone(
        &mut self,
        _tenant_id: &TenantId,
        _record_id: RecordId,
    ) -> CoreResult<bool> {
        Err(Self::unsupported("delete_or_tombstone"))
    }

    fn query_candidates(&self, _query: &QueryRequest) -> CoreResult<Vec<RecordId>> {
        Err(Self::unsupported("query_candidates"))
    }

    fn score_and_rank(
        &self,
        _query: &QueryRequest,
        _candidates: Vec<RecordId>,
    ) -> CoreResult<Vec<ScoredRecord>> {
        Err(Self::unsupported("score_and_rank"))
    }

    fn hydrate(
        &self,
        _tenant_id: &TenantId,
        _scored: Vec<ScoredRecord>,
    ) -> CoreResult<Vec<HydratedRecord>> {
        Err(Self::unsupported("hydrate"))
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_candidate_generation: false,
            supports_vector_scoring: false,
            supports_hydration: false,
            supports_mutations: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CerebrasStubBackend, KernelInputEnvelope, KernelOperation, KernelScoredCandidate,
        SUPPORTED_ENVELOPE_VERSION,
    };
    use idb_core::{RecordId, ScoredRecord, TenantId};

    #[test]
    fn rejects_unsupported_envelope_versions() {
        let backend = CerebrasStubBackend::new();
        let envelope = KernelInputEnvelope {
            version: 99,
            operation: KernelOperation::ScoreAndRank,
            tenant_id: TenantId("tenant_a".to_string()),
            payload: serde_json::json!({"k": 10}),
        };

        let err = backend
            .dispatch_envelope(&envelope)
            .expect_err("must reject unsupported version");
        assert!(err
            .to_string()
            .contains("unsupported kernel envelope version"));
    }

    #[test]
    fn score_and_rank_contract_requires_host_hydration() {
        let backend = CerebrasStubBackend::new();
        let envelope = KernelInputEnvelope {
            version: SUPPORTED_ENVELOPE_VERSION,
            operation: KernelOperation::ScoreAndRank,
            tenant_id: TenantId("tenant_a".to_string()),
            payload: serde_json::json!({"k": 10}),
        };

        let output = backend.dispatch_envelope(&envelope).expect("dispatch");
        assert_eq!(output.operation, KernelOperation::ScoreAndRank);
        assert!(output.host_hydration_required);
    }

    #[test]
    fn conformance_check_rejects_record_mismatch() {
        let backend = CerebrasStubBackend::new();
        let cpu = vec![ScoredRecord {
            record_id: RecordId(1),
            score: 0.9,
        }];
        let accel = vec![KernelScoredCandidate {
            record_id: RecordId(2),
            score: 0.9,
        }];

        let err = backend
            .compare_with_cpu_oracle(&cpu, &accel, 1e-6)
            .expect_err("must reject record mismatch");
        assert!(err.to_string().contains("conformance mismatch"));
    }
}
