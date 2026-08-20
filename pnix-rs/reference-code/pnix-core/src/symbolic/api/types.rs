//! MCP 요청/응답 타입 정의
//!
//! pnix-old의 symbolic_core/api/types.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 정의만, 실행 없음
//!
//! ## 사용 목적
//!
//! - MCP 서버 API 타입 정의
//! - 리소스 한도 설정
//! - CT 컨텍스트 스펙
//! - 요청/응답 구조

use std::collections::HashMap;

//─────────────────────────────────────────────────
// 리소스 한도 (OOM/DoS 방지)
//─────────────────────────────────────────────────

/// 심볼릭 연산 리소스 한도: MCP 서버에서 무한 루프, OOM, DoS 공격을 방지하기 위한 설정
///
/// 기본값은 합리적인 범위로 설정되어 있으며, 필요시 조정 가능.
#[derive(Clone, Debug)]
pub struct ResourceLimits {
  /// e-graph 최대 노드 수 (기본: 10,000)
  pub max_nodes: usize,
  /// e-graph 최대 반복 횟수 (기본: 10)
  pub max_iterations: usize,
  /// 표현식 최대 깊이 (기본: 100)
  pub max_depth: usize,
  /// 연산 타임아웃 (밀리초, 기본: 5초)
  pub timeout_ms: u64,
  /// 최대 표현식 크기 (AST 노드 수, 기본: 1,000)
  pub max_expr_size: usize,
}

impl Default for ResourceLimits {
  fn default() -> Self {
    Self {
      max_nodes: 10_000,
      max_iterations: 10,
      max_depth: 100,
      timeout_ms: 5_000,
      max_expr_size: 1_000,
    }
  }
}

impl ResourceLimits {
  /// 새 리소스 한도 생성
  pub fn new() -> Self {
    Self::default()
  }

  /// 엄격한 한도 (테스트/개발용)
  pub fn strict() -> Self {
    Self {
      max_nodes: 1_000,
      max_iterations: 5,
      max_depth: 50,
      timeout_ms: 1_000,
      max_expr_size: 100,
    }
  }

  /// 관대한 한도 (복잡한 연산용)
  pub fn relaxed() -> Self {
    Self {
      max_nodes: 100_000,
      max_iterations: 50,
      max_depth: 500,
      timeout_ms: 30_000,
      max_expr_size: 10_000,
    }
  }

  /// 노드 한도 설정
  pub fn with_max_nodes(mut self, n: usize) -> Self {
    self.max_nodes = n;
    self
  }

  /// 반복 횟수 한도 설정
  pub fn with_max_iterations(mut self, n: usize) -> Self {
    self.max_iterations = n;
    self
  }

  /// 깊이 한도 설정
  pub fn with_max_depth(mut self, n: usize) -> Self {
    self.max_depth = n;
    self
  }

  /// 타임아웃 설정
  /// 타임아웃 설정 (밀리초)
  pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
    self.timeout_ms = timeout_ms;
    self
  }

  /// 표현식 크기 한도 설정
  pub fn with_max_expr_size(mut self, n: usize) -> Self {
    self.max_expr_size = n;
    self
  }
}

/// 리소스 한도 초과 에러: 리소스 한도를 초과했을 때 발생하는 에러
#[derive(Clone, Debug)]
pub enum ResourceLimitError {
  /// 노드 수 초과 (e-graph 노드 수가 한도를 초과)
  NodeLimitExceeded {
    /// 허용된 최대 노드 수
    limit: usize,
    /// 실제 노드 수
    actual: usize,
  },
  /// 반복 횟수 초과 (e-graph 반복 횟수가 한도를 초과)
  IterationLimitExceeded {
    /// 허용된 최대 반복 횟수
    limit: usize,
    /// 실제 반복 횟수
    actual: usize,
  },
  /// 깊이 초과 (표현식 깊이가 한도를 초과)
  DepthLimitExceeded {
    /// 허용된 최대 깊이
    limit: usize,
    /// 실제 깊이
    actual: usize,
  },
  /// 타임아웃 (연산 시간이 타임아웃을 초과)
  Timeout {
    /// 타임아웃 시간 (밀리초)
    limit_ms: u64,
  },
  /// 표현식 크기 초과 (AST 노드 수가 한도를 초과)
  ExprSizeLimitExceeded {
    /// 허용된 최대 표현식 크기
    limit: usize,
    /// 실제 표현식 크기
    actual: usize,
  },
  /// CT 제약 위반 (rewrite 전후 검증 실패)
  CtViolation {
    /// 위반 메시지
    message: String,
  },
}

