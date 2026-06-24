use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkloadClass {
    ReadHeavy,
    WriteHeavy,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkloadOperation {
    Insert,
    Update,
    Delete,
    StructuredQuery,
    HybridQuery,
    PointLookup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkCorpusSpec {
    pub name: String,
    pub entities: Vec<String>,
    pub total_records: u64,
    pub vector_dimension: usize,
    pub tenants: u32,
    pub workload_class: WorkloadClass,
    pub operation_mix: Vec<(WorkloadOperation, u8)>,
}

impl BenchmarkCorpusSpec {
    pub fn v0_default() -> Self {
        Self {
            name: "idb-v0-synthetic".to_string(),
            entities: vec![
                "Product".to_string(),
                "Order".to_string(),
                "Customer".to_string(),
            ],
            total_records: 1_000_000,
            vector_dimension: 384,
            tenants: 100,
            workload_class: WorkloadClass::Mixed,
            operation_mix: vec![
                (WorkloadOperation::Insert, 15),
                (WorkloadOperation::Update, 10),
                (WorkloadOperation::Delete, 5),
                (WorkloadOperation::StructuredQuery, 30),
                (WorkloadOperation::HybridQuery, 35),
                (WorkloadOperation::PointLookup, 5),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.total_records == 0 {
            return Err("total_records must be > 0".to_string());
        }
        if self.vector_dimension == 0 {
            return Err("vector_dimension must be > 0".to_string());
        }
        if self.tenants == 0 {
            return Err("tenants must be > 0".to_string());
        }

        let weight_sum: u32 = self.operation_mix.iter().map(|(_, w)| *w as u32).sum();
        if weight_sum != 100 {
            return Err(format!("operation mix must sum to 100, got {}", weight_sum));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BenchmarkCorpusSpec;

    #[test]
    fn default_corpus_is_valid() {
        let corpus = BenchmarkCorpusSpec::v0_default();
        corpus.validate().expect("default corpus must validate");
    }
}
