//! FxCore 최적화 패스 구조 정의
//!
//! pnix-old의 meaning_core/src/fx_opt.rs에서 마이그레이션.
//!
//! ## 헌법 준수 (P0-1)
//!
//! 구조 변환만, 값 계산 로직은 executor로 이동
//!
//! ## 설계 원칙
//!
//! - FxOptPass trait: 최적화 패스 인터페이스
//! - FxConstFold: 상수 폴딩 (값 계산은 executor에서)
//! - FxAlgebraicSimplify: 대수적 단순화 (구조 변환만)
//! - FxFrpPatternLift: FRP 패턴 리프팅 (구조 변환만)

use super::core_expr::FxCoreExpr;
use crate::diagnostics::Diagnostics;
use crate::fx::meaning_op::{MeaningMeta, MeaningOpId};
use serde::{Deserialize, Serialize};

/// 최적화 패스 트레잇
///
/// 실제 실행 로직은 executor에서 구현합니다.
pub trait FxOptPass {
  /// 최적화 실행 (구조 변환만, 값 계산 없음)
  ///
  /// **주의**: 이 메서드는 구조 변환만 수행해야 합니다.
  /// 값 계산이 필요한 경우 executor에서 구현하세요.
  fn run(&self, expr: FxCoreExpr) -> FxCoreExpr;

  /// 진단을 수집하며 최적화 실행 (구조 변환만)
  fn run_with_diags(&self, expr: FxCoreExpr, diags: &mut Diagnostics) -> FxCoreExpr {
    let _ = diags;
    self.run(expr)
  }
}

/// 상수 폴딩 패스 구조
///
/// 컴파일 타임에 상수 표현식을 평가합니다.
/// 실제 실행 로직은 executor에서 구현합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FxConstFold;

impl FxOptPass for FxConstFold {
  fn run(&self, expr: FxCoreExpr) -> FxCoreExpr {
    // **주의**: 상수 폴딩은 값 계산이 필요하므로 executor에서 구현됩니다.
    // pnix-core에서는 구조만 정의하고 패스스루합니다.
    //
    // Executor 구현 예시:
    // - ConstInt(a) + ConstInt(b) → ConstInt(a + b) (executor에서)
    // - ConstFloat(x).sin() → ConstFloat(x.sin()) (executor에서)
    expr
  }
}

/// 대수적 단순화 패스 구조
///
/// 대수 법칙을 활용한 표현식 단순화 (예: x + 0 = x, x * 1 = x).
/// 실제 실행 로직은 executor에서 구현합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FxAlgebraicSimplify;

impl FxOptPass for FxAlgebraicSimplify {
  fn run(&self, expr: FxCoreExpr) -> FxCoreExpr {
    algebraic_simplify(expr)
  }

  fn run_with_diags(&self, expr: FxCoreExpr, diags: &mut Diagnostics) -> FxCoreExpr {
    algebraic_simplify_with_diags(expr, diags)
  }
}

/// 대수적 단순화 (구조 변환만, 헌법 준수)
///
/// 대수 법칙을 활용한 표현식 단순화:
/// - x + 0 = x
/// - x * 1 = x
/// - x * 0 = 0
/// - neg(neg(x)) = x
/// - floor(floor(x)) = floor(x)
///
/// **헌법 준수**: 값 계산 없이 구조 변환만 수행합니다.
/// 표현식에 사이드 이펙트(Throw/Interop)가 포함되어 있는지 확인
fn has_side_effects(expr: &FxCoreExpr) -> bool {
  match expr {
    FxCoreExpr::Throw { .. } | FxCoreExpr::Interop { .. } => true,
    FxCoreExpr::Binary { lhs, rhs, .. } => has_side_effects(lhs) || has_side_effects(rhs),
    FxCoreExpr::Unary { arg, .. } => has_side_effects(arg),
    FxCoreExpr::If {
      cond, then_, else_, ..
    } => has_side_effects(cond) || has_side_effects(then_) || has_side_effects(else_),
    FxCoreExpr::Let { value, body, .. } => has_side_effects(value) || has_side_effects(body),
    FxCoreExpr::List(items) => items.iter().any(has_side_effects),
    FxCoreExpr::AttrSet(fields) => fields.iter().any(|(_, field)| has_side_effects(field)),
    FxCoreExpr::Derived { args, .. } => args.iter().any(has_side_effects),
    FxCoreExpr::Select { expr, .. } => has_side_effects(expr),
    FxCoreExpr::Construct { args, .. } => args.iter().any(has_side_effects),
    FxCoreExpr::Lambda { body, .. } => has_side_effects(body),
    _ => false,
  }
}

