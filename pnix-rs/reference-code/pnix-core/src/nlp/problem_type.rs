//! Mathematical Problem Type Classification
//!
//! pnix-old의 symbolic_core/src/llm/problem.rs에서 마이그레이션 (legacy path).
//!
//! ## 헌법 준수
//!
//! 순수 분류 enum, 값 연산 없음. **pnix 는 LLM 없이 작동하는 deterministic AI
//! substrate** 다. 이전 doc 에 적힌 "LLM 연동 시 문제 유형 전달" 항목은 owner-law
//! 위반으로 *superseded* — substrate 안에 LLM 연동 lane 은 없다 (`CLAUDE.md`
//! OWNER-LAW CONSTITUTION).
//!
//! ## 사용 목적
//!
//! - 수학 문제 유형 분류
//! - NLP 파이프라인에서 문제 타입 감지
//! - downstream solver / route adapter 에 문제 유형 전달 (deterministic)

use serde::{Deserialize, Serialize};

/// 수학 문제 유형: 자연어로 표현된 수학 문제의 유형 분류
///
/// # 예시
/// ```ignore
/// use pnix_core::nlp::ProblemType;
///
/// let problem = ProblemType::infer("Find the derivative of x^2");
/// assert_eq!(problem, ProblemType::Differentiation);
/// ```
/// 헌법 P0-1 준수: 순수 분류 enum, 값 연산 없음
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProblemType {
  /// 미분 문제 (derivative, differentiate, d/dx)
  Differentiation,
  /// 적분 문제 (integral, integrate)
  Integration,
  /// 방정식 풀이 (solve, equation, find x)
  Equation,
  /// 단순화 (simplify, reduce)
  Simplification,
  /// 전개 (expand, distribute)
  Expansion,
  /// 인수분해 (factor)
  Factorization,
  /// 극한 (limit, lim)
  Limit,
  /// 시뮬레이션/시각화 (simulate, plot, graph)
  Simulation,
  /// 텐서 연산 (tensor, contract, index)
  Tensor,
  /// 복합 문제 (여러 단계)
  Composite,
  /// 알 수 없음
  #[default]
  Unknown,
}

impl ProblemType {
  /// 텍스트에서 문제 타입 추론
  ///
  /// 대소문자 구분 없이 키워드로 분류합니다.
  ///
  /// # 예시
  /// ```ignore
  /// use pnix_core::nlp::ProblemType;
  ///
  /// assert_eq!(ProblemType::infer("Find the derivative"), ProblemType::Differentiation);
  /// assert_eq!(ProblemType::infer("Simplify x + x"), ProblemType::Simplification);
  /// ```
  pub fn infer(text: &str) -> Self {
    let lower = text.to_lowercase();

    if lower.contains("derivative") || lower.contains("differentiate") || lower.contains("d/dx") {
      ProblemType::Differentiation
    } else if lower.contains("integral") || lower.contains("integrate") {
      ProblemType::Integration
    } else if lower.contains("solve") || lower.contains("equation") || lower.contains("find x") {
      ProblemType::Equation
    } else if lower.contains("simplify") || lower.contains("reduce") {
      ProblemType::Simplification
    } else if lower.contains("expand") || lower.contains("distribute") {
      ProblemType::Expansion
    } else if lower.contains("factor") {
      ProblemType::Factorization
    } else if lower.contains("limit") || lower.contains("lim ") {
      ProblemType::Limit
    } else if lower.contains("simulate") || lower.contains("plot") || lower.contains("graph") {
      ProblemType::Simulation
    } else if lower.contains("tensor") || lower.contains("contract") || lower.contains("index") {
      ProblemType::Tensor
    } else {
      ProblemType::Unknown
    }
  }

  /// 타입 이름 반환
  pub fn name(&self) -> &'static str {
    match self {
      ProblemType::Differentiation => "differentiation",
      ProblemType::Integration => "integration",
      ProblemType::Equation => "equation",
      ProblemType::Simplification => "simplification",
      ProblemType::Expansion => "expansion",
      ProblemType::Factorization => "factorization",
      ProblemType::Limit => "limit",
      ProblemType::Simulation => "simulation",
      ProblemType::Tensor => "tensor",
      ProblemType::Composite => "composite",
      ProblemType::Unknown => "unknown",
    }
  }

  /// 순수 수학 문제인지 확인
  ///
  /// 미분, 적분, 방정식, 단순화, 전개, 인수분해, 극한은 순수 수학
  pub fn is_pure_math(&self) -> bool {
    matches!(
      self,
      ProblemType::Differentiation
        | ProblemType::Integration
        | ProblemType::Equation
        | ProblemType::Simplification
        | ProblemType::Expansion
        | ProblemType::Factorization
        | ProblemType::Limit
    )
  }

  /// 심볼릭 연산이 필요한지 확인
  pub fn needs_symbolic(&self) -> bool {
    matches!(
      self,
      ProblemType::Differentiation
        | ProblemType::Integration
        | ProblemType::Simplification
        | ProblemType::Expansion
        | ProblemType::Factorization
        | ProblemType::Tensor
    )
  }

  /// 수치 연산이 필요한지 확인
  pub fn needs_numeric(&self) -> bool {
    matches!(
      self,
      ProblemType::Equation | ProblemType::Limit | ProblemType::Simulation
    )
  }

  /// 모든 문제 타입 반환
  pub fn all() -> &'static [ProblemType] {
    &[
      ProblemType::Differentiation,
      ProblemType::Integration,
      ProblemType::Equation,
      ProblemType::Simplification,
      ProblemType::Expansion,
      ProblemType::Factorization,
      ProblemType::Limit,
      ProblemType::Simulation,
      ProblemType::Tensor,
      ProblemType::Composite,
      ProblemType::Unknown,
    ]
  }
}

