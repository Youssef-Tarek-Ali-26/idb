use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryMode {
    Once,
    Watch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryAst {
    pub mode: QueryMode,
    pub root: QueryNode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryNode {
    EntityScan {
        entity: String,
        predicates: Vec<Predicate>,
    },
    Traversal {
        steps: Vec<TraversalStep>,
        directions: Vec<TraversalDirection>,
        predicates: Vec<Predicate>,
    },
    TopK {
        k: u64,
        order_by: Option<OrderBy>,
        source: Box<QueryNode>,
    },
    Sort {
        field: String,
        direction: SortDirection,
        source: Box<QueryNode>,
    },
    Group {
        field: String,
        source: Box<QueryNode>,
    },
    Aggregate {
        op: AggregateOp,
        field: Option<String>,
        source: Box<QueryNode>,
    },
    Take {
        n: u64,
        source: Box<QueryNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TraversalStep {
    EntityScan(String),
    EntityRef { entity: String, id: Literal },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraversalDirection {
    Outbound,
    Inbound,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Predicate {
    Field {
        field: String,
        op: PredicateOp,
        value: Literal,
    },
    Range {
        field: String,
        start: Literal,
        end: Literal,
    },
    Semantic {
        query: String,
        threshold: Option<f64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredicateOp {
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    String(String),
    Number(f64),
    Bool(bool),
    Ident(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderBy {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregateOp {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl QueryAst {
    pub fn root_name(&self) -> &'static str {
        match self.root {
            QueryNode::EntityScan { .. } => "EntityScan",
            QueryNode::Traversal { .. } => "Traversal",
            QueryNode::TopK { .. } => "TopK",
            QueryNode::Sort { .. } => "Sort",
            QueryNode::Group { .. } => "Group",
            QueryNode::Aggregate { .. } => "Aggregate",
            QueryNode::Take { .. } => "Take",
        }
    }

    pub fn mode_name(&self) -> &'static str {
        match self.mode {
            QueryMode::Once => "once",
            QueryMode::Watch => "watch",
        }
    }

    pub fn predicate_count(&self) -> usize {
        self.root.predicate_count()
    }

    pub fn has_semantic_predicate(&self) -> bool {
        self.root.has_semantic_predicate()
    }

    pub fn traversal_hops(&self) -> Option<usize> {
        self.root.traversal_hops()
    }

    pub fn to_canonical_fixture(&self) -> Value {
        self.root.to_canonical_with_mode(self.mode_name())
    }
}

impl QueryNode {
    fn predicate_count(&self) -> usize {
        match self {
            Self::EntityScan { predicates, .. } | Self::Traversal { predicates, .. } => {
                predicates.len()
            }
            Self::TopK { source, .. }
            | Self::Sort { source, .. }
            | Self::Group { source, .. }
            | Self::Aggregate { source, .. }
            | Self::Take { source, .. } => source.predicate_count(),
        }
    }

    fn has_semantic_predicate(&self) -> bool {
        match self {
            Self::EntityScan { predicates, .. } | Self::Traversal { predicates, .. } => predicates
                .iter()
                .any(|p| matches!(p, Predicate::Semantic { .. })),
            Self::TopK { source, .. }
            | Self::Sort { source, .. }
            | Self::Group { source, .. }
            | Self::Aggregate { source, .. }
            | Self::Take { source, .. } => source.has_semantic_predicate(),
        }
    }

    fn traversal_hops(&self) -> Option<usize> {
        match self {
            Self::Traversal { directions, .. } => Some(directions.len()),
            Self::TopK { source, .. }
            | Self::Sort { source, .. }
            | Self::Group { source, .. }
            | Self::Aggregate { source, .. }
            | Self::Take { source, .. } => source.traversal_hops(),
            Self::EntityScan { .. } => None,
        }
    }

    fn to_canonical_with_mode(&self, mode: &str) -> Value {
        match self {
            Self::TopK {
                k,
                order_by,
                source,
            } => json!({
                "type": "TopK",
                "k": *k,
                "order_by": order_by.as_ref().map(|o| json!({
                    "field": o.field,
                    "direction": sort_direction_name(o.direction),
                })),
                "source": source.to_source_fixture(),
                "mode": mode,
            }),
            Self::Traversal {
                steps,
                directions,
                predicates,
            } => {
                let mut serialized_steps = Vec::new();
                for (idx, step) in steps.iter().enumerate() {
                    serialized_steps.push(step.to_fixture_step());
                    if idx < directions.len() {
                        serialized_steps.push(json!({
                            "type": "Edge",
                            "direction": traversal_direction_name(directions[idx]),
                        }));
                    }
                }

                json!({
                    "type": "Traversal",
                    "mode": mode,
                    "steps": serialized_steps,
                    "where": predicates.iter().map(predicate_to_fixture).collect::<Vec<_>>(),
                })
            }
            _ => self.to_source_fixture(),
        }
    }

    fn to_source_fixture(&self) -> Value {
        match self {
            Self::EntityScan { entity, predicates } => {
                let base = json!({ "type": "EntityScan", "entity": entity });
                if predicates.is_empty() {
                    base
                } else {
                    json!({
                        "type": "Filter",
                        "predicates": predicates.iter().map(predicate_to_fixture).collect::<Vec<_>>(),
                        "source": base,
                    })
                }
            }
            Self::Traversal {
                steps,
                directions,
                predicates,
            } => {
                let mut serialized_steps = Vec::new();
                for (idx, step) in steps.iter().enumerate() {
                    serialized_steps.push(step.to_fixture_step());
                    if idx < directions.len() {
                        serialized_steps.push(json!({
                            "type": "Edge",
                            "direction": traversal_direction_name(directions[idx]),
                        }));
                    }
                }
                json!({
                    "type": "Traversal",
                    "mode": "once",
                    "steps": serialized_steps,
                    "where": predicates.iter().map(predicate_to_fixture).collect::<Vec<_>>(),
                })
            }
            Self::TopK {
                k,
                order_by,
                source,
            } => json!({
                "type": "TopK",
                "k": *k,
                "order_by": order_by.as_ref().map(|o| json!({
                    "field": o.field,
                    "direction": sort_direction_name(o.direction),
                })),
                "source": source.to_source_fixture(),
                "mode": "once",
            }),
            Self::Sort {
                field,
                direction,
                source,
            } => json!({
                "type": "Sort",
                "field": field,
                "direction": sort_direction_name(*direction),
                "source": source.to_source_fixture(),
            }),
            Self::Group { field, source } => json!({
                "type": "Group",
                "field": field,
                "source": source.to_source_fixture(),
            }),
            Self::Aggregate { op, field, source } => json!({
                "type": "Aggregate",
                "op": aggregate_op_name(*op),
                "field": field,
                "source": source.to_source_fixture(),
            }),
            Self::Take { n, source } => json!({
                "type": "Take",
                "n": *n,
                "source": source.to_source_fixture(),
            }),
        }
    }
}

impl TraversalStep {
    fn to_fixture_step(&self) -> Value {
        match self {
            Self::EntityScan(entity) => json!({ "type": "EntityScan", "entity": entity }),
            Self::EntityRef { entity, id } => json!({
                "type": "EntityRef",
                "entity": entity,
                "id": literal_to_json(id),
            }),
        }
    }
}

fn predicate_to_fixture(predicate: &Predicate) -> Value {
    match predicate {
        Predicate::Field { field, op, value } => json!({
            "type": "FieldPredicate",
            "field": field,
            "op": predicate_op_name(*op),
            "value": literal_to_json(value),
        }),
        Predicate::Range { field, start, end } => json!({
            "type": "RangePredicate",
            "field": field,
            "start": literal_to_json(start),
            "end": literal_to_json(end),
        }),
        Predicate::Semantic { query, threshold } => json!({
            "type": "SemanticPredicate",
            "function": "meaning",
            "query": query,
            "threshold": threshold,
        }),
    }
}

fn literal_to_json(literal: &Literal) -> Value {
    match literal {
        Literal::String(v) => json!(v),
        Literal::Number(v) => {
            if v.fract() == 0.0 {
                json!(*v as i64)
            } else {
                json!(v)
            }
        }
        Literal::Bool(v) => json!(v),
        Literal::Ident(v) => json!(v),
    }
}

fn sort_direction_name(direction: SortDirection) -> &'static str {
    match direction {
        SortDirection::Asc => "asc",
        SortDirection::Desc => "desc",
    }
}

fn traversal_direction_name(direction: TraversalDirection) -> &'static str {
    match direction {
        TraversalDirection::Outbound => "out",
        TraversalDirection::Inbound => "in",
    }
}

fn aggregate_op_name(op: AggregateOp) -> &'static str {
    match op {
        AggregateOp::Count => "count",
        AggregateOp::Sum => "sum",
        AggregateOp::Avg => "avg",
        AggregateOp::Min => "min",
        AggregateOp::Max => "max",
    }
}

fn predicate_op_name(op: PredicateOp) -> &'static str {
    match op {
        PredicateOp::Eq => "eq",
        PredicateOp::Ne => "ne",
        PredicateOp::Lt => "lt",
        PredicateOp::Lte => "lte",
        PredicateOp::Gt => "gt",
        PredicateOp::Gte => "gte",
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ParserError {
    #[error("lex error at byte {position}: {message}")]
    Lex { position: usize, message: String },
    #[error("parse error at token {position}: {message}")]
    Parse { position: usize, message: String },
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    String(String),
    Number(f64),
    LParen,
    RParen,
    Comma,
    Pipe,
    ArrowRight,
    ArrowLeft,
    EqEq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    DotDot,
    Assign,
}

pub fn parse_query(input: &str) -> Result<QueryAst, ParserError> {
    let tokens = lex(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse_query()
}

fn lex(input: &str) -> Result<Vec<Token>, ParserError> {
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    let mut out = Vec::new();

    while i < chars.len() {
        let ch = chars[i];

        if ch.is_whitespace() {
            i += 1;
            continue;
        }

        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let ident: String = chars[start..i].iter().collect();
            out.push(Token::Ident(ident));
            continue;
        }

        if ch.is_ascii_digit() {
            let start = i;
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            if i < chars.len()
                && chars[i] == '.'
                && (i + 1) < chars.len()
                && chars[i + 1] != '.'
                && chars[i + 1].is_ascii_digit()
            {
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let raw: String = chars[start..i].iter().collect();
            let number = raw.parse::<f64>().map_err(|e| ParserError::Lex {
                position: start,
                message: format!("invalid number {raw}: {e}"),
            })?;
            out.push(Token::Number(number));
            continue;
        }

        match ch {
            '"' => {
                let start = i;
                i += 1;
                let mut value = String::new();
                let mut terminated = false;
                while i < chars.len() {
                    let cur = chars[i];
                    if cur == '"' {
                        i += 1;
                        terminated = true;
                        break;
                    }
                    if cur == '\\' {
                        i += 1;
                        if i >= chars.len() {
                            return Err(ParserError::Lex {
                                position: start,
                                message: "unterminated escape sequence".to_string(),
                            });
                        }
                        let escaped = match chars[i] {
                            'n' => '\n',
                            't' => '\t',
                            '"' => '"',
                            '\\' => '\\',
                            other => other,
                        };
                        value.push(escaped);
                        i += 1;
                        continue;
                    }
                    value.push(cur);
                    i += 1;
                }
                if !terminated {
                    return Err(ParserError::Lex {
                        position: start,
                        message: "unterminated string literal".to_string(),
                    });
                }
                out.push(Token::String(value));
            }
            '(' => {
                out.push(Token::LParen);
                i += 1;
            }
            ')' => {
                out.push(Token::RParen);
                i += 1;
            }
            ',' => {
                out.push(Token::Comma);
                i += 1;
            }
            '|' => {
                out.push(Token::Pipe);
                i += 1;
            }
            '-' => {
                if (i + 1) < chars.len() && chars[i + 1] == '>' {
                    out.push(Token::ArrowRight);
                    i += 2;
                } else {
                    return Err(ParserError::Lex {
                        position: i,
                        message: "unexpected '-'".to_string(),
                    });
                }
            }
            '<' => {
                if (i + 1) < chars.len() && chars[i + 1] == '-' {
                    out.push(Token::ArrowLeft);
                    i += 2;
                } else if (i + 1) < chars.len() && chars[i + 1] == '=' {
                    out.push(Token::Lte);
                    i += 2;
                } else {
                    out.push(Token::Lt);
                    i += 1;
                }
            }
            '>' => {
                if (i + 1) < chars.len() && chars[i + 1] == '=' {
                    out.push(Token::Gte);
                    i += 2;
                } else {
                    out.push(Token::Gt);
                    i += 1;
                }
            }
            '=' => {
                if (i + 1) < chars.len() && chars[i + 1] == '=' {
                    out.push(Token::EqEq);
                    i += 2;
                } else {
                    out.push(Token::Assign);
                    i += 1;
                }
            }
            '!' => {
                if (i + 1) < chars.len() && chars[i + 1] == '=' {
                    out.push(Token::Ne);
                    i += 2;
                } else {
                    return Err(ParserError::Lex {
                        position: i,
                        message: "unexpected '!'".to_string(),
                    });
                }
            }
            '.' => {
                if (i + 1) < chars.len() && chars[i + 1] == '.' {
                    out.push(Token::DotDot);
                    i += 2;
                } else {
                    return Err(ParserError::Lex {
                        position: i,
                        message: "unexpected '.'".to_string(),
                    });
                }
            }
            _ => {
                return Err(ParserError::Lex {
                    position: i,
                    message: format!("unexpected character '{}'", ch),
                })
            }
        }
    }

    Ok(out)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_query(&mut self) -> Result<QueryAst, ParserError> {
        let mode = if self.consume_keyword("watch") {
            QueryMode::Watch
        } else {
            QueryMode::Once
        };

        let mut root = self.parse_source()?;
        while self.consume_token(&Token::Pipe) {
            root = self.parse_transform(root)?;
        }

        if self.peek().is_some() {
            return Err(self.error("unexpected trailing tokens"));
        }

        Ok(QueryAst { mode, root })
    }

    fn parse_source(&mut self) -> Result<QueryNode, ParserError> {
        let first_step = self.parse_traversal_step()?;

        if let Some(first_dir) = self.parse_traversal_direction_if_any() {
            let mut steps = vec![first_step];
            let mut directions = vec![first_dir];
            steps.push(self.parse_traversal_step()?);

            while let Some(dir) = self.parse_traversal_direction_if_any() {
                directions.push(dir);
                steps.push(self.parse_traversal_step()?);
            }

            let predicates = self.parse_optional_where_clause()?;
            return Ok(QueryNode::Traversal {
                steps,
                directions,
                predicates,
            });
        }

        match first_step {
            TraversalStep::EntityScan(entity) => {
                let predicates = self.parse_optional_where_clause()?;
                Ok(QueryNode::EntityScan { entity, predicates })
            }
            step @ TraversalStep::EntityRef { .. } => {
                let predicates = self.parse_optional_where_clause()?;
                Ok(QueryNode::Traversal {
                    steps: vec![step],
                    directions: vec![],
                    predicates,
                })
            }
        }
    }

    fn parse_transform(&mut self, source: QueryNode) -> Result<QueryNode, ParserError> {
        let name = self.parse_ident()?;
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "sort" => {
                self.expect_token(&Token::LParen)?;
                let field = self.parse_ident()?;
                let direction = if self.consume_keyword("desc") {
                    SortDirection::Desc
                } else {
                    let _ = self.consume_keyword("asc");
                    SortDirection::Asc
                };
                self.expect_token(&Token::RParen)?;
                Ok(QueryNode::Sort {
                    field,
                    direction,
                    source: Box::new(source),
                })
            }
            "top" => {
                self.expect_token(&Token::LParen)?;
                let k = self.parse_u64_number()?;
                let order_by = if self.consume_token(&Token::Comma) {
                    let field = self.parse_ident()?;
                    let direction = if self.consume_keyword("desc") {
                        SortDirection::Desc
                    } else {
                        let _ = self.consume_keyword("asc");
                        SortDirection::Asc
                    };
                    Some(OrderBy { field, direction })
                } else {
                    None
                };
                self.expect_token(&Token::RParen)?;
                Ok(QueryNode::TopK {
                    k,
                    order_by,
                    source: Box::new(source),
                })
            }
            "group" => {
                self.expect_token(&Token::LParen)?;
                let field = self.parse_ident()?;
                self.expect_token(&Token::RParen)?;
                Ok(QueryNode::Group {
                    field,
                    source: Box::new(source),
                })
            }
            "count" | "sum" | "avg" | "min" | "max" => {
                self.expect_token(&Token::LParen)?;
                let field = if self.peek_is(&Token::RParen) {
                    None
                } else {
                    Some(self.parse_ident()?)
                };
                self.expect_token(&Token::RParen)?;
                let op = match lower.as_str() {
                    "count" => AggregateOp::Count,
                    "sum" => AggregateOp::Sum,
                    "avg" => AggregateOp::Avg,
                    "min" => AggregateOp::Min,
                    "max" => AggregateOp::Max,
                    _ => unreachable!(),
                };
                Ok(QueryNode::Aggregate {
                    op,
                    field,
                    source: Box::new(source),
                })
            }
            "take" => {
                self.expect_token(&Token::LParen)?;
                let n = self.parse_u64_number()?;
                self.expect_token(&Token::RParen)?;
                Ok(QueryNode::Take {
                    n,
                    source: Box::new(source),
                })
            }
            _ => Err(self.error(&format!("unknown transform: {name}"))),
        }
    }

    fn parse_optional_where_clause(&mut self) -> Result<Vec<Predicate>, ParserError> {
        if !self.consume_keyword("where") {
            return Ok(Vec::new());
        }

        let mut predicates = vec![self.parse_predicate()?];
        while self.consume_keyword("and") {
            predicates.push(self.parse_predicate()?);
        }
        Ok(predicates)
    }

    fn parse_predicate(&mut self) -> Result<Predicate, ParserError> {
        if self.consume_keyword("meaning") {
            self.expect_token(&Token::LParen)?;
            let query = self.parse_string_literal()?;
            let threshold = if self.consume_token(&Token::Comma) {
                self.expect_keyword("threshold")?;
                self.expect_token(&Token::Assign)?;
                Some(self.parse_number()?)
            } else {
                None
            };
            self.expect_token(&Token::RParen)?;
            return Ok(Predicate::Semantic { query, threshold });
        }

        let field = self.parse_ident()?;
        if self.consume_keyword("in") {
            let start = self.parse_literal()?;
            self.expect_token(&Token::DotDot)?;
            let end = self.parse_literal()?;
            return Ok(Predicate::Range { field, start, end });
        }

        let op = self.parse_predicate_op()?;
        let value = self.parse_literal()?;
        Ok(Predicate::Field { field, op, value })
    }

    fn parse_predicate_op(&mut self) -> Result<PredicateOp, ParserError> {
        match self.next() {
            Some(Token::EqEq) => Ok(PredicateOp::Eq),
            Some(Token::Ne) => Ok(PredicateOp::Ne),
            Some(Token::Lt) => Ok(PredicateOp::Lt),
            Some(Token::Lte) => Ok(PredicateOp::Lte),
            Some(Token::Gt) => Ok(PredicateOp::Gt),
            Some(Token::Gte) => Ok(PredicateOp::Gte),
            other => Err(self.error_expected("predicate operator", other.as_ref())),
        }
    }

    fn parse_traversal_step(&mut self) -> Result<TraversalStep, ParserError> {
        let entity = self.parse_ident()?;
        if self.consume_token(&Token::LParen) {
            let id = self.parse_literal()?;
            self.expect_token(&Token::RParen)?;
            Ok(TraversalStep::EntityRef { entity, id })
        } else {
            Ok(TraversalStep::EntityScan(entity))
        }
    }

    fn parse_traversal_direction_if_any(&mut self) -> Option<TraversalDirection> {
        if self.consume_token(&Token::ArrowRight) {
            Some(TraversalDirection::Outbound)
        } else if self.consume_token(&Token::ArrowLeft) {
            Some(TraversalDirection::Inbound)
        } else {
            None
        }
    }

    fn parse_literal(&mut self) -> Result<Literal, ParserError> {
        match self.next() {
            Some(Token::String(v)) => Ok(Literal::String(v)),
            Some(Token::Number(v)) => Ok(Literal::Number(v)),
            Some(Token::Ident(v)) if v.eq_ignore_ascii_case("true") => Ok(Literal::Bool(true)),
            Some(Token::Ident(v)) if v.eq_ignore_ascii_case("false") => Ok(Literal::Bool(false)),
            Some(Token::Ident(v)) => Ok(Literal::Ident(v)),
            other => Err(self.error_expected("literal", other.as_ref())),
        }
    }

    fn parse_ident(&mut self) -> Result<String, ParserError> {
        match self.next() {
            Some(Token::Ident(v)) => Ok(v),
            other => Err(self.error_expected("identifier", other.as_ref())),
        }
    }

    fn parse_string_literal(&mut self) -> Result<String, ParserError> {
        match self.next() {
            Some(Token::String(v)) => Ok(v),
            other => Err(self.error_expected("string literal", other.as_ref())),
        }
    }

    fn parse_number(&mut self) -> Result<f64, ParserError> {
        match self.next() {
            Some(Token::Number(v)) => Ok(v),
            other => Err(self.error_expected("number", other.as_ref())),
        }
    }

    fn parse_u64_number(&mut self) -> Result<u64, ParserError> {
        let v = self.parse_number()?;
        if v < 0.0 || v.fract() != 0.0 {
            return Err(self.error(&format!("expected integer number, got {v}")));
        }
        Ok(v as u64)
    }

    fn expect_keyword(&mut self, kw: &str) -> Result<(), ParserError> {
        if self.consume_keyword(kw) {
            Ok(())
        } else {
            Err(self.error(&format!("expected keyword '{kw}'")))
        }
    }

    fn consume_keyword(&mut self, kw: &str) -> bool {
        if let Some(Token::Ident(v)) = self.peek() {
            if v.eq_ignore_ascii_case(kw) {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn expect_token(&mut self, expected: &Token) -> Result<(), ParserError> {
        if self.consume_token(expected) {
            Ok(())
        } else {
            Err(self.error(&format!("expected token {expected:?}")))
        }
    }

    fn consume_token(&mut self, expected: &Token) -> bool {
        if self.peek_is(expected) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn peek_is(&self, expected: &Token) -> bool {
        self.peek()
            .is_some_and(|t| std::mem::discriminant(t) == std::mem::discriminant(expected))
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn error(&self, message: &str) -> ParserError {
        ParserError::Parse {
            position: self.pos,
            message: message.to_string(),
        }
    }

    fn error_expected(&self, expected: &str, found: Option<&Token>) -> ParserError {
        ParserError::Parse {
            position: self.pos,
            message: format!("expected {expected}, found {found:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_query, QueryMode};
    use serde::Deserialize;
    use serde_json::Value;

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
    fn parses_open_spec_conformance_cases() {
        let file = std::fs::read_to_string(artifacts_root().join("conformance-cases.yaml"))
            .expect("read conformance file");
        let conformance: ConformanceFile =
            serde_yaml::from_str(&file).expect("parse conformance yaml");

        for case in conformance.cases {
            let ast =
                parse_query(&case.query).unwrap_or_else(|e| panic!("case {} failed: {e}", case.id));
            assert_eq!(
                ast.root_name(),
                case.expect.root,
                "root mismatch for case {}",
                case.id
            );
            assert_eq!(
                ast.mode_name(),
                case.expect.mode,
                "mode mismatch for case {}",
                case.id
            );

            if let Some(predicates) = case.expect.predicates {
                assert_eq!(
                    ast.predicate_count(),
                    predicates,
                    "predicate count mismatch for case {}",
                    case.id
                );
            }
            if let Some(semantic) = case.expect.semantic {
                assert_eq!(
                    ast.has_semantic_predicate(),
                    semantic,
                    "semantic mismatch for case {}",
                    case.id
                );
            }
            if let Some(hops) = case.expect.hops {
                assert_eq!(
                    ast.traversal_hops().unwrap_or(0),
                    hops,
                    "hop mismatch for case {}",
                    case.id
                );
            }
        }
    }

    #[test]
    fn canonical_fixture_matches_hybrid_ast() {
        let query =
            "Product where price < 3000 and meaning(\"elegant traditional\") | top(5, price desc)";
        let ast = parse_query(query).expect("parse hybrid");
        assert!(matches!(ast.mode, QueryMode::Once));

        let got = ast.to_canonical_fixture();
        let expected_file =
            std::fs::read_to_string(artifacts_root().join("sdk-fixtures/hybrid_query_ast.json"))
                .expect("read fixture");
        let expected: Value = serde_json::from_str(&expected_file).expect("parse fixture json");

        assert_eq!(got, expected);
    }

    #[test]
    fn canonical_fixture_matches_watch_traversal_ast() {
        let query = "watch Brand(\"Norn Gold\") -> Product where price < 3000";
        let ast = parse_query(query).expect("parse watch traversal");
        assert!(matches!(ast.mode, QueryMode::Watch));

        let got = ast.to_canonical_fixture();
        let expected_file =
            std::fs::read_to_string(artifacts_root().join("sdk-fixtures/watch_traversal_ast.json"))
                .expect("read fixture");
        let expected: Value = serde_json::from_str(&expected_file).expect("parse fixture json");

        assert_eq!(got, expected);
    }
}
