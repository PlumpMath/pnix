//! SETO Query Builder - Fluent API
//!
//! pnix-old의 pnix_llm/src/query_builder.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 쿼리 빌더/검증 로직만, 실행 없음

use crate::nlp::truth_domain::TruthDomain;

/// SETO 쿼리 타입: SETO 지식 그래프 쿼리의 타입
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetoQueryType {
  /// 개념 검색
  Concept,
  /// 정리/법칙 검색
  Theorem,
  /// 관계 검색
  Relation,
  /// 증명 검색
  Proof,
  /// 정의 검색
  Definition,
  /// 예시 검색
  Example,
}

/// SETO 노드 필터: SETO 쿼리의 필터 조건
#[derive(Debug, Clone)]
pub struct SetoFilter {
  /// 도메인 필터 목록
  pub domains: Vec<TruthDomain>,
  /// 최소 신뢰도 (0.0 ~ 1.0)
  pub min_confidence: f32,
  /// 최대 결과 수
  pub limit: usize,
  /// 검색 깊이 (관계 탐색 시 사용)
  pub depth: usize,
  /// 검증된 노드만 필터링 여부
  pub verified_only: bool,
}

impl Default for SetoFilter {
  fn default() -> Self {
    Self {
      domains: Vec::new(),
      min_confidence: 0.0,
      limit: 100,
      depth: 1,
      verified_only: false,
    }
  }
}

/// SETO 쿼리 결과 정렬 기준: 쿼리 결과를 정렬하는 기준
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortBy {
  /// 관련도 순
  Relevance,
  /// 신뢰도 순
  Confidence,
  /// 최신순
  Recent,
  /// 접근 빈도순
  Popular,
}

/// SETO Query Builder: Fluent API로 쿼리를 구성하는 빌더
///
/// 헌법 P0-1 준수: 쿼리 빌드/검증 로직만, 실행 없음
#[derive(Debug, Clone)]
pub struct SetoQueryBuilder {
  /// 쿼리 타입 (선택적)
  query_type: Option<SetoQueryType>,
  /// 검색 패턴/키워드 (선택적)
  pattern: Option<String>,
  /// 필터 조건
  filter: SetoFilter,
  /// 정렬 기준
  sort_by: SortBy,
  /// 시작 노드 (관계 탐색용, 선택적)
  from_node: Option<String>,
  /// 타겟 노드 (관계 탐색용, 선택적)
  to_node: Option<String>,
  /// 관계 타입 필터 목록
  relation_types: Vec<String>,
}

impl Default for SetoQueryBuilder {
  fn default() -> Self {
    Self::new()
  }
}

/// 쿼리 빌드 에러: 쿼리 빌드 과정에서 발생하는 에러
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryBuildError {
  /// 에러 메시지
  pub message: String,
}

impl QueryBuildError {
  /// 새 에러 생성
  pub fn new(message: impl Into<String>) -> Self {
    Self {
      message: message.into(),
    }
  }
}

impl std::fmt::Display for QueryBuildError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "QueryBuildError: {}", self.message)
  }
}

impl std::error::Error for QueryBuildError {}

impl SetoQueryBuilder {
  /// 새로운 쿼리 빌더 생성
  pub fn new() -> Self {
    Self {
      query_type: None,
      pattern: None,
      filter: SetoFilter::default(),
      sort_by: SortBy::Relevance,
      from_node: None,
      to_node: None,
      relation_types: Vec::new(),
    }
  }

  /// 개념 검색 쿼리
  pub fn concept(pattern: impl Into<String>) -> Self {
    Self::new()
      .query_type(SetoQueryType::Concept)
      .pattern(pattern)
  }

  /// 정리 검색 쿼리
  pub fn theorem(pattern: impl Into<String>) -> Self {
    Self::new()
      .query_type(SetoQueryType::Theorem)
      .pattern(pattern)
  }

  /// 관계 검색 쿼리
  pub fn relation() -> Self {
    Self::new().query_type(SetoQueryType::Relation)
  }

  /// 증명 검색 쿼리
  pub fn proof(pattern: impl Into<String>) -> Self {
    Self::new()
      .query_type(SetoQueryType::Proof)
      .pattern(pattern)
  }

  /// 정의 검색 쿼리
  pub fn definition(pattern: impl Into<String>) -> Self {
    Self::new()
      .query_type(SetoQueryType::Definition)
      .pattern(pattern)
  }

  /// 예시 검색 쿼리
  pub fn example(pattern: impl Into<String>) -> Self {
    Self::new()
      .query_type(SetoQueryType::Example)
      .pattern(pattern)
  }

  /// 쿼리 타입 설정
  pub fn query_type(mut self, qt: SetoQueryType) -> Self {
    self.query_type = Some(qt);
    self
  }

  /// 검색 패턴 설정
  pub fn pattern(mut self, p: impl Into<String>) -> Self {
    self.pattern = Some(p.into());
    self
  }

