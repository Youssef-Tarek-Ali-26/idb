use idb_core::{
    FieldValue, HybridScorePolicy, Predicate, PredicateOp, QueryOrderBy, QueryOrderDirection,
    QueryRequest, TenantId, VectorQuery,
};
use idb_parser::{
    parse_query, AggregateOp, Literal, OrderBy, Predicate as AstPredicate,
    PredicateOp as AstPredicateOp, QueryAst, QueryMode, QueryNode, SortDirection,
    TraversalDirection, TraversalStep,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanMode {
    Once,
    Watch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogicalPlan {
    pub mode: PlanMode,
    pub source: PlanSource,
    pub filters: Vec<FilterExpr>,
    pub transforms: Vec<TransformExpr>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanSource {
    EntityScan {
        entity: String,
    },
    Traversal {
        steps: Vec<PlanTraversalStep>,
        directions: Vec<PlanTraversalDirection>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanTraversalStep {
    EntityScan(String),
    EntityRef { entity: String, id: LiteralExpr },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanTraversalDirection {
    Outbound,
    Inbound,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterExpr {
    Field {
        field: String,
        op: ComparisonOp,
        value: LiteralExpr,
    },
    Range {
        field: String,
        start: LiteralExpr,
        end: LiteralExpr,
    },
    Semantic {
        query: String,
        threshold: Option<f64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralExpr {
    String(String),
    Number(f64),
    Bool(bool),
    Ident(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransformExpr {
    TopK {
        k: u64,
        order_by: Option<PlanOrderBy>,
    },
    Sort {
        field: String,
        direction: PlanSortDirection,
    },
    Group {
        field: String,
    },
    Aggregate {
        op: PlanAggregateOp,
        field: Option<String>,
    },
    Take {
        n: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanOrderBy {
    pub field: String,
    pub direction: PlanSortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanSortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanAggregateOp {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueryRequestBridgeOptions {
    pub tenant_id: TenantId,
    pub top_k_default: usize,
    pub score_policy: HybridScorePolicy,
    pub semantic_embedding_field: String,
    pub semantic_embedding_dims: usize,
}

impl QueryRequestBridgeOptions {
    pub fn for_tenant(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: TenantId(tenant_id.into()),
            top_k_default: 100,
            score_policy: HybridScorePolicy::default(),
            semantic_embedding_field: "text_embedding".to_string(),
            semantic_embedding_dims: 16,
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum PlannerError {
    #[error("parse failure: {0}")]
    Parse(String),
    #[error("unsupported plan feature: {0}")]
    Unsupported(String),
    #[error("invalid plan: {0}")]
    InvalidPlan(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryExplain {
    pub plan: LogicalPlan,
    pub summary: LogicalPlanSummary,
    pub request_projection: RequestProjectionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalPlanSummary {
    pub mode: PlanMode,
    pub source_kind: PlanSourceKind,
    pub filter_count: usize,
    pub transform_count: usize,
    pub has_semantic_filter: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanSourceKind {
    EntityScan,
    Traversal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RequestProjectionStatus {
    Supported(QueryRequest),
    Unsupported { reason: String },
}

pub fn plan_query_text(query: &str) -> Result<LogicalPlan, PlannerError> {
    let ast = parse_query(query).map_err(|e| PlannerError::Parse(e.to_string()))?;
    logical_plan_from_ast(&ast)
}

pub fn explain_query_text(
    query: &str,
    options: QueryRequestBridgeOptions,
) -> Result<QueryExplain, PlannerError> {
    let plan = plan_query_text(query)?;
    let summary = LogicalPlanSummary::from_plan(&plan);
    let request_projection = match logical_plan_to_query_request_for_execution(&plan, options) {
        Ok(request) => RequestProjectionStatus::Supported(request),
        Err(err) => RequestProjectionStatus::Unsupported {
            reason: err.to_string(),
        },
    };

    Ok(QueryExplain {
        plan,
        summary,
        request_projection,
    })
}

pub fn query_text_to_request(
    query: &str,
    options: QueryRequestBridgeOptions,
) -> Result<QueryRequest, PlannerError> {
    let plan = plan_query_text(query)?;
    logical_plan_to_query_request(&plan, options)
}

pub fn logical_plan_to_query_request_for_execution(
    plan: &LogicalPlan,
    options: QueryRequestBridgeOptions,
) -> Result<QueryRequest, PlannerError> {
    compile_query_request(plan, options, false)
}

pub fn logical_plan_from_ast(ast: &QueryAst) -> Result<LogicalPlan, PlannerError> {
    let (source_node, mut transforms) = split_pipeline(&ast.root);
    transforms.reverse();

    let (source, filters) = match source_node {
        QueryNode::EntityScan { entity, predicates } => (
            PlanSource::EntityScan {
                entity: entity.clone(),
            },
            predicates
                .iter()
                .map(filter_from_ast)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        QueryNode::Traversal {
            steps,
            directions,
            predicates,
        } => (
            PlanSource::Traversal {
                steps: steps
                    .iter()
                    .map(traversal_step_from_ast)
                    .collect::<Result<Vec<_>, _>>()?,
                directions: directions
                    .iter()
                    .map(|direction| match direction {
                        TraversalDirection::Outbound => Ok(PlanTraversalDirection::Outbound),
                        TraversalDirection::Inbound => Ok(PlanTraversalDirection::Inbound),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            },
            predicates
                .iter()
                .map(filter_from_ast)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        _ => {
            return Err(PlannerError::InvalidPlan(
                "unexpected non-source AST node after transform split".to_string(),
            ));
        }
    };

    let transforms = transforms
        .into_iter()
        .map(transform_from_ast)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LogicalPlan {
        mode: match ast.mode {
            QueryMode::Once => PlanMode::Once,
            QueryMode::Watch => PlanMode::Watch,
        },
        source,
        filters,
        transforms,
    })
}

pub fn logical_plan_to_query_request(
    plan: &LogicalPlan,
    options: QueryRequestBridgeOptions,
) -> Result<QueryRequest, PlannerError> {
    compile_query_request(plan, options, true)
}

fn compile_query_request(
    plan: &LogicalPlan,
    options: QueryRequestBridgeOptions,
    enforce_entity_scan_source: bool,
) -> Result<QueryRequest, PlannerError> {
    if matches!(plan.mode, PlanMode::Watch) {
        return Err(PlannerError::Unsupported(
            "watch mode requires changefeed orchestration and cannot map to one-shot QueryRequest"
                .to_string(),
        ));
    }

    if enforce_entity_scan_source && !matches!(plan.source, PlanSource::EntityScan { .. }) {
        return Err(PlannerError::Unsupported(
            "traversal sources are not in the CPU request bridge subset".to_string(),
        ));
    }

    let mut predicates = Vec::new();
    let mut semantic_filters: Vec<(String, Option<f64>)> = Vec::new();
    for filter in &plan.filters {
        match filter {
            FilterExpr::Field { field, op, value } => {
                predicates.push(Predicate {
                    field: field.clone(),
                    op: predicate_op_from_comparison(*op),
                    value: literal_to_field_value(value),
                });
            }
            FilterExpr::Range { field, start, end } => {
                predicates.push(Predicate {
                    field: field.clone(),
                    op: PredicateOp::Gte,
                    value: literal_to_field_value(start),
                });
                predicates.push(Predicate {
                    field: field.clone(),
                    op: PredicateOp::Lte,
                    value: literal_to_field_value(end),
                });
            }
            FilterExpr::Semantic { query, threshold } => {
                semantic_filters.push((query.clone(), *threshold));
            }
        }
    }

    let mut min_vector_score = None;
    let vector_query = if semantic_filters.is_empty() {
        None
    } else {
        let mut semantic_queries = Vec::with_capacity(semantic_filters.len());
        let mut strictest_threshold = f32::NEG_INFINITY;
        let mut has_threshold = false;

        for (semantic_query, threshold) in semantic_filters {
            semantic_queries.push(semantic_query);
            if let Some(threshold) = threshold {
                if !(-1.0..=1.0).contains(&threshold) {
                    return Err(PlannerError::InvalidPlan(format!(
                        "semantic threshold must be between -1.0 and 1.0, got {threshold}"
                    )));
                }
                has_threshold = true;
                strictest_threshold = strictest_threshold.max(threshold as f32);
            }
        }

        if has_threshold {
            min_vector_score = Some(strictest_threshold);
        }

        let vector =
            compile_semantic_query_vector(&semantic_queries, options.semantic_embedding_dims)?;
        Some(VectorQuery {
            field: options.semantic_embedding_field.clone(),
            vector,
        })
    };

    let mut top_k = options.top_k_default;
    let mut request_order_by = None;
    for transform in &plan.transforms {
        match transform {
            TransformExpr::TopK {
                k,
                order_by: top_order_by,
            } => {
                if let Some(order) = top_order_by {
                    let compiled = QueryOrderBy {
                        field: order.field.clone(),
                        direction: query_order_direction_from_plan(order.direction),
                    };
                    if let Some(existing) = &request_order_by {
                        if existing != &compiled {
                            return Err(PlannerError::Unsupported(
                                "conflicting top() order-by clauses are not supported".to_string(),
                            ));
                        }
                    }
                    request_order_by = Some(compiled);
                }
                let k_usize = usize::try_from(*k)
                    .map_err(|_| PlannerError::InvalidPlan(format!("top k out of range: {k}")))?;
                top_k = top_k.min(k_usize);
            }
            TransformExpr::Sort { field, direction } => {
                let compiled = QueryOrderBy {
                    field: field.clone(),
                    direction: query_order_direction_from_plan(*direction),
                };
                if let Some(existing) = &request_order_by {
                    if existing != &compiled {
                        return Err(PlannerError::Unsupported(
                            "conflicting ordering transforms are not supported".to_string(),
                        ));
                    }
                }
                request_order_by = Some(compiled);
            }
            TransformExpr::Take { n } => {
                let n_usize = usize::try_from(*n)
                    .map_err(|_| PlannerError::InvalidPlan(format!("take n out of range: {n}")))?;
                top_k = top_k.min(n_usize);
            }
            TransformExpr::Group { .. } | TransformExpr::Aggregate { .. } => {
                return Err(PlannerError::Unsupported(
                    "only sort/top/take transforms are currently supported in CPU request bridge"
                        .to_string(),
                ));
            }
        }
    }

    if top_k == 0 {
        return Err(PlannerError::InvalidPlan(
            "top_k resolved to 0, expected positive bound".to_string(),
        ));
    }

    Ok(QueryRequest {
        tenant_id: options.tenant_id,
        predicates,
        vector_query,
        min_vector_score,
        order_by: request_order_by,
        candidate_hint: None,
        top_k,
        score_policy: options.score_policy,
    })
}

pub fn deterministic_text_embedding(
    text: &str,
    dimensions: usize,
) -> Result<Vec<f32>, PlannerError> {
    if dimensions == 0 {
        return Err(PlannerError::InvalidPlan(
            "semantic embedding dimensions must be > 0".to_string(),
        ));
    }

    let mut embedding = vec![0.0f32; dimensions];
    let mut tokens = text
        .split_whitespace()
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    if tokens.is_empty() {
        tokens.push(text.to_ascii_lowercase());
    }

    for token in tokens {
        for (idx, slot) in embedding.iter_mut().enumerate() {
            let hash = stable_hash_with_salt(token.as_bytes(), idx as u64);
            *slot += hash_to_unit_float(hash);
        }
    }

    normalize_vector(&mut embedding);

    Ok(embedding)
}

pub fn compile_semantic_query_vector(
    semantic_queries: &[String],
    dimensions: usize,
) -> Result<Vec<f32>, PlannerError> {
    if semantic_queries.is_empty() {
        return Err(PlannerError::InvalidPlan(
            "semantic query list must not be empty".to_string(),
        ));
    }

    let mut centroid = vec![0.0f32; dimensions];
    for query in semantic_queries {
        let embedding = deterministic_text_embedding(query, dimensions)?;
        for (value, acc) in embedding.iter().zip(centroid.iter_mut()) {
            *acc += *value;
        }
    }

    normalize_vector(&mut centroid);
    Ok(centroid)
}

fn normalize_vector(vector: &mut [f32]) {
    let norm_sq = vector.iter().map(|v| v * v).sum::<f32>();
    if norm_sq > f32::EPSILON {
        let norm = norm_sq.sqrt();
        for slot in vector {
            *slot /= norm;
        }
    }
}

fn stable_hash_with_salt(bytes: &[u8], salt: u64) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0001_0000_01b3;

    let mut hash = FNV_OFFSET_BASIS ^ salt;
    for b in bytes {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn hash_to_unit_float(hash: u64) -> f32 {
    let unit = (hash as f64) / (u64::MAX as f64);
    (unit as f32 * 2.0) - 1.0
}

fn split_pipeline<'a>(node: &'a QueryNode) -> (&'a QueryNode, Vec<&'a QueryNode>) {
    let mut transforms = Vec::new();
    let mut current = node;

    loop {
        match current {
            QueryNode::TopK { source, .. }
            | QueryNode::Sort { source, .. }
            | QueryNode::Group { source, .. }
            | QueryNode::Aggregate { source, .. }
            | QueryNode::Take { source, .. } => {
                transforms.push(current);
                current = source;
            }
            _ => return (current, transforms),
        }
    }
}

fn transform_from_ast(node: &QueryNode) -> Result<TransformExpr, PlannerError> {
    match node {
        QueryNode::TopK { k, order_by, .. } => Ok(TransformExpr::TopK {
            k: *k,
            order_by: order_by.as_ref().map(order_by_from_ast),
        }),
        QueryNode::Sort {
            field, direction, ..
        } => Ok(TransformExpr::Sort {
            field: field.clone(),
            direction: sort_direction_from_ast(*direction),
        }),
        QueryNode::Group { field, .. } => Ok(TransformExpr::Group {
            field: field.clone(),
        }),
        QueryNode::Aggregate { op, field, .. } => Ok(TransformExpr::Aggregate {
            op: aggregate_op_from_ast(*op),
            field: field.clone(),
        }),
        QueryNode::Take { n, .. } => Ok(TransformExpr::Take { n: *n }),
        _ => Err(PlannerError::InvalidPlan(
            "attempted to convert non-transform node to transform".to_string(),
        )),
    }
}

fn order_by_from_ast(order_by: &OrderBy) -> PlanOrderBy {
    PlanOrderBy {
        field: order_by.field.clone(),
        direction: sort_direction_from_ast(order_by.direction),
    }
}

fn filter_from_ast(predicate: &AstPredicate) -> Result<FilterExpr, PlannerError> {
    match predicate {
        AstPredicate::Field { field, op, value } => Ok(FilterExpr::Field {
            field: field.clone(),
            op: comparison_from_ast(*op),
            value: literal_from_ast(value),
        }),
        AstPredicate::Range { field, start, end } => Ok(FilterExpr::Range {
            field: field.clone(),
            start: literal_from_ast(start),
            end: literal_from_ast(end),
        }),
        AstPredicate::Semantic { query, threshold } => Ok(FilterExpr::Semantic {
            query: query.clone(),
            threshold: *threshold,
        }),
    }
}

fn traversal_step_from_ast(step: &TraversalStep) -> Result<PlanTraversalStep, PlannerError> {
    match step {
        TraversalStep::EntityScan(entity) => Ok(PlanTraversalStep::EntityScan(entity.clone())),
        TraversalStep::EntityRef { entity, id } => Ok(PlanTraversalStep::EntityRef {
            entity: entity.clone(),
            id: literal_from_ast(id),
        }),
    }
}

fn comparison_from_ast(op: AstPredicateOp) -> ComparisonOp {
    match op {
        AstPredicateOp::Eq => ComparisonOp::Eq,
        AstPredicateOp::Ne => ComparisonOp::Ne,
        AstPredicateOp::Lt => ComparisonOp::Lt,
        AstPredicateOp::Lte => ComparisonOp::Lte,
        AstPredicateOp::Gt => ComparisonOp::Gt,
        AstPredicateOp::Gte => ComparisonOp::Gte,
    }
}

fn sort_direction_from_ast(direction: SortDirection) -> PlanSortDirection {
    match direction {
        SortDirection::Asc => PlanSortDirection::Asc,
        SortDirection::Desc => PlanSortDirection::Desc,
    }
}

fn aggregate_op_from_ast(op: AggregateOp) -> PlanAggregateOp {
    match op {
        AggregateOp::Count => PlanAggregateOp::Count,
        AggregateOp::Sum => PlanAggregateOp::Sum,
        AggregateOp::Avg => PlanAggregateOp::Avg,
        AggregateOp::Min => PlanAggregateOp::Min,
        AggregateOp::Max => PlanAggregateOp::Max,
    }
}

fn query_order_direction_from_plan(direction: PlanSortDirection) -> QueryOrderDirection {
    match direction {
        PlanSortDirection::Asc => QueryOrderDirection::Asc,
        PlanSortDirection::Desc => QueryOrderDirection::Desc,
    }
}

impl LogicalPlanSummary {
    fn from_plan(plan: &LogicalPlan) -> Self {
        let source_kind = match plan.source {
            PlanSource::EntityScan { .. } => PlanSourceKind::EntityScan,
            PlanSource::Traversal { .. } => PlanSourceKind::Traversal,
        };

        Self {
            mode: plan.mode.clone(),
            source_kind,
            filter_count: plan.filters.len(),
            transform_count: plan.transforms.len(),
            has_semantic_filter: plan
                .filters
                .iter()
                .any(|filter| matches!(filter, FilterExpr::Semantic { .. })),
        }
    }
}

fn literal_from_ast(literal: &Literal) -> LiteralExpr {
    match literal {
        Literal::String(v) => LiteralExpr::String(v.clone()),
        Literal::Number(v) => LiteralExpr::Number(*v),
        Literal::Bool(v) => LiteralExpr::Bool(*v),
        Literal::Ident(v) => LiteralExpr::Ident(v.clone()),
    }
}

fn literal_to_field_value(literal: &LiteralExpr) -> FieldValue {
    match literal {
        LiteralExpr::String(v) => FieldValue::String(v.clone()),
        LiteralExpr::Bool(v) => FieldValue::Bool(*v),
        LiteralExpr::Number(v) => {
            if v.fract() == 0.0 && *v >= i64::MIN as f64 && *v <= i64::MAX as f64 {
                FieldValue::Int(*v as i64)
            } else {
                FieldValue::Float(*v)
            }
        }
        LiteralExpr::Ident(v) => FieldValue::String(v.clone()),
    }
}

fn predicate_op_from_comparison(op: ComparisonOp) -> PredicateOp {
    match op {
        ComparisonOp::Eq => PredicateOp::Eq,
        ComparisonOp::Ne => PredicateOp::Ne,
        ComparisonOp::Lt => PredicateOp::Lt,
        ComparisonOp::Lte => PredicateOp::Lte,
        ComparisonOp::Gt => PredicateOp::Gt,
        ComparisonOp::Gte => PredicateOp::Gte,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compile_semantic_query_vector, deterministic_text_embedding, explain_query_text,
        logical_plan_from_ast, logical_plan_to_query_request,
        logical_plan_to_query_request_for_execution, parse_query, plan_query_text,
        query_text_to_request, FilterExpr, PlanMode, PlanSource, PlanSourceKind,
        QueryRequestBridgeOptions, RequestProjectionStatus, TransformExpr,
    };
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct ConformanceFile {
        cases: Vec<ConformanceCase>,
    }

    #[derive(Debug, Deserialize)]
    struct ConformanceCase {
        id: String,
        query: String,
        expect: ConformanceExpect,
    }

    #[derive(Debug, Deserialize)]
    struct ConformanceExpect {
        root: String,
        mode: String,
        predicates: Option<usize>,
        semantic: Option<bool>,
        hops: Option<usize>,
    }

    fn artifacts_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../openspec/changes/finalize-query-language-v0/artifacts")
    }

    #[test]
    fn planner_matches_open_spec_conformance_cases() {
        let file = std::fs::read_to_string(artifacts_root().join("conformance-cases.yaml"))
            .expect("read conformance file");
        let conformance: ConformanceFile =
            serde_yaml::from_str(&file).expect("parse conformance yaml");

        for case in conformance.cases {
            let ast = parse_query(&case.query)
                .unwrap_or_else(|e| panic!("case {} parse failed: {e}", case.id));
            let plan = logical_plan_from_ast(&ast)
                .unwrap_or_else(|e| panic!("case {} planning failed: {e}", case.id));

            match case.expect.mode.as_str() {
                "once" => assert!(
                    matches!(plan.mode, PlanMode::Once),
                    "mode mismatch for {}",
                    case.id
                ),
                "watch" => assert!(
                    matches!(plan.mode, PlanMode::Watch),
                    "mode mismatch for {}",
                    case.id
                ),
                other => panic!("unknown mode in fixture for {}: {other}", case.id),
            }

            if let Some(predicates) = case.expect.predicates {
                assert_eq!(
                    plan.filters.len(),
                    predicates,
                    "predicate count mismatch for {}",
                    case.id
                );
            }

            if let Some(semantic) = case.expect.semantic {
                let has_semantic = plan
                    .filters
                    .iter()
                    .any(|f| matches!(f, FilterExpr::Semantic { .. }));
                assert_eq!(has_semantic, semantic, "semantic mismatch for {}", case.id);
            }

            if let Some(hops) = case.expect.hops {
                match &plan.source {
                    PlanSource::Traversal { directions, .. } => {
                        assert_eq!(directions.len(), hops, "hop mismatch for {}", case.id)
                    }
                    _ => panic!("expected traversal source for {}", case.id),
                }
            }

            match case.expect.root.as_str() {
                "EntityScan" => assert!(
                    matches!(plan.source, PlanSource::EntityScan { .. }),
                    "root mismatch for {}",
                    case.id
                ),
                "Traversal" => assert!(
                    matches!(plan.source, PlanSource::Traversal { .. }),
                    "root mismatch for {}",
                    case.id
                ),
                "TopK" => assert!(
                    matches!(plan.transforms.last(), Some(TransformExpr::TopK { .. })),
                    "root mismatch for {}",
                    case.id
                ),
                other => panic!("unsupported root in fixture for {}: {other}", case.id),
            }
        }
    }

    #[test]
    fn transform_stage_order_is_source_to_sink() {
        let plan = plan_query_text("Product where price < 3000 | sort(price desc) | top(3)")
            .expect("plan query");

        assert_eq!(plan.transforms.len(), 2);
        assert!(matches!(plan.transforms[0], TransformExpr::Sort { .. }));
        assert!(matches!(plan.transforms[1], TransformExpr::TopK { .. }));
    }

    #[test]
    fn bridge_maps_supported_scan_filter_top_subset() {
        let plan = plan_query_text("Product where price < 1000 and status == active | top(4)")
            .expect("plan query");

        let request =
            logical_plan_to_query_request(&plan, QueryRequestBridgeOptions::for_tenant("tenant_a"))
                .expect("bridge request");

        assert_eq!(request.top_k, 4);
        assert_eq!(request.predicates.len(), 2);
        assert_eq!(request.tenant_id.0, "tenant_a");
        assert!(request.order_by.is_none());
    }

    #[test]
    fn bridge_compiles_topk_ordering() {
        let request = query_text_to_request(
            "Product where price < 1000 | top(4, price desc)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect("ordered top should compile");

        let order_by = request.order_by.expect("order_by");
        assert_eq!(order_by.field, "price");
        assert!(matches!(
            order_by.direction,
            idb_core::QueryOrderDirection::Desc
        ));
    }

    #[test]
    fn bridge_compiles_sort_take_ordering() {
        let request = query_text_to_request(
            "Product where price < 1000 | sort(price desc) | take(2)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect("sort+take should compile");

        assert_eq!(request.top_k, 2);
        let order_by = request.order_by.expect("order_by");
        assert_eq!(order_by.field, "price");
        assert!(matches!(
            order_by.direction,
            idb_core::QueryOrderDirection::Desc
        ));
    }

    #[test]
    fn execution_projection_supports_traversal_source() {
        let plan = plan_query_text("Brand(\"Norn Gold\") -> Product where price < 3000 | top(5)")
            .expect("plan traversal");

        let request = logical_plan_to_query_request_for_execution(
            &plan,
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect("execution projection should compile traversal");
        assert_eq!(request.top_k, 5);
        assert_eq!(request.predicates.len(), 1);

        let strict =
            logical_plan_to_query_request(&plan, QueryRequestBridgeOptions::for_tenant("tenant_a"))
                .expect_err("strict bridge should keep rejecting traversal source");
        assert!(strict.to_string().contains("traversal sources"));
    }

    #[test]
    fn bridge_compiles_single_semantic_filter_to_vector_query() {
        let request = query_text_to_request(
            "Product where meaning(\"trending shoes\") | top(4)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect("semantic query should compile");

        assert_eq!(request.top_k, 4);
        assert!(request.predicates.is_empty());
        let vector_query = request.vector_query.expect("vector query");
        assert_eq!(vector_query.field, "text_embedding");
        assert_eq!(vector_query.vector.len(), 16);
    }

    #[test]
    fn bridge_rejects_watch_mode() {
        let watch_err = query_text_to_request(
            "watch Product where price < 1000 | top(4)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect_err("watch should be unsupported");
        assert!(watch_err.to_string().contains("watch mode"));
    }

    #[test]
    fn bridge_compiles_semantic_threshold_and_rejects_invalid_bounds() {
        let request = query_text_to_request(
            "Product where meaning(\"trending\", threshold=0.8) | top(4)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect("threshold semantic query should compile");

        assert_eq!(request.min_vector_score, Some(0.8));
        assert!(request.vector_query.is_some());

        let invalid = query_text_to_request(
            "Product where meaning(\"trending\", threshold=1.5) | top(4)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect_err("invalid threshold should fail");
        assert!(invalid.to_string().contains("between -1.0 and 1.0"));
    }

    #[test]
    fn bridge_compiles_multi_semantic_centroid() {
        let request = query_text_to_request(
            "Product where meaning(\"a\") and meaning(\"b\") | top(4)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect("multi semantic query should compile");

        assert!(request.vector_query.is_some());
        assert_eq!(request.min_vector_score, None);
    }

    #[test]
    fn bridge_compiles_multi_semantic_strictest_threshold() {
        let request = query_text_to_request(
            "Product where meaning(\"a\", threshold=0.1) and meaning(\"b\", threshold=0.8) | top(4)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect("multi semantic threshold query should compile");

        assert_eq!(request.min_vector_score, Some(0.8));
    }

    #[test]
    fn bridge_rejects_conflicting_sort_and_top_ordering() {
        let conflict = query_text_to_request(
            "Product | sort(price asc) | top(2, price desc)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect_err("conflicting ordering should fail");
        let message = conflict.to_string();
        assert!(
            message.contains("conflicting ordering transforms")
                || message.contains("conflicting top() order-by clauses")
        );
    }

    #[test]
    fn deterministic_embeddings_are_stable_and_normalized() {
        let a = deterministic_text_embedding("hello world", 8).expect("embed");
        let b = deterministic_text_embedding("hello world", 8).expect("embed");
        let c = deterministic_text_embedding("hello world!", 8).expect("embed");

        assert_eq!(a, b);
        assert_ne!(a, c);

        let norm = a.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4);
    }

    #[test]
    fn semantic_centroid_vector_is_stable_and_normalized() {
        let queries = vec!["trending".to_string(), "sport".to_string()];
        let a = compile_semantic_query_vector(&queries, 8).expect("centroid");
        let b = compile_semantic_query_vector(&queries, 8).expect("centroid");
        assert_eq!(a, b);

        let norm = a.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4);
    }

    #[test]
    fn explain_reports_supported_projection() {
        let explain = explain_query_text(
            "Product where price < 1000 | top(4, price desc)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect("explain should succeed");

        assert_eq!(explain.summary.filter_count, 1);
        assert_eq!(explain.summary.transform_count, 1);
        assert!(matches!(
            explain.summary.source_kind,
            PlanSourceKind::EntityScan
        ));

        match explain.request_projection {
            RequestProjectionStatus::Supported(request) => {
                assert_eq!(request.top_k, 4);
                assert_eq!(request.order_by.expect("order").field, "price");
            }
            RequestProjectionStatus::Unsupported { reason } => {
                panic!("expected supported projection, got unsupported: {reason}")
            }
        }
    }

    #[test]
    fn explain_reports_unsupported_projection_reason() {
        let explain = explain_query_text(
            "watch Product where price < 1000 | top(4)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect("plan-level explain should still succeed");

        match explain.request_projection {
            RequestProjectionStatus::Supported(_) => {
                panic!("expected unsupported projection")
            }
            RequestProjectionStatus::Unsupported { reason } => {
                assert!(reason.contains("watch mode"));
            }
        }
    }

    #[test]
    fn explain_reports_supported_projection_for_traversal() {
        let explain = explain_query_text(
            "Brand(\"Norn Gold\") -> Product where price < 3000 | top(5)",
            QueryRequestBridgeOptions::for_tenant("tenant_a"),
        )
        .expect("explain should succeed");

        assert!(matches!(
            explain.summary.source_kind,
            PlanSourceKind::Traversal
        ));
        match explain.request_projection {
            RequestProjectionStatus::Supported(request) => {
                assert_eq!(request.top_k, 5);
                assert_eq!(request.predicates.len(), 1);
            }
            RequestProjectionStatus::Unsupported { reason } => {
                panic!("expected traversal explain to be supported, got: {reason}")
            }
        }
    }
}
