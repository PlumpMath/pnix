//! Optimization Pass Traits
//!
//! pnix-old의 opt_traits.rs를 pnix-new에 적응.
//!
//! 공통 최적화 패스 인터페이스 정의.
//! CT Optimizer, SSA Optimizer, FxCore Optimizer가 이 trait을 공유합니다.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 모든 최적화 패스는 구조 변환만 수행하며, 값을 계산하지 않는다.
//!
//! ## 사용 예시
//!
//! ```ignore
//! struct DeadCodePass;
//! impl OptimizationPass<SsaModule> for DeadCodePass {
//!     fn name(&self) -> &'static str { "dead_code_elimination" }
//!     fn run(&self, module: SsaModule) -> OptResult<SsaModule> {
//!         // 최적화 로직
//!         OptResult::changed(optimized_module)
//!     }
//! }
//! ```

use crate::effects::EffectZone;
use serde::{Deserialize, Serialize};

// ============================================================
// Optimization Result
// ============================================================

/// 최적화 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptResult<T> {
  /// 최적화된 결과
  pub value: T,
  /// 변경 여부
  pub changed: bool,
  /// 적용된 최적화 설명 (디버깅용)
  pub description: Option<String>,
}

impl<T> OptResult<T> {
  /// 변경 없음
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn unchanged(value: T) -> Self {
    Self {
      value,
      changed: false,
      description: None,
    }
  }

  /// 변경됨
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn changed(value: T) -> Self {
    Self {
      value,
      changed: true,
      description: None,
    }
  }

  /// 변경됨 + 설명
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn changed_with(value: T, description: impl Into<String>) -> Self {
    Self {
      value,
      changed: true,
      description: Some(description.into()),
    }
  }

  /// Map the value
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> OptResult<U> {
    OptResult {
      value: f(self.value),
      changed: self.changed,
      description: self.description,
    }
  }
}

// ============================================================
// Optimization Pass Trait
// ============================================================

/// 공통 최적화 패스 trait
///
/// 모든 최적화 패스는 이 trait을 구현합니다.
pub trait OptimizationPass<T> {
  /// 패스 이름 (디버깅/로깅용)
  fn name(&self) -> &'static str;

  /// 최적화 실행
  fn run(&self, input: T) -> OptResult<T>;

  /// 이 패스가 순수한지 (side-effect 없음)
  fn is_pure(&self) -> bool {
    true
  }

  /// 이 패스가 비용이 큰지 (실행 시간이 오래 걸림)
  fn is_expensive(&self) -> bool {
    false
  }
}

// ============================================================
// Optimization Pipeline
// ============================================================

/// 최적화 파이프라인: 여러 최적화 패스를 순차 실행하는 파이프라인
pub struct OptPipeline<T> {
  /// 최적화 패스 목록
  passes: Vec<Box<dyn OptimizationPass<T>>>,
  /// 최대 반복 횟수 (fixed-point까지)
  max_iterations: usize,
}

impl<T> OptPipeline<T> {
  /// 새로운 최적화 파이프라인 생성
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 생성만, 값 계산 없음
  pub fn new() -> Self {
    Self {
      passes: Vec::new(),
      max_iterations: 10,
    }
  }

  /// 패스 추가
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 추가만, 값 계산 없음
  pub fn add_pass(mut self, pass: impl OptimizationPass<T> + 'static) -> Self {
    self.passes.push(Box::new(pass));
    self
  }

  /// 최대 반복 횟수 설정
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 설정만, 값 계산 없음
  pub fn with_max_iterations(mut self, max: usize) -> Self {
    self.max_iterations = max;
    self
  }

  /// 모든 패스 한 번 실행
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn run_once(&self, mut input: T) -> PipelineResult<T> {
    let mut applied = Vec::new();
    let mut total_changed = false;

    for pass in &self.passes {
      let result = pass.run(input);
      if result.changed {
        total_changed = true;
        applied.push(pass.name().to_string());
        if let Some(desc) = result.description {
          applied.push(format!("  └─ {}", desc));
        }
      }
      input = result.value;
    }

    PipelineResult {
      value: input,
      changed: total_changed,
      applied_passes: applied,
      iterations: 1,
    }
  }

  /// 패스 개수
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn len(&self) -> usize {
    self.passes.len()
  }

  /// 패스가 비어있는지
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 접근만, 값 계산 없음
  pub fn is_empty(&self) -> bool {
    self.passes.is_empty()
  }
}

impl<T: Clone> OptPipeline<T> {
  /// Fixed-point까지 반복 실행
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn run_to_fixpoint(&self, mut input: T) -> PipelineResult<T> {
    let mut all_applied = Vec::new();
    let mut iterations = 0;

    loop {
      iterations += 1;
      let result = self.run_once(input.clone());
      all_applied.extend(result.applied_passes);

      if !result.changed || iterations >= self.max_iterations {
        return PipelineResult {
          value: result.value,
          changed: !all_applied.is_empty(),
          applied_passes: all_applied,
          iterations,
        };
      }

      input = result.value;
    }
  }
}

