use std::cmp::Ordering;
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{FieldValue, RecordEnvelope, RecordId, TenantId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryRequest {
    pub tenant_id: TenantId,
    pub predicates: Vec<Predicate>,
    pub vector_query: Option<VectorQuery>,
    pub min_vector_score: Option<f32>,
    pub order_by: Option<QueryOrderBy>,
    pub candidate_hint: Option<CandidateGenerationHint>,
    pub top_k: usize,
    pub score_policy: HybridScorePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryOrderBy {
    pub field: String,
    pub direction: QueryOrderDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryOrderDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateGenerationHint {
    pub key_ranges: Vec<KeyRange>,
    pub ann_probe: Option<AnnProbeHint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyRange {
    pub min: u128,
    pub max: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnProbeHint {
    pub field: String,
    pub probe_factor: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VectorQuery {
    pub field: String,
    pub vector: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridScorePolicy {
    pub policy_version: u32,
    pub structured_weight: f32,
    pub vector_weight: f32,
}

impl Default for HybridScorePolicy {
    fn default() -> Self {
        Self {
            policy_version: 1,
            structured_weight: 0.5,
            vector_weight: 0.5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Predicate {
    pub field: String,
    pub op: PredicateOp,
    pub value: FieldValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredicateOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredRecord {
    pub record_id: RecordId,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HydratedRecord {
    pub envelope: RecordEnvelope,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageType {
    CandidateGeneration,
    ScoringAndRanking,
    Hydration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageTrace {
    pub stage: StageType,
    pub input_count: usize,
    pub output_count: usize,
    pub elapsed_micros: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryTrace {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub stages: Vec<StageTrace>,
    pub score_policy_version: u32,
}

pub fn record_matches_predicates(record: &RecordEnvelope, predicates: &[Predicate]) -> bool {
    fields_match_predicates(&record.structured_fields, predicates)
}

pub fn fields_match_predicates(
    fields: &BTreeMap<String, FieldValue>,
    predicates: &[Predicate],
) -> bool {
    predicates
        .iter()
        .all(|predicate| match fields.get(&predicate.field) {
            Some(value) => compare_values(value, &predicate.op, &predicate.value),
            None => false,
        })
}

pub fn compare_field_values(left: &FieldValue, right: &FieldValue) -> Option<Ordering> {
    use FieldValue::*;
    match (left, right) {
        (Int(a), Int(b)) => Some(a.cmp(b)),
        (Float(a), Float(b)) => a.partial_cmp(b),
        (Int(a), Float(b)) => (*a as f64).partial_cmp(b),
        (Float(a), Int(b)) => a.partial_cmp(&(*b as f64)),
        (String(a), String(b)) => Some(a.cmp(b)),
        (Bool(a), Bool(b)) => Some(a.cmp(b)),
        (Timestamp(a), Timestamp(b)) => Some(a.cmp(b)),
        _ => None,
    }
}

fn compare_values(left: &FieldValue, op: &PredicateOp, right: &FieldValue) -> bool {
    use FieldValue::*;
    match (left, right) {
        (Int(a), Int(b)) => compare_f64(*a as f64, op, *b as f64),
        (Float(a), Float(b)) => compare_f64(*a, op, *b),
        (Int(a), Float(b)) => compare_f64(*a as f64, op, *b),
        (Float(a), Int(b)) => compare_f64(*a, op, *b as f64),
        (String(a), String(b)) => compare_ord(a, op, b),
        (Bool(a), Bool(b)) => compare_ord(a, op, b),
        (Timestamp(a), Timestamp(b)) => compare_ord(a, op, b),
        _ => false,
    }
}

fn compare_f64(left: f64, op: &PredicateOp, right: f64) -> bool {
    match op {
        PredicateOp::Eq => (left - right).abs() < f64::EPSILON,
        PredicateOp::Ne => (left - right).abs() >= f64::EPSILON,
        PredicateOp::Lt => left < right,
        PredicateOp::Lte => left <= right,
        PredicateOp::Gt => left > right,
        PredicateOp::Gte => left >= right,
    }
}

fn compare_ord<T: PartialOrd + PartialEq>(left: &T, op: &PredicateOp, right: &T) -> bool {
    match op {
        PredicateOp::Eq => left == right,
        PredicateOp::Ne => left != right,
        PredicateOp::Lt => left < right,
        PredicateOp::Lte => left <= right,
        PredicateOp::Gt => left > right,
        PredicateOp::Gte => left >= right,
    }
}
