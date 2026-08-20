//! egg rewrite 규칙 정의
//!
//! pnix-old의 symbolic_core/rewrite/egg_rules.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 규칙 정의만, 실행 없음
//!
//! ## 참고
//!
//! egg 라이브러리의 `rewrite!` 매크로를 사용하는 규칙 정의입니다.
//! 실제 사용 시에는 egg 의존성을 추가하고 매크로를 활성화해야 합니다.
//!
//! 현재는 구조 정의만 포함하여 헌법을 준수합니다.

/// Rewrite 규칙 타입
///
/// 실제 사용 예시:
/// ```rust,ignore
/// use egg::{rewrite as rw, Rewrite};
/// // SymLang language definition is intentionally out-of-tree/feature-gated.
/// // See `docs/` for the current stance on egg integration.
///
/// pub fn basic_rules() -> Vec<Rewrite<SymLang, ()>> {
///     vec![
///         rw!("add-comm"; "(+ ?a ?b)" => "(+ ?b ?a)"),
///         rw!("mul-comm"; "(* ?a ?b)" => "(* ?b ?a)"),
///         // ...
///     ]
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteRule {
  /// 규칙 이름
  pub name: String,
  /// 패턴 (왼쪽)
  pub pattern: String,
  /// 치환 (오른쪽)
  pub replacement: String,
  /// 규칙 카테고리
  pub category: RuleCategory,
  /// 규칙 적용 전제 조건
  pub guard: RuleGuard,
}

/// 규칙 적용 전제 조건
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleGuard {
  /// 스칼라 대수 전용 (텐서/비가환 연산 제외)
  ScalarOnly,
}

/// 규칙 카테고리
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleCategory {
  /// 기본 대수 규칙
  Basic,
  /// 삼각함수 규칙
  Trig,
  /// 지수/로그 규칙
  ExpLog,
  /// 미분 규칙
  Diff,
}

impl RewriteRule {
  /// 새 규칙 생성
  pub fn new(
    name: impl Into<String>,
    pattern: impl Into<String>,
    replacement: impl Into<String>,
    category: RuleCategory,
    guard: RuleGuard,
  ) -> Self {
    Self {
      name: name.into(),
      pattern: pattern.into(),
      replacement: replacement.into(),
      category,
      guard,
    }
  }
}

/// 기본 대수 규칙 목록
///
/// 포함 규칙:
/// - 교환법칙: add-comm, mul-comm
/// - 결합법칙: add-assoc, mul-assoc
/// - 항등원: add-zero, mul-one
/// - 영원: mul-zero
/// - 역원: add-neg
/// - 분배법칙: distrib, factor
/// - 거듭제곱: pow-zero, pow-one, pow-add
// NOTE: Rules assume scalar algebra. Non-commutative tensor operations must be excluded.
pub const BASIC_RULES: &[(&str, &str, &str)] = &[
  // 교환법칙
  ("add-comm", "(+ ?a ?b)", "(+ ?b ?a)"),
  ("mul-comm", "(* ?a ?b)", "(* ?b ?a)"),
  // 결합법칙
  ("add-assoc", "(+ (+ ?a ?b) ?c)", "(+ ?a (+ ?b ?c))"),
  ("mul-assoc", "(* (* ?a ?b) ?c)", "(* ?a (* ?b ?c))"),
  // 항등원
  ("add-zero", "(+ ?a 0)", "?a"),
  ("mul-one", "(* ?a 1)", "?a"),
  // 영원
  ("mul-zero", "(* ?a 0)", "0"),
  // 역원
  ("add-neg", "(+ ?a (- ?a))", "0"),
  // 분배법칙
  ("distrib", "(* ?a (+ ?b ?c))", "(+ (* ?a ?b) (* ?a ?c))"),
  ("factor", "(+ (* ?a ?b) (* ?a ?c))", "(* ?a (+ ?b ?c))"),
  // 거듭제곱
  ("pow-zero", "(^ ?a 0)", "1"),
  ("pow-one", "(^ ?a 1)", "?a"),
  ("pow-add", "(* (^ ?a ?b) (^ ?a ?c))", "(^ ?a (+ ?b ?c))"),
];

/// 삼각함수 규칙 목록
///
/// 포함 규칙:
/// - 피타고라스 항등식: pythagorean
/// - 특수값: sin-zero, cos-zero
pub const TRIG_RULES: &[(&str, &str, &str)] = &[
  // sin²x + cos²x = 1
  ("pythagorean", "(+ (^ (sin ?x) 2) (^ (cos ?x) 2))", "1"),
  // sin(0) = 0, cos(0) = 1
  ("sin-zero", "(sin 0)", "0"),
  ("cos-zero", "(cos 0)", "1"),
];

