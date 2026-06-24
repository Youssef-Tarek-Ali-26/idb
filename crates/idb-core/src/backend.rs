use serde::{Deserialize, Serialize};

use crate::errors::CoreResult;
use crate::query::{HydratedRecord, QueryRequest, ScoredRecord};
use crate::types::{RecordEnvelope, RecordId, TenantId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendCapabilities {
    pub supports_candidate_generation: bool,
    pub supports_vector_scoring: bool,
    pub supports_hydration: bool,
    pub supports_mutations: bool,
}

impl BackendCapabilities {
    pub fn cpu_reference() -> Self {
        Self {
            supports_candidate_generation: true,
            supports_vector_scoring: true,
            supports_hydration: true,
            supports_mutations: true,
        }
    }
}

pub trait StorageBackend {
    fn ingest_batch(&mut self, records: Vec<RecordEnvelope>) -> CoreResult<Vec<RecordId>>;

    fn delete_or_tombstone(
        &mut self,
        tenant_id: &TenantId,
        record_id: RecordId,
    ) -> CoreResult<bool>;

    fn query_candidates(&self, query: &QueryRequest) -> CoreResult<Vec<RecordId>>;

    fn score_and_rank(
        &self,
        query: &QueryRequest,
        candidates: Vec<RecordId>,
    ) -> CoreResult<Vec<ScoredRecord>>;

    fn hydrate(
        &self,
        tenant_id: &TenantId,
        scored: Vec<ScoredRecord>,
    ) -> CoreResult<Vec<HydratedRecord>>;

    fn capabilities(&self) -> BackendCapabilities;
}

#[derive(Debug)]
pub struct FallbackBackend<P, F> {
    preferred: P,
    fallback: F,
}

impl<P, F> FallbackBackend<P, F> {
    pub fn new(preferred: P, fallback: F) -> Self {
        Self {
            preferred,
            fallback,
        }
    }

    pub fn preferred(&self) -> &P {
        &self.preferred
    }

    pub fn fallback(&self) -> &F {
        &self.fallback
    }
}

impl<P, F> StorageBackend for FallbackBackend<P, F>
where
    P: StorageBackend,
    F: StorageBackend,
{
    fn ingest_batch(&mut self, records: Vec<RecordEnvelope>) -> CoreResult<Vec<RecordId>> {
        if self.preferred.capabilities().supports_mutations {
            return self.preferred.ingest_batch(records);
        }
        self.fallback.ingest_batch(records)
    }

    fn delete_or_tombstone(
        &mut self,
        tenant_id: &TenantId,
        record_id: RecordId,
    ) -> CoreResult<bool> {
        if self.preferred.capabilities().supports_mutations {
            return self.preferred.delete_or_tombstone(tenant_id, record_id);
        }
        self.fallback.delete_or_tombstone(tenant_id, record_id)
    }

    fn query_candidates(&self, query: &QueryRequest) -> CoreResult<Vec<RecordId>> {
        if self.preferred.capabilities().supports_candidate_generation {
            return self.preferred.query_candidates(query);
        }
        self.fallback.query_candidates(query)
    }

    fn score_and_rank(
        &self,
        query: &QueryRequest,
        candidates: Vec<RecordId>,
    ) -> CoreResult<Vec<ScoredRecord>> {
        if self.preferred.capabilities().supports_vector_scoring {
            return self.preferred.score_and_rank(query, candidates);
        }
        self.fallback.score_and_rank(query, candidates)
    }

    fn hydrate(
        &self,
        tenant_id: &TenantId,
        scored: Vec<ScoredRecord>,
    ) -> CoreResult<Vec<HydratedRecord>> {
        if self.preferred.capabilities().supports_hydration {
            return self.preferred.hydrate(tenant_id, scored);
        }
        self.fallback.hydrate(tenant_id, scored)
    }

    fn capabilities(&self) -> BackendCapabilities {
        self.fallback.capabilities()
    }
}