fn algebraic_simplify(expr: FxCoreExpr) -> FxCoreExpr {
  let mut diags = None;
  algebraic_simplify_inner(expr, &mut diags)
}

fn algebraic_simplify_with_diags(expr: FxCoreExpr, diags: &mut Diagnostics) -> FxCoreExpr {
  let mut diags = Some(diags);
  algebraic_simplify_inner(expr, &mut diags)
}

// MEDIUM: 대수 단순화 x*0 타입 결정 취약 수정 완료
// sin(y)*0이 ConstInt(0) 반환 문제 해결
// Float 연산(Unary, Binary, Derived)을 감지하여 타입 정보 보존
fn algebraic_simplify_inner(expr: FxCoreExpr, diags: &mut Option<&mut Diagnostics>) -> FxCoreExpr {
  // Float 연산인지 확인하는 헬퍼 함수
  fn is_float_expr(expr: &FxCoreExpr) -> bool {
    match expr {
      FxCoreExpr::ConstFloat(_) => true,
      FxCoreExpr::Unary { meta, .. } => {
        // Float 반환 연산: sin, cos, tan, exp, ln, sqrt, floor, ceil, abs, neg
        matches!(
          meta.op,
          MeaningOpId::Sin
            | MeaningOpId::Cos
            | MeaningOpId::Tan
            | MeaningOpId::Exp
            | MeaningOpId::Ln
            | MeaningOpId::Sqrt
            | MeaningOpId::Floor
            | MeaningOpId::Ceil
            | MeaningOpId::Abs
            | MeaningOpId::Neg
        )
      }
      FxCoreExpr::Binary { meta, .. } => {
        // Float 반환 연산: div (나눗셈은 Float 반환)
        matches!(meta.op, MeaningOpId::Div)
      }
      FxCoreExpr::Derived { .. } => {
        // Derived 연산은 대부분 Float 반환 (sin, cos 등)
        true
      }
      _ => false,
    }
  }
  match expr {
    FxCoreExpr::Binary { meta, lhs, rhs } => {
      let lhs = algebraic_simplify_inner(*lhs, diags);
      let rhs = algebraic_simplify_inner(*rhs, diags);

      match &meta.op {
        // x + 0 = x, 0 + x = x
        MeaningOpId::Add => {
          if is_zero(&rhs) {
            return lhs;
          }
          if is_zero(&lhs) {
            return rhs;
          }
        }

        // x - 0 = x
        MeaningOpId::Sub => {
          if is_zero(&rhs) {
            return lhs;
          }
        }

        // x * 1 = x, 1 * x = x
        MeaningOpId::Mul => {
          if is_one(&rhs) {
            return lhs;
          }
          if is_one(&lhs) {
            return rhs;
          }
          // x * 0 = 0, 0 * x = 0 (타입 보존)
          // 단, 사이드 이펙트(Throw/Interop)가 있는 경우 최적화하지 않음
          if is_zero(&rhs) && !has_side_effects(&lhs) {
            // MEDIUM: 대수 단순화 x*0 타입 결정 취약 수정 완료
            // lhs나 rhs 중 하나라도 float이면 ConstFloat(0.0) 반환, 둘 다 int이면 ConstInt(0)
            // Float 연산(Unary, Binary, Derived)도 Float로 간주하여 타입 정보 보존
            return if matches!(&lhs, FxCoreExpr::ConstFloat(_))
              || matches!(&rhs, FxCoreExpr::ConstFloat(_))
              || is_float_expr(&lhs)
              || is_float_expr(&rhs)
            {
              FxCoreExpr::ConstFloat(0.0)
            } else {
              FxCoreExpr::ConstInt(0)
            };
          }
          if is_zero(&lhs) && !has_side_effects(&rhs) {
            // MEDIUM: 대수 단순화 x*0 타입 결정 취약 수정 완료
            // lhs나 rhs 중 하나라도 float이면 ConstFloat(0.0) 반환, 둘 다 int이면 ConstInt(0)
            // Float 연산(Unary, Binary, Derived)도 Float로 간주하여 타입 정보 보존
            return if matches!(&lhs, FxCoreExpr::ConstFloat(_))
              || matches!(&rhs, FxCoreExpr::ConstFloat(_))
              || is_float_expr(&lhs)
              || is_float_expr(&rhs)
            {
              FxCoreExpr::ConstFloat(0.0)
            } else {
              FxCoreExpr::ConstInt(0)
            };
          }
        }

        // x / 1 = x
        MeaningOpId::Div => {
          // Policy: compile-time constant division by zero emits a diagnostic,
          // while preserving structure (no evaluation).
          if is_zero(&rhs) {
            if let Some(diags) = diags.as_deref_mut() {
              diags.push("error: division by zero in constant expression", None);
            }
          }
          if is_one(&rhs) {
            return lhs;
          }
        }

        _ => {}
      }

      FxCoreExpr::Binary {
        meta,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
      }
    }

    // Unary: recursive + simplifications
    FxCoreExpr::Unary { meta, arg } => {
      let arg = algebraic_simplify_inner(*arg, diags);

      // floor(floor(x)) = floor(x), ceil(ceil(x)) = ceil(x)
      if matches!(meta.op, MeaningOpId::Floor | MeaningOpId::Ceil) {
        if let FxCoreExpr::Unary {
          meta: inner_meta, ..
        } = &arg
        {
          if inner_meta.op == meta.op {
            return arg;
          }
        }
      }

      // neg(neg(x)) = x
      if meta.op == MeaningOpId::Neg {
        if let FxCoreExpr::Unary {
          meta: inner_meta,
          arg: inner_arg,
        } = &arg
        {
          if inner_meta.op == MeaningOpId::Neg {
            return *inner_arg.clone();
          }
        }
      }

      FxCoreExpr::Unary {
        meta,
        arg: Box::new(arg),
      }
    }

    // If: recursive
    FxCoreExpr::If { cond, then_, else_ } => FxCoreExpr::If {
      cond: Box::new(algebraic_simplify_inner(*cond, diags)),
      then_: Box::new(algebraic_simplify_inner(*then_, diags)),
      else_: Box::new(algebraic_simplify_inner(*else_, diags)),
    },

    // Derived: recursive
    FxCoreExpr::Derived { meta, args } => FxCoreExpr::Derived {
      meta,
      args: args
        .into_iter()
        .map(|arg| algebraic_simplify_inner(arg, diags))
        .collect(),
    },

    // List: recursive
    FxCoreExpr::List(items) => FxCoreExpr::List(
      items
        .into_iter()
        .map(|item| algebraic_simplify_inner(item, diags))
        .collect(),
    ),

    // AttrSet: recursive
    FxCoreExpr::AttrSet(pairs) => FxCoreExpr::AttrSet(
      pairs
        .into_iter()
        .map(|(k, v)| (k, algebraic_simplify_inner(v, diags)))
        .collect(),
    ),

    other => other,
  }
}