/// 지수/로그 규칙 목록
///
/// 포함 규칙:
/// - 특수값: exp-zero, log-one
/// - 역함수 관계: exp-log, log-exp
pub const EXP_LOG_RULES: &[(&str, &str, &str)] = &[
  ("exp-zero", "(exp 0)", "1"),
  ("log-one", "(log 1)", "0"),
  ("exp-log", "(exp (log ?x))", "?x"),
  ("log-exp", "(log (exp ?x))", "?x"),
];

/// 미분 규칙 목록
///
/// 포함 규칙:
/// - 기본 미분 규칙들
pub const DIFF_RULES: &[(&str, &str, &str)] = &[
  // 상수 미분
  ("diff-const", "(diff ?c ?x)", "0"), // c는 상수
  // 변수 미분
  ("diff-var", "(diff ?x ?x)", "1"),
  // 합 미분
  (
    "diff-add",
    "(diff (+ ?a ?b) ?x)",
    "(+ (diff ?a ?x) (diff ?b ?x))",
  ),
  // 곱 미분
  (
    "diff-mul",
    "(diff (* ?a ?b) ?x)",
    "(+ (* (diff ?a ?x) ?b) (* ?a (diff ?b ?x)))",
  ),
  // 거듭제곱 미분
  (
    "diff-pow",
    "(diff (^ ?a ?n) ?x)",
    "(* (* ?n (^ ?a (- ?n 1))) (diff ?a ?x))",
  ), // n은 상수
];

/// 모든 규칙 반환
pub fn all_rules() -> Vec<RewriteRule> {
  let mut rules = vec![];

  for (name, pattern, replacement) in BASIC_RULES {
    rules.push(RewriteRule::new(
      *name,
      *pattern,
      *replacement,
      RuleCategory::Basic,
      RuleGuard::ScalarOnly,
    ));
  }

  for (name, pattern, replacement) in TRIG_RULES {
    rules.push(RewriteRule::new(
      *name,
      *pattern,
      *replacement,
      RuleCategory::Trig,
      RuleGuard::ScalarOnly,
    ));
  }

  for (name, pattern, replacement) in EXP_LOG_RULES {
    rules.push(RewriteRule::new(
      *name,
      *pattern,
      *replacement,
      RuleCategory::ExpLog,
      RuleGuard::ScalarOnly,
    ));
  }

  for (name, pattern, replacement) in DIFF_RULES {
    rules.push(RewriteRule::new(
      *name,
      *pattern,
      *replacement,
      RuleCategory::Diff,
      RuleGuard::ScalarOnly,
    ));
  }

  rules
}

/// 이름으로 규칙 찾기
pub fn find_rule(name: &str) -> Option<RewriteRule> {
  all_rules().into_iter().find(|r| r.name == name)
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_basic_rules_count() {
    assert_eq!(BASIC_RULES.len(), 13);
  }

  #[test]
  fn test_trig_rules_count() {
    assert_eq!(TRIG_RULES.len(), 3);
  }

  #[test]
  fn test_exp_log_rules_count() {
    assert_eq!(EXP_LOG_RULES.len(), 4);
  }

  #[test]
  fn test_diff_rules_count() {
    assert_eq!(DIFF_RULES.len(), 5);
  }

  #[test]
  fn test_all_rules() {
    let rules = all_rules();
    assert!(rules.len() >= 25); // BASIC + TRIG + EXP_LOG + DIFF
  }

  #[test]
  fn test_find_rule() {
    let rule = find_rule("add-comm");
    assert!(rule.is_some());
    let rule = rule.unwrap();
    assert_eq!(rule.name, "add-comm");
    assert_eq!(rule.category, RuleCategory::Basic);
  }

  #[test]
  fn test_find_nonexistent_rule() {
    let rule = find_rule("nonexistent");
    assert!(rule.is_none());
  }

  #[test]
  fn test_rule_categories() {
    let add_comm = find_rule("add-comm").unwrap();
    assert_eq!(add_comm.category, RuleCategory::Basic);
    assert_eq!(add_comm.guard, RuleGuard::ScalarOnly);

    let pythagorean = find_rule("pythagorean").unwrap();
    assert_eq!(pythagorean.category, RuleCategory::Trig);

    let exp_zero = find_rule("exp-zero").unwrap();
    assert_eq!(exp_zero.category, RuleCategory::ExpLog);

    let diff_add = find_rule("diff-add").unwrap();
    assert_eq!(diff_add.category, RuleCategory::Diff);
  }
}
