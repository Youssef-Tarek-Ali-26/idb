pub mod auth;
pub mod backend;
pub mod benchmark;
pub mod dimensions;
pub mod errors;
pub mod keyspace;
pub mod metrics;
pub mod nspace;
pub mod page;
pub mod query;
pub mod types;

pub use auth::{
    AllowAllAuthorizationProvider, AuthAction, AuthRuntime, AuthorizationDecision,
    AuthorizationProvider, AuthorizationRequest, CallerContext, PrincipalKind,
};
pub use backend::{BackendCapabilities, FallbackBackend, StorageBackend};
pub use benchmark::{BenchmarkCorpusSpec, WorkloadClass, WorkloadOperation};
pub use dimensions::{
    CoordinateVector, DimensionDefinition, DimensionRegistry, DimensionSource, DimensionType,
    MissingValuePolicy, NormalizationPolicy, ProjectionPolicy,
};
pub use errors::{CoreError, CoreResult};
pub use keyspace::{InterleavedKeyMapper, SpaceKeyMapper};
pub use metrics::{EngineMetrics, LatencyStats};
pub use nspace::{
    FusedSimilarityKernel, NSpaceContract, NSpacePoint, NSpaceProjector, NSpaceWeights,
    SemanticSimilarityPolicy, StructuredSimilarityPolicy, TopologySimilarityPolicy,
};
pub use page::{PageConfig, PageMetadata};
pub use query::{
    compare_field_values, fields_match_predicates, record_matches_predicates, AnnProbeHint,
    CandidateGenerationHint, HybridScorePolicy, HydratedRecord, KeyRange, Predicate, PredicateOp,
    QueryOrderBy, QueryOrderDirection, QueryRequest, QueryTrace, ScoredRecord, StageTrace,
    StageType, VectorQuery,
};
pub use types::{BlobRef, EdgeRef, EntityType, FieldValue, RecordEnvelope, RecordId, TenantId};