/// 표현식이 0인지 확인 (구조 검사만, 값 계산 없음)
///
/// **헌법 준수**: 값 계산 없이 구조만 확인합니다.
fn is_zero(expr: &FxCoreExpr) -> bool {
  matches!(expr, FxCoreExpr::ConstInt(0)) || matches!(expr, FxCoreExpr::ConstFloat(x) if *x == 0.0)
}

/// 표현식이 1인지 확인 (구조 검사만, 값 계산 없음)
///
/// **헌법 준수**: 값 계산 없이 구조만 확인합니다.
fn is_one(expr: &FxCoreExpr) -> bool {
  matches!(expr, FxCoreExpr::ConstInt(1)) || matches!(expr, FxCoreExpr::ConstFloat(x) if *x == 1.0)
}

/// FRP 패턴 리프트 패스 구조
///
/// FRP 패턴을 고수준 연산으로 리프트합니다 (예: floor(time) % 60 → SecondsFromTime).
/// 실제 실행 로직은 executor에서 구현합니다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FxFrpPatternLift;

impl FxOptPass for FxFrpPatternLift {
  fn run(&self, expr: FxCoreExpr) -> FxCoreExpr {
    frp_pattern_lift(expr)
  }
}

/// FRP 패턴 리프팅 (구조 변환만, 헌법 준수)
///
/// FRP 패턴을 고수준 연산으로 리프트:
/// - floor(time) % 60 → SecondsFromTime
/// - floor(time / 60) % 60 → MinutesFromTime
///
/// **헌법 준수**: 값 계산 없이 구조 변환만 수행합니다.
fn frp_pattern_lift(expr: FxCoreExpr) -> FxCoreExpr {
  match &expr {
    // Pattern: floor(time) % 60 → SecondsFromTime
    FxCoreExpr::Binary { meta, lhs, rhs } if meta.op == MeaningOpId::Mod => {
      if let FxCoreExpr::Unary {
        meta: unary_meta,
        arg,
      } = lhs.as_ref()
      {
        if unary_meta.op == MeaningOpId::Floor {
          if let FxCoreExpr::ParamSysTime = arg.as_ref() {
            if let FxCoreExpr::ConstInt(60) = rhs.as_ref() {
              return FxCoreExpr::Derived {
                meta: MeaningMeta::continuous(MeaningOpId::SecondsFromTime),
                args: vec![],
              };
            }
          }
        }
      }
    }

    // Pattern: floor(time / 60) % 60 → MinutesFromTime
    FxCoreExpr::Binary { meta, lhs, rhs } if meta.op == MeaningOpId::Mod => {
      if let FxCoreExpr::Unary {
        meta: unary_meta,
        arg,
      } = lhs.as_ref()
      {
        if unary_meta.op == MeaningOpId::Floor {
          if let FxCoreExpr::Binary {
            meta: div_meta,
            lhs: div_lhs,
            rhs: div_rhs,
          } = arg.as_ref()
          {
            if div_meta.op == MeaningOpId::Div {
              if let FxCoreExpr::ParamSysTime = div_lhs.as_ref() {
                if let FxCoreExpr::ConstInt(60) = div_rhs.as_ref() {
                  if let FxCoreExpr::ConstInt(60) = rhs.as_ref() {
                    return FxCoreExpr::Derived {
                      meta: MeaningMeta::continuous(MeaningOpId::MinutesFromTime),
                      args: vec![],
                    };
                  }
                }
              }
            }
          }
        }
      }
    }

    _ => {}
  }

  // Recursive cases
  match expr {
    FxCoreExpr::Unary { meta, arg } => FxCoreExpr::Unary {
      meta,
      arg: Box::new(frp_pattern_lift(*arg)),
    },
    FxCoreExpr::Binary { meta, lhs, rhs } => FxCoreExpr::Binary {
      meta,
      lhs: Box::new(frp_pattern_lift(*lhs)),
      rhs: Box::new(frp_pattern_lift(*rhs)),
    },
    FxCoreExpr::If { cond, then_, else_ } => FxCoreExpr::If {
      cond: Box::new(frp_pattern_lift(*cond)),
      then_: Box::new(frp_pattern_lift(*then_)),
      else_: Box::new(frp_pattern_lift(*else_)),
    },
    FxCoreExpr::Derived { meta, args } => FxCoreExpr::Derived {
      meta,
      args: args.into_iter().map(frp_pattern_lift).collect(),
    },
    FxCoreExpr::List(items) => FxCoreExpr::List(items.into_iter().map(frp_pattern_lift).collect()),
    FxCoreExpr::AttrSet(pairs) => FxCoreExpr::AttrSet(
      pairs
        .into_iter()
        .map(|(k, v)| (k, frp_pattern_lift(v)))
        .collect(),
    ),
    other => other,
  }
}

