use serde::{Deserialize, Serialize};

use crate::errors::{CoreError, CoreResult};
use crate::types::RecordEnvelope;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NSpacePoint {
    pub structured: Vec<f32>,
    pub semantic: Vec<f32>,
    pub topology: Vec<f32>,
}

impl NSpacePoint {
    pub fn total_dimensions(&self) -> usize {
        self.structured.len() + self.semantic.len() + self.topology.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NSpaceWeights {
    pub structured: f32,
    pub semantic: f32,
    pub topology: f32,
}

impl Default for NSpaceWeights {
    fn default() -> Self {
        Self {
            structured: 1.0,
            semantic: 1.0,
            topology: 1.0,
        }
    }
}

impl NSpaceWeights {
    pub fn validate(&self) -> CoreResult<()> {
        if self.structured < 0.0 || self.semantic < 0.0 || self.topology < 0.0 {
            return Err(CoreError::InvalidDimension(
                "n-space weights must be non-negative".to_string(),
            ));
        }
        if self.structured + self.semantic + self.topology <= 0.0 {
            return Err(CoreError::InvalidDimension(
                "n-space weights must have positive total".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructuredSimilarityPolicy {
    RelativeL1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticSimilarityPolicy {
    CosineToUnitInterval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologySimilarityPolicy {
    WeightedJaccard,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FusedSimilarityKernel {
    pub weights: NSpaceWeights,
    pub structured_policy: StructuredSimilarityPolicy,
    pub semantic_policy: SemanticSimilarityPolicy,
    pub topology_policy: TopologySimilarityPolicy,
}

impl Default for FusedSimilarityKernel {
    fn default() -> Self {
        Self {
            weights: NSpaceWeights::default(),
            structured_policy: StructuredSimilarityPolicy::RelativeL1,
            semantic_policy: SemanticSimilarityPolicy::CosineToUnitInterval,
            topology_policy: TopologySimilarityPolicy::WeightedJaccard,
        }
    }
}

impl FusedSimilarityKernel {
    pub fn validate(&self) -> CoreResult<()> {
        self.weights.validate()
    }

    pub fn similarity(&self, a: &NSpacePoint, b: &NSpacePoint) -> CoreResult<f32> {
        self.validate()?;
        validate_block_dims("structured", &a.structured, &b.structured)?;
        validate_block_dims("semantic", &a.semantic, &b.semantic)?;
        validate_block_dims("topology", &a.topology, &b.topology)?;

        let structured = match self.structured_policy {
            StructuredSimilarityPolicy::RelativeL1 => {
                relative_l1_similarity(&a.structured, &b.structured)
            }
        }?;
        let semantic = match self.semantic_policy {
            SemanticSimilarityPolicy::CosineToUnitInterval => {
                cosine_similarity(&a.semantic, &b.semantic)?
            }
        };
        let topology = match self.topology_policy {
            TopologySimilarityPolicy::WeightedJaccard => {
                weighted_jaccard_similarity(&a.topology, &b.topology)?
            }
        };

        let weighted = self.weights.structured * structured
            + self.weights.semantic * semantic
            + self.weights.topology * topology;
        let total = self.weights.structured + self.weights.semantic + self.weights.topology;
        Ok(clamp01(weighted / total))
    }

    pub fn distance(&self, a: &NSpacePoint, b: &NSpacePoint) -> CoreResult<f32> {
        Ok(1.0 - self.similarity(a, b)?)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NSpaceContract {
    pub version: u32,
    pub structured_dims: usize,
    pub semantic_dims: usize,
    pub topology_dims: usize,
    pub kernel: FusedSimilarityKernel,
}

impl NSpaceContract {
    pub fn validate(&self) -> CoreResult<()> {
        self.kernel.validate()?;
        if self.structured_dims + self.semantic_dims + self.topology_dims == 0 {
            return Err(CoreError::InvalidDimension(
                "n-space contract must define at least one dimension".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate_point(&self, point: &NSpacePoint) -> CoreResult<()> {
        if point.structured.len() != self.structured_dims {
            return Err(CoreError::InvalidDimension(format!(
                "structured dims mismatch: expected {}, got {}",
                self.structured_dims,
                point.structured.len()
            )));
        }
        if point.semantic.len() != self.semantic_dims {
            return Err(CoreError::InvalidDimension(format!(
                "semantic dims mismatch: expected {}, got {}",
                self.semantic_dims,
                point.semantic.len()
            )));
        }
        if point.topology.len() != self.topology_dims {
            return Err(CoreError::InvalidDimension(format!(
                "topology dims mismatch: expected {}, got {}",
                self.topology_dims,
                point.topology.len()
            )));
        }
        Ok(())
    }

    pub fn fused_similarity(&self, a: &NSpacePoint, b: &NSpacePoint) -> CoreResult<f32> {
        self.validate()?;
        self.validate_point(a)?;
        self.validate_point(b)?;
        self.kernel.similarity(a, b)
    }
}

pub trait NSpaceProjector {
    fn contract(&self) -> &NSpaceContract;
    fn project(&self, record: &RecordEnvelope) -> CoreResult<NSpacePoint>;
}

fn validate_block_dims(name: &str, a: &[f32], b: &[f32]) -> CoreResult<()> {
    if a.len() != b.len() {
        return Err(CoreError::InvalidDimension(format!(
            "n-space {name} block mismatch: {} vs {}",
            a.len(),
            b.len()
        )));
    }
    Ok(())
}

fn relative_l1_similarity(a: &[f32], b: &[f32]) -> CoreResult<f32> {
    validate_block_dims("structured", a, b)?;
    if a.is_empty() {
        return Ok(1.0);
    }

    let mut diff_sum = 0.0f32;
    let mut scale_sum = 0.0f32;
    for (av, bv) in a.iter().zip(b.iter()) {
        diff_sum += (*av - *bv).abs();
        scale_sum += av.abs().max(bv.abs());
    }

    if scale_sum <= f32::EPSILON {
        return Ok(1.0);
    }
    Ok(clamp01(1.0 - diff_sum / scale_sum))
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> CoreResult<f32> {
    validate_block_dims("semantic", a, b)?;
    if a.is_empty() {
        return Ok(1.0);
    }

    let mut dot = 0.0f32;
    let mut anorm = 0.0f32;
    let mut bnorm = 0.0f32;
    for (av, bv) in a.iter().zip(b.iter()) {
        dot += av * bv;
        anorm += av * av;
        bnorm += bv * bv;
    }

    if anorm <= f32::EPSILON && bnorm <= f32::EPSILON {
        return Ok(1.0);
    }
    if anorm <= f32::EPSILON || bnorm <= f32::EPSILON {
        return Ok(0.0);
    }

    let cosine = (dot / (anorm.sqrt() * bnorm.sqrt())).clamp(-1.0, 1.0);
    Ok(clamp01((cosine + 1.0) * 0.5))
}

fn weighted_jaccard_similarity(a: &[f32], b: &[f32]) -> CoreResult<f32> {
    validate_block_dims("topology", a, b)?;
    if a.is_empty() {
        return Ok(1.0);
    }

    let mut intersection = 0.0f32;
    let mut union = 0.0f32;
    for (av, bv) in a.iter().zip(b.iter()) {
        if *av < 0.0 || *bv < 0.0 {
            return Err(CoreError::Normalization(
                "topology vector must be non-negative for weighted jaccard".to_string(),
            ));
        }
        intersection += av.min(*bv);
        union += av.max(*bv);
    }

    if union <= f32::EPSILON {
        return Ok(1.0);
    }
    Ok(clamp01(intersection / union))
}

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_point() -> NSpacePoint {
        NSpacePoint {
            structured: vec![0.1, 0.9],
            semantic: vec![0.5, -0.25, 0.75],
            topology: vec![2.0, 0.0, 1.0],
        }
    }

    fn sample_contract() -> NSpaceContract {
        NSpaceContract {
            version: 1,
            structured_dims: 2,
            semantic_dims: 3,
            topology_dims: 3,
            kernel: FusedSimilarityKernel::default(),
        }
    }

    #[test]
    fn fused_similarity_is_maximal_for_identical_points() {
        let contract = sample_contract();
        let point = sample_point();
        let score = contract
            .fused_similarity(&point, &point)
            .expect("score identical points");
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn fused_similarity_is_symmetric() {
        let contract = sample_contract();
        let a = sample_point();
        let b = NSpacePoint {
            structured: vec![0.3, 0.6],
            semantic: vec![0.2, 0.1, -0.4],
            topology: vec![1.0, 1.0, 0.0],
        };

        let ab = contract.fused_similarity(&a, &b).expect("ab");
        let ba = contract.fused_similarity(&b, &a).expect("ba");
        assert!((ab - ba).abs() < 1e-6);
    }

    #[test]
    fn fused_similarity_is_bounded() {
        let contract = sample_contract();
        let a = sample_point();
        let b = NSpacePoint {
            structured: vec![10.0, 0.0],
            semantic: vec![-1.0, 0.0, 1.0],
            topology: vec![0.0, 5.0, 0.0],
        };

        let score = contract.fused_similarity(&a, &b).expect("score");
        assert!((0.0..=1.0).contains(&score));
    }

    #[test]
    fn contract_rejects_dimension_mismatch() {
        let contract = sample_contract();
        let a = sample_point();
        let bad = NSpacePoint {
            structured: vec![0.0],
            semantic: vec![0.0, 0.0, 0.0],
            topology: vec![0.0, 0.0, 0.0],
        };
        let err = contract
            .fused_similarity(&a, &bad)
            .expect_err("mismatch must fail");
        assert!(err.to_string().contains("structured dims mismatch"));
    }

    #[test]
    fn topology_similarity_rejects_negative_components() {
        let contract = sample_contract();
        let a = sample_point();
        let bad = NSpacePoint {
            structured: vec![0.1, 0.9],
            semantic: vec![0.5, -0.25, 0.75],
            topology: vec![1.0, -1.0, 0.0],
        };
        let err = contract
            .fused_similarity(&a, &bad)
            .expect_err("negative topology value must fail");
        assert!(err
            .to_string()
            .contains("topology vector must be non-negative"));
    }
}