impl std::fmt::Display for ResourceLimitError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NodeLimitExceeded { limit, actual } => {
        write!(f, "Node limit exceeded: {} > {}", actual, limit)
      }
      Self::IterationLimitExceeded { limit, actual } => {
        write!(f, "Iteration limit exceeded: {} > {}", actual, limit)
      }
      Self::DepthLimitExceeded { limit, actual } => {
        write!(f, "Depth limit exceeded: {} > {}", actual, limit)
      }
      Self::Timeout { limit_ms } => {
        write!(f, "Operation timed out after {}ms", limit_ms)
      }
      Self::ExprSizeLimitExceeded { limit, actual } => {
        write!(f, "Expression size limit exceeded: {} > {}", actual, limit)
      }
      Self::CtViolation { message } => {
        write!(f, "CT constraint violation: {}", message)
      }
    }
  }
}

impl std::error::Error for ResourceLimitError {}

/// CT 컨텍스트 스펙: 변수별 단위/카테고리 정보
#[derive(Clone, Debug, Default)]
pub struct CtContextSpec {
  /// 변수 이름 → 단위 매핑 (예: "x" → "m", "v" → "m/s")
  pub units: HashMap<String, String>,
  /// 변수 이름 → 카테고리 매핑 (예: "x" → "position", "v" → "velocity")
  pub categories: HashMap<String, String>,
}

impl CtContextSpec {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn with_var(mut self, name: &str, unit: &str) -> Self {
    self.units.insert(name.to_string(), unit.to_string());
    self
  }

  pub fn with_category(mut self, name: &str, category: &str) -> Self {
    self
      .categories
      .insert(name.to_string(), category.to_string());
    self
  }
}

/// 텐서 컨텍스트 스펙: 인덱스 공간 정의
#[derive(Clone, Debug, Default)]
pub struct TensorContextSpec {
  /// 인덱스 이름 → 공간 매핑 (예: "μ" → "spacetime", "i" → "euclidean")
  pub index_spaces: HashMap<String, String>,
}

impl TensorContextSpec {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn with_index(mut self, name: &str, space: &str) -> Self {
    self
      .index_spaces
      .insert(name.to_string(), space.to_string());
    self
  }
}

/// 시뮬레이션 파라미터: 수치 시뮬레이션을 위한 파라미터
#[derive(Clone, Debug)]
pub struct SimParams {
  /// 시작 시간
  pub t_min: f64,
  /// 종료 시간
  pub t_max: f64,
  /// 시뮬레이션 스텝 수
  pub steps: usize,
  /// 변수별 초기값 (예: "v0" → 5.0, "x0" → 0.0)
  pub variables: HashMap<String, f64>,
}

impl Default for SimParams {
  fn default() -> Self {
    Self {
      t_min: 0.0,
      t_max: 10.0,
      steps: 100,
      variables: HashMap::new(),
    }
  }
}

impl SimParams {
  pub fn new(t_min: f64, t_max: f64, steps: usize) -> Self {
    Self {
      t_min,
      t_max,
      steps,
      variables: HashMap::new(),
    }
  }

  pub fn with_var(mut self, name: &str, value: f64) -> Self {
    self.variables.insert(name.to_string(), value);
    self
  }
}

//─────────────────────────────────────────────────
// MCP 툴 요청 타입
//─────────────────────────────────────────────────

/// symbolic_normalize 요청
#[derive(Clone, Debug)]
pub struct NormalizeRequest {
  pub expr: String,
  pub context: Option<CtContextSpec>,
}

/// symbolic_diff 요청
#[derive(Clone, Debug)]
pub struct DiffRequest {
  pub expr: String,
  pub var: String,
}

/// symbolic_simulate 요청
#[derive(Clone, Debug)]
pub struct SimulateRequest {
  pub expr: String,
  pub context: Option<CtContextSpec>,
  pub params: SimParams,
}

/// symbolic_tensor_contract 요청
#[derive(Clone, Debug)]
pub struct TensorContractRequest {
  pub expr: String,
  pub tensor_context: Option<TensorContextSpec>,
}

/// symbolic_simplify 요청
#[derive(Clone, Debug)]
pub struct SimplifyRequest {
  pub expr: String,
  /// 최대 반복 횟수 (기본: 10)
  pub max_iterations: Option<usize>,
}

/// symbolic_expand 요청
#[derive(Clone, Debug)]
pub struct ExpandRequest {
  pub expr: String,
}

//─────────────────────────────────────────────────
// MCP 툴 응답 타입
//─────────────────────────────────────────────────

/// symbolic_normalize 응답
#[derive(Clone, Debug)]
pub struct NormalizeResponse {
  pub latex: String,
  pub normalized_expr: String,
  pub ct_warnings: Vec<String>,
  pub is_valid: bool,
}

/// symbolic_diff 응답
#[derive(Clone, Debug)]
pub struct DiffResponse {
  pub latex: String,
  pub normalized_expr: String,
}

