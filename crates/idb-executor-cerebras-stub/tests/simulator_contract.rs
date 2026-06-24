use idb_core::TenantId;
use idb_executor_cerebras_stub::{
    CerebrasStubBackend, KernelInputEnvelope, KernelOperation, SUPPORTED_ENVELOPE_VERSION,
};

fn envelope(op: KernelOperation) -> KernelInputEnvelope {
    KernelInputEnvelope {
        version: SUPPORTED_ENVELOPE_VERSION,
        operation: op,
        tenant_id: TenantId("tenant_a".to_string()),
        payload: serde_json::json!({"k": 10}),
    }
}

#[test]
fn simulator_contract_matrix_for_stub_backend() {
    let backend = CerebrasStubBackend::new();

    let score = backend.dispatch_envelope(&envelope(KernelOperation::ScoreAndRank));
    assert!(score.is_ok(), "score-and-rank envelope should validate");

    let scan = backend.dispatch_envelope(&envelope(KernelOperation::CandidateScan));
    assert!(scan.is_err(), "candidate scan is unsupported in stub");

    let hydrate = backend.dispatch_envelope(&envelope(KernelOperation::Hydrate));
    assert!(
        hydrate.is_err(),
        "hydrate stays host-side and is unsupported in stub"
    );
}