impl<T> Default for OptPipeline<T> {
  fn default() -> Self {
    Self::new()
  }
}

// ============================================================
// Pipeline Result
// ============================================================

/// 파이프라인 실행 결과: 최적화 파이프라인 실행 결과 구조
#[derive(Debug, Clone)]
pub struct PipelineResult<T> {
  /// 최적화된 결과 값
  pub value: T,
  /// 변경 여부
  pub changed: bool,
  /// 적용된 패스 이름 목록
  pub applied_passes: Vec<String>,
  /// 반복 횟수
  pub iterations: usize,
}

impl<T> PipelineResult<T> {
  /// Map the value
  ///
  /// ## 헌법 준수 (P0-1)
  ///
  /// 구조 변환만, 값 계산 없음
  pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> PipelineResult<U> {
    PipelineResult {
      value: f(self.value),
      changed: self.changed,
      applied_passes: self.applied_passes,
      iterations: self.iterations,
    }
  }
}

// ============================================================
// Helper Functions
// ============================================================

/// SSA 연산이 순수한지 판별 (CSE 가능 여부)
///
/// 순수 연산: 동일 입력 → 동일 출력, side-effect 없음
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn is_pure_ssa_op(op_name: &str) -> bool {
  matches!(
    op_name,
    "Add"
      | "Sub"
      | "Mul"
      | "Div"
      | "Mod"
      | "Neg"
      | "Floor"
      | "Ceil"
      | "Abs"
      | "Sqrt"
      | "Sin"
      | "Cos"
      | "Tan"
      | "Asin"
      | "Acos"
      | "Atan"
      | "Exp"
      | "Log"
      | "Lt"
      | "Gt"
      | "Le"
      | "Ge"
      | "Eq"
      | "Ne"
      | "And"
      | "Or"
      | "Not"
      | "Select"
      | "Phi"
  )
}

/// Effect zone 경계에서 융합 가능 여부
///
/// 같은 zone 내에서만 융합 가능
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 비교만, 값 계산 없음
pub fn can_fuse_across_zones(zone_a: EffectZone, zone_b: EffectZone) -> bool {
  // 같은 zone이거나 둘 다 Pure일 때만 융합 가능
  zone_a == zone_b || (zone_a == EffectZone::Pure && zone_b == EffectZone::Pure)
}

/// Zone 레벨에서 융합 가능 여부 (숫자 레벨 버전)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 비교만, 값 계산 없음
pub fn can_fuse_across_zone_levels(level_a: u8, level_b: u8) -> bool {
  // Zone levels: Pure(0) < Symbolic(1) < Frp(2) < Animation(3) < STM(4) < Interop(5) < World(6)
  // 같은 zone이거나 Pure 간에만 융합 가능
  level_a == level_b || (level_a == 0 && level_b == 0)
}

/// Functor law 적용 가능 여부
///
/// fmap f ∘ fmap g = fmap (f ∘ g)
/// 두 함수 모두 순수할 때만 적용
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn can_apply_functor_fusion(f_is_pure: bool, g_is_pure: bool) -> bool {
  f_is_pure && g_is_pure
}

/// Monad law 적용 가능 여부
///
/// return x >>= f = f x (left identity)
/// m >>= return = m (right identity)
/// (m >>= f) >>= g = m >>= (λx. f x >>= g) (associativity)
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn can_apply_monad_simplify(is_return: bool, is_bind: bool) -> bool {
  is_return || is_bind
}

/// 최적화가 안전한지 검사 (헌법 P0-1)
///
/// 구조 변환만 허용, 값 계산 금지
///
/// ## 헌법 준수 (P0-1)
///
/// 구조 검증만, 값 계산 없음
pub fn is_safe_optimization(pass_name: &str) -> bool {
  // 금지된 최적화 패스 (값을 계산하는 것들)
  let forbidden = [
    "constant_propagation", // 상수 전파 (값 계산)
    "partial_evaluation",   // 부분 평가 (값 계산)
    "strength_reduction",   // 강도 감소 (값 계산)
  ];

  !forbidden.contains(&pass_name)
}

// ============================================================
// Common Pass Implementations
// ============================================================

/// No-op 패스 (테스트용)
pub struct NoOpPass;

impl<T> OptimizationPass<T> for NoOpPass {
  fn name(&self) -> &'static str {
    "no_op"
  }

  fn run(&self, input: T) -> OptResult<T> {
    OptResult::unchanged(input)
  }
}

/// 조건부 패스 래퍼
pub struct ConditionalPass<T, P: OptimizationPass<T>, F: Fn(&T) -> bool> {
  inner: P,
  predicate: F,
  _phantom: std::marker::PhantomData<T>,
}

impl<T, P: OptimizationPass<T>, F: Fn(&T) -> bool> ConditionalPass<T, P, F> {
  pub fn new(inner: P, predicate: F) -> Self {
    Self {
      inner,
      predicate,
      _phantom: std::marker::PhantomData,
    }
  }
}