impl std::fmt::Display for ProblemType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.name())
  }
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_infer_differentiation() {
    assert_eq!(
      ProblemType::infer("Find the derivative of x^2"),
      ProblemType::Differentiation
    );
    assert_eq!(
      ProblemType::infer("differentiate sin(x)"),
      ProblemType::Differentiation
    );
    assert_eq!(
      ProblemType::infer("Calculate d/dx of x^3"),
      ProblemType::Differentiation
    );
  }

  #[test]
  fn test_infer_integration() {
    assert_eq!(
      ProblemType::infer("Integrate x^2"),
      ProblemType::Integration
    );
    assert_eq!(
      ProblemType::infer("Find the integral"),
      ProblemType::Integration
    );
  }

  #[test]
  fn test_infer_equation() {
    assert_eq!(ProblemType::infer("Solve x^2 = 4"), ProblemType::Equation);
    assert_eq!(
      ProblemType::infer("find x in equation"),
      ProblemType::Equation
    );
  }

  #[test]
  fn test_infer_simplification() {
    assert_eq!(
      ProblemType::infer("Simplify x + x"),
      ProblemType::Simplification
    );
    assert_eq!(
      ProblemType::infer("reduce the expression"),
      ProblemType::Simplification
    );
  }

  #[test]
  fn test_infer_expansion() {
    assert_eq!(ProblemType::infer("Expand (a+b)^2"), ProblemType::Expansion);
    assert_eq!(
      ProblemType::infer("Distribute 2(x+1)"),
      ProblemType::Expansion
    );
  }

  #[test]
  fn test_infer_factorization() {
    assert_eq!(
      ProblemType::infer("Factor x^2 - 1"),
      ProblemType::Factorization
    );
  }

  #[test]
  fn test_infer_limit() {
    assert_eq!(
      ProblemType::infer("Find the limit as x->0"),
      ProblemType::Limit
    );
    assert_eq!(ProblemType::infer("lim x->infinity"), ProblemType::Limit);
  }

  #[test]
  fn test_infer_simulation() {
    assert_eq!(
      ProblemType::infer("simulate the motion"),
      ProblemType::Simulation
    );
    assert_eq!(
      ProblemType::infer("plot the function"),
      ProblemType::Simulation
    );
    assert_eq!(ProblemType::infer("graph y=x^2"), ProblemType::Simulation);
  }

  #[test]
  fn test_infer_tensor() {
    assert_eq!(ProblemType::infer("tensor product"), ProblemType::Tensor);
    assert_eq!(ProblemType::infer("contract indices"), ProblemType::Tensor);
  }

  #[test]
  fn test_infer_unknown() {
    assert_eq!(ProblemType::infer("hello world"), ProblemType::Unknown);
    assert_eq!(ProblemType::infer(""), ProblemType::Unknown);
  }

  #[test]
  fn test_name() {
    assert_eq!(ProblemType::Differentiation.name(), "differentiation");
    assert_eq!(ProblemType::Unknown.name(), "unknown");
  }

  #[test]
  fn test_is_pure_math() {
    assert!(ProblemType::Differentiation.is_pure_math());
    assert!(ProblemType::Integration.is_pure_math());
    assert!(!ProblemType::Simulation.is_pure_math());
    assert!(!ProblemType::Unknown.is_pure_math());
  }

  #[test]
  fn test_needs_symbolic() {
    assert!(ProblemType::Differentiation.needs_symbolic());
    assert!(ProblemType::Tensor.needs_symbolic());
    assert!(!ProblemType::Equation.needs_symbolic());
  }

  #[test]
  fn test_needs_numeric() {
    assert!(ProblemType::Equation.needs_numeric());
    assert!(ProblemType::Simulation.needs_numeric());
    assert!(!ProblemType::Differentiation.needs_numeric());
  }

  #[test]
  fn test_all() {
    let all = ProblemType::all();
    assert_eq!(all.len(), 11);
    assert!(all.contains(&ProblemType::Differentiation));
    assert!(all.contains(&ProblemType::Unknown));
  }

  #[test]
  fn test_display() {
    assert_eq!(format!("{}", ProblemType::Integration), "integration");
  }

  #[test]
  fn test_default() {
    assert_eq!(ProblemType::default(), ProblemType::Unknown);
  }

  #[test]
  fn test_serde_roundtrip() {
    let pt = ProblemType::Differentiation;
    let json = serde_json::to_string(&pt).unwrap();
    assert_eq!(json, "\"differentiation\""); // snake_case

    let restored: ProblemType = serde_json::from_str(&json).unwrap();
    assert_eq!(pt, restored);
  }
}