/// symbolic_simulate 응답
#[derive(Clone, Debug)]
pub struct SimulateResponse {
  pub times: Vec<f64>,
  pub values: Vec<f64>,
  pub latex: String,
}

/// symbolic_tensor_contract 응답
#[derive(Clone, Debug)]
pub struct TensorContractResponse {
  pub latex: String,
  pub contracted_expr: String,
  pub free_indices: Vec<String>,
  pub identities_applied: Vec<String>,
}

/// symbolic_simplify 응답
#[derive(Clone, Debug)]
pub struct SimplifyResponse {
  pub latex: String,
  pub simplified_expr: String,
  /// 적용된 규칙들
  pub rules_applied: Vec<String>,
  /// 반복 횟수
  pub iterations: usize,
}

/// symbolic_expand 응답
#[derive(Clone, Debug)]
pub struct ExpandResponse {
  pub latex: String,
  pub expanded_expr: String,
}

/// symbolic_substitute 요청
#[derive(Clone, Debug)]
pub struct SubstituteRequest {
  pub expr: String,
  /// 변수 → 값 치환
  pub substitutions: HashMap<String, f64>,
}

/// symbolic_substitute 응답
#[derive(Clone, Debug)]
pub struct SubstituteResponse {
  pub latex: String,
  pub result_expr: String,
  /// 수치 결과 (완전 치환된 경우)
  pub numeric_result: Option<f64>,
}

/// API 에러
#[derive(Clone, Debug)]
pub struct ApiError {
  pub code: String,
  pub message: String,
  pub details: Option<String>,
}

impl ApiError {
  pub fn parse_error(msg: impl Into<String>) -> Self {
    Self {
      code: "PARSE_ERROR".into(),
      message: msg.into(),
      details: None,
    }
  }

  pub fn ct_error(msg: impl Into<String>) -> Self {
    Self {
      code: "CT_ERROR".into(),
      message: msg.into(),
      details: None,
    }
  }

  pub fn lower_error(msg: impl Into<String>) -> Self {
    Self {
      code: "LOWER_ERROR".into(),
      message: msg.into(),
      details: None,
    }
  }

  pub fn internal_error(msg: impl Into<String>) -> Self {
    Self {
      code: "INTERNAL_ERROR".into(),
      message: msg.into(),
      details: None,
    }
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_resource_limits_default() {
    let limits = ResourceLimits::default();
    assert_eq!(limits.max_nodes, 10_000);
    assert_eq!(limits.max_iterations, 10);
    assert_eq!(limits.max_depth, 100);
  }

  #[test]
  fn test_resource_limits_strict() {
    let limits = ResourceLimits::strict();
    assert_eq!(limits.max_nodes, 1_000);
    assert_eq!(limits.max_iterations, 5);
  }

  #[test]
  fn test_resource_limits_relaxed() {
    let limits = ResourceLimits::relaxed();
    assert_eq!(limits.max_nodes, 100_000);
    assert_eq!(limits.max_iterations, 50);
  }

  #[test]
  fn test_resource_limits_builder() {
    let limits = ResourceLimits::new()
      .with_max_nodes(5_000)
      .with_max_iterations(20)
      .with_max_depth(200);
    assert_eq!(limits.max_nodes, 5_000);
    assert_eq!(limits.max_iterations, 20);
    assert_eq!(limits.max_depth, 200);
  }

  #[test]
  fn test_resource_limit_error_display() {
    let err = ResourceLimitError::NodeLimitExceeded {
      limit: 1000,
      actual: 2000,
    };
    let msg = format!("{}", err);
    assert!(msg.contains("2000"));
    assert!(msg.contains("1000"));
  }

  #[test]
  fn test_ct_context_spec() {
    let spec = CtContextSpec::new()
      .with_var("x", "m")
      .with_category("x", "position");
    assert_eq!(spec.units.get("x"), Some(&"m".to_string()));
    assert_eq!(spec.categories.get("x"), Some(&"position".to_string()));
  }

  #[test]
  fn test_tensor_context_spec() {
    let spec = TensorContextSpec::new().with_index("μ", "spacetime");
    assert_eq!(spec.index_spaces.get("μ"), Some(&"spacetime".to_string()));
  }

  #[test]
  fn test_sim_params() {
    let params = SimParams::new(0.0, 10.0, 100).with_var("v0", 5.0);
    assert_eq!(params.t_min, 0.0);
    assert_eq!(params.t_max, 10.0);
    assert_eq!(params.steps, 100);
    assert_eq!(params.variables.get("v0"), Some(&5.0));
  }

  #[test]
  fn test_api_error() {
    let err = ApiError::parse_error("Invalid syntax");
    assert_eq!(err.code, "PARSE_ERROR");
    assert_eq!(err.message, "Invalid syntax");
  }
}