impl<T, P: OptimizationPass<T>, F: Fn(&T) -> bool> OptimizationPass<T>
  for ConditionalPass<T, P, F>
{
  fn name(&self) -> &'static str {
    self.inner.name()
  }

  fn run(&self, input: T) -> OptResult<T> {
    if (self.predicate)(&input) {
      self.inner.run(input)
    } else {
      OptResult::unchanged(input)
    }
  }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
  use super::*;

  struct DoublePass;
  impl OptimizationPass<i32> for DoublePass {
    fn name(&self) -> &'static str {
      "double"
    }
    fn run(&self, input: i32) -> OptResult<i32> {
      OptResult::changed(input * 2)
    }
  }

  struct AddOnePass;
  impl OptimizationPass<i32> for AddOnePass {
    fn name(&self) -> &'static str {
      "add_one"
    }
    fn run(&self, input: i32) -> OptResult<i32> {
      OptResult::changed(input + 1)
    }
  }

  struct ReducePass;
  impl OptimizationPass<i32> for ReducePass {
    fn name(&self) -> &'static str {
      "reduce"
    }
    fn run(&self, input: i32) -> OptResult<i32> {
      if input > 10 {
        OptResult::changed(input - 5)
      } else {
        OptResult::unchanged(input)
      }
    }
  }

  #[test]
  fn test_opt_result() {
    let unchanged: OptResult<i32> = OptResult::unchanged(5);
    assert!(!unchanged.changed);
    assert_eq!(unchanged.value, 5);

    let changed = OptResult::changed(10);
    assert!(changed.changed);
    assert_eq!(changed.value, 10);

    let with_desc = OptResult::changed_with(15, "doubled");
    assert!(with_desc.changed);
    assert_eq!(with_desc.description, Some("doubled".to_string()));
  }

  #[test]
  fn test_pipeline() {
    let pipeline = OptPipeline::new().add_pass(DoublePass).add_pass(AddOnePass);

    let result = pipeline.run_once(5);
    assert_eq!(result.value, 11); // (5 * 2) + 1
    assert!(result.changed);
    assert_eq!(result.applied_passes.len(), 2);
  }

  #[test]
  fn test_pipeline_fixpoint() {
    let pipeline = OptPipeline::new()
      .add_pass(ReducePass)
      .with_max_iterations(20);

    let result = pipeline.run_to_fixpoint(25);
    assert_eq!(result.value, 10); // 25 -> 20 -> 15 -> 10 (stops at <=10)
    assert!(result.changed);
    assert_eq!(result.iterations, 4); // 3 reductions + 1 unchanged
  }

  #[test]
  fn test_no_op_pass() {
    let pass = NoOpPass;
    let result = pass.run(42);
    assert!(!result.changed);
    assert_eq!(result.value, 42);
  }

  #[test]
  fn test_conditional_pass() {
    let pass = ConditionalPass::new(DoublePass, |&x| x > 5);

    // Should apply
    let result = pass.run(10);
    assert!(result.changed);
    assert_eq!(result.value, 20);

    // Should not apply
    let result = pass.run(3);
    assert!(!result.changed);
    assert_eq!(result.value, 3);
  }

  #[test]
  fn test_pure_ssa_op() {
    assert!(is_pure_ssa_op("Add"));
    assert!(is_pure_ssa_op("Sin"));
    assert!(is_pure_ssa_op("Phi"));
    assert!(!is_pure_ssa_op("LoadSignal"));
    assert!(!is_pure_ssa_op("Unknown"));
  }

  #[test]
  fn test_zone_fusion() {
    assert!(can_fuse_across_zones(EffectZone::Pure, EffectZone::Pure));
    assert!(!can_fuse_across_zones(EffectZone::Pure, EffectZone::Frp));
    assert!(can_fuse_across_zones(EffectZone::Frp, EffectZone::Frp));
  }

  #[test]
  fn test_zone_level_fusion() {
    assert!(can_fuse_across_zone_levels(0, 0)); // Pure + Pure
    assert!(!can_fuse_across_zone_levels(0, 1)); // Pure + Symbolic
    assert!(can_fuse_across_zone_levels(2, 2)); // Frp + Frp
  }

  #[test]
  fn test_functor_fusion() {
    assert!(can_apply_functor_fusion(true, true));
    assert!(!can_apply_functor_fusion(true, false));
    assert!(!can_apply_functor_fusion(false, true));
  }

  #[test]
  fn test_monad_simplify() {
    assert!(can_apply_monad_simplify(true, false)); // return
    assert!(can_apply_monad_simplify(false, true)); // bind
    assert!(can_apply_monad_simplify(true, true)); // both
    assert!(!can_apply_monad_simplify(false, false)); // neither
  }

  #[test]
  fn test_safe_optimization() {
    assert!(is_safe_optimization("dead_code_elimination"));
    assert!(is_safe_optimization("identity_elimination"));
    assert!(!is_safe_optimization("constant_propagation"));
    assert!(!is_safe_optimization("partial_evaluation"));
  }
}