  /// 도메인 필터 추가
  pub fn in_domain(mut self, domain: TruthDomain) -> Self {
    self.filter.domains.push(domain);
    self
  }

  /// 여러 도메인 필터 추가
  pub fn in_domains(mut self, domains: impl IntoIterator<Item = TruthDomain>) -> Self {
    self.filter.domains.extend(domains);
    self
  }

  /// 최소 신뢰도 설정
  pub fn min_confidence(mut self, confidence: f32) -> Self {
    self.filter.min_confidence = confidence.clamp(0.0, 1.0);
    self
  }

  /// 결과 수 제한
  pub fn limit(mut self, n: usize) -> Self {
    self.filter.limit = n;
    self
  }

  /// 검색 깊이 설정
  pub fn depth(mut self, d: usize) -> Self {
    self.filter.depth = d;
    self
  }

  /// 검증된 노드만 필터
  pub fn verified_only(mut self) -> Self {
    self.filter.verified_only = true;
    self
  }

  /// 정렬 기준 설정
  pub fn sort_by(mut self, sort: SortBy) -> Self {
    self.sort_by = sort;
    self
  }

  /// 관련도순 정렬
  pub fn sort_by_relevance(self) -> Self {
    self.sort_by(SortBy::Relevance)
  }

  /// 신뢰도순 정렬
  pub fn sort_by_confidence(self) -> Self {
    self.sort_by(SortBy::Confidence)
  }

  /// 최신순 정렬
  pub fn sort_by_recent(self) -> Self {
    self.sort_by(SortBy::Recent)
  }

  /// 인기순 정렬
  pub fn sort_by_popular(self) -> Self {
    self.sort_by(SortBy::Popular)
  }

  /// 시작 노드 설정 (관계 탐색)
  pub fn from(mut self, node_id: impl Into<String>) -> Self {
    self.from_node = Some(node_id.into());
    self
  }

  /// 타겟 노드 설정 (관계 탐색)
  pub fn to(mut self, node_id: impl Into<String>) -> Self {
    self.to_node = Some(node_id.into());
    self
  }

  /// 관계 타입 필터 추가
  pub fn with_relation(mut self, relation_type: impl Into<String>) -> Self {
    self.relation_types.push(relation_type.into());
    self
  }

  /// 쿼리 유효성 검사
  pub fn validate(&self) -> Result<(), QueryBuildError> {
    if self.query_type.is_none() {
      return Err(QueryBuildError::new("query type is required"));
    }

    match &self.query_type {
      Some(SetoQueryType::Relation) => {
        if self.from_node.is_none() && self.to_node.is_none() {
          return Err(QueryBuildError::new(
            "relation query requires at least one of: from_node, to_node",
          ));
        }
      }
      Some(_) => {
        if self.pattern.is_none() {
          return Err(QueryBuildError::new(
            "search pattern is required for this query type",
          ));
        }
      }
      None => unreachable!(),
    }

    Ok(())
  }

  /// 쿼리 빌드 (SetoQuery 생성)
  pub fn build(self) -> Result<SetoQuery, QueryBuildError> {
    self.validate()?;

    let query_type = self
      .query_type
      .ok_or_else(|| QueryBuildError::new("query type is required"))?;

    Ok(SetoQuery {
      query_type,
      pattern: self.pattern,
      filter: self.filter,
      sort_by: self.sort_by,
      from_node: self.from_node,
      to_node: self.to_node,
      relation_types: self.relation_types,
    })
  }
}

/// 빌드된 SETO 쿼리: 완성된 SETO 지식 그래프 쿼리
#[derive(Debug, Clone)]
pub struct SetoQuery {
  /// 쿼리 타입
  pub query_type: SetoQueryType,
  /// 검색 패턴 (선택적)
  pub pattern: Option<String>,
  /// 필터 조건
  pub filter: SetoFilter,
  /// 정렬 기준
  pub sort_by: SortBy,
  /// 시작 노드 (선택적)
  pub from_node: Option<String>,
  /// 타겟 노드 (선택적)
  pub to_node: Option<String>,
  /// 관계 타입 필터 목록
  pub relation_types: Vec<String>,
}

