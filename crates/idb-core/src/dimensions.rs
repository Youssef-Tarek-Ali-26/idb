use serde::{Deserialize, Serialize};

use crate::errors::{CoreError, CoreResult};
use crate::types::{FieldValue, RecordEnvelope};

pub type CoordinateVector = Vec<u32>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DimensionSource {
    StructuredField(String),
    EmbeddingProjection {
        field: String,
        projection: ProjectionPolicy,
    },
    Derived(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectionPolicy {
    FirstN { index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DimensionType {
    Numeric,
    CategoricalOrdinal,
    Temporal,
    ProjectedFloat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissingValuePolicy {
    Zero,
    SkipRecord,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NormalizationPolicy {
    Identity,
    MinMax { min: f64, max: f64, bins: u32 },
    CategoryList { categories: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimensionDefinition {
    pub name: String,
    pub source: DimensionSource,
    pub dimension_type: DimensionType,
    pub normalization: NormalizationPolicy,
    pub missing_value: MissingValuePolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimensionRegistry {
    pub version: u32,
    pub dimensions: Vec<DimensionDefinition>,
}

impl DimensionRegistry {
    pub fn validate(&self) -> CoreResult<()> {
        for dimension in &self.dimensions {
            match &dimension.normalization {
                NormalizationPolicy::MinMax { min, max, bins } => {
                    if max <= min {
                        return Err(CoreError::InvalidDimension(format!(
                            "{} has max <= min",
                            dimension.name
                        )));
                    }
                    if *bins < 2 {
                        return Err(CoreError::InvalidDimension(format!(
                            "{} has bins < 2",
                            dimension.name
                        )));
                    }
                }
                NormalizationPolicy::CategoryList { categories } if categories.is_empty() => {
                    return Err(CoreError::InvalidDimension(format!(
                        "{} has empty category list",
                        dimension.name
                    )));
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn map_record(&self, record: &RecordEnvelope) -> CoreResult<CoordinateVector> {
        if record.dimension_version != self.version {
            return Err(CoreError::InvalidDimension(format!(
                "record dimension version {} does not match registry {}",
                record.dimension_version, self.version
            )));
        }

        let mut out = Vec::with_capacity(self.dimensions.len());
        for dimension in &self.dimensions {
            let value = self.resolve_dimension_value(record, dimension)?;
            let normalized = normalize_value(value, &dimension.normalization, &dimension.name)?;
            out.push(normalized);
        }
        Ok(out)
    }

    fn resolve_dimension_value(
        &self,
        record: &RecordEnvelope,
        dimension: &DimensionDefinition,
    ) -> CoreResult<FieldValue> {
        let value = match &dimension.source {
            DimensionSource::StructuredField(field) => record.structured_fields.get(field).cloned(),
            DimensionSource::EmbeddingProjection { field, projection } => {
                let embedding = record.embedding_fields.get(field);
                match (embedding, projection) {
                    (Some(embedding), ProjectionPolicy::FirstN { index }) => {
                        embedding.get(*index).map(|v| FieldValue::Float(*v as f64))
                    }
                    _ => None,
                }
            }
            DimensionSource::Derived(name) => {
                return Err(CoreError::InvalidDimension(format!(
                    "unsupported derived expression: {}",
                    name
                )))
            }
        };

        match value {
            Some(v) => Ok(v),
            None => match dimension.missing_value {
                MissingValuePolicy::Zero => Ok(FieldValue::Int(0)),
                MissingValuePolicy::SkipRecord => {
                    Err(CoreError::MissingField(dimension.name.clone()))
                }
                MissingValuePolicy::Error => Err(CoreError::MissingField(dimension.name.clone())),
            },
        }
    }
}

fn normalize_value(value: FieldValue, policy: &NormalizationPolicy, dim: &str) -> CoreResult<u32> {
    match policy {
        NormalizationPolicy::Identity => {
            let raw = value.as_f64().ok_or_else(|| CoreError::InvalidFieldType {
                field: dim.to_string(),
                expected: "numeric",
            })?;
            if raw < 0.0 {
                return Err(CoreError::Normalization(format!(
                    "{} identity normalization cannot encode negative value {}",
                    dim, raw
                )));
            }
            Ok(raw as u32)
        }
        NormalizationPolicy::MinMax { min, max, bins } => {
            let raw = value.as_f64().ok_or_else(|| CoreError::InvalidFieldType {
                field: dim.to_string(),
                expected: "numeric",
            })?;
            let clamped = raw.max(*min).min(*max);
            let normalized = (clamped - min) / (max - min);
            let idx = (normalized * (*bins as f64 - 1.0)).round();
            Ok(idx as u32)
        }
        NormalizationPolicy::CategoryList { categories } => match value {
            FieldValue::String(v) => {
                let pos = categories.iter().position(|c| c == &v).ok_or_else(|| {
                    CoreError::Normalization(format!("unknown category {v} for {dim}"))
                })?;
                Ok(pos as u32)
            }
            _ => Err(CoreError::InvalidFieldType {
                field: dim.to_string(),
                expected: "string",
            }),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use proptest::prelude::*;

    use super::*;
    use crate::types::RecordEnvelope;

    #[test]
    fn mapping_is_deterministic() {
        let registry = DimensionRegistry {
            version: 1,
            dimensions: vec![
                DimensionDefinition {
                    name: "price".to_string(),
                    source: DimensionSource::StructuredField("price".to_string()),
                    dimension_type: DimensionType::Numeric,
                    normalization: NormalizationPolicy::MinMax {
                        min: 0.0,
                        max: 10000.0,
                        bins: 32,
                    },
                    missing_value: MissingValuePolicy::Error,
                },
                DimensionDefinition {
                    name: "embed_0".to_string(),
                    source: DimensionSource::EmbeddingProjection {
                        field: "text_embedding".to_string(),
                        projection: ProjectionPolicy::FirstN { index: 0 },
                    },
                    dimension_type: DimensionType::ProjectedFloat,
                    normalization: NormalizationPolicy::MinMax {
                        min: -1.0,
                        max: 1.0,
                        bins: 32,
                    },
                    missing_value: MissingValuePolicy::Error,
                },
            ],
        };

        let mut record = RecordEnvelope::new(42, "tenant_a", "Product");
        record.dimension_version = 1;
        record.structured_fields =
            BTreeMap::from([("price".to_string(), FieldValue::Float(4500.0))]);
        record
            .embedding_fields
            .insert("text_embedding".to_string(), vec![0.5, -0.2, 0.1]);

        let first = registry.map_record(&record).expect("first mapping");
        let second = registry.map_record(&record).expect("second mapping");

        assert_eq!(first, second);
    }

    #[test]
    fn mapping_respects_minmax_bounds() {
        let registry = DimensionRegistry {
            version: 1,
            dimensions: vec![DimensionDefinition {
                name: "price".to_string(),
                source: DimensionSource::StructuredField("price".to_string()),
                dimension_type: DimensionType::Numeric,
                normalization: NormalizationPolicy::MinMax {
                    min: 0.0,
                    max: 1000.0,
                    bins: 32,
                },
                missing_value: MissingValuePolicy::Error,
            }],
        };

        let mut low = RecordEnvelope::new(1, "tenant_a", "Product");
        low.dimension_version = 1;
        low.structured_fields
            .insert("price".to_string(), FieldValue::Float(-100.0));

        let mut high = RecordEnvelope::new(2, "tenant_a", "Product");
        high.dimension_version = 1;
        high.structured_fields
            .insert("price".to_string(), FieldValue::Float(5000.0));

        let low_mapped = registry.map_record(&low).expect("low map");
        let high_mapped = registry.map_record(&high).expect("high map");
        assert_eq!(low_mapped[0], 0);
        assert_eq!(high_mapped[0], 31);
    }

    proptest! {
        #[test]
        fn deterministic_mapping_property(price in 0.0_f64..10000.0_f64, embed in -1.0_f32..1.0_f32) {
            let registry = DimensionRegistry {
                version: 1,
                dimensions: vec![
                    DimensionDefinition {
                        name: "price".to_string(),
                        source: DimensionSource::StructuredField("price".to_string()),
                        dimension_type: DimensionType::Numeric,
                        normalization: NormalizationPolicy::MinMax {
                            min: 0.0,
                            max: 10000.0,
                            bins: 32,
                        },
                        missing_value: MissingValuePolicy::Error,
                    },
                    DimensionDefinition {
                        name: "embed_0".to_string(),
                        source: DimensionSource::EmbeddingProjection {
                            field: "text_embedding".to_string(),
                            projection: ProjectionPolicy::FirstN { index: 0 },
                        },
                        dimension_type: DimensionType::ProjectedFloat,
                        normalization: NormalizationPolicy::MinMax {
                            min: -1.0,
                            max: 1.0,
                            bins: 32,
                        },
                        missing_value: MissingValuePolicy::Error,
                    },
                ],
            };

            let mut record = RecordEnvelope::new(42, "tenant_a", "Product");
            record.dimension_version = 1;
            record.structured_fields.insert("price".to_string(), FieldValue::Float(price));
            record.embedding_fields.insert("text_embedding".to_string(), vec![embed, 0.0, 0.1]);

            let a = registry.map_record(&record).expect("first mapping");
            let b = registry.map_record(&record).expect("second mapping");
            prop_assert_eq!(&a, &b);
            prop_assert!(a.iter().all(|v| *v < 32));
        }
    }
}