/// 최적화 파이프라인 구성
///
/// 여러 최적화 패스를 순차적으로 적용합니다.
/// 실제 실행 로직은 executor에서 구현합니다.
pub struct FxOptPipeline {
  /// 적용할 최적화 패스들
  pub passes: Vec<Box<dyn FxOptPass>>,
}

impl FxOptPipeline {
  /// 새 파이프라인 생성
  pub fn new() -> Self {
    Self { passes: Vec::new() }
  }

  /// 패스 추가
  pub fn add_pass(&mut self, pass: Box<dyn FxOptPass>) {
    self.passes.push(pass);
  }

  /// 최적화 실행 (구조 변환만)
  ///
  /// **주의**: 실제 값 계산은 executor에서 수행합니다.
  pub fn optimize(&self, expr: FxCoreExpr) -> FxCoreExpr {
    self.passes.iter().fold(expr, |e, pass| pass.run(e))
  }

  pub fn optimize_with_diags(&self, expr: FxCoreExpr, diags: &mut Diagnostics) -> FxCoreExpr {
    self
      .passes
      .iter()
      .fold(expr, |e, pass| pass.run_with_diags(e, diags))
  }
}

impl Default for FxOptPipeline {
  fn default() -> Self {
    // Avoid recursion by constructing the default passes explicitly.
    let mut pipeline = FxOptPipeline::new();
    pipeline.add_pass(Box::new(FxConstFold));
    pipeline.add_pass(Box::new(FxAlgebraicSimplify));
    pipeline.add_pass(Box::new(FxFrpPatternLift));
    pipeline
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_fx_const_fold_structure() {
    let pass = FxConstFold;
    let expr = FxCoreExpr::ConstInt(42);
    let result = pass.run(expr.clone());
    // 구조만 확인 (값 계산은 executor에서)
    assert!(matches!(result, FxCoreExpr::ConstInt(_)));
  }

  #[test]
  fn test_fx_algebraic_simplify_structure() {
    let pass = FxAlgebraicSimplify;
    let expr = FxCoreExpr::ConstInt(42);
    let result = pass.run(expr.clone());
    // 구조만 확인
    assert!(matches!(result, FxCoreExpr::ConstInt(_)));
  }

  #[test]
  fn test_fx_frp_pattern_lift_structure() {
    let pass = FxFrpPatternLift;
    let expr = FxCoreExpr::ConstInt(42);
    let result = pass.run(expr.clone());
    // 구조만 확인
    assert!(matches!(result, FxCoreExpr::ConstInt(_)));
  }

  #[test]
  fn test_algebraic_simplify_mul_zero_preserves_type() {
    let pass = FxAlgebraicSimplify;

    // Int * 0 = Int(0) (타입 보존)
    let expr_int = FxCoreExpr::Binary {
      meta: MeaningMeta::pure(MeaningOpId::Mul),
      lhs: Box::new(FxCoreExpr::ConstInt(5)),
      rhs: Box::new(FxCoreExpr::ConstInt(0)),
    };
    let result_int = pass.run(expr_int);
    assert!(
      matches!(result_int, FxCoreExpr::ConstInt(0)),
      "Int * 0 should return ConstInt(0), got {:?}",
      result_int
    );

    // Float * 0 = Float(0.0) (타입 보존)
    let expr_float = FxCoreExpr::Binary {
      meta: MeaningMeta::pure(MeaningOpId::Mul),
      lhs: Box::new(FxCoreExpr::ConstFloat(2.71)),
      rhs: Box::new(FxCoreExpr::ConstFloat(0.0)),
    };
    let result_float = pass.run(expr_float);
    assert!(
      matches!(result_float, FxCoreExpr::ConstFloat(0.0)),
      "Float * 0 should return ConstFloat(0.0), got {:?}",
      result_float
    );

    // 0 * Float = Float(0.0) (타입 보존)
    let expr_zero_float = FxCoreExpr::Binary {
      meta: MeaningMeta::pure(MeaningOpId::Mul),
      lhs: Box::new(FxCoreExpr::ConstInt(0)),
      rhs: Box::new(FxCoreExpr::ConstFloat(2.5)),
    };
    let result_zero_float = pass.run(expr_zero_float);
    assert!(
      matches!(result_zero_float, FxCoreExpr::ConstFloat(0.0)),
      "0 * Float should return ConstFloat(0.0), got {:?}",
      result_zero_float
    );
  }

  #[test]
  fn test_algebraic_simplify_div_by_zero_emits_diag() {
    let pass = FxAlgebraicSimplify;
    let expr = FxCoreExpr::Binary {
      meta: MeaningMeta::pure(MeaningOpId::Div),
      lhs: Box::new(FxCoreExpr::ConstInt(4)),
      rhs: Box::new(FxCoreExpr::ConstInt(0)),
    };
    let mut diags = Diagnostics::default();
    let result = pass.run_with_diags(expr.clone(), &mut diags);

    assert!(matches!(result, FxCoreExpr::Binary { .. }));
    assert_eq!(diags.items.len(), 1);
    assert!(
      diags.items[0].message.contains("division by zero"),
      "diag should mention division by zero"
    );
  }

  #[test]
  fn test_fx_opt_pipeline() {
    let pipeline = FxOptPipeline::default();
    let expr = FxCoreExpr::ConstInt(42);
    let result = pipeline.optimize(expr.clone());
    // 구조만 확인
    assert!(matches!(result, FxCoreExpr::ConstInt(_)));
  }
}