impl SetoQuery {
  /// 쿼리를 문자열로 직렬화 (디버깅/로깅용)
  pub fn to_query_string(&self) -> String {
    let mut parts = Vec::new();

    parts.push(format!("type={:?}", self.query_type));

    if let Some(ref p) = self.pattern {
      parts.push(format!("pattern=\"{}\"", p));
    }

    if !self.filter.domains.is_empty() {
      let domains: Vec<_> = self
        .filter
        .domains
        .iter()
        .map(|d| format!("{:?}", d))
        .collect();
      parts.push(format!("domains=[{}]", domains.join(",")));
    }

    if self.filter.min_confidence > 0.0 {
      parts.push(format!("min_conf={}", self.filter.min_confidence));
    }

    parts.push(format!("limit={}", self.filter.limit));
    parts.push(format!("sort={:?}", self.sort_by));

    if let Some(ref from) = self.from_node {
      parts.push(format!("from=\"{}\"", from));
    }

    if let Some(ref to) = self.to_node {
      parts.push(format!("to=\"{}\"", to));
    }

    parts.join(" ")
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;
  use crate::nlp::truth_domain::TruthDomain;

  #[test]
  fn test_concept_query() {
    let query = SetoQueryBuilder::concept("피타고라스")
      .in_domain(TruthDomain::Mathematics)
      .min_confidence(0.8)
      .limit(10)
      .build();

    assert!(query.is_ok());
    let q = query.unwrap();
    assert_eq!(q.query_type, SetoQueryType::Concept);
    assert_eq!(q.pattern, Some("피타고라스".to_string()));
    assert_eq!(q.filter.domains.len(), 1);
    assert_eq!(q.filter.min_confidence, 0.8);
    assert_eq!(q.filter.limit, 10);
  }

  #[test]
  fn test_theorem_query() {
    let query = SetoQueryBuilder::theorem("fundamental theorem")
      .in_domains([TruthDomain::Mathematics, TruthDomain::Logic])
      .verified_only()
      .sort_by_confidence()
      .build();

    assert!(query.is_ok());
    let q = query.unwrap();
    assert_eq!(q.query_type, SetoQueryType::Theorem);
    assert!(q.filter.verified_only);
    assert_eq!(q.sort_by, SortBy::Confidence);
  }

  #[test]
  fn test_relation_query() {
    let query = SetoQueryBuilder::relation()
      .from("seto://math/theorem/pythagoras")
      .to("seto://math/concept/triangle")
      .with_relation("proves")
      .depth(2)
      .build();

    assert!(query.is_ok());
    let q = query.unwrap();
    assert_eq!(q.query_type, SetoQueryType::Relation);
    assert!(q.from_node.is_some());
    assert!(q.to_node.is_some());
    assert_eq!(q.filter.depth, 2);
  }

  #[test]
  fn test_relation_query_validation() {
    // 관계 쿼리는 from 또는 to 노드 필요
    let result = SetoQueryBuilder::relation().build();
    assert!(result.is_err());
  }

  #[test]
  fn test_concept_query_requires_pattern() {
    let result = SetoQueryBuilder::new()
      .query_type(SetoQueryType::Concept)
      .build();
    assert!(result.is_err());
  }

  #[test]
  fn test_query_string_serialization() {
    let query = SetoQueryBuilder::concept("gravity")
      .in_domain(TruthDomain::Physics)
      .min_confidence(0.7)
      .limit(5)
      .build()
      .unwrap();

    let qs = query.to_query_string();
    assert!(qs.contains("type=Concept"));
    assert!(qs.contains("pattern=\"gravity\""));
    assert!(qs.contains("Physics"));
    assert!(qs.contains("min_conf=0.7"));
    assert!(qs.contains("limit=5"));
  }

  #[test]
  fn test_proof_query() {
    let query = SetoQueryBuilder::proof("induction")
      .in_domain(TruthDomain::Mathematics)
      .sort_by_recent()
      .build();

    assert!(query.is_ok());
    let q = query.unwrap();
    assert_eq!(q.query_type, SetoQueryType::Proof);
    assert_eq!(q.sort_by, SortBy::Recent);
  }

  #[test]
  fn test_definition_query() {
    let query = SetoQueryBuilder::definition("entropy")
      .in_domains([TruthDomain::Physics, TruthDomain::InformationTheory])
      .build();

    assert!(query.is_ok());
    let q = query.unwrap();
    assert_eq!(q.query_type, SetoQueryType::Definition);
    assert_eq!(q.filter.domains.len(), 2);
  }

  #[test]
  fn test_example_query() {
    let query = SetoQueryBuilder::example("prime number")
      .limit(20)
      .sort_by_popular()
      .build();

    assert!(query.is_ok());
    let q = query.unwrap();
    assert_eq!(q.query_type, SetoQueryType::Example);
    assert_eq!(q.filter.limit, 20);
    assert_eq!(q.sort_by, SortBy::Popular);
  }

  #[test]
  fn test_default_filter_values() {
    let filter = SetoFilter::default();
    assert!(filter.domains.is_empty());
    assert_eq!(filter.min_confidence, 0.0);
    assert_eq!(filter.limit, 100);
    assert_eq!(filter.depth, 1);
    assert!(!filter.verified_only);
  }

  #[test]
  fn test_chained_builder() {
    let query = SetoQueryBuilder::new()
      .query_type(SetoQueryType::Concept)
      .pattern("quantum")
      .in_domain(TruthDomain::Physics)
      .min_confidence(0.9)
      .verified_only()
      .limit(50)
      .depth(3)
      .sort_by_relevance()
      .build();

    assert!(query.is_ok());
    let q = query.unwrap();
    assert_eq!(q.filter.min_confidence, 0.9);
    assert!(q.filter.verified_only);
    assert_eq!(q.filter.limit, 50);
    assert_eq!(q.filter.depth, 3);
  }
}
